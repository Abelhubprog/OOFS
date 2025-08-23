use axum::{
    extract::{ws, Path, Query, State},
    http::StatusCode,
    response::{sse, IntoResponse, Response, Sse},
    Extension,
};
use axum_extra::TypedHeader;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AuthUser, Claims},
    error::{ApiError, ApiResult},
    observability::MetricsRegistry,
    types::{
        chain::ChainEvent,
        moment::{Moment, MomentKind},
        wallet::WalletAnalysis,
    },
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{broadcast, RwLock},
    time::interval,
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::state::AppState;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Ping message for keepalive
    Ping,
    /// Pong response to ping
    Pong,
    /// Subscribe to specific channels
    Subscribe { channels: Vec<String> },
    /// Unsubscribe from channels
    Unsubscribe { channels: Vec<String> },
    /// Moment detected
    MomentDetected { moment: Moment },
    /// Analysis update
    AnalysisUpdate {
        wallet: String,
        analysis: WalletAnalysis,
    },
    /// Chain event received
    ChainEvent { event: ChainEvent },
    /// Error message
    Error {
        message: String,
        code: Option<String>,
    },
    /// Authentication required
    AuthRequired,
    /// Authentication successful
    AuthSuccess,
    /// Subscription confirmation
    Subscribed { channel: String },
    /// Unsubscription confirmation
    Unsubscribed { channel: String },
}

/// SSE event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub id: String,
    pub event: String,
    pub data: serde_json::Value,
    pub retry: Option<u64>,
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
pub struct SseQuery {
    /// Channels to subscribe to
    channels: Option<String>,
    /// Last event ID for resumption
    last_event_id: Option<String>,
}

/// Connection manager for WebSocket and SSE connections
#[derive(Debug)]
pub struct ConnectionManager {
    /// Active WebSocket connections
    ws_connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    /// Active SSE connections
    sse_connections: Arc<RwLock<HashMap<String, SseConnection>>>,
    /// Broadcast channels for different event types
    moment_tx: broadcast::Sender<Moment>,
    analysis_tx: broadcast::Sender<(String, WalletAnalysis)>,
    chain_event_tx: broadcast::Sender<ChainEvent>,
    /// Metrics registry
    metrics: Arc<MetricsRegistry>,
}

/// WebSocket connection info
#[derive(Debug)]
struct WsConnection {
    id: String,
    user_id: String,
    channels: Vec<String>,
    tx: tokio::sync::mpsc::UnboundedSender<ws::Message>,
    last_ping: std::time::Instant,
}

/// SSE connection info
#[derive(Debug)]
struct SseConnection {
    id: String,
    user_id: String,
    channels: Vec<String>,
    tx: tokio::sync::mpsc::UnboundedSender<sse::Event>,
    last_activity: std::time::Instant,
}

impl ConnectionManager {
    pub fn new(metrics: Arc<MetricsRegistry>) -> Self {
        let (moment_tx, _) = broadcast::channel(1000);
        let (analysis_tx, _) = broadcast::channel(1000);
        let (chain_event_tx, _) = broadcast::channel(1000);

        Self {
            ws_connections: Arc::new(RwLock::new(HashMap::new())),
            sse_connections: Arc::new(RwLock::new(HashMap::new())),
            moment_tx,
            analysis_tx,
            chain_event_tx,
            metrics,
        }
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(self: Arc<Self>) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                manager.cleanup_stale_connections().await;
            }
        });
    }

    /// Cleanup stale connections
    #[instrument(skip(self))]
    async fn cleanup_stale_connections(&self) {
        let now = std::time::Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes

        // Cleanup WebSocket connections
        {
            let mut connections = self.ws_connections.write().await;
            let stale_ids: Vec<String> = connections
                .iter()
                .filter(|(_, conn)| now.duration_since(conn.last_ping) > timeout)
                .map(|(id, _)| id.clone())
                .collect();

            for id in stale_ids {
                connections.remove(&id);
                debug!("Removed stale WebSocket connection: {}", id);
            }
        }

        // Cleanup SSE connections
        {
            let mut connections = self.sse_connections.write().await;
            let stale_ids: Vec<String> = connections
                .iter()
                .filter(|(_, conn)| now.duration_since(conn.last_activity) > timeout)
                .map(|(id, _)| id.clone())
                .collect();

            for id in stale_ids {
                connections.remove(&id);
                debug!("Removed stale SSE connection: {}", id);
            }
        }
    }

    /// Broadcast moment to all subscribed connections
    #[instrument(skip(self, moment))]
    pub async fn broadcast_moment(&self, moment: &Moment) -> ApiResult<()> {
        if let Err(e) = self.moment_tx.send(moment.clone()) {
            warn!("Failed to broadcast moment: {}", e);
        }

        // Send to WebSocket connections
        let connections = self.ws_connections.read().await;
        for (id, conn) in connections.iter() {
            if conn.channels.contains(&"moments".to_string())
                || conn
                    .channels
                    .contains(&format!("moments:{}", moment.moment_type))
            {
                let message = WsMessage::MomentDetected {
                    moment: moment.clone(),
                };
                if let Ok(json) = serde_json::to_string(&message) {
                    if conn.tx.send(ws::Message::Text(json)).is_err() {
                        debug!("Failed to send to WebSocket connection: {}", id);
                    }
                }
            }
        }

        // Send to SSE connections
        let sse_connections = self.sse_connections.read().await;
        for (id, conn) in sse_connections.iter() {
            if conn.channels.contains(&"moments".to_string())
                || conn
                    .channels
                    .contains(&format!("moments:{}", moment.moment_type))
            {
                let event = sse::Event::default()
                    .event("moment_detected")
                    .id(Uuid::new_v4().to_string())
                    .json_data(moment)
                    .map_err(|e| ApiError::Internal(format!("SSE serialization error: {}", e)))?;

                if conn.tx.send(event).is_err() {
                    debug!("Failed to send to SSE connection: {}", id);
                }
            }
        }

        Ok(())
    }

    /// Broadcast analysis update
    #[instrument(skip(self, analysis))]
    pub async fn broadcast_analysis(
        &self,
        wallet: &str,
        analysis: &WalletAnalysis,
    ) -> ApiResult<()> {
        if let Err(e) = self
            .analysis_tx
            .send((wallet.to_string(), analysis.clone()))
        {
            warn!("Failed to broadcast analysis: {}", e);
        }

        // Send to WebSocket connections
        let connections = self.ws_connections.read().await;
        for (id, conn) in connections.iter() {
            if conn.channels.contains(&"analysis".to_string())
                || conn.channels.contains(&format!("wallet:{}", wallet))
            {
                let message = WsMessage::AnalysisUpdate {
                    wallet: wallet.to_string(),
                    analysis: analysis.clone(),
                };
                if let Ok(json) = serde_json::to_string(&message) {
                    if conn.tx.send(ws::Message::Text(json)).is_err() {
                        debug!("Failed to send to WebSocket connection: {}", id);
                    }
                }
            }
        }

        // Send to SSE connections
        let sse_connections = self.sse_connections.read().await;
        for (id, conn) in sse_connections.iter() {
            if conn.channels.contains(&"analysis".to_string())
                || conn.channels.contains(&format!("wallet:{}", wallet))
            {
                let data = serde_json::json!({
                    "wallet": wallet,
                    "analysis": analysis
                });
                let event = sse::Event::default()
                    .event("analysis_update")
                    .id(Uuid::new_v4().to_string())
                    .json_data(&data)
                    .map_err(|e| ApiError::Internal(format!("SSE serialization error: {}", e)))?;

                if conn.tx.send(event).is_err() {
                    debug!("Failed to send to SSE connection: {}", id);
                }
            }
        }

        Ok(())
    }
}

/// WebSocket handler
#[instrument(skip(state, ws))]
pub async fn websocket_handler(
    State(state): State<AppState>,
    user: AuthUser,
    ws: ws::WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(state, user, socket))
}

/// Handle WebSocket connection
#[instrument(skip(state, socket))]
async fn handle_websocket(state: AppState, user: AuthUser, socket: ws::WebSocket) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Add connection to manager
    let connection = WsConnection {
        id: connection_id.clone(),
        user_id: user.claims.sub.clone(),
        channels: vec![],
        tx: tx.clone(),
        last_ping: std::time::Instant::now(),
    };

    // Register connection
    state
        .ws_connections
        .write()
        .await
        .insert(connection_id.clone(), connection);

    info!(
        "WebSocket connection established: {} for user: {}",
        connection_id, user.claims.sub
    );

    // Spawn task to handle outgoing messages
    let outgoing_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let connection_id_clone = connection_id.clone();
    let state_clone = state.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(ws::Message::Text(text)) => {
                    if let Err(e) =
                        handle_ws_message(&state_clone, &connection_id_clone, &text).await
                    {
                        error!("Error handling WebSocket message: {}", e);
                        let error_msg = WsMessage::Error {
                            message: e.to_string(),
                            code: None,
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let _ = tx.send(ws::Message::Text(json));
                        }
                    }
                }
                Ok(ws::Message::Ping(data)) => {
                    let _ = tx.send(ws::Message::Pong(data));
                    // Update last ping time
                    if let Some(conn) = state_clone
                        .ws_connections
                        .write()
                        .await
                        .get_mut(&connection_id_clone)
                    {
                        conn.last_ping = std::time::Instant::now();
                    }
                }
                Ok(ws::Message::Close(_)) => {
                    debug!("WebSocket connection closed: {}", connection_id_clone);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = outgoing_task => {},
        _ = incoming_task => {},
    }

    // Remove connection from manager
    state.ws_connections.write().await.remove(&connection_id);
    info!("WebSocket connection removed: {}", connection_id);
}

/// Handle WebSocket message
#[instrument(skip(state, message))]
async fn handle_ws_message(state: &AppState, connection_id: &str, message: &str) -> ApiResult<()> {
    let msg: WsMessage = serde_json::from_str(message)
        .map_err(|e| ApiError::BadRequest(format!("Invalid message format: {}", e)))?;

    match msg {
        WsMessage::Ping => {
            // Send pong response
            if let Some(conn) = state.ws_connections.read().await.get(connection_id) {
                let response = WsMessage::Pong;
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = conn.tx.send(ws::Message::Text(json));
                }
            }
        }
        WsMessage::Subscribe { channels } => {
            // Add channels to connection
            if let Some(conn) = state.ws_connections.write().await.get_mut(connection_id) {
                for channel in &channels {
                    if !conn.channels.contains(channel) {
                        conn.channels.push(channel.clone());
                    }
                    // Send confirmation
                    let response = WsMessage::Subscribed {
                        channel: channel.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&response) {
                        let _ = conn.tx.send(ws::Message::Text(json));
                    }
                }
            }
        }
        WsMessage::Unsubscribe { channels } => {
            // Remove channels from connection
            if let Some(conn) = state.ws_connections.write().await.get_mut(connection_id) {
                for channel in &channels {
                    conn.channels.retain(|c| c != channel);
                    // Send confirmation
                    let response = WsMessage::Unsubscribed {
                        channel: channel.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&response) {
                        let _ = conn.tx.send(ws::Message::Text(json));
                    }
                }
            }
        }
        _ => {
            return Err(ApiError::BadRequest("Unsupported message type".to_string()));
        }
    }

    Ok(())
}

/// SSE handler
#[instrument(skip(state))]
pub async fn sse_handler(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<SseQuery>,
) -> impl IntoResponse {
    let connection_id = Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Parse channels from query
    let channels = query
        .channels
        .map(|c| c.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|| vec!["moments".to_string()]);

    // Add connection to manager
    let connection = SseConnection {
        id: connection_id.clone(),
        user_id: user.claims.sub.clone(),
        channels: channels.clone(),
        tx: tx.clone(),
        last_activity: std::time::Instant::now(),
    };

    state
        .sse_connections
        .write()
        .await
        .insert(connection_id.clone(), connection);

    info!(
        "SSE connection established: {} for user: {} with channels: {:?}",
        connection_id, user.claims.sub, channels
    );

    // Create stream from receiver
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    // Send initial connection event
    let initial_event = sse::Event::default()
        .event("connected")
        .data(format!("Connection established: {}", connection_id));

    let _ = tx.send(initial_event);

    // Create SSE response
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

/// SSE moments endpoint - streams only moment events
#[instrument(skip(state))]
pub async fn sse_moments_handler(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
) -> impl IntoResponse {
    let connection_id = Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Add connection to manager with wallet-specific channel
    let channels = vec!["moments".to_string(), format!("wallet:{}", wallet)];
    let connection = SseConnection {
        id: connection_id.clone(),
        user_id: user.claims.sub.clone(),
        channels: channels.clone(),
        tx: tx.clone(),
        last_activity: std::time::Instant::now(),
    };

    state
        .sse_connections
        .write()
        .await
        .insert(connection_id.clone(), connection);

    info!(
        "SSE moments connection established: {} for wallet: {}",
        connection_id, wallet
    );

    // Create stream from receiver
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    // Send initial connection event
    let initial_event = sse::Event::default()
        .event("connected")
        .data(format!("Moments stream for wallet: {}", wallet));

    let _ = tx.send(initial_event);

    // Create SSE response
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}
