use rust_decimal::Decimal;
use time::{OffsetDateTime, Duration};
use ulid::Ulid;
use anyhow::Result;

/// Generate a new ULID as string
pub fn new_id() -> String {
    Ulid::new().to_string()
}

/// Generate a request ID for tracing
pub fn new_request_id() -> String {
    format!("req_{}", Ulid::new().to_string().to_lowercase())
}

/// Round timestamp to nearest bucket interval
pub fn round_to_bucket(timestamp: OffsetDateTime, bucket_duration: Duration) -> OffsetDateTime {
    let unix_timestamp = timestamp.unix_timestamp();
    let bucket_seconds = bucket_duration.whole_seconds();
    let rounded_timestamp = (unix_timestamp / bucket_seconds) * bucket_seconds;
    OffsetDateTime::from_unix_timestamp(rounded_timestamp).unwrap_or(timestamp)
}

/// Calculate percentage change between two decimals
pub fn percentage_change(from: Decimal, to: Decimal) -> Decimal {
    if from == Decimal::ZERO {
        return Decimal::ZERO;
    }
    ((to - from) / from) * Decimal::from(100)
}

/// Format decimal as USD string
pub fn format_usd(amount: Decimal) -> String {
    format!("${:.2}", amount)
}

/// Format decimal as percentage string
pub fn format_percentage(pct: Decimal) -> String {
    format!("{:.2}%", pct * Decimal::from(100))
}

/// Clamp decimal between min and max values
pub fn clamp_decimal(value: Decimal, min: Decimal, max: Decimal) -> Decimal {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Parse decimal from string with fallback
pub fn parse_decimal_safe(s: &str) -> Option<Decimal> {
    s.parse::<Decimal>().ok()
        .or_else(|| s.parse::<f64>().ok().and_then(|f| Decimal::try_from(f).ok()))
}

/// Normalize Solana address (handle SOL mint special case)
pub fn normalize_mint_address(mint: &str) -> String {
    match mint {
        "11111111111111111111111111111111" => crate::constants::solana::SOL_MINT.to_string(),
        _ => mint.to_string(),
    }
}

/// Validate and normalize wallet address
pub fn normalize_wallet_address(addr: &str) -> Result<String> {
    crate::validation::validate_wallet_address(addr)?;
    Ok(addr.trim().to_string())
}

/// Extract wallet address from various formats
pub fn extract_wallet_from_url_or_address(input: &str) -> Result<String> {
    // Handle URLs like solscan.io/address/... or solanaexplorer.com/address/...
    if input.contains("://") {
        let url = url::Url::parse(input)?;
        let path_segments: Vec<&str> = url.path().split('/').collect();

        // Look for address after "address" or "account" segment
        for (i, segment) in path_segments.iter().enumerate() {
            if matches!(*segment, "address" | "account" | "wallet") && i + 1 < path_segments.len() {
                return normalize_wallet_address(path_segments[i + 1]);
            }
        }

        // If no pattern found, try the last segment
        if let Some(last) = path_segments.last() {
            if !last.is_empty() {
                return normalize_wallet_address(last);
            }
        }

        return Err(anyhow::anyhow!("Could not extract wallet address from URL"));
    }

    // Direct address
    normalize_wallet_address(input)
}

/// Truncate wallet address for display
pub fn truncate_wallet(addr: &str, start_chars: usize, end_chars: usize) -> String {
    if addr.len() <= start_chars + end_chars + 3 {
        return addr.to_string();
    }

    format!(
        "{}...{}",
        &addr[..start_chars],
        &addr[addr.len() - end_chars..]
    )
}

/// Calculate time until next reset (daily)
pub fn time_until_daily_reset() -> Duration {
    let now = OffsetDateTime::now_utc();
    let tomorrow = now.date().next_day().unwrap().midnight().assume_utc();
    tomorrow - now
}

/// Calculate severity color for UI
pub fn severity_to_color(severity: Decimal) -> &'static str {
    if severity >= Decimal::from_str_exact("0.8").unwrap() {
        "#dc2626" // red-600
    } else if severity >= Decimal::from_str_exact("0.6").unwrap() {
        "#ea580c" // orange-600
    } else if severity >= Decimal::from_str_exact("0.4").unwrap() {
        "#ca8a04" // yellow-600
    } else if severity >= Decimal::from_str_exact("0.2").unwrap() {
        "#16a34a" // green-600
    } else {
        "#6b7280" // gray-500
    }
}

/// Safe division that returns zero if denominator is zero
pub fn safe_divide(numerator: Decimal, denominator: Decimal) -> Decimal {
    if denominator == Decimal::ZERO {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

/// Convert duration to human readable string
pub fn duration_to_human(duration: Duration) -> String {
    let total_seconds = duration.whole_seconds().abs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if days > 0 {
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if hours > 0 {
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

/// Compress JSON data
pub fn compress_json(data: &serde_json::Value) -> Result<Vec<u8>> {
    let json_bytes = serde_json::to_vec(data)?;
    let compressed = zstd::encode_all(&json_bytes[..], 3)?;
    Ok(compressed)
}

/// Decompress JSON data
pub fn decompress_json(compressed: &[u8]) -> Result<serde_json::Value> {
    let decompressed = zstd::decode_all(compressed)?;
    let value = serde_json::from_slice(&decompressed)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_change() {
        let from = Decimal::from(100);
        let to = Decimal::from(150);
        let change = percentage_change(from, to);
        assert_eq!(change, Decimal::from(50));
    }

    #[test]
    fn test_truncate_wallet() {
        let addr = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let truncated = truncate_wallet(addr, 4, 4);
        assert_eq!(truncated, "9WzD...AWWM");
    }

    #[test]
    fn test_extract_wallet_from_url() {
        let url = "https://solscan.io/address/9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let wallet = extract_wallet_from_url_or_address(url).unwrap();
        assert_eq!(wallet, "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");
    }
}
