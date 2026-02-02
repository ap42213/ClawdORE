use clawdbot::{
    client::OreClient,
    config::BotConfig,
    simulation::SimulationEngine,
};
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Signer};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::init();
    
    info!("🎮 ORE Simulation Bot Starting...");
    
    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.json".to_string());
    
    let config_data = match std::fs::read_to_string(&config_path) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to read config file: {}", e);
            error!("Usage: simulation-bot [config.json]");
            return;
        }
    };
    
    let config: BotConfig = match serde_json::from_str(&config_data) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to parse config: {}", e);
            return;
        }
    };
    
    // Load keypair
    let keypair = match read_keypair_file(&config.keypair_path) {
        Ok(kp) => kp,
        Err(e) => {
            error!("Failed to load keypair: {}", e);
            return;
        }
    };
    
    info!("📡 Connecting to: {}", config.rpc_url);
    info!("🔑 Wallet: {}", keypair.pubkey());
    
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
    
    // Create simulation engine
    let initial_balance = 10.0; // Start with 10 SOL simulation
    let mut engine = SimulationEngine::new(client, config, initial_balance);
    
    info!("🎮 Simulation mode: Paper trading with real mainnet data");
    info!("💰 Starting with {:.4} SOL (simulated)", initial_balance);
    info!("⏰ Monitoring rounds every 60 seconds");
    info!("📊 Press Ctrl+C to stop and view results");
    info!("");
    
    // Set up Ctrl+C handler
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        info!("");
        info!("🛑 Stopping simulation...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    
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
    info!("📊 Final Results:");
    let (sol_balance, ore_balance) = engine.get_balances();
    info!("💰 Final SOL: {:.4}", sol_balance);
    info!("⛏️  Final ORE: {:.4}", ore_balance);
    
    let profit = sol_balance - initial_balance;
    let roi = (profit / initial_balance) * 100.0;
    info!("📈 Profit/Loss: {:.4} SOL ({:.2}%)", profit, roi);
    
    // Export to file
    if let Err(e) = engine.export_results("simulation_results.json") {
        error!("Failed to export results: {}", e);
    }
    
    info!("✅ Simulation complete!");
}
