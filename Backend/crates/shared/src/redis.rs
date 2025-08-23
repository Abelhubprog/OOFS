use redis::{Client, aio::ConnectionManager, AsyncCommands};
use std::time::Duration;
use anyhow::Result;

#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let client = Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { manager })
    }

    /// Set a key-value pair with optional expiration
    pub async fn set<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &self,
        key: K,
        value: V,
        expiration: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self.manager.clone();
        if let Some(exp) = expiration {
            conn.set_ex(key, value, exp.as_secs()).await?;
        } else {
            conn.set(key, value).await?;
        }
        Ok(())
    }

    /// Get a value by key
    pub async fn get<K: redis::ToRedisArgs, V: redis::FromRedisValue>(
        &self,
        key: K,
    ) -> Result<Option<V>> {
        let mut conn = self.manager.clone();
        let result: Option<V> = conn.get(key).await?;
        Ok(result)
    }

    /// Increment a counter with optional expiration on first set
    pub async fn incr_with_expiry<K: redis::ToRedisArgs>(
        &self,
        key: K,
        delta: i64,
        expiration: Duration,
    ) -> Result<i64> {
        let mut conn = self.manager.clone();

        // Use Lua script for atomic incr with expiry
        let script = redis::Script::new(r"
            local key = KEYS[1]
            local delta = tonumber(ARGV[1])
            local expiry = tonumber(ARGV[2])

            local current = redis.call('GET', key)
            if current == false then
                redis.call('SET', key, delta, 'EX', expiry)
                return delta
            else
                return redis.call('INCRBY', key, delta)
            end
        ");

        let result: i64 = script
            .key(&key)
            .arg(delta)
            .arg(expiration.as_secs())
            .invoke_async(&mut conn)
            .await?;

        Ok(result)
    }

    /// Delete a key
    pub async fn del<K: redis::ToRedisArgs>(&self, key: K) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.del(key).await?;
        Ok(())
    }

    /// Check if key exists
    pub async fn exists<K: redis::ToRedisArgs>(&self, key: K) -> Result<bool> {
        let mut conn = self.manager.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// Set expiration on existing key
    pub async fn expire<K: redis::ToRedisArgs>(&self, key: K, expiration: Duration) -> Result<bool> {
        let mut conn = self.manager.clone();
        let result: bool = conn.expire(key, expiration.as_secs() as usize).await?;
        Ok(result)
    }

    /// Publish to a channel
    pub async fn publish<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &self,
        channel: K,
        message: V,
    ) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.publish(channel, message).await?;
        Ok(())
    }
}

/// Optional Redis client wrapper
#[derive(Clone)]
pub enum MaybeRedis {
    Connected(RedisClient),
    Disabled,
}

impl MaybeRedis {
    pub async fn from_url_optional(url: Option<&str>) -> Self {
        match url {
            Some(url) => match RedisClient::connect(url).await {
                Ok(client) => Self::Connected(client),
                Err(e) => {
                    tracing::warn!("Redis connection failed: {}, running without cache", e);
                    Self::Disabled
                }
            },
            None => Self::Disabled,
        }
    }

    pub async fn set<K: redis::ToRedisArgs + Send, V: redis::ToRedisArgs + Send>(
        &self,
        key: K,
        value: V,
        expiration: Option<Duration>,
    ) -> Result<()> {
        match self {
            Self::Connected(client) => client.set(key, value, expiration).await,
            Self::Disabled => Ok(()),
        }
    }

    pub async fn get<K: redis::ToRedisArgs + Send, V: redis::FromRedisValue>(
        &self,
        key: K,
    ) -> Result<Option<V>> {
        match self {
            Self::Connected(client) => client.get(key).await,
            Self::Disabled => Ok(None),
        }
    }

    pub async fn incr_with_expiry<K: redis::ToRedisArgs + Send>(
        &self,
        key: K,
        delta: i64,
        expiration: Duration,
    ) -> Result<i64> {
        match self {
            Self::Connected(client) => client.incr_with_expiry(key, delta, expiration).await,
            Self::Disabled => Ok(delta), // Return delta as if it was the first increment
        }
    }

    pub async fn publish<K: redis::ToRedisArgs + Send, V: redis::ToRedisArgs + Send>(
        &self,
        channel: K,
        message: V,
    ) -> Result<()> {
        match self {
            Self::Connected(client) => client.publish(channel, message).await,
            Self::Disabled => Ok(()),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}
