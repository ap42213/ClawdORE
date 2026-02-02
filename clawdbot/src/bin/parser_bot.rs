use clawdbot::{
    blockchain_parser::{BlockchainParser, OreInstructionType},
    config::BotConfig,
};
use colored::*;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

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

fn print_board_visual(deployed: &[u64; 25], winning_square: Option<usize>) {
    println!("\n╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║                              ORE BOARD STATE                               ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");
    
    for row in 0..5 {
        print!("║  ");
        for col in 0..5 {
            let idx = row * 5 + col;
            let amount = deployed[idx] as f64 / 1_000_000_000.0;
            let is_winner = winning_square == Some(idx);
            
            let cell = if is_winner {
                format!("#{:02}:{:>6.2}⭐", idx, amount).yellow().bold()
            } else if amount > 0.1 {
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

fn print_square_analysis(square_amounts: &[u64; 25]) {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    SQUARE POPULARITY                          ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    
    // Sort squares by amount
    let mut indexed: Vec<_> = square_amounts.iter().enumerate().collect();
    indexed.sort_by(|a, b| b.1.cmp(a.1));
    
    // Show top 10
    for (i, (square, amount)) in indexed.iter().take(10).enumerate() {
        let sol = **amount as f64 / 1_000_000_000.0;
        let bar_len = (sol * 10.0).min(30.0) as usize;
        let bar: String = "█".repeat(bar_len);
        
        let rank_emoji = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        
        println!("║  {} Square #{:02}: {:>8.4} SOL  {}                    ║", 
            rank_emoji, square, sol, bar.green());
    }
    
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

#[tokio::main]
async fn main() {
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ██████╗ ██████╗ ███████╗    ██████╗  █████╗ ██████╗ ███████╗██████╗ ║
    ║  ██╔═══██╗██╔══██╗██╔════╝    ██╔══██╗██╔══██╗██╔══██╗██╔════╝██╔══██╗║
    ║  ██║   ██║██████╔╝█████╗      ██████╔╝███████║██████╔╝███████╗██████╔╝║
    ║  ██║   ██║██╔══██╗██╔══╝      ██╔═══╝ ██╔══██║██╔══██╗╚════██║██╔══██╗║
    ║  ╚██████╔╝██║  ██║███████╗    ██║     ██║  ██║██║  ██║███████║██║  ██║║
    ║   ╚═════╝ ╚═╝  ╚═╝╚══════╝    ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝║
    ║                                                                       ║
    ║               Solana ORE Blockchain Parser & Monitor                  ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.cyan());

    info!("🔍 ORE Parser Bot Starting...");
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

    // Load keypair (optional for parser, but used to show wallet info)
    let wallet_info = match load_keypair(&config.keypair_path) {
        Ok(kp) => {
            info!("🔑 Wallet: {}", kp.pubkey());
            Some(kp)
        }
        Err(e) => {
            warn!("No keypair loaded (parser works without it): {}", e);
            None
        }
    };

    info!("📡 RPC: {}", config.rpc_url);
    info!("🎯 ORE Program: OREdv7MP3vLxV9TveRrPDNLAbSYaGDM7KhSHRwAr2cz");
    info!("═══════════════════════════════════════════════════════════════");

    // Create parser
    let mut parser = match BlockchainParser::new(&config.rpc_url) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create parser: {}", e);
            return;
        }
    };

    // Get update interval from env or default to 30 seconds
    let update_interval: u64 = std::env::var("PARSER_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    // Get transaction limit from env or default to 50
    let tx_limit: usize = std::env::var("PARSER_TX_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    info!("⏱️  Update interval: {} seconds", update_interval);
    info!("📊 Transaction limit: {}", tx_limit);
    info!("═══════════════════════════════════════════════════════════════\n");

    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    if let Err(e) = ctrlc::set_handler(move || {
        println!("\n🛑 Stopping parser...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }) {
        warn!("Could not set Ctrl-C handler: {}", e);
    }

    let mut iteration = 0;

    // Main loop
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        iteration += 1;
        
        println!("\n{}", format!("═══════════════════════════════════════════════════════════════").cyan());
        println!("{}", format!("                    ITERATION #{}", iteration).cyan().bold());
        println!("{}", format!("═══════════════════════════════════════════════════════════════").cyan());

        // Fetch board state
        match parser.get_board() {
            Ok(board) => {
                info!("📊 Current Round: {}", board.round_id);
                info!("🕐 Start Slot: {} | End Slot: {}", board.start_slot, board.end_slot);
                
                // Fetch current round details
                match parser.get_round(board.round_id) {
                    Ok(round) => {
                        let total_deployed: u64 = round.deployed.iter().sum();
                        info!("💰 Total Deployed: {:.4} SOL", total_deployed as f64 / 1_000_000_000.0);
                        
                        // Find squares with deployments
                        let active_squares: Vec<_> = round.deployed.iter()
                            .enumerate()
                            .filter(|(_, &d)| d > 0)
                            .collect();
                        info!("📍 Active Squares: {}", active_squares.len());
                        
                        // Print visual board
                        print_board_visual(&round.deployed, None);
                    }
                    Err(e) => {
                        warn!("Could not fetch round: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Could not fetch board: {}", e);
            }
        }

        // Fetch treasury state
        match parser.get_treasury() {
            Ok(treasury) => {
                info!("🏦 Treasury Balance: {:.4} SOL", treasury.balance as f64 / 1_000_000_000.0);
                info!("📈 Total Staked: {:.4} ORE", treasury.total_staked as f64 / 1e11);
            }
            Err(e) => {
                warn!("Could not fetch treasury: {}", e);
            }
        }

        // Fetch recent transactions
        println!("\n{}", "🔍 Fetching recent ORE transactions...".yellow());
        
        match parser.fetch_recent_transactions(tx_limit) {
            Ok(transactions) => {
                info!("📥 Fetched {} transactions", transactions.len());
                
                // Show recent transactions
                println!("\n╔═══════════════════════════════════════════════════════════════════════════════════════╗");
                println!("║                              RECENT TRANSACTIONS                                      ║");
                println!("╠═══════════════════════════════════════════════════════════════════════════════════════╣");
                
                for tx in transactions.iter().take(15) {
                    println!("║  {}  ║", parser.format_transaction(tx));
                }
                
                println!("╚═══════════════════════════════════════════════════════════════════════════════════════╝");
                
                // Show instruction breakdown
                let stats = parser.get_stats();
                print_instruction_counts(&stats.instruction_counts);
                
                // Show square analysis
                let square_amounts = parser.analyze_square_popularity();
                print_square_analysis(&square_amounts);
                
                // Show top miners
                let top_miners = parser.get_top_deployers(5);
                if !top_miners.is_empty() {
                    println!("\n╔═══════════════════════════════════════════════════════════════╗");
                    println!("║                      TOP DEPLOYERS                             ║");
                    println!("╠═══════════════════════════════════════════════════════════════╣");
                    
                    for (i, (addr, miner)) in top_miners.iter().enumerate() {
                        let sol = miner.total_deployed as f64 / 1_000_000_000.0;
                        let short_addr = &addr[..12];
                        let automation = if miner.automation_enabled { "🤖" } else { "  " };
                        
                        println!("║  #{} {}...  {:>8.4} SOL  {:>4} deploys  {}         ║",
                            i + 1, short_addr, sol, miner.deploy_count, automation);
                    }
                    
                    println!("╚═══════════════════════════════════════════════════════════════╝");
                }

                // Summary stats
                println!("\n╔═══════════════════════════════════════════════════════════════╗");
                println!("║                        SESSION STATS                          ║");
                println!("╠═══════════════════════════════════════════════════════════════╣");
                println!("║  📊 Transactions Parsed: {:>10}                          ║", stats.total_transactions);
                println!("║  👥 Miners Tracked:      {:>10}                          ║", stats.total_miners_tracked);
                println!("║  💰 Total SOL Deployed:  {:>10.4}                          ║", stats.total_sol_deployed);
                println!("╚═══════════════════════════════════════════════════════════════╝");
            }
            Err(e) => {
                error!("Failed to fetch transactions: {}", e);
            }
        }

        // Check wallet if we have one
        if let Some(ref wallet) = wallet_info {
            if let Some(miner) = parser.get_miner(wallet.pubkey()).ok().flatten() {
                println!("\n╔═══════════════════════════════════════════════════════════════╗");
                println!("║                       YOUR MINER STATS                        ║");
                println!("╠═══════════════════════════════════════════════════════════════╣");
                println!("║  Claimable SOL: {:>12.4}                                  ║", miner.rewards_sol as f64 / 1_000_000_000.0);
                println!("║  Claimable ORE: {:>12.4}                                  ║", miner.rewards_ore as f64 / 1e11);
                println!("║  Lifetime Deployed: {:>12.4} SOL                          ║", miner.lifetime_deployed as f64 / 1_000_000_000.0);
                println!("║  Lifetime Rewards: {:>12.4} SOL                           ║", miner.lifetime_rewards_sol as f64 / 1_000_000_000.0);
                println!("╚═══════════════════════════════════════════════════════════════╝");
            }
        }

        // Wait for next iteration
        info!("\n⏳ Next update in {} seconds...", update_interval);
        
        for _ in 0..update_interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    println!("\n✅ Parser bot stopped gracefully.");
}
