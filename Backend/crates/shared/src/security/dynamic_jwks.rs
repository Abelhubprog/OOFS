use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use time::{Duration, OffsetDateTime};
use anyhow::{Result, anyhow};
use reqwest::Client;

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // Subject (user ID)
    pub iss: String,           // Issuer
    pub aud: String,           // Audience
    pub exp: i64,              // Expiration time (Unix timestamp)
    pub iat: i64,              // Issued at (Unix timestamp)
    pub nbf: Option<i64>,      // Not before (Unix timestamp)
    pub jti: Option<String>,   // JWT ID
    pub email: Option<String>, // Email claim
    pub wallet: Option<String>, // Wallet address claim

    // Custom claims
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// JWKS (JSON Web Key Set) response from Dynamic
#[derive(Debug, Clone, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JsonWebKey>,
}

/// Individual JSON Web Key
#[derive(Debug, Clone, Deserialize)]
pub struct JsonWebKey {
    pub kty: String,           // Key type
    pub kid: String,           // Key ID
    pub r#use: Option<String>, // Usage
    pub alg: Option<String>,   // Algorithm
    pub n: Option<String>,     // RSA modulus
    pub e: Option<String>,     // RSA exponent
    pub x: Option<String>,     // EC x coordinate
    pub y: Option<String>,     // EC y coordinate
    pub crv: Option<String>,   // EC curve
}

/// Cached key entry
#[derive(Debug, Clone)]
struct CachedKey {
    key: DecodingKey,
    expires_at: OffsetDateTime,
}

/// JWKS client for fetching and caching keys
pub struct JwksClient {
    client: Client,
    jwks_url: String,
    keys_cache: Arc<RwLock<HashMap<String, CachedKey>>>,
    cache_duration: Duration,
}

impl JwksClient {
    /// Create a new JWKS client
    pub fn new(jwks_url: String) -> Self {
        Self {
            client: Client::new(),
            jwks_url,
            keys_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_duration: Duration::hours(1), // Cache keys for 1 hour
        }
    }

    /// Fetch JWKS from the endpoint
    async fn fetch_jwks(&self) -> Result<JwksResponse> {
        let response = self.client
            .get(&self.jwks_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?
            .error_for_status()?;

        let jwks: JwksResponse = response.json().await?;
        Ok(jwks)
    }

    /// Get decoding key by key ID
    pub async fn get_key(&self, kid: &str) -> Result<DecodingKey> {
        // Check cache first
        {
            let cache = self.keys_cache.read().await;
            if let Some(cached) = cache.get(kid) {
                if cached.expires_at > OffsetDateTime::now_utc() {
                    return Ok(cached.key.clone());
                }
            }
        }

        // Fetch fresh keys
        let jwks = self.fetch_jwks().await?;
        let mut cache = self.keys_cache.write().await;

        // Process all keys and update cache
        for jwk in jwks.keys {
            if let Ok(key) = self.jwk_to_decoding_key(&jwk) {
                let cached_key = CachedKey {
                    key: key.clone(),
                    expires_at: OffsetDateTime::now_utc() + self.cache_duration,
                };
                cache.insert(jwk.kid.clone(), cached_key);

                // Return the requested key if found
                if jwk.kid == kid {
                    return Ok(key);
                }
            }
        }

        Err(anyhow!("Key not found: {}", kid))
    }

    /// Convert JWK to DecodingKey
    fn jwk_to_decoding_key(&self, jwk: &JsonWebKey) -> Result<DecodingKey> {
        match jwk.kty.as_str() {
            "RSA" => {
                let n = jwk.n.as_ref().ok_or_else(|| anyhow!("Missing RSA modulus"))?;
                let e = jwk.e.as_ref().ok_or_else(|| anyhow!("Missing RSA exponent"))?;

                // Decode base64url values
                let n_bytes = base64::decode_config(n, base64::URL_SAFE_NO_PAD)?;
                let e_bytes = base64::decode_config(e, base64::URL_SAFE_NO_PAD)?;

                // Create RSA public key components
                let key = DecodingKey::from_rsa_components(n, e)?;
                Ok(key)
            },
            "EC" => {
                // For ECDSA keys, we need the x and y coordinates
                let x = jwk.x.as_ref().ok_or_else(|| anyhow!("Missing EC x coordinate"))?;
                let y = jwk.y.as_ref().ok_or_else(|| anyhow!("Missing EC y coordinate"))?;

                // This is a simplified implementation - in practice you'd need to
                // properly construct the EC public key from x, y coordinates
                Err(anyhow!("ECDSA keys not fully implemented"))
            },
            "oct" => {
                // For symmetric keys (HMAC)
                let k = jwk.n.as_ref().ok_or_else(|| anyhow!("Missing symmetric key"))?;
                let key_bytes = base64::decode_config(k, base64::URL_SAFE_NO_PAD)?;
                let key = DecodingKey::from_secret(&key_bytes);
                Ok(key)
            },
            _ => Err(anyhow!("Unsupported key type: {}", jwk.kty))
        }
    }

    /// Validate and decode JWT token
    pub async fn validate_token(&self, token: &str) -> Result<Claims> {
        // Decode header to get key ID
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header.kid.ok_or_else(|| anyhow!("Missing key ID in JWT header"))?;

        // Get decoding key
        let key = self.get_key(&kid).await?;

        // Set up validation
        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        // Decode and validate token
        let token_data = decode::<Claims>(token, &key, &validation)?;

        Ok(token_data.claims)
    }

    /// Clear expired keys from cache
    pub async fn cleanup_cache(&self) {
        let mut cache = self.keys_cache.write().await;
        let now = OffsetDateTime::now_utc();
        cache.retain(|_, cached| cached.expires_at > now);
    }
}

/// JWT token validator
pub struct JwtValidator {
    jwks_client: Option<JwksClient>,
    static_secret: Option<String>,
    required_issuer: Option<String>,
    required_audience: Option<String>,
}

impl JwtValidator {
    /// Create validator with Dynamic JWKS
    pub fn with_jwks(jwks_url: String) -> Self {
        Self {
            jwks_client: Some(JwksClient::new(jwks_url)),
            static_secret: None,
            required_issuer: None,
            required_audience: None,
        }
    }

    /// Create validator with static secret
    pub fn with_secret(secret: String) -> Self {
        Self {
            jwks_client: None,
            static_secret: Some(secret),
            required_issuer: None,
            required_audience: None,
        }
    }

    /// Set required issuer
    pub fn require_issuer(mut self, issuer: String) -> Self {
        self.required_issuer = Some(issuer);
        self
    }

    /// Set required audience
    pub fn require_audience(mut self, audience: String) -> Self {
        self.required_audience = Some(audience);
        self
    }

    /// Validate JWT token
    pub async fn validate(&self, token: &str) -> Result<Claims> {
        let claims = if let Some(jwks_client) = &self.jwks_client {
            // Use JWKS validation
            jwks_client.validate_token(token).await?
        } else if let Some(secret) = &self.static_secret {
            // Use static secret validation
            let key = DecodingKey::from_secret(secret.as_bytes());
            let validation = Validation::new(Algorithm::HS256);
            let token_data = decode::<Claims>(token, &key, &validation)?;
            token_data.claims
        } else {
            return Err(anyhow!("No validation method configured"));
        };

        // Additional validation
        if let Some(required_iss) = &self.required_issuer {
            if &claims.iss != required_iss {
                return Err(anyhow!("Invalid issuer: expected {}, got {}", required_iss, claims.iss));
            }
        }

        if let Some(required_aud) = &self.required_audience {
            if &claims.aud != required_aud {
                return Err(anyhow!("Invalid audience: expected {}, got {}", required_aud, claims.aud));
            }
        }

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_validation_with_secret() {
        let secret = "test_secret";
        let validator = JwtValidator::with_secret(secret.to_string());

        // This test would require creating a valid JWT token
        // In practice, you'd use a library like `jsonwebtoken` to create test tokens
    }
}
