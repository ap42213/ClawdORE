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
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bot_manager;
use bot_manager::BotManager;

#[derive(Clone)]
struct AppState {
    bot_manager: Arc<Mutex<BotManager>>,
    db_url: Option<String>,
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
    let db_url = std::env::var("DATABASE_URL").ok();
    let state = AppState { bot_manager, db_url };

    // CORS layer for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build routes
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/state", get(get_dashboard_state))
        .route("/api/bots", get(list_bots))
        .route("/api/bot/start", post(start_bot))
        .route("/api/bot/stop", post(stop_bot))
        .route("/ws", get(ws_handler))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/assets", ServeDir::new("../ore-dashboard/assets"))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    
    tracing::info!("🚀 ORE Dashboard running on http://localhost:{}", port);
    
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

// Dashboard state response for the Dioxus frontend
#[derive(Serialize)]
struct DashboardState {
    board: Option<BoardState>,
    last_winner: Option<WinnerInfo>,
    stats: Option<DashboardStats>,
    recent_rounds: Option<Vec<RecentRound>>,
}

#[derive(Serialize, Default)]
struct BoardState {
    round_id: u64,
    start_slot: u64,
    end_slot: u64,
    current_slot: u64,
    deployed: [u64; 25],
    time_remaining_secs: u64,
    round_duration_secs: u64,
    slots_remaining: u64,
}

#[derive(Serialize)]
struct WinnerInfo {
    round_id: u64,
    winning_square: u8,
    total_pot: u64,
    is_motherlode: bool,
    timestamp: Option<String>,
}

#[derive(Serialize, Default)]
struct DashboardStats {
    total_rounds_today: u64,
    total_sol_deployed: f64,
    avg_round_time: f64,
    motherlode_count: u64,
}

#[derive(Serialize)]
struct RecentRound {
    round_id: u64,
    winning_square: u8,
    total_pot: f64,
    is_motherlode: bool,
}

async fn get_dashboard_state(State(state): State<AppState>) -> Json<DashboardState> {
    // Try to fetch from database if available
    if let Some(ref db_url) = state.db_url {
        if let Ok(pool) = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
        {
            // Get monitor_status from bot_state table
            if let Ok(row) = sqlx::query_as::<_, (serde_json::Value,)>(
                "SELECT state_value FROM bot_state WHERE state_key = 'monitor_status'"
            )
            .fetch_one(&pool)
            .await
            {
                let status = &row.0;
                
                // Parse deployed squares
                let mut deployed = [0u64; 25];
                if let Some(deployed_arr) = status.get("deployed_squares").and_then(|v| v.as_array()) {
                    for (i, val) in deployed_arr.iter().enumerate() {
                        if i < 25 {
                            deployed[i] = val.as_u64().unwrap_or(0);
                        }
                    }
                }
                
                let board = BoardState {
                    round_id: status.get("round_id").and_then(|v| v.as_u64()).unwrap_or(0),
                    start_slot: status.get("start_slot").and_then(|v| v.as_u64()).unwrap_or(0),
                    end_slot: status.get("end_slot").and_then(|v| v.as_u64()).unwrap_or(0),
                    current_slot: status.get("current_slot").and_then(|v| v.as_u64()).unwrap_or(0),
                    deployed,
                    time_remaining_secs: status.get("time_remaining_secs").and_then(|v| v.as_u64()).unwrap_or(0),
                    round_duration_secs: status.get("round_duration_secs").and_then(|v| v.as_u64()).unwrap_or(60),
                    slots_remaining: status.get("slots_remaining").and_then(|v| v.as_u64()).unwrap_or(0),
                };
                
                // Get last winner
                let last_winner = sqlx::query_as::<_, (i64, i16, i64, bool, Option<chrono::DateTime<chrono::Utc>>)>(
                    "SELECT round_id, winning_square, total_pot, is_motherlode, timestamp 
                     FROM wins ORDER BY round_id DESC LIMIT 1"
                )
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
                .map(|(round_id, winning_square, total_pot, is_motherlode, timestamp)| {
                    WinnerInfo {
                        round_id: round_id as u64,
                        winning_square: winning_square as u8,
                        total_pot: total_pot as u64,
                        is_motherlode,
                        timestamp: timestamp.map(|t| t.to_rfc3339()),
                    }
                });
                
                // Get recent rounds
                let recent_rounds = sqlx::query_as::<_, (i64, i16, i64, bool)>(
                    "SELECT round_id, winning_square, total_pot, is_motherlode 
                     FROM wins ORDER BY round_id DESC LIMIT 10"
                )
                .fetch_all(&pool)
                .await
                .ok()
                .map(|rows| {
                    rows.into_iter()
                        .map(|(round_id, winning_square, total_pot, is_motherlode)| {
                            RecentRound {
                                round_id: round_id as u64,
                                winning_square: winning_square as u8,
                                total_pot: total_pot as f64 / 1_000_000_000.0,
                                is_motherlode,
                            }
                        })
                        .collect()
                });
                
                // Get stats
                let stats = sqlx::query_as::<_, (i64, i64, i64)>(
                    "SELECT COUNT(*), COALESCE(SUM(total_pot), 0), 
                            COUNT(*) FILTER (WHERE is_motherlode = true)
                     FROM wins WHERE timestamp > NOW() - INTERVAL '24 hours'"
                )
                .fetch_one(&pool)
                .await
                .ok()
                .map(|(rounds, total_pot, motherlodes)| {
                    DashboardStats {
                        total_rounds_today: rounds as u64,
                        total_sol_deployed: total_pot as f64 / 1_000_000_000.0,
                        avg_round_time: 55.0, // Could calculate from actual data
                        motherlode_count: motherlodes as u64,
                    }
                })
                .unwrap_or_default();
                
                return Json(DashboardState {
                    board: Some(board),
                    last_winner,
                    stats: Some(stats),
                    recent_rounds,
                });
            }
        }
    }
    
    // Return empty state if no database
    Json(DashboardState {
        board: Some(BoardState::default()),
        last_winner: None,
        stats: Some(DashboardStats::default()),
        recent_rounds: Some(vec![]),
    })
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
