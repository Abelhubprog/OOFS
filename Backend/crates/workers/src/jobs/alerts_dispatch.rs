use serde::{Deserialize, Serialize};
use shared::{
    error::{ApiError, ApiResult},
    types::{
        common::UsdAmount,
        moment::{Moment, MomentSeverity},
        wallet::WalletAnalysis,
    },
};
use sqlx::PgPool;
use std::{collections::HashMap, time::Duration};
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Alert dispatch job for sending notifications about OOF moments and analysis updates
pub struct AlertsDispatchJob {
    pool: PgPool,
    redis: Option<redis::Client>,
    webhook_client: reqwest::Client,
    email_client: Option<EmailClient>,
    push_client: Option<PushClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub user_id: String,
    pub channels: Vec<AlertChannel>,
    pub min_severity: MomentSeverity,
    pub min_value_threshold: UsdAmount,
    pub rate_limit_minutes: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannel {
    Email {
        address: String,
    },
    Webhook {
        url: String,
        secret: Option<String>,
    },
    Discord {
        webhook_url: String,
    },
    Telegram {
        chat_id: String,
        bot_token: String,
    },
    Push {
        device_token: String,
        platform: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub alert_id: String,
    pub user_id: String,
    pub alert_type: AlertType,
    pub timestamp: OffsetDateTime,
    pub data: serde_json::Value,
    pub priority: AlertPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    MomentDetected,
    AnalysisComplete,
    PriceAlert,
    PortfolioUpdate,
    SystemNotification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertsDispatchJob {
    pub fn new(pool: PgPool, redis: Option<redis::Client>) -> Self {
        Self {
            pool,
            redis,
            webhook_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create webhook HTTP client"),
            email_client: None, // Would initialize email service
            push_client: None,  // Would initialize push notification service
        }
    }

    /// Process pending alerts from the queue
    #[instrument(skip(self))]
    pub async fn process_pending_alerts(&self) -> ApiResult<u32> {
        info!("Processing pending alerts");

        let mut processed = 0;

        // Fetch pending alerts from database
        let pending_alerts = self.fetch_pending_alerts().await?;

        for alert in pending_alerts {
            match self.dispatch_alert(&alert).await {
                Ok(_) => {
                    self.mark_alert_processed(&alert.alert_id).await?;
                    processed += 1;
                    info!("Successfully dispatched alert: {}", alert.alert_id);
                }
                Err(e) => {
                    error!("Failed to dispatch alert {}: {}", alert.alert_id, e);
                    self.mark_alert_failed(&alert.alert_id, &e.to_string())
                        .await?;
                }
            }

            // Rate limiting between dispatches
            sleep(Duration::from_millis(100)).await;
        }

        info!("Processed {} alerts", processed);
        Ok(processed)
    }

    /// Create alert for new OOF moment
    #[instrument(skip(self, moment))]
    pub async fn create_moment_alert(
        &self,
        moment: &Moment,
        user_configs: &[AlertConfig],
    ) -> ApiResult<()> {
        info!("Creating moment alert for moment: {}", moment.id);

        for config in user_configs {
            // Check if user should receive this alert
            if !self.should_send_alert(config, moment).await? {
                continue;
            }

            // Check rate limiting
            if self
                .is_rate_limited(&config.user_id, AlertType::MomentDetected)
                .await?
            {
                debug!("Rate limited alert for user: {}", config.user_id);
                continue;
            }

            let alert = AlertPayload {
                alert_id: Uuid::new_v4().to_string(),
                user_id: config.user_id.clone(),
                alert_type: AlertType::MomentDetected,
                timestamp: OffsetDateTime::now_utc(),
                data: serde_json::to_value(moment)?,
                priority: self.determine_priority(&moment.severity),
            };

            self.queue_alert(&alert).await?;
        }

        Ok(())
    }

    /// Create alert for completed analysis
    #[instrument(skip(self, analysis))]
    pub async fn create_analysis_alert(
        &self,
        analysis: &WalletAnalysis,
        user_id: &str,
    ) -> ApiResult<()> {
        info!("Creating analysis alert for user: {}", user_id);

        let alert = AlertPayload {
            alert_id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            alert_type: AlertType::AnalysisComplete,
            timestamp: OffsetDateTime::now_utc(),
            data: serde_json::to_value(analysis)?,
            priority: AlertPriority::Medium,
        };

        self.queue_alert(&alert).await?;
        Ok(())
    }

    /// Dispatch alert to configured channels
    #[instrument(skip(self, alert))]
    async fn dispatch_alert(&self, alert: &AlertPayload) -> ApiResult<()> {
        debug!(
            "Dispatching alert: {} to user: {}",
            alert.alert_id, alert.user_id
        );

        // Get user's alert configuration
        let config = self
            .get_user_alert_config(&alert.user_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("User alert config not found".to_string()))?;

        if !config.enabled {
            debug!("Alerts disabled for user: {}", alert.user_id);
            return Ok(());
        }

        // Send to all configured channels
        let mut success_count = 0;
        let mut error_count = 0;

        for channel in &config.channels {
            match self.send_to_channel(alert, channel).await {
                Ok(_) => {
                    success_count += 1;
                    debug!("Successfully sent alert to channel: {:?}", channel);
                }
                Err(e) => {
                    error_count += 1;
                    warn!("Failed to send alert to channel {:?}: {}", channel, e);
                }
            }
        }

        if success_count == 0 && error_count > 0 {
            return Err(ApiError::Internal(
                "Failed to send alert to any channel".to_string(),
            ));
        }

        // Update rate limiting
        self.update_rate_limit(&alert.user_id, &alert.alert_type)
            .await?;

        Ok(())
    }

    /// Send alert to specific channel
    #[instrument(skip(self, alert))]
    async fn send_to_channel(&self, alert: &AlertPayload, channel: &AlertChannel) -> ApiResult<()> {
        match channel {
            AlertChannel::Email { address } => self.send_email_alert(alert, address).await,
            AlertChannel::Webhook { url, secret } => {
                self.send_webhook_alert(alert, url, secret.as_deref()).await
            }
            AlertChannel::Discord { webhook_url } => {
                self.send_discord_alert(alert, webhook_url).await
            }
            AlertChannel::Telegram { chat_id, bot_token } => {
                self.send_telegram_alert(alert, chat_id, bot_token).await
            }
            AlertChannel::Push {
                device_token,
                platform,
            } => self.send_push_alert(alert, device_token, platform).await,
        }
    }

    /// Send webhook alert
    #[instrument(skip(self, alert))]
    async fn send_webhook_alert(
        &self,
        alert: &AlertPayload,
        url: &str,
        secret: Option<&str>,
    ) -> ApiResult<()> {
        debug!("Sending webhook alert to: {}", url);

        let mut request = self
            .webhook_client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "OOF-Alerts/1.0");

        // Add HMAC signature if secret is provided
        if let Some(secret) = secret {
            let payload_json = serde_json::to_string(alert)?;
            let signature = self.generate_hmac_signature(&payload_json, secret)?;
            request = request.header("X-OOF-Signature", signature);
        }

        let response = request
            .json(alert)
            .send()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Webhook".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(ApiError::ExternalService {
                service: "Webhook".to_string(),
                error: format!("HTTP {}", response.status()),
            });
        }

        Ok(())
    }

    /// Send Discord alert
    #[instrument(skip(self, alert))]
    async fn send_discord_alert(&self, alert: &AlertPayload, webhook_url: &str) -> ApiResult<()> {
        debug!("Sending Discord alert");

        let embed = self.create_discord_embed(alert)?;

        let payload = serde_json::json!({
            "embeds": [embed]
        });

        let response = self
            .webhook_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Discord".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(ApiError::ExternalService {
                service: "Discord".to_string(),
                error: format!("HTTP {}", response.status()),
            });
        }

        Ok(())
    }

    // Helper methods

    async fn fetch_pending_alerts(&self) -> ApiResult<Vec<AlertPayload>> {
        let alerts = sqlx::query!(
            "SELECT alert_id, user_id, alert_type, timestamp, data, priority FROM alert_queue WHERE status = 'pending' ORDER BY timestamp LIMIT 100"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch pending alerts: {}", e)))?;

        let mut alert_payloads = Vec::new();

        for alert in alerts {
            let alert_payload = AlertPayload {
                alert_id: alert.alert_id,
                user_id: alert.user_id,
                alert_type: serde_json::from_str(&alert.alert_type)?,
                timestamp: alert.timestamp.assume_utc(),
                data: serde_json::from_str(&alert.data)?,
                priority: serde_json::from_str(&alert.priority)?,
            };
            alert_payloads.push(alert_payload);
        }

        Ok(alert_payloads)
    }

    async fn queue_alert(&self, alert: &AlertPayload) -> ApiResult<()> {
        sqlx::query!(
            "INSERT INTO alert_queue (alert_id, user_id, alert_type, timestamp, data, priority, status) VALUES ($1, $2, $3, $4, $5, $6, 'pending')",
            alert.alert_id,
            alert.user_id,
            serde_json::to_string(&alert.alert_type)?,
            alert.timestamp,
            serde_json::to_string(&alert.data)?,
            serde_json::to_string(&alert.priority)?
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to queue alert: {}", e)))?;

        Ok(())
    }

    async fn mark_alert_processed(&self, alert_id: &str) -> ApiResult<()> {
        sqlx::query!(
            "UPDATE alert_queue SET status = 'sent', processed_at = $1 WHERE alert_id = $2",
            OffsetDateTime::now_utc(),
            alert_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to mark alert processed: {}", e)))?;

        Ok(())
    }

    async fn mark_alert_failed(&self, alert_id: &str, error: &str) -> ApiResult<()> {
        sqlx::query!(
            "UPDATE alert_queue SET status = 'failed', error_message = $1, processed_at = $2 WHERE alert_id = $3",
            error,
            OffsetDateTime::now_utc(),
            alert_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to mark alert failed: {}", e)))?;

        Ok(())
    }

    async fn should_send_alert(&self, config: &AlertConfig, moment: &Moment) -> ApiResult<bool> {
        // Check severity threshold
        if self.severity_to_int(&moment.severity) < self.severity_to_int(&config.min_severity) {
            return Ok(false);
        }

        // Check value threshold
        if moment.value_lost_usd.0 < config.min_value_threshold.0 {
            return Ok(false);
        }

        Ok(true)
    }

    async fn is_rate_limited(&self, user_id: &str, alert_type: AlertType) -> ApiResult<bool> {
        // Check if user has been sent this type of alert recently
        let recent_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM alert_queue WHERE user_id = $1 AND alert_type = $2 AND timestamp > $3",
            user_id,
            serde_json::to_string(&alert_type)?,
            OffsetDateTime::now_utc() - time::Duration::minutes(15)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check rate limit: {}", e)))?;

        Ok(recent_count.unwrap_or(0) >= 5) // Max 5 alerts per 15 minutes
    }

    async fn get_user_alert_config(&self, user_id: &str) -> ApiResult<Option<AlertConfig>> {
        let config = sqlx::query!(
            "SELECT config_json FROM user_alert_configs WHERE user_id = $1 AND enabled = true",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch user alert config: {}", e)))?;

        match config {
            Some(row) => {
                let config: AlertConfig = serde_json::from_str(&row.config_json)?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    async fn update_rate_limit(&self, user_id: &str, alert_type: &AlertType) -> ApiResult<()> {
        // Rate limiting is handled by the database query in is_rate_limited
        Ok(())
    }

    fn determine_priority(&self, severity: &MomentSeverity) -> AlertPriority {
        match severity {
            MomentSeverity::Low => AlertPriority::Low,
            MomentSeverity::Medium => AlertPriority::Medium,
            MomentSeverity::High => AlertPriority::High,
            MomentSeverity::Critical => AlertPriority::Critical,
        }
    }

    fn severity_to_int(&self, severity: &MomentSeverity) -> u8 {
        match severity {
            MomentSeverity::Low => 1,
            MomentSeverity::Medium => 2,
            MomentSeverity::High => 3,
            MomentSeverity::Critical => 4,
        }
    }

    fn generate_hmac_signature(&self, payload: &str, secret: &str) -> ApiResult<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| ApiError::Internal(format!("HMAC error: {}", e)))?;

        mac.update(payload.as_bytes());
        let result = mac.finalize();

        Ok(hex::encode(result.into_bytes()))
    }

    fn create_discord_embed(&self, alert: &AlertPayload) -> ApiResult<serde_json::Value> {
        let color = match alert.priority {
            AlertPriority::Low => 0x00ff00,      // Green
            AlertPriority::Medium => 0xffff00,   // Yellow
            AlertPriority::High => 0xff8000,     // Orange
            AlertPriority::Critical => 0xff0000, // Red
        };

        Ok(serde_json::json!({
            "title": format!("{:?} Alert", alert.alert_type),
            "description": "You have a new OOF notification",
            "color": color,
            "timestamp": alert.timestamp.format(&time::format_description::well_known::Rfc3339).unwrap(),
            "fields": [
                {
                    "name": "Alert ID",
                    "value": alert.alert_id,
                    "inline": true
                },
                {
                    "name": "Priority",
                    "value": format!("{:?}", alert.priority),
                    "inline": true
                }
            ]
        }))
    }

    // Placeholder implementations for other channels
    async fn send_email_alert(&self, _alert: &AlertPayload, _address: &str) -> ApiResult<()> {
        // Would integrate with email service (SendGrid, SES, etc.)
        debug!("Email alert functionality not implemented");
        Ok(())
    }

    async fn send_telegram_alert(
        &self,
        _alert: &AlertPayload,
        _chat_id: &str,
        _bot_token: &str,
    ) -> ApiResult<()> {
        // Would integrate with Telegram Bot API
        debug!("Telegram alert functionality not implemented");
        Ok(())
    }

    async fn send_push_alert(
        &self,
        _alert: &AlertPayload,
        _device_token: &str,
        _platform: &str,
    ) -> ApiResult<()> {
        // Would integrate with FCM/APNS
        debug!("Push alert functionality not implemented");
        Ok(())
    }
}

// Supporting types for email and push services
struct EmailClient;
struct PushClient;
