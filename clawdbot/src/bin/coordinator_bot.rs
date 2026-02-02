use clawdbot::{
    blockchain_parser::{BlockchainParser, OreInstructionType},
    config::BotConfig,
    db::{is_database_available, Signal, SignalType},
};
use colored::*;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, DbRound, DbTransaction};

/// Load keypair from file path or from environment variable
fn load_keypair(keypair_path: &str) -> Result<Keypair, String> {
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

const BOT_NAME: &str = "coordinator";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ██████╗ ██████╗ ██████╗ ██████╗ ██████╗ ██╗███╗   ██╗ █████╗       ║
    ║  ██╔════╝██╔═══██╗██╔══██╗██╔══██╗██╔══██╗██║████╗  ██║██╔══██╗      ║
    ║  ██║     ██║   ██║██████╔╝██║  ██║██████╔╝██║██╔██╗ ██║███████║      ║
    ║  ██║     ██║   ██║██╔══██╗██║  ██║██╔══██╗██║██║╚██╗██║██╔══██║      ║
    ║  ╚██████╗╚██████╔╝██║  ██║██████╔╝██║  ██║██║██║ ╚████║██║  ██║      ║
    ║   ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝      ║
    ║                                                                       ║
    ║          ORE Bot Coordinator - Central Intelligence Hub               ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.cyan());

    info!("🎯 ORE Coordinator Bot Starting...");
    info!("═══════════════════════════════════════════════════════════════");

    // Check database availability
    if !is_database_available() {
        error!("❌ DATABASE_URL not set!");
        error!("   The coordinator requires PostgreSQL for bot coordination.");
        error!("   Add a PostgreSQL database in Railway and it will auto-configure.");
        return;
    }

    info!("✅ Database URL found");

    // Load configuration
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

    // Load keypair (optional)
    let wallet_info = load_keypair(&config.keypair_path).ok();
    if let Some(ref kp) = wallet_info {
        info!("🔑 Wallet: {}", kp.pubkey());
    }

    info!("📡 RPC: {}", config.rpc_url);
    info!("═══════════════════════════════════════════════════════════════");

    // Connect to database
    #[cfg(feature = "database")]
    let db = match SharedDb::connect().await {
        Ok(db) => {
            info!("✅ Database connected and schema initialized");
            Some(db)
        }
        Err(e) => {
            error!("❌ Database connection failed: {}", e);
            return;
        }
    };

    #[cfg(not(feature = "database"))]
    {
        error!("❌ Coordinator requires database feature. Build with: cargo build --features database");
        return;
    }

    // Create parser
    let mut parser = match BlockchainParser::new(&config.rpc_url) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create parser: {}", e);
            return;
        }
    };

    let update_interval: u64 = std::env::var("COORDINATOR_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15); // Faster updates for coordination

    let tx_limit: usize = std::env::var("COORDINATOR_TX_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    info!("⏱️  Update interval: {} seconds", update_interval);
    info!("═══════════════════════════════════════════════════════════════\n");

    // Track state for detecting changes
    let mut last_round_id: u64 = 0;
    let mut last_slot: u64 = 0;
    let mut round_start_detected = false;

    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\n🛑 Stopping coordinator...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    // Main coordination loop
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        info!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
        
        // 1. Fetch current board state
        match parser.get_board() {
            Ok(board) => {
                let current_round = board.round_id;
                let current_slot = board.end_slot;

                // Detect new round
                if current_round != last_round_id && last_round_id != 0 {
                    info!("{}", format!("🆕 NEW ROUND DETECTED: {} → {}", last_round_id, current_round).green().bold());
                    
                    #[cfg(feature = "database")]
                    if let Some(ref db) = db {
                        // Signal round started
                        let signal = Signal::round_started(BOT_NAME, current_round);
                        if let Err(e) = db.send_signal(&signal).await {
                            warn!("Failed to send round_started signal: {}", e);
                        } else {
                            info!("📤 Sent ROUND_STARTED signal");
                        }
                    }
                    
                    round_start_detected = true;
                }

                // Detect round ending soon (within 10 slots ~4 seconds)
                if let Ok(current) = parser.get_round(current_round) {
                    let total_deployed: u64 = current.deployed.iter().sum();
                    
                    info!("📊 Round {} | Deployed: {:.4} SOL | Slot: {}/{}", 
                        current_round,
                        total_deployed as f64 / 1_000_000_000.0,
                        board.start_slot,
                        board.end_slot);

                    // Store round data in database
                    #[cfg(feature = "database")]
                    if let Some(ref db) = db {
                        let db_round = DbRound {
                            round_id: current_round as i64,
                            start_slot: Some(board.start_slot as i64),
                            end_slot: Some(board.end_slot as i64),
                            winning_square: None,
                            total_deployed: total_deployed as i64,
                            deployed_squares: current.deployed.iter().map(|&d| d as i64).collect(),
                            total_winnings: 0,
                            total_vaulted: 0,
                            motherlode: false,
                            num_deploys: current.deployed.iter().filter(|&&d| d > 0).count() as i32,
                            completed_at: None,
                        };
                        
                        if let Err(e) = db.upsert_round(&db_round).await {
                            warn!("Failed to store round: {}", e);
                        }
                    }

                    // Analyze squares and send signals
                    let mut hot_squares = Vec::new();
                    let mut cold_squares = Vec::new();
                    let avg_deploy = total_deployed / 25;

                    for (i, &amount) in current.deployed.iter().enumerate() {
                        if amount > avg_deploy * 2 {
                            hot_squares.push((i, amount));
                        } else if amount == 0 && total_deployed > 0 {
                            cold_squares.push(i);
                        }
                    }

                    if !hot_squares.is_empty() {
                        info!("🔥 Hot squares: {:?}", hot_squares.iter().map(|(i, _)| i).collect::<Vec<_>>());
                        
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            for (square, amount) in &hot_squares {
                                let signal = Signal::hot_square(BOT_NAME, *square, *amount);
                                db.send_signal(&signal).await.ok();
                            }
                        }
                    }

                    if !cold_squares.is_empty() && cold_squares.len() <= 10 {
                        info!("❄️  Cold squares (contrarian opportunity): {:?}", cold_squares);
                        
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            for square in &cold_squares {
                                let signal = Signal::cold_square(BOT_NAME, *square);
                                db.send_signal(&signal).await.ok();
                            }
                        }
                    }
                }

                last_round_id = current_round;
                last_slot = current_slot;
            }
            Err(e) => {
                warn!("Could not fetch board: {}", e);
            }
        }

        // 2. Fetch and process recent transactions
        match parser.fetch_recent_transactions(tx_limit) {
            Ok(transactions) => {
                info!("📥 Processed {} transactions", transactions.len());
                
                #[cfg(feature = "database")]
                if let Some(ref db) = db {
                    for tx in &transactions {
                        let db_tx = DbTransaction {
                            signature: tx.signature.clone(),
                            slot: tx.slot as i64,
                            block_time: tx.block_time.and_then(|t| 
                                chrono::DateTime::from_timestamp(t, 0)),
                            instruction_type: tx.instruction_type.name().to_string(),
                            signer: tx.signer.clone(),
                            round_id: None, // Would need to extract from accounts
                            amount_lamports: tx.deploy_data.as_ref().map(|d| d.amount_lamports as i64),
                            squares: tx.deploy_data.as_ref()
                                .map(|d| d.squares.iter().map(|&s| s as i32).collect())
                                .unwrap_or_default(),
                            success: tx.success,
                        };
                        
                        db.insert_transaction(&db_tx).await.ok();
                    }
                }

                // Count instruction types
                let stats = parser.get_stats();
                let deploy_count = stats.instruction_counts.get(&OreInstructionType::Deploy).unwrap_or(&0);
                let claim_count = stats.instruction_counts.get(&OreInstructionType::ClaimSOL).unwrap_or(&0)
                    + stats.instruction_counts.get(&OreInstructionType::ClaimORE).unwrap_or(&0);
                
                info!("📈 Deploys: {} | Claims: {} | Miners: {}", 
                    deploy_count, claim_count, stats.total_miners_tracked);
            }
            Err(e) => {
                warn!("Failed to fetch transactions: {}", e);
            }
        }

        // 3. Check treasury and send claim recommendations
        match parser.get_treasury() {
            Ok(treasury) => {
                info!("🏦 Treasury: {:.4} SOL | Staked: {:.4} ORE",
                    treasury.balance as f64 / 1_000_000_000.0,
                    treasury.total_staked as f64 / 1e11);
            }
            Err(e) => {
                warn!("Could not fetch treasury: {}", e);
            }
        }

        // 4. Check wallet and send claim signals if rewards available
        if let Some(ref wallet) = wallet_info {
            if let Ok(Some(miner)) = parser.get_miner(wallet.pubkey()) {
                let sol_rewards = miner.rewards_sol as f64 / 1_000_000_000.0;
                let ore_rewards = miner.rewards_ore as f64 / 1e11;
                
                if sol_rewards > 0.01 || ore_rewards > 0.1 {
                    info!("💰 Your claimable: {:.4} SOL | {:.4} ORE", sol_rewards, ore_rewards);
                    
                    #[cfg(feature = "database")]
                    if let Some(ref db) = db {
                        let signal = Signal::new(
                            SignalType::ClaimRecommended,
                            BOT_NAME,
                            serde_json::json!({
                                "sol_rewards": sol_rewards,
                                "ore_rewards": ore_rewards,
                                "wallet": wallet.pubkey().to_string()
                            }),
                        ).to_bot("miner-bot");
                        
                        db.send_signal(&signal).await.ok();
                    }
                }
            }
        }

        // 5. Send heartbeat
        #[cfg(feature = "database")]
        if let Some(ref db) = db {
            let heartbeat = Signal::new(
                SignalType::Heartbeat,
                BOT_NAME,
                serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "last_round": last_round_id,
                }),
            );
            db.send_signal(&heartbeat).await.ok();
            
            // Store current state
            db.set_state("current_round", serde_json::json!(last_round_id)).await.ok();
            db.set_state("last_update", serde_json::json!(chrono::Utc::now().to_rfc3339())).await.ok();
        }

        info!("⏳ Next update in {} seconds...\n", update_interval);
        
        for _ in 0..update_interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    info!("✅ Coordinator stopped gracefully.");
}
