use clawdbot::{
    blockchain_parser::BlockchainParser,
    config::BotConfig,
    db::is_database_available,
};
use colored::*;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, Signal, SignalType};

const BOT_NAME: &str = "monitor-bot";

fn load_keypair(keypair_path: &str) -> std::result::Result<Keypair, String> {
    if let Ok(keypair_b58) = std::env::var("KEYPAIR_B58") {
        let bytes = bs58::decode(&keypair_b58)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58 keypair: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    if let Ok(keypair_json) = std::env::var("KEYPAIR_JSON") {
        let bytes: Vec<u8> = serde_json::from_str(&keypair_json)
            .map_err(|e| format!("Failed to parse keypair JSON: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    read_keypair_file(keypair_path)
        .map_err(|e| format!("Failed to read keypair file '{}': {}", keypair_path, e))
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ███╗   ███╗ ██████╗ ███╗   ██╗██╗████████╗ ██████╗ ██████╗          ║
    ║   ████╗ ████║██╔═══██╗████╗  ██║██║╚══██╔══╝██╔═══██╗██╔══██╗         ║
    ║   ██╔████╔██║██║   ██║██╔██╗ ██║██║   ██║   ██║   ██║██████╔╝         ║
    ║   ██║╚██╔╝██║██║   ██║██║╚██╗██║██║   ██║   ██║   ██║██╔══██╗         ║
    ║   ██║ ╚═╝ ██║╚██████╔╝██║ ╚████║██║   ██║   ╚██████╔╝██║  ██║         ║
    ║   ╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝         ║
    ║                                                                       ║
    ║                 ORE Monitor Bot - Real-time Monitoring                ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.cyan());

    info!("👁️ ORE Monitor Bot Starting...");
    info!("═══════════════════════════════════════════════════════════════");

    // Check database
    #[cfg(feature = "database")]
    let db = if is_database_available() {
        info!("✅ Database URL found");
        match SharedDb::connect().await {
            Ok(db) => {
                info!("✅ Database connected");
                Some(db)
            }
            Err(e) => {
                warn!("⚠️ Database connection failed: {} - running standalone", e);
                None
            }
        }
    } else {
        info!("ℹ️ No DATABASE_URL - running standalone mode");
        None
    };

    #[cfg(not(feature = "database"))]
    let db: Option<()> = None;

    let config = if std::env::var("RPC_URL").is_ok() {
        info!("📋 Loading config from environment variables");
        BotConfig::from_env()
    } else {
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "config.json".to_string());
        
        match std::fs::read_to_string(&config_path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|_| BotConfig::from_env()),
            Err(_) => BotConfig::from_env(),
        }
    };

    let wallet_info = match load_keypair(&config.keypair_path) {
        Ok(kp) => {
            info!("🔑 Wallet: {}", kp.pubkey());
            Some(kp)
        }
        Err(e) => {
            warn!("No keypair loaded: {}", e);
            None
        }
    };

    info!("📡 RPC: {}", config.rpc_url);
    info!("═══════════════════════════════════════════════════════════════");

    let mut parser = match BlockchainParser::new(&config.rpc_url) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create parser: {}", e);
            return;
        }
    };

    let update_interval: u64 = std::env::var("MONITOR_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\n🛑 Stopping monitor...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    let mut last_round_id: u64 = 0;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        info!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());

        // Heartbeat
        #[cfg(feature = "database")]
        if let Some(ref db) = db {
            let signal = Signal::new(
                SignalType::Heartbeat,
                BOT_NAME,
                serde_json::json!({
                    "status": "monitoring",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
            db.send_signal(&signal).await.ok();
        }

        // Fetch board state
        match parser.get_board() {
            Ok(board) => {
                let current_round = board.round_id;
                
                if current_round != last_round_id && last_round_id != 0 {
                    info!("{}", format!("🆕 NEW ROUND: {} → {}", last_round_id, current_round).green().bold());
                    
                    // Get winning square from completed round
                    // Note: ore_api returns 0-24, we convert to 1-25 for display
                    if let Ok(Some((sq, motherlode))) = parser.get_round_result(last_round_id) {
                        let winning_square = sq + 1; // Convert 0-24 to 1-25
                        info!("🎯 Round {} RESULT: Winning square {} {}", 
                            last_round_id, winning_square, if motherlode { "🎰 MOTHERLODE!" } else { "" });
                        
                        // Update database with winning square (1-25)
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            if let Err(e) = db.complete_round(
                                last_round_id as i64,
                                winning_square as i16,
                                motherlode
                            ).await {
                                warn!("Failed to update round with winning square: {}", e);
                            } else {
                                info!("✅ Updated round {} with winning_square = {}", last_round_id, winning_square);
                            }
                        }
                    }
                    
                    #[cfg(feature = "database")]
                    if let Some(ref db) = db {
                        let signal = Signal::new(
                            SignalType::RoundStarted,
                            BOT_NAME,
                            serde_json::json!({
                                "round_id": current_round,
                                "previous_round": last_round_id,
                            }),
                        );
                        db.send_signal(&signal).await.ok();
                    }
                }
                
                last_round_id = current_round;
                
                match parser.get_round(current_round) {
                    Ok(round) => {
                        let total_deployed: u64 = round.deployed.iter().sum();
                        let active_squares = round.deployed.iter().filter(|&&d| d > 0).count();
                        
                        // Use actual block times for accurate round timing
                        let (time_remaining_secs, round_duration_secs) = parser.get_round_timing(&board);
                        let current_slot = parser.get_slot().unwrap_or(board.start_slot);
                        let slots_remaining = board.end_slot.saturating_sub(current_slot);
                        
                        info!("📊 Round {} | Deployed: {:.4} SOL | Active: {} squares | ~{}s remaining", 
                            current_round,
                            total_deployed as f64 / 1_000_000_000.0,
                            active_squares,
                            time_remaining_secs);

                        // Store monitoring data
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            db.set_state("monitor_status", serde_json::json!({
                                "round_id": current_round,
                                "total_deployed": total_deployed,
                                "active_squares": active_squares,
                                "start_slot": board.start_slot,
                                "end_slot": board.end_slot,
                                "round_duration_secs": round_duration_secs,
                                "time_remaining_secs": time_remaining_secs,
                                "slots_remaining": slots_remaining,
                                "deployed_squares": round.deployed.iter().map(|&d| d).collect::<Vec<_>>(),
                                "updated_at": chrono::Utc::now().to_rfc3339(),
                            })).await.ok();
                        }
                    }
                    Err(e) => warn!("Could not fetch round: {}", e),
                }
            }
            Err(e) => warn!("Could not fetch board: {}", e),
        }

        // Check treasury
        match parser.get_treasury() {
            Ok(treasury) => {
                info!("🏦 Treasury: {:.4} SOL | Staked: {:.4} ORE",
                    treasury.balance as f64 / 1_000_000_000.0,
                    treasury.total_staked as f64 / 1e11);
            }
            Err(e) => warn!("Could not fetch treasury: {}", e),
        }

        // Check wallet rewards if available
        if let Some(ref wallet) = wallet_info {
            if let Ok(Some(miner)) = parser.get_miner(wallet.pubkey()) {
                let sol = miner.rewards_sol as f64 / 1_000_000_000.0;
                let ore = miner.rewards_ore as f64 / 1e11;
                if sol > 0.001 || ore > 0.01 {
                    info!("💰 Claimable: {:.4} SOL | {:.4} ORE", sol, ore);
                }
            }
        }

        info!("⏳ Next update in {} seconds...", update_interval);

        for _ in 0..update_interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    info!("✅ Monitor bot stopped gracefully.");
}
