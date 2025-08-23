use crate::routes::AppState;
use axum::http::header::AUTHORIZATION;
use axum::{extract::State, http::StatusCode, response::Response};
use shared::auth::{verify_jwt_jwks, verify_jwt_secret, Claims};

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: String,
}

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthUser>() {
            Ok(user.clone())
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

pub async fn require_auth<B>(
    State(st): State<AppState>,
    mut req: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let token = match auth.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let claims: Claims = if let Some(jwks_url) = &st.cfg.dynamic_jwks_url {
        verify_jwt_jwks(token, jwks_url).await?
    } else if let Some(secret) = &st.cfg.jwt_secret {
        verify_jwt_secret(token, secret)?
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    req.extensions_mut().insert(AuthUser {
        user_id: claims.sub,
    });
    Ok(next.run(req).await)
}
