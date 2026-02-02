use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bot_manager;
use bot_manager::BotManager;

#[derive(Clone)]
struct AppState {
    bot_manager: Arc<Mutex<BotManager>>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clawdbot_web=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create bot manager
    let bot_manager = Arc::new(Mutex::new(BotManager::new()));
    let state = AppState { bot_manager };

    // Build routes
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/bots", get(list_bots))
        .route("/api/bot/start", post(start_bot))
        .route("/api/bot/stop", post(stop_bot))
        .route("/ws", get(ws_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    tracing::info!("🚀 ClawdBot Web Terminal running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

#[derive(Serialize)]
struct BotInfo {
    name: String,
    status: String,
    description: String,
}

async fn list_bots(State(state): State<AppState>) -> Json<Vec<BotInfo>> {
    let bots = vec![
        BotInfo {
            name: "monitor".to_string(),
            status: "stopped".to_string(),
            description: "Monitors balance and rounds".to_string(),
        },
        BotInfo {
            name: "analytics".to_string(),
            status: "stopped".to_string(),
            description: "Analyzes past rounds".to_string(),
        },
        BotInfo {
            name: "miner".to_string(),
            status: "stopped".to_string(),
            description: "Mines ORE automatically".to_string(),
        },
        BotInfo {
            name: "betting".to_string(),
            status: "stopped".to_string(),
            description: "Places strategic bets".to_string(),
        },
    ];
    Json(bots)
}

#[derive(Deserialize)]
struct StartBotRequest {
    bot_name: String,
}

async fn start_bot(
    State(state): State<AppState>,
    Json(payload): Json<StartBotRequest>,
) -> Json<serde_json::Value> {
    let mut manager = state.bot_manager.lock().await;
    match manager.start_bot(&payload.bot_name).await {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "message": format!("Started {} bot", payload.bot_name)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

#[derive(Deserialize)]
struct StopBotRequest {
    bot_name: String,
}

async fn stop_bot(
    State(state): State<AppState>,
    Json(payload): Json<StopBotRequest>,
) -> Json<serde_json::Value> {
    let mut manager = state.bot_manager.lock().await;
    match manager.stop_bot(&payload.bot_name).await {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "message": format!("Stopped {} bot", payload.bot_name)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Send welcome message
    let _ = sender
        .send(axum::extract::ws::Message::Text(
            "Connected to ClawdBot Web Terminal\n".to_string(),
        ))
        .await;

    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            // Handle commands
            let response = format!("Received: {}\n", text);
            let _ = sender
                .send(axum::extract::ws::Message::Text(response))
                .await;
        }
    }
}
