use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{OffsetDateTime, Date};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub code: String,
    pub price_usd: String,
    pub daily_wallets: i32,
    pub backfill_days: i32,
    pub cadence: String,
    pub alerts: i32,
    pub api_rows: i64,
}

#[derive(Debug, Clone)]
pub struct UserContext { pub user_id: String, pub plan_code: String }

pub struct PolicyService<'a> { pub pg: &'a PgPool }

impl<'a> PolicyService<'a> {
    pub fn new(pg: &'a PgPool) -> Self { Self { pg } }

    pub async fn get_user_plan(&self, user_id: &str) -> anyhow::Result<Plan> {
        if let Some(rec) = sqlx::query!("SELECT p.code, p.price_usd_dec, p.daily_wallets, p.backfill_days, p.cadence, p.alerts, p.api_rows FROM user_plans up JOIN plans p ON p.code=up.plan_code WHERE up.user_id=$1 AND (up.expires_at IS NULL OR up.expires_at>NOW()) ORDER BY up.started_at DESC LIMIT 1", user_id)
            .fetch_optional(self.pg).await? {
            return Ok(Plan { code: rec.code, price_usd: rec.price_usd_dec.to_string(), daily_wallets: rec.daily_wallets, backfill_days: rec.backfill_days, cadence: rec.cadence.unwrap_or_default(), alerts: rec.alerts.unwrap_or(0), api_rows: rec.api_rows.unwrap_or(0) });
        }
        // default FREE
        if let Some(rec) = sqlx::query!("SELECT code, price_usd_dec, daily_wallets, backfill_days, cadence, alerts, api_rows FROM plans WHERE code='FREE'")
            .fetch_optional(self.pg).await? {
            return Ok(Plan { code: rec.code.unwrap_or("FREE".into()), price_usd: rec.price_usd_dec.unwrap_or_default().to_string(), daily_wallets: rec.daily_wallets.unwrap_or(2), backfill_days: rec.backfill_days.unwrap_or(180), cadence: rec.cadence.unwrap_or_default(), alerts: rec.alerts.unwrap_or(0), api_rows: rec.api_rows.unwrap_or(0) });
        }
        Ok(Plan { code: "FREE".into(), price_usd: "0.00".into(), daily_wallets: 2, backfill_days: 180, cadence: "manual".into(), alerts: 0, api_rows: 0 })
    }

    pub async fn check_and_consume_quota(&self, user_id: &str, wallets: i32) -> anyhow::Result<bool> {
        let mut tx = self.pg.begin().await?;
        // Reset daily counter if day changed
        let st = sqlx::query!("SELECT analyses_today, last_reset_at FROM policy_state WHERE user_id=$1", user_id)
            .fetch_optional(&mut *tx).await?;
        let today = OffsetDateTime::now_utc().date();
        let mut analyses_today = st.as_ref().and_then(|r| r.analyses_today).unwrap_or(0);
        let last_date = st.as_ref().and_then(|r| r.last_reset_at).map(|t| t.date());
        if last_date != Some(today) {
            analyses_today = 0;
            sqlx::query!("INSERT INTO policy_state (user_id, analyses_today, last_reset_at) VALUES ($1,$2,NOW()) ON CONFLICT (user_id) DO UPDATE SET analyses_today=$2, last_reset_at=NOW()", user_id, analyses_today)
                .execute(&mut *tx).await?;
        }
        let plan = self.get_user_plan(user_id).await?;
        if analyses_today + wallets > plan.daily_wallets { tx.rollback().await.ok(); return Ok(false); }
        analyses_today += wallets;
        sqlx::query!("INSERT INTO policy_state (user_id, analyses_today, last_reset_at) VALUES ($1,$2,NOW()) ON CONFLICT (user_id) DO UPDATE SET analyses_today=$2", user_id, analyses_today)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(true)
    }
}

