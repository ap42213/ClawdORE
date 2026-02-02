use clawdbot::{
    blockchain_parser::{BlockchainParser, OreInstructionType},
    config::BotConfig,
    db::is_database_available,
};
use colored::*;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, Signal, SignalType, DbTransaction};

const BOT_NAME: &str = "parser-bot";

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

fn print_board_visual(deployed: &[u64; 25]) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║                              ORE BOARD STATE                               ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");
    
    for row in 0..5 {
        print!("║  ");
        for col in 0..5 {
            let idx = row * 5 + col;
            let amount = deployed[idx] as f64 / 1_000_000_000.0;
            
            let cell = if amount > 0.1 {
                format!("#{:02}:{:>6.2} ", idx, amount).green()
            } else if amount > 0.0 {
                format!("#{:02}:{:>6.2} ", idx, amount).white()
            } else {
                format!("#{:02}:{:>6.2} ", idx, amount).dimmed()
            };
            print!("{}", cell);
            if col < 4 {
                print!(" │ ");
            }
        }
        println!("  ║");
        if row < 4 {
            println!("║  ─────────────┼───────────────┼───────────────┼───────────────┼─────────────  ║");
        }
    }
    
    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
}

fn print_instruction_counts(counts: &HashMap<OreInstructionType, u64>) {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    INSTRUCTION BREAKDOWN                      ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    
    for (ix_type, count) in sorted {
        println!("║  {} {:15} {:>10}                             ║", 
            ix_type.emoji(), ix_type.name(), count);
    }
    
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ██████╗  █████╗ ██████╗ ███████╗███████╗██████╗                     ║
    ║   ██╔══██╗██╔══██╗██╔══██╗██╔════╝██╔════╝██╔══██╗                    ║
    ║   ██████╔╝███████║██████╔╝███████╗█████╗  ██████╔╝                    ║
    ║   ██╔═══╝ ██╔══██║██╔══██╗╚════██║██╔══╝  ██╔══██╗                    ║
    ║   ██║     ██║  ██║██║  ██║███████║███████╗██║  ██║                    ║
    ║   ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═╝                    ║
    ║                                                                       ║
    ║               ORE Parser Bot - Blockchain Transaction Parser          ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.cyan());

    info!("🔍 ORE Parser Bot Starting...");
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
    info!("🎯 ORE Program: oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv");
    info!("═══════════════════════════════════════════════════════════════");

    let mut parser = match BlockchainParser::new(&config.rpc_url) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create parser: {}", e);
            return;
        }
    };

    let update_interval: u64 = std::env::var("PARSER_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let tx_limit: usize = std::env::var("PARSER_TX_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    info!("⏱️  Update interval: {} seconds", update_interval);
    info!("📊 Transaction limit: {}", tx_limit);

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\n🛑 Stopping parser...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    let mut iteration = 0;

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        iteration += 1;
        
        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
        info!("📊 Iteration #{}", iteration);

        // Heartbeat
        #[cfg(feature = "database")]
        if let Some(ref db) = db {
            let signal = Signal::new(
                SignalType::Heartbeat,
                BOT_NAME,
                serde_json::json!({
                    "iteration": iteration,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
            db.send_signal(&signal).await.ok();
        }

        // Fetch board state
        match parser.get_board() {
            Ok(board) => {
                info!("📊 Current Round: {}", board.round_id);
                
                match parser.get_round(board.round_id) {
                    Ok(round) => {
                        let total_deployed: u64 = round.deployed.iter().sum();
                        info!("💰 Total Deployed: {:.4} SOL", total_deployed as f64 / 1_000_000_000.0);
                        print_board_visual(&round.deployed);
                    }
                    Err(e) => warn!("Could not fetch round: {}", e),
                }
            }
            Err(e) => warn!("Could not fetch board: {}", e),
        }

        // Fetch and store transactions
        match parser.fetch_recent_transactions(tx_limit) {
            Ok(transactions) => {
                info!("📥 Fetched {} transactions", transactions.len());
                
                // Store in database
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
                            round_id: None,
                            amount_lamports: tx.deploy_data.as_ref().map(|d| d.amount_lamports as i64),
                            squares: tx.deploy_data.as_ref()
                                .map(|d| d.squares.iter().map(|&s| s as i32).collect())
                                .unwrap_or_default(),
                            success: tx.success,
                        };
                        db.insert_transaction(&db_tx).await.ok();
                    }
                    info!("💾 Stored {} transactions to database", transactions.len());
                }
                
                let stats = parser.get_stats();
                print_instruction_counts(&stats.instruction_counts);
                
                info!("📈 Session: {} txs | {} miners | {:.4} SOL",
                    stats.total_transactions,
                    stats.total_miners_tracked,
                    stats.total_sol_deployed);
            }
            Err(e) => error!("Failed to fetch transactions: {}", e),
        }

        info!("⏳ Next update in {} seconds...", update_interval);
        
        for _ in 0..update_interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    println!("\n✅ Parser bot stopped gracefully.");
}
