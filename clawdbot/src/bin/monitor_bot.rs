use clawdbot::{
    client::OreClient,
    config::BotConfig,
    error::Result,
    monitor::MonitorBot,
};
use log::{error, info};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::sync::Arc;

/// Load keypair from file path or from environment variable
fn load_keypair(keypair_path: &str) -> std::result::Result<Keypair, String> {
    // First try environment variable (for Railway - base58 private key)
    if let Ok(keypair_b58) = std::env::var("KEYPAIR_B58") {
        let bytes = bs58::decode(&keypair_b58)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58 keypair: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    // Try KEYPAIR_JSON (JSON array format)
    if let Ok(keypair_json) = std::env::var("KEYPAIR_JSON") {
        let bytes: Vec<u8> = serde_json::from_str(&keypair_json)
            .map_err(|e| format!("Failed to parse keypair JSON: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    // Try file path
    read_keypair_file(keypair_path)
        .map_err(|e| format!("Failed to read keypair file '{}': {}", keypair_path, e))
}

#[tokio::main]
async fn main() {
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("👁️ ORE Monitor Bot Starting...");
    info!("═══════════════════════════════════════════════════════════════");

    // Load configuration from env or file
    let config = if std::env::var("RPC_URL").is_ok() {
        info!("📋 Loading config from environment variables");
        BotConfig::from_env()
    } else {
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "config.json".to_string());
        
        info!("📋 Loading config from: {}", config_path);
        
        match std::fs::read_to_string(&config_path) {
            Ok(data) => {
                match serde_json::from_str(&data) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        error!("Failed to parse config: {}", e);
                        return;
                    }
                }
            }
            Err(_) => {
                info!("📋 No config file found, using defaults with env vars");
                BotConfig::from_env()
            }
        }
    };

    // Load keypair
    let keypair = match load_keypair(&config.keypair_path) {
        Ok(kp) => kp,
        Err(e) => {
            error!("Failed to load keypair: {}", e);
            error!("");
            error!("Set one of:");
            error!("  - KEYPAIR_B58 (base58 encoded private key)");
            error!("  - KEYPAIR_JSON (JSON array of bytes)");
            error!("  - KEYPAIR_PATH pointing to a keypair file");
            return;
        }
    };

    info!("📡 RPC: {}", config.rpc_url);
    info!("🔑 Wallet: {}", keypair.pubkey());
    info!("═══════════════════════════════════════════════════════════════");

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);
    let client_arc = Arc::new(client);

    // Create and run monitor bot directly
    let mut monitor_bot = MonitorBot::new(config.monitor.clone(), Arc::clone(&client_arc));
    
    // Run monitor loop
    if let Err(e) = monitor_bot.start().await {
        error!("Monitor bot error: {}", e);
    }
}
