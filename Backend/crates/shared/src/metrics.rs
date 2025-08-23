use axum::{routing::get, Router};
use once_cell::sync::Lazy;
use prometheus::{Encoder, IntCounter, Opts, Registry, TextEncoder};
use std::sync::Arc;

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

#[derive(Clone)]
pub struct Metrics {
    pub ingest_count: IntCounter,
    pub detector_count: IntCounter,
    pub render_count: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let ingest_count = IntCounter::with_opts(Opts::new("ingest_total", "Total ingested tx"))
            .expect("metric");
        let detector_count = IntCounter::with_opts(Opts::new("detector_total", "Total moments detected"))
            .expect("metric");
        let render_count = IntCounter::with_opts(Opts::new("render_total", "Total cards rendered"))
            .expect("metric");
        REGISTRY.register(Box::new(ingest_count.clone())).ok();
        REGISTRY.register(Box::new(detector_count.clone())).ok();
        REGISTRY.register(Box::new(render_count.clone())).ok();
        Self { ingest_count, detector_count, render_count }
    }
}

async fn metrics_handler() -> axum::response::Response {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    let mf = REGISTRY.gather();
    encoder.encode(&mf, &mut buf).ok();
    axum::response::Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(axum::body::Body::from(buf))
        .unwrap()
}

pub fn metrics_router() -> Router {
    Router::new().route("/metrics", get(metrics_handler))
}

