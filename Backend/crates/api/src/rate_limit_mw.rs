use std::{sync::Mutex, collections::HashMap, time::{SystemTime, UNIX_EPOCH}};
use axum::{http::StatusCode, response::Response};

static RL: once_cell::sync::Lazy<Mutex<HashMap<String, (u64, u32)>>> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

fn now_minute() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 60 }

fn extract_client_ip<B>(req: &axum::http::Request<B>) -> String {
    // Prefer X-Forwarded-For, then X-Real-IP, else fallback token
    if let Some(ip) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        return ip.split(',').next().unwrap_or("unknown").trim().to_string();
    }
    if let Some(ip) = req.headers().get("x-real-ip").and_then(|h| h.to_str().ok()) {
        return ip.to_string();
    }
    "unknown".to_string()
}

pub async fn per_ip_limit<B>(req: axum::http::Request<B>, next: axum::middleware::Next<B>) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&req);
    let mut map = RL.lock().unwrap();
    let (win, cnt) = map.get(&ip).cloned().unwrap_or((now_minute(), 0));
    let limit = 60u32;
    let (new_win, mut new_cnt) = if win == now_minute() { (win, cnt) } else { (now_minute(), 0) };
    if new_cnt >= limit { return Err(StatusCode::TOO_MANY_REQUESTS); }
    new_cnt += 1;
    map.insert(ip, (new_win, new_cnt));
    Ok(next.run(req).await)
}
