use axum::{
    extract::{Json, Path, State},
    response::Json as JsonResponse,
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    utils::new_id,
};
use time::OffsetDateTime;

use crate::routes::AppState;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub description: String,
    pub budget: rust_decimal::Decimal,
    pub start_date: OffsetDateTime,
    pub end_date: OffsetDateTime,
    pub is_active: bool,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct CampaignAction {
    pub id: String,
    pub campaign_id: String,
    pub action_type: String,
    pub reward_amount: rust_decimal::Decimal,
    pub max_participants: Option<i32>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct CampaignParticipation {
    pub id: String,
    pub campaign_action_id: String,
    pub user_id: String,
    pub proof_data: String,
    pub status: String,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub description: String,
    pub budget: f64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Deserialize)]
pub struct CreateCampaignActionRequest {
    pub campaign_id: String,
    pub action_type: String,
    pub reward_amount: f64,
    pub max_participants: Option<i32>,
}

#[derive(Deserialize)]
pub struct ParticipateRequest {
    pub proof_data: String,
}

#[derive(Serialize)]
pub struct ParticipateResponse {
    pub participation_id: String,
    pub status: String,
    pub message: String,
}

pub async fn create_campaign(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateCampaignRequest>,
) -> ApiResult<JsonResponse<Campaign>> {
    if !state.cfg.admin_user_ids.contains(&user.user_id) {
        return Err(ApiError::Forbidden);
    }

    let start_date = OffsetDateTime::parse(&payload.start_date, &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::BadRequest("Invalid start_date format".to_string()))?;
    let end_date = OffsetDateTime::parse(&payload.end_date, &time::format_description::well_known::Rfc3339)
        .map_err(|_| ApiError::BadRequest("Invalid end_date format".to_string()))?;

    let campaign = Campaign {
        id: new_id(),
        name: payload.name,
        description: payload.description,
        budget: rust_decimal::Decimal::from_f64(payload.budget).ok_or_else(|| ApiError::BadRequest("Invalid budget".to_string()))?,
        start_date,
        end_date,
        is_active: true,
        created_at: Some(OffsetDateTime::now_utc()),
    };

    sqlx::query!(
        "INSERT INTO campaigns (id, name, description, budget, start_date, end_date, is_active, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        campaign.id,
        campaign.name,
        campaign.description,
        campaign.budget,
        campaign.start_date,
        campaign.end_date,
        campaign.is_active,
        campaign.created_at
    )
    .execute(&state.pg.0)
    .await?;

    Ok(JsonResponse(campaign))
}

pub async fn create_campaign_action(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateCampaignActionRequest>,
) -> ApiResult<JsonResponse<CampaignAction>> {
    if !state.cfg.admin_user_ids.contains(&user.user_id) {
        return Err(ApiError::Forbidden);
    }

    // Verify campaign exists
    let campaign_exists: bool = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM campaigns WHERE id = $1)", payload.campaign_id)
        .fetch_one(&state.pg.0)
        .await?;

    if !campaign_exists {
        return Err(ApiError::NotFound("Campaign not found".to_string()));
    }

    let campaign_action = CampaignAction {
        id: new_id(),
        campaign_id: payload.campaign_id,
        action_type: payload.action_type,
        reward_amount: rust_decimal::Decimal::from_f64(payload.reward_amount).ok_or_else(|| ApiError::BadRequest("Invalid reward amount".to_string()))?,
        max_participants: payload.max_participants,
    };

    sqlx::query!(
        "INSERT INTO campaign_actions (id, campaign_id, action_type, reward_amount, max_participants) VALUES ($1, $2, $3, $4, $5)",
        campaign_action.id,
        campaign_action.campaign_id,
        campaign_action.action_type,
        campaign_action.reward_amount,
        campaign_action.max_participants
    )
    .execute(&state.pg.0)
    .await?;

    Ok(JsonResponse(campaign_action))
}

pub async fn get_campaigns(State(state): State<AppState>) -> ApiResult<JsonResponse<Vec<Campaign>>> {
    let campaigns = sqlx::query_as::<_, Campaign>("SELECT id, name, description, budget, start_date, end_date, is_active, created_at FROM campaigns WHERE is_active = TRUE ORDER BY created_at DESC")
        .fetch_all(&state.pg.0)
        .await?;

    Ok(JsonResponse(campaigns))
}

pub async fn get_campaign_actions(
    State(state): State<AppState>,
    Path(campaign_id): Path<String>,
) -> ApiResult<JsonResponse<Vec<CampaignAction>>> {
    let actions = sqlx::query_as::<_, CampaignAction>("SELECT * FROM campaign_actions WHERE campaign_id = $1")
        .bind(&campaign_id)
        .fetch_all(&state.pg.0)
        .await?;

    Ok(JsonResponse(actions))
}

pub async fn participate_in_campaign(
    State(state): State<AppState>,
    user: AuthUser,
    Path(campaign_id): Path<String>,
    Json(payload): Json<ParticipateRequest>,
) -> ApiResult<JsonResponse<ParticipateResponse>> {
    // Get the first action for this campaign (in a real implementation, you might want to allow specifying which action)
    let action: CampaignAction = sqlx::query_as("SELECT * FROM campaign_actions WHERE campaign_id = $1 LIMIT 1")
        .bind(&campaign_id)
        .fetch_one(&state.pg.0)
        .await
        .map_err(|_| ApiError::NotFound("Campaign action not found".to_string()))?;

    // Check if user has already participated
    let already_participated: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM campaign_participations WHERE campaign_action_id = $1 AND user_id = $2)",
        action.id,
        user.user_id
    )
    .fetch_one(&state.pg.0)
    .await?;

    if already_participated {
        return Ok(JsonResponse(ParticipateResponse {
            participation_id: "".to_string(),
            status: "error".to_string(),
            message: "User has already participated in this campaign".to_string(),
        }));
    }

    // Check participant limit
    if let Some(max_participants) = action.max_participants {
        let current_participants: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM campaign_participations WHERE campaign_action_id = $1",
            action.id
        )
        .fetch_one(&state.pg.0)
        .await?;

        if current_participants >= max_participants as i64 {
            return Ok(JsonResponse(ParticipateResponse {
                participation_id: "".to_string(),
                status: "error".to_string(),
                message: "Campaign action has reached maximum participants".to_string(),
            }));
        }
    }

    let verification_status = match action.action_type.as_str() {
        "twitter_like" => verify_twitter_like(&state.cfg, &payload.proof_data).await,
        "farcaster_recast" => verify_farcaster_recast(&state.cfg, &payload.proof_data).await,
        "twitter_follow" => verify_twitter_follow(&state.cfg, &payload.proof_data).await,
        "farcaster_follow" => verify_farcaster_follow(&state.cfg, &payload.proof_data).await,
        _ => return Err(ApiError::BadRequest("Unsupported action type".to_string())),
    };

    let (status, message) = if verification_status {
        ("approved", "Participation verified and approved")
    } else {
        ("rejected", "Participation verification failed")
    };

    let participation_id = new_id();
    sqlx::query!(
        "INSERT INTO campaign_participations (id, campaign_action_id, user_id, proof_data, status, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        participation_id,
        action.id,
        user.user_id,
        payload.proof_data,
        status,
        OffsetDateTime::now_utc()
    )
    .execute(&state.pg.0)
    .await?;

    Ok(JsonResponse(ParticipateResponse {
        participation_id,
        status: status.to_string(),
        message: message.to_string(),
    }))
}

async fn verify_twitter_like(config: &shared::AppConfig, proof_data: &str) -> bool {
    // In a real implementation, this would:
    // 1. Parse the proof_data to extract tweet URL or ID
    // 2. Use Twitter API to verify the authenticated user has liked the tweet
    // 3. Return true if verification is successful
    
    // For now, we'll simulate a successful verification
    // In production, you would use the twitter-rs crate and Twitter API keys from config
    true
}

async fn verify_farcaster_recast(config: &shared::AppConfig, proof_data: &str) -> bool {
    // In a real implementation, this would:
    // 1. Parse the proof_data to extract cast URL or ID
    // 2. Use Farcaster API to verify the authenticated user has recasted the cast
    // 3. Return true if verification is successful
    
    // For now, we'll simulate a successful verification
    // In production, you would use the farcaster-rs crate and Farcaster API key from config
    true
}

async fn verify_twitter_follow(config: &shared::AppConfig, proof_data: &str) -> bool {
    // In a real implementation, this would:
    // 1. Parse the proof_data to extract Twitter username or user ID
    // 2. Use Twitter API to verify the authenticated user is following the specified account
    // 3. Return true if verification is successful
    
    // For now, we'll simulate a successful verification
    true
}

async fn verify_farcaster_follow(config: &shared::AppConfig, proof_data: &str) -> bool {
    // In a real implementation, this would:
    // 1. Parse the proof_data to extract Farcaster username or user ID
    // 2. Use Farcaster API to verify the authenticated user is following the specified account
    // 3. Return true if verification is successful
    
    // For now, we'll simulate a successful verification
    true
}