use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use clawdbot::ore_stats::OreStatsService;
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
use tracing::{info, warn, error};

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
    ore_stats: Arc<RwLock<Option<OreStatsService>>>,
    rpc_url: String,
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
        
        // Get RPC URL from environment or use default
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        Self {
            bots: Arc::new(RwLock::new(bots)),
            ore_stats: Arc::new(RwLock::new(None)),
            rpc_url,
        }
    }
    
    async fn get_ore_stats(&self) -> Result<OreStatsService, String> {
        // Lazy initialization of OreStatsService
        {
            let stats = self.ore_stats.read().await;
            if stats.is_some() {
                // Clone not possible, so we recreate
            }
        }
        
        OreStatsService::new(&self.rpc_url)
            .map_err(|e| format!("Failed to create OreStatsService: {}", e))
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
        // ORE Stats endpoints
        .route("/api/ore/live", get(ore_live_round))
        .route("/api/ore/stats", get(ore_full_stats))
        .route("/api/ore/protocol", get(ore_protocol_stats))
        .route("/api/ore/history", get(ore_round_history))
        .route("/api/ore/squares", get(ore_square_analysis))
        .route("/api/ore/recommendations", get(ore_recommendations))
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

// ═══════════════════════════════════════════════════════════════════════════════
// ORE STATS ENDPOINTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Get live round data (5x5 grid, deployments, miners, timing)
async fn ore_live_round(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.get_live_round() {
                Ok(live) => Ok(Json(serde_json::json!(live))),
                Err(e) => {
                    error!("Failed to get live round: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to fetch live round: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get comprehensive ORE stats (live + protocol + history)
async fn ore_full_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.get_full_stats() {
                Ok(full) => Ok(Json(serde_json::json!(full))),
                Err(e) => {
                    error!("Failed to get full stats: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to fetch stats: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get protocol-wide stats (treasury, motherlode, staking)
async fn ore_protocol_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.get_protocol_stats() {
                Ok(protocol) => Ok(Json(serde_json::json!(protocol))),
                Err(e) => {
                    error!("Failed to get protocol stats: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to fetch protocol stats: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get round history (last 20 completed rounds)
async fn ore_round_history(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.get_round_history(20) {
                Ok(history) => Ok(Json(serde_json::json!({
                    "rounds": history,
                    "count": history.len()
                }))),
                Err(e) => {
                    error!("Failed to get round history: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to fetch history: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get square analysis (win rates, patterns)
async fn ore_square_analysis(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.analyze_squares(100) {
                Ok(analysis) => Ok(Json(serde_json::json!({
                    "squares": analysis
                }))),
                Err(e) => {
                    error!("Failed to analyze squares: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to analyze squares: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get bot recommendations (which squares to deploy on)
async fn ore_recommendations(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.get_ore_stats().await {
        Ok(stats) => {
            match stats.get_bot_recommendations() {
                Ok(recs) => Ok(Json(serde_json::json!(recs))),
                Err(e) => {
                    error!("Failed to get recommendations: {}", e);
                    Ok(Json(serde_json::json!({
                        "error": format!("Failed to get recommendations: {}", e)
                    })))
                }
            }
        }
        Err(e) => {
            error!("Failed to create OreStatsService: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
