use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use axum::http::HeaderMap;

type HmacSha256 = Hmac<Sha256>;

/// Verify Helius webhook HMAC signature
pub fn verify_webhook_signature(secret: &str, payload: &[u8], signature: Option<&str>) -> bool {
    let signature = match signature {
        Some(sig) => sig,
        None => return false,
    };

    // Remove "sha256=" prefix if present
    let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

    // Decode the signature from hex or base64
    let expected_signature = if let Ok(bytes) = hex::decode(signature) {
        bytes
    } else if let Ok(bytes) = BASE64.decode(signature) {
        bytes
    } else {
        return false;
    };

    // Create HMAC with secret
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };

    // Update with payload
    mac.update(payload);

    // Verify signature
    mac.verify_slice(&expected_signature).is_ok()
}

/// Extract Helius signature from HTTP headers
pub fn extract_signature_from_headers(headers: &HeaderMap) -> Option<String> {
    // Try common header names
    let header_names = [
        "x-helius-signature",
        "x-signature",
        "signature",
        "authorization",
    ];

    for header_name in &header_names {
        if let Some(value) = headers.get(*header_name) {
            if let Ok(signature) = value.to_str() {
                return Some(signature.to_string());
            }
        }
    }

    None
}

/// Generate HMAC signature for outgoing webhooks
pub fn generate_signature(secret: &str, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

/// Constant-time string comparison to prevent timing attacks
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut result = 0u8;
    for i in 0..a_bytes.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_webhook_signature() {
        let secret = "test_secret";
        let payload = b"test payload";
        let signature = generate_signature(secret, payload);

        assert!(verify_webhook_signature(secret, payload, Some(&signature)));
        assert!(!verify_webhook_signature(secret, payload, Some("invalid")));
        assert!(!verify_webhook_signature(secret, payload, None));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("hello", "hello world"));
    }
}
