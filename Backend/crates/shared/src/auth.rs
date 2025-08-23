use axum::http::{HeaderMap, StatusCode};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub fn verify_helius_hmac(secret: &str, body: &[u8], sig_header: Option<&str>) -> bool {
    // Helius sends header `x-helius-signature: sha256=<base64>`
    let header = match sig_header {
        Some(s) => s,
        None => return false,
    };
    let parts: Vec<_> = header.split('=').collect();
    if parts.len() != 2 || parts[0] != "sha256" {
        return false;
    }
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    match base64::decode(parts[1]) {
        Ok(sig) => sig.as_slice() == expected.as_slice(),
        Err(_) => false,
    }
}

pub fn get_helius_sig_from_headers(headers: &HeaderMap) -> Option<String> {
    for k in ["x-helius-signature", "x-hel-sig", "x-hel"] {
        if let Some(v) = headers.get(k).and_then(|h| h.to_str().ok()) {
            return Some(v.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hmac_roundtrip() {
        let secret = "abc";
        let body = b"{\"hello\":true}";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = base64::encode(mac.finalize().into_bytes());
        let header = format!("sha256={}", sig);
        assert!(verify_helius_hmac(secret, body, Some(&header)));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,                  // User ID from Dynamic.xyz
    pub exp: i64,                     // Token expiration timestamp
    pub iat: i64,                     // Issued at timestamp
    pub iss: String,                  // Issuer (Dynamic.xyz)
    pub aud: Vec<String>,             // Audience
    pub email: Option<String>,        // User email if available
    pub email_verified: Option<bool>, // Email verification status

    // Dynamic.xyz specific fields
    pub environment_id: Option<String>, // Dynamic environment ID
    pub user_id: Option<String>,        // Dynamic user ID
    pub wallet_public_key: Option<String>, // Connected wallet public key
    pub wallet_name: Option<String>,    // Wallet name (e.g., "phantom", "metamask")
    pub auth_provider: Option<String>,  // Auth method ("wallet", "email", "social")
    pub social_provider: Option<String>, // Social provider if applicable

    // App-specific fields
    pub roles: Vec<String>, // User roles ("user", "premium", "admin")
    pub permissions: Option<Vec<String>>, // Specific permissions
    pub subscription_tier: Option<String>, // Subscription level
    pub rate_limit_tier: Option<String>, // Rate limiting tier
}

/// Verify Dynamic.xyz JWT token using JWKS endpoint
pub async fn verify_dynamic_jwt(
    token: &str,
    jwks_url: &str,
    environment_id: &str,
) -> Result<Claims, StatusCode> {
    let cache = JWKS.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().unwrap();
    let need_fetch = guard
        .as_ref()
        .map(|c| c.fetched_at.elapsed() > Duration::from_secs(300))
        .unwrap_or(true);

    if need_fetch {
        // Format Dynamic.xyz JWKS URL with environment ID
        let formatted_url = jwks_url.replace("{ENVIRONMENT_ID}", environment_id);
        let jwks: Jwks = reqwest::get(&formatted_url)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .json()
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        *guard = Some(JwksCache {
            fetched_at: Instant::now(),
            jwks,
        });
    }

    let jwks = guard.as_ref().unwrap().jwks.clone();
    drop(guard);

    let header = jsonwebtoken::decode_header(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let kid = header.kid.ok_or(StatusCode::UNAUTHORIZED)?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|k| k.kid.as_deref() == Some(&kid))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let key = if jwk.kty == "RSA" {
        let n = jwk.n.ok_or(StatusCode::UNAUTHORIZED)?;
        let e = jwk.e.ok_or(StatusCode::UNAUTHORIZED)?;
        DecodingKey::from_rsa_components(&n, &e).map_err(|_| StatusCode::UNAUTHORIZED)?
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true;
    validation.set_audience(&[environment_id]); // Validate audience matches environment

    decode::<Claims>(token, &key, &validation)
        .map(|d| d.claims)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Enhanced JWT service for Dynamic.xyz integration
#[derive(Clone)]
pub struct JwtService {
    dynamic_environment_id: String,
    dynamic_jwks_url: String,
    app_secret: String,
}

impl JwtService {
    pub fn new(
        dynamic_environment_id: &str,
        dynamic_jwks_url: &str,
        app_secret: &str,
    ) -> Result<Self, &'static str> {
        if dynamic_environment_id.is_empty() || dynamic_jwks_url.is_empty() || app_secret.is_empty()
        {
            return Err("Missing required JWT configuration");
        }

        Ok(Self {
            dynamic_environment_id: dynamic_environment_id.to_string(),
            dynamic_jwks_url: dynamic_jwks_url.to_string(),
            app_secret: app_secret.to_string(),
        })
    }

    /// Validate Dynamic.xyz JWT token
    pub async fn validate_token(&self, token: &str) -> Result<Claims, String> {
        verify_dynamic_jwt(token, &self.dynamic_jwks_url, &self.dynamic_environment_id)
            .await
            .map_err(|_| "Invalid or expired token".to_string())
    }

    /// Generate app-specific JWT for internal use
    pub fn generate_internal_token(
        &self,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<String, String> {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "System time error")?;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: now.as_secs() as i64 + 3600, // 1 hour expiry
            iat: now.as_secs() as i64,
            iss: "oof-backend".to_string(),
            aud: vec!["oof-api".to_string()],
            email: None,
            email_verified: None,
            environment_id: Some(self.dynamic_environment_id.clone()),
            user_id: Some(user_id.to_string()),
            wallet_public_key: None,
            wallet_name: None,
            auth_provider: Some("internal".to_string()),
            social_provider: None,
            roles,
            permissions: None,
            subscription_tier: None,
            rate_limit_tier: None,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.app_secret.as_bytes()),
        )
        .map_err(|_| "Failed to generate token".to_string())
    }
}

/// Authenticated user information
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: Claims,
    pub permissions: Vec<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kty: String,
    kid: Option<String>,
    n: Option<String>,
    e: Option<String>,
    crv: Option<String>,
    x: Option<String>,
    y: Option<String>,
    k: Option<String>,
    alg: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

struct JwksCache {
    fetched_at: Instant,
    jwks: Jwks,
}
static JWKS: OnceCell<Mutex<Option<JwksCache>>> = OnceCell::new();

pub async fn verify_jwt_jwks(token: &str, jwks_url: &str) -> Result<Claims, StatusCode> {
    let cache = JWKS.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().unwrap();
    let need_fetch = guard
        .as_ref()
        .map(|c| c.fetched_at.elapsed() > Duration::from_secs(300))
        .unwrap_or(true);
    if need_fetch {
        let jwks: Jwks = reqwest::get(jwks_url)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?
            .json()
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        *guard = Some(JwksCache {
            fetched_at: Instant::now(),
            jwks,
        });
    }
    let jwks = guard.as_ref().unwrap().jwks.clone();
    let header = jsonwebtoken::decode_header(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let kid = header.kid.ok_or(StatusCode::UNAUTHORIZED)?;
    let jwk = jwks
        .keys
        .into_iter()
        .find(|k| k.kid.as_deref() == Some(&kid))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let key = if jwk.kty == "RSA" {
        let n = jwk.n.ok_or(StatusCode::UNAUTHORIZED)?;
        let e = jwk.e.ok_or(StatusCode::UNAUTHORIZED)?;
        DecodingKey::from_rsa_components(&n, &e).map_err(|_| StatusCode::UNAUTHORIZED)?
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let mut validation = Validation::new(header.alg);
    validation.validate_exp = true;
    decode::<Claims>(token, &key, &validation)
        .map(|d| d.claims)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}
