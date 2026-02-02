use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    process::Stdio,
    sync::Arc,
};
use tokio::{
    process::{Child, Command},
    sync::RwLock,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bot {
    id: String,
    name: String,
    status: String,
    uptime: u64,
}

#[derive(Clone)]
struct AppState {
    bots: Arc<RwLock<HashMap<String, BotProcess>>>,
}

struct BotProcess {
    name: String,
    child: Option<Child>,
    start_time: Option<std::time::Instant>,
}

impl AppState {
    fn new() -> Self {
        let mut bots = HashMap::new();
        
        // Initialize bot definitions
        for (id, name) in [
            ("monitor", "Monitor Bot"),
            ("analytics", "Analytics Bot"),
            ("miner", "Miner Bot"),
            ("betting", "Betting Bot"),
        ] {
            bots.insert(
                id.to_string(),
                BotProcess {
                    name: name.to_string(),
                    child: None,
                    start_time: None,
                },
            );
        }

        Self {
            bots: Arc::new(RwLock::new(bots)),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState::new();

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/bots", get(list_bots))
        .route("/api/bots/:id/start", post(start_bot))
        .route("/api/bots/:id/stop", post(stop_bot))
        .route("/api/bots/:id/status", get(bot_status))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("🚀 ClawdBot API listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn list_bots(State(state): State<AppState>) -> Json<serde_json::Value> {
    let bots = state.bots.read().await;
    
    let bot_list: Vec<Bot> = bots
        .iter()
        .map(|(id, process)| {
            let status = if process.child.is_some() {
                "running"
            } else {
                "stopped"
            };
            
            let uptime = process
                .start_time
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);

            Bot {
                id: id.clone(),
                name: process.name.clone(),
                status: status.to_string(),
                uptime,
            }
        })
        .collect();

    Json(serde_json::json!({ "bots": bot_list }))
}

async fn start_bot(
    State(state): State<AppState>,
    Path(bot_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Starting bot: {}", bot_id);
    
    let mut bots = state.bots.write().await;
    
    let bot = bots
        .get_mut(&bot_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if already running
    if bot.child.is_some() {
        return Ok(Json(serde_json::json!({
            "status": "already_running",
            "message": format!("{} is already running", bot.name)
        })));
    }

    // Spawn bot process
    let binary_name = format!("{}-bot", bot_id);
    
    match Command::new(format!("./target/release/{}", binary_name))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            bot.child = Some(child);
            bot.start_time = Some(std::time::Instant::now());
            
            info!("✅ {} started successfully", bot.name);
            
            Ok(Json(serde_json::json!({
                "status": "started",
                "message": format!("{} started successfully", bot.name)
            })))
        }
        Err(e) => {
            warn!("❌ Failed to start {}: {}", bot.name, e);
            
            Ok(Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to start {}: {}", bot.name, e)
            })))
        }
    }
}

async fn stop_bot(
    State(state): State<AppState>,
    Path(bot_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Stopping bot: {}", bot_id);
    
    let mut bots = state.bots.write().await;
    
    let bot = bots
        .get_mut(&bot_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(mut child) = bot.child.take() {
        match child.kill().await {
            Ok(_) => {
                bot.start_time = None;
                info!("✅ {} stopped successfully", bot.name);
                
                Ok(Json(serde_json::json!({
                    "status": "stopped",
                    "message": format!("{} stopped successfully", bot.name)
                })))
            }
            Err(e) => {
                warn!("❌ Failed to stop {}: {}", bot.name, e);
                
                Ok(Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to stop {}: {}", bot.name, e)
                })))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "status": "not_running",
            "message": format!("{} is not running", bot.name)
        })))
    }
}

async fn bot_status(
    State(state): State<AppState>,
    Path(bot_id): Path<String>,
) -> Result<Json<Bot>, StatusCode> {
    let bots = state.bots.read().await;
    
    let bot = bots
        .get(&bot_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let status = if bot.child.is_some() {
        "running"
    } else {
        "stopped"
    };
    
    let uptime = bot
        .start_time
        .map(|start| start.elapsed().as_secs())
        .unwrap_or(0);

    Ok(Json(Bot {
        id: bot_id,
        name: bot.name.clone(),
        status: status.to_string(),
        uptime,
    }))
}
