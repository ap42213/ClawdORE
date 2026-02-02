use clawdbot::{
    client::OreClient,
    config::BotConfig,
    simulation::SimulationEngine,
};
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::sync::Arc;

/// Load keypair from file path or from environment variable
fn load_keypair(keypair_path: &str) -> Result<Keypair, String> {
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
    
    info!("🎮 ORE Simulation Bot Starting...");
    info!("═══════════════════════════════════════════════════════════════");
    
    // Determine config source - environment or file
    let config = if std::env::var("RPC_URL").is_ok() {
        info!("📋 Loading config from environment variables");
        BotConfig::from_env()
    } else {
        // Load from file
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
                        error!("Set RPC_URL environment variable or provide valid config.json");
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
    info!("🎯 Mode: {}", config.mode);
    info!("⛏️  Mining Strategy: {}", config.mining.strategy);
    info!("═══════════════════════════════════════════════════════════════");
    
    // Create client
    let client = Arc::new(OreClient::new(config.rpc_url.clone(), keypair));
    
    // Check balance
    match client.get_balance() {
        Ok(balance) => {
            let sol = balance as f64 / 1_000_000_000.0;
            info!("💰 Wallet balance: {:.4} SOL", sol);
        }
        Err(e) => {
            warn!("Could not fetch balance: {}", e);
        }
    }
    
    // Check ORE accounts
    match client.get_board() {
        Ok(board) => {
            info!("📊 Current round: {}", board.round_id);
        }
        Err(e) => {
            warn!("Could not fetch board: {}", e);
        }
    }
    
    // Get simulation starting balance from env or default
    let initial_balance: f64 = std::env::var("SIMULATION_BALANCE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10.0);
    
    // Create simulation engine
    let mut engine = SimulationEngine::new(client, config.clone(), initial_balance);
    
    info!("");
    info!("🎮 SIMULATION MODE - Paper Trading with Real Mainnet Data");
    info!("═══════════════════════════════════════════════════════════════");
    info!("💰 Starting balance: {:.4} SOL (simulated)", initial_balance);
    info!("⏰ Monitoring ORE rounds (60 second cycles)");
    info!("📊 Strategy: {}", config.mining.strategy);
    info!("🎲 Betting: {}", if config.betting.enabled { "Enabled" } else { "Disabled" });
    info!("═══════════════════════════════════════════════════════════════");
    info!("");
    
    // Set up Ctrl+C handler
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    if let Err(e) = ctrlc::set_handler(move || {
        info!("");
        info!("🛑 Stopping simulation...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }) {
        warn!("Could not set Ctrl-C handler: {}", e);
    }
    
    // Run simulation
    match engine.run().await {
        Ok(_) => {
            info!("Simulation completed successfully");
        }
        Err(e) => {
            error!("Simulation error: {}", e);
        }
    }
    
    // Export results
    info!("");
    info!("═══════════════════════════════════════════════════════════════");
    info!("📊 FINAL RESULTS");
    info!("═══════════════════════════════════════════════════════════════");
    let (sol_balance, ore_balance) = engine.get_balances();
    info!("💰 Final SOL: {:.4}", sol_balance);
    info!("⛏️  Final ORE: {:.4}", ore_balance);
    
    let profit = sol_balance - initial_balance;
    let roi = (profit / initial_balance) * 100.0;
    
    if profit >= 0.0 {
        info!("📈 Profit: +{:.4} SOL (+{:.2}%)", profit, roi);
    } else {
        info!("📉 Loss: {:.4} SOL ({:.2}%)", profit, roi);
    }
    
    // Export to file
    let export_path = std::env::var("EXPORT_PATH")
        .unwrap_or_else(|_| "simulation_results.json".to_string());
    
    if let Err(e) = engine.export_results(&export_path) {
        warn!("Failed to export results: {}", e);
    } else {
        info!("📁 Results exported to: {}", export_path);
    }
    
    info!("═══════════════════════════════════════════════════════════════");
    info!("✅ Simulation complete!");
}
