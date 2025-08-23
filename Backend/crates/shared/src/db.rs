use sqlx::{Pool, Postgres};
use std::time::Duration;

#[derive(Clone)]
pub struct Pg(pub Pool<Postgres>);

impl Pg {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await?;
        Ok(Self(pool))
    }
}

