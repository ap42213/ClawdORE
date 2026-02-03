use clawdbot::{
    blockchain_parser::{BlockchainParser, OreInstructionType},
    config::BotConfig,
    db::is_database_available,
    learning_engine::{LearningEngine, WinRecord, DetectedStrategy},
};
use colored::*;
use log::{error, info, warn};
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::SharedDb;

const BOT_NAME: &str = "learning-bot";
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ██╗     ███████╗ █████╗ ██████╗ ███╗   ██╗██╗███╗   ██╗ ██████╗    ║
    ║   ██║     ██╔════╝██╔══██╗██╔══██╗████╗  ██║██║████╗  ██║██╔════╝    ║
    ║   ██║     █████╗  ███████║██████╔╝██╔██╗ ██║██║██╔██╗ ██║██║  ███╗   ║
    ║   ██║     ██╔══╝  ██╔══██║██╔══██╗██║╚██╗██║██║██║╚██╗██║██║   ██║   ║
    ║   ███████╗███████╗██║  ██║██║  ██║██║ ╚████║██║██║ ╚████║╚██████╔╝   ║
    ║   ╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝╚═╝  ╚═══╝ ╚═════╝    ║
    ║                                                                       ║
    ║    Deep Learning Bot - Tracking ALL ORE Program Wallets On-Chain     ║
    ║    Program: oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv              ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.bright_magenta());

    info!("🧠 ORE Learning Bot Starting...");
    info!("═══════════════════════════════════════════════════════════════");
    info!("🎯 TRACKING: ALL wallets using ORE program on-chain");
    info!("📍 Program ID: oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv");
    info!("🔍 Looking for: Deploy, Reset, ClaimSOL, ClaimORE transactions");
    info!("📊 Goal: Build profiles of ALL ORE miners, find winning patterns");
    info!("");

    // Check database availability
    if !is_database_available() {
        error!("❌ DATABASE_URL not set!");
        error!("   The learning bot requires PostgreSQL for data storage.");
        return;
    }

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

    info!("📡 RPC: {}", config.rpc_url);

    // Connect to database
    #[cfg(feature = "database")]
    let db = match SharedDb::connect().await {
        Ok(db) => {
            info!("✅ Database connected");
            Some(db)
        }
        Err(e) => {
            error!("❌ Database connection failed: {}", e);
            return;
        }
    };

    #[cfg(not(feature = "database"))]
    {
        error!("❌ Learning bot requires database feature");
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

    // Initialize learning engine
    let mut learning_engine = LearningEngine::new();

    // Load existing data from database
    #[cfg(feature = "database")]
    if let Some(ref db) = db {
        info!("\n📚 Loading historical learning data...");
        
        // Load previous wins
        if let Ok(wins) = db.load_wins(1000).await {
            info!("   ✅ Loaded {} historical wins", wins.len());
            
            // Convert and load into learning engine
            for win_json in &wins {
                if let Ok(win) = serde_json::from_value::<WinRecordFromDb>(win_json.clone()) {
                    learning_engine.record_win(WinRecord {
                        round_id: win.round_id as u64,
                        winner_address: win.winner.clone(),
                        winning_square: win.winning_square as u8,
                        amount_bet: win.amount_bet as u64,
                        amount_won: win.amount_won as u64,
                        squares_bet: win.squares_bet.iter().map(|&s| s as u8).collect(),
                        num_squares: win.num_squares as u8,
                        total_round_sol: win.total_round_sol as u64,
                        num_deployers: win.num_deployers as u32,
                        is_motherlode: win.is_motherlode,
                        is_full_ore: win.is_full_ore,
                        ore_earned: win.ore_earned as f64,
                        competition_on_square: 0,
                        winner_share_pct: 0.0,
                        slot: 0,
                        timestamp: None,
                    });
                }
            }
        }
        
        // Load detected strategies
        if let Ok(strategies) = db.load_detected_strategies().await {
            info!("   ✅ Loaded {} detected strategies", strategies.len());
            for s in &strategies {
                info!("      • {}: {} (conf: {:.0}%)", 
                    s["name"].as_str().unwrap_or("?"),
                    s["description"].as_str().unwrap_or(""),
                    s["confidence"].as_f64().unwrap_or(0.0) * 100.0);
            }
        }
        
        // Get win stats
        if let Ok(stats) = db.get_win_stats().await {
            info!("\n📊 Win Statistics:");
            info!("   • Total wins tracked: {}", stats["total_wins_tracked"]);
            info!("   • Full ORE wins: {}", stats["full_ore_wins"]);
            info!("   • Motherlode wins: {}", stats["motherlode_wins"]);
            info!("   • Most common winning squares: {}", stats["most_common_winning_squares"]);
            
            if let Some(avg_sq) = stats["full_ore_avg_squares"].as_f64() {
                info!("\n🎯 Full ORE Winners Analysis:");
                info!("   • Avg squares used: {:.1}", avg_sq);
                info!("   • Avg bet size: {:.4} SOL", stats["full_ore_avg_bet_sol"].as_f64().unwrap_or(0.0));
                info!("   • Avg round competition: {:.2} SOL", stats["full_ore_avg_round_sol"].as_f64().unwrap_or(0.0));
            }
        }
    }

    let update_interval: u64 = std::env::var("LEARNING_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let tx_limit: usize = std::env::var("LEARNING_TX_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);

    info!("\n⏱️  Update interval: {} seconds", update_interval);
    info!("📥 Transaction limit: {}", tx_limit);
    info!("═══════════════════════════════════════════════════════════════\n");

    // Set up Ctrl+C handler
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\n🛑 Stopping learning bot...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    let mut last_round_id: u64 = 0;
    let mut current_round_deploys: std::collections::HashMap<String, (u64, Vec<u8>)> = std::collections::HashMap::new();
    let mut iteration_count: u32 = 0;

    // Main learning loop
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        iteration_count += 1;
        info!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_magenta());
        
        // Send heartbeat
        #[cfg(feature = "database")]
        if let Some(ref db) = db {
            use clawdbot::db::{Signal, SignalType};
            let signal = Signal::new(
                SignalType::Heartbeat,
                BOT_NAME,
                serde_json::json!({
                    "iteration": iteration_count,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
            db.send_signal(&signal).await.ok();
        }
        
        // Fetch current round state
        let current_round = match parser.get_board() {
            Ok(board) => {
                let round_id = board.round_id;
                
                // Detect new round - time to analyze completed round
                if round_id != last_round_id && last_round_id != 0 {
                    info!("{}", format!("🔄 Round {} completed, analyzing...", last_round_id).green());
                    
                    // Get winning square from completed round's slot_hash
                    // Note: ore_api returns 0-24 (array index), we convert to 1-25 for display/storage
                    if let Ok(Some((winning_sq_idx, motherlode))) = parser.get_round_result(last_round_id) {
                        let winning_square = winning_sq_idx + 1; // Convert to 1-25 for display
                        info!("🎯 Round {} RESULT: Winning square {} {}", 
                            last_round_id, winning_square, if motherlode { "🎰 MOTHERLODE!" } else { "" });
                        
                        // Get round data for analysis
                        if let Ok(round_data) = parser.get_round(last_round_id) {
                            let total_sol: u64 = round_data.deployed.iter().sum();
                            let competition_on_square = if (winning_sq_idx as usize) < 25 { round_data.deployed[winning_sq_idx as usize] } else { 0 };
                            let num_deployers = round_data.deployed.iter().filter(|&&d| d > 0).count() as u32;
                            let is_full_ore = total_sol < 2 * LAMPORTS_PER_SOL;
                            let ore_earned = if is_full_ore { 1.0 } else { 
                                1.0 / (num_deployers.max(1) as f64 / 2.0) 
                            };
                            
                            info!("   📊 Round Analysis:");
                            info!("      • Total deployed: {:.4} SOL", total_sol as f64 / LAMPORTS_PER_SOL as f64);
                            info!("      • Deployers: {}", num_deployers);
                            info!("      • Competition on square {}: {:.4} SOL", 
                                winning_square, competition_on_square as f64 / LAMPORTS_PER_SOL as f64);
                            info!("      • Full ORE: {}", if is_full_ore { "YES ✅" } else { "No" });
                            
                            // Find who won from tracked deploys (use 1-25 for comparison since we stored that way)
                            let mut winners_found = 0;
                            for (address, (amount, squares)) in &current_round_deploys {
                                if squares.contains(&winning_square) {
                                    winners_found += 1;
                                    let num_squares = squares.len() as u8;
                                    let winner_share = if competition_on_square > 0 {
                                        *amount as f64 / competition_on_square as f64
                                    } else {
                                        1.0
                                    };
                                    
                                    info!("{}", format!(
                                        "   👤 Winner: {} bet {:.4} SOL on {} squares, share: {:.1}%",
                                        &address[..8.min(address.len())],
                                        *amount as f64 / LAMPORTS_PER_SOL as f64,
                                        num_squares,
                                        winner_share * 100.0
                                    ).green());
                                    
                                    // Create win record
                                    let win = WinRecord {
                                        round_id: last_round_id,
                                        winner_address: address.clone(),
                                        winning_square,
                                        amount_bet: *amount,
                                        amount_won: (competition_on_square as f64 * winner_share) as u64,
                                        squares_bet: squares.clone(),
                                        num_squares,
                                        total_round_sol: total_sol,
                                        num_deployers,
                                        is_motherlode: motherlode,
                                        is_full_ore,
                                        ore_earned,
                                        competition_on_square,
                                        winner_share_pct: winner_share,
                                        slot: 0,
                                        timestamp: None,
                                    };
                                    
                                    learning_engine.record_win(win.clone());
                                    
                                    #[cfg(feature = "database")]
                                    if let Some(ref db) = db {
                                        let squares_i32: Vec<i32> = squares.iter().map(|s| *s as i32).collect();
                                        db.record_win(
                                            last_round_id as i64,
                                            &address,
                                            winning_square as i16,
                                            *amount as i64,
                                            (competition_on_square as f64 * winner_share) as i64,
                                            &squares_i32,
                                            num_squares as i16,
                                            total_sol as i64,
                                            num_deployers as i32,
                                            motherlode,
                                            is_full_ore,
                                            ore_earned as f32,
                                            competition_on_square as i64,
                                            winner_share as f32,
                                            0_i64,
                                        ).await.ok();
                                    }
                                }
                            }
                            
                            if winners_found > 0 {
                                info!("🏆 Detected {} winners on square {}", winners_found, winning_square);
                            }
                        }
                    } else {
                        warn!("⚠️ Could not determine winning square for round {}", last_round_id);
                    }
                    
                    // Clear deploys for new round
                    current_round_deploys.clear();
                }
                
                last_round_id = round_id;
                round_id
            }
            Err(e) => {
                warn!("Could not get board: {}", e);
                0
            }
        };

        // Fetch and analyze ALL transactions
        match parser.fetch_recent_transactions(tx_limit) {
            Ok(transactions) => {
                let mut new_deploys = 0;
                let mut new_wins = 0;
                
                for tx in &transactions {
                    // Track ALL deploys (not just whales)
                    if let Some(ref deploy) = tx.deploy_data {
                        let square_count = deploy.squares.len() as u8;
                        let total_round_sol = 0u64; // Would need to track per-round
                        let is_motherlode = false; // Would need to detect
                        
                        // Convert 0-24 to 1-25 for consistency
                        let squares_1_25: Vec<u8> = deploy.squares.iter().map(|&s| (s + 1) as u8).collect();
                        
                        // Record in learning engine (1-25)
                        learning_engine.record_deploy(
                            &tx.signer,
                            deploy.amount_lamports,
                            &squares_1_25,
                            total_round_sol,
                            is_motherlode,
                            tx.slot,
                        );
                        
                        // Track for this round (1-25)
                        current_round_deploys.insert(
                            tx.signer.clone(),
                            (deploy.amount_lamports, squares_1_25.clone())
                        );
                        
                        // Record to database
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            db.record_player_deploy(
                                &tx.signer,
                                deploy.amount_lamports as i64,
                                square_count as i16,
                                tx.slot as i64,
                            ).await.ok();
                            
                            // Also update square count statistics (critical for learning optimal count)
                            db.record_square_count_deploy(
                                square_count as i16,
                                deploy.amount_lamports as i64,
                            ).await.ok();
                        }
                        
                        new_deploys += 1;
                    }
                    
                    // Detect wins from Reset transactions
                    // Note: reset.winning_square is 0-24 from blockchain, convert to 1-25 for display
                    if let Some(ref reset) = tx.reset_data {
                        let winning_sq_display = reset.winning_square + 1; // Convert to 1-25
                        let winning_sq_idx = reset.winning_square as usize; // Keep 0-24 for array access
                        
                        info!("{}", format!(
                            "🎯 ROUND {} WINNER! Square: {} {}",
                            reset.round_id,
                            winning_sq_display,
                            if reset.motherlode { "🎰 MOTHERLODE!" } else { "" }
                        ).yellow().bold());
                        
                        // Try to find who won (this would need more blockchain parsing)
                        // For now, we track what we know
                        
                        // Get the round data to understand competition
                        if let Ok(round_data) = parser.get_round(reset.round_id) {
                            let total_sol: u64 = round_data.deployed.iter().sum();
                            let competition_on_square = if winning_sq_idx < 25 { round_data.deployed[winning_sq_idx] } else { 0 };
                            let num_deployers = round_data.deployed.iter().filter(|&&d| d > 0).count() as u32;
                            
                            // Determine if this was a full ORE win
                            // Full ORE = low competition, high share
                            let is_full_ore = total_sol < 2 * LAMPORTS_PER_SOL; // < 2 SOL total
                            let ore_earned = if is_full_ore { 1.0 } else { 
                                1.0 / (num_deployers.max(1) as f64 / 2.0) 
                            };
                            
                            info!("   📊 Round Analysis:");
                            info!("      • Total deployed: {:.4} SOL", total_sol as f64 / LAMPORTS_PER_SOL as f64);
                            info!("      • Deployers: {}", num_deployers);
                            info!("      • Competition on winning square: {:.4} SOL", 
                                competition_on_square as f64 / LAMPORTS_PER_SOL as f64);
                            info!("      • Estimated ORE: {:.2}", ore_earned);
                            info!("      • Full ORE: {}", if is_full_ore { "YES ✅" } else { "No" });
                            
                            // Find who bet on the winning square from our tracked deploys (using 1-25)
                            for (address, (amount, squares)) in &current_round_deploys {
                                if squares.contains(&winning_sq_display) {
                                    let num_squares = squares.len() as u8;
                                    let winner_share = if competition_on_square > 0 {
                                        *amount as f64 / competition_on_square as f64
                                    } else {
                                        1.0
                                    };
                                    
                                    info!("{}", format!(
                                        "   👤 Winner: {} bet {:.4} SOL on {} squares, share: {:.1}%",
                                        &address[..8],
                                        *amount as f64 / LAMPORTS_PER_SOL as f64,
                                        num_squares,
                                        winner_share * 100.0
                                    ).green());
                                    
                                    // Create win record (using 1-25 for storage)
                                    let win = WinRecord {
                                        round_id: reset.round_id,
                                        winner_address: address.clone(),
                                        winning_square: winning_sq_display,
                                        amount_bet: *amount,
                                        amount_won: (competition_on_square as f64 * winner_share) as u64,
                                        squares_bet: squares.clone(),
                                        num_squares,
                                        total_round_sol: total_sol,
                                        num_deployers,
                                        is_motherlode: reset.motherlode,
                                        is_full_ore,
                                        ore_earned: ore_earned as f64,
                                        competition_on_square,
                                        winner_share_pct: winner_share,
                                        slot: tx.slot,
                                        timestamp: tx.block_time,
                                    };
                                    
                                    // Record in learning engine
                                    learning_engine.record_win(win.clone());
                                    
                                    // Record in database (1-25)
                                    #[cfg(feature = "database")]
                                    if let Some(ref db) = db {
                                        db.record_win(
                                            reset.round_id as i64,
                                            address,
                                            winning_sq_display as i16,
                                            *amount as i64,
                                            (competition_on_square as f64 * winner_share) as i64,
                                            &squares.iter().map(|&s| s as i32).collect::<Vec<_>>(),
                                            num_squares as i16,
                                            total_sol as i64,
                                            num_deployers as i32,
                                            reset.motherlode,
                                            is_full_ore,
                                            ore_earned as f32,
                                            competition_on_square as i64,
                                            winner_share as f32,
                                            tx.slot as i64,
                                        ).await.ok();
                                        
                                        // Update player win record
                                        db.record_player_win(
                                            address,
                                            (competition_on_square as f64 * winner_share) as i64,
                                        ).await.ok();
                                        
                                        // Update square count win statistics (critical for learning)
                                        db.record_square_count_win(
                                            num_squares as i16,
                                            (competition_on_square as f64 * winner_share) as i64,
                                        ).await.ok();
                                    }
                                    
                                    new_wins += 1;
                                }
                            }
                        }
                    }
                }
                
                info!("📥 Processed {} txs: {} deploys, {} wins detected", 
                    transactions.len(), new_deploys, new_wins);
            }
            Err(e) => {
                warn!("Failed to fetch transactions: {}", e);
            }
        }

        // Periodically analyze and save strategies
        if learning_engine.total_wins_tracked > 0 && learning_engine.total_wins_tracked % 10 == 0 {
            info!("\n{}", "═══ STRATEGY ANALYSIS ═══".yellow().bold());
            
            learning_engine.analyze_and_detect_strategies();
            
            let strategies = learning_engine.get_all_strategies();
            for (i, strategy) in strategies.iter().take(5).enumerate() {
                let emoji = match i {
                    0 => "🥇",
                    1 => "🥈", 
                    2 => "🥉",
                    _ => "📊",
                };
                
                info!("{} {} (conf: {:.0}%)", emoji, strategy.name.bright_white(), strategy.confidence * 100.0);
                info!("   └─ {}", strategy.description);
                info!("   └─ {} squares, {:.4} SOL, target: {}", 
                    strategy.square_count, strategy.bet_size_sol, strategy.target_competition);
                
                // Save to database
                #[cfg(feature = "database")]
                if let Some(ref db) = db {
                    db.save_detected_strategy(
                        &strategy.name,
                        &strategy.description,
                        strategy.sample_size as i32,
                        strategy.win_rate as f32,
                        strategy.avg_roi as f32,
                        strategy.avg_ore_per_round as f32,
                        strategy.square_count as i16,
                        strategy.bet_size_sol as f32,
                        &strategy.target_competition,
                        &strategy.preferred_squares.iter().map(|&s| s as i32).collect::<Vec<_>>(),
                        strategy.play_motherlode,
                        strategy.confidence as f32,
                        strategy.consistent,
                        &strategy.examples,
                    ).await.ok();
                }
            }
            
            // Show players to copy
            let top_players = learning_engine.get_players_to_copy(3);
            if !top_players.is_empty() {
                info!("\n{}", "═══ PLAYERS TO COPY ═══".green().bold());
                for player in top_players {
                    info!("👤 {} - {:.1}% win rate, {:.2} ORE/SOL, {} squares avg",
                        &player.address[..8],
                        player.win_rate * 100.0,
                        player.ore_per_sol,
                        player.preferred_square_count);
                }
            }
        }
        
        // Show summary
        let summary = learning_engine.get_summary();
        info!("\n📈 Learning Progress:");
        info!("   • Wins tracked: {}", summary["total_wins_tracked"]);
        info!("   • Full ORE wins: {}", summary["full_ore_wins"]);
        info!("   • Players tracked: {}", summary["players_tracked"]);
        info!("   • Strategies detected: {}", summary["strategies_detected"]);
        
        if let Some(best) = summary["best_strategy"].as_object() {
            info!("\n🎯 Current Best Strategy: {}", best["name"].as_str().unwrap_or("?"));
            info!("   {} squares, {:.4} SOL, competition: {}", 
                best["square_count"], 
                best["bet_size_sol"].as_f64().unwrap_or(0.0),
                best["target_competition"].as_str().unwrap_or("?"));
        }

        info!("\n⏳ Next analysis in {} seconds...\n", update_interval);
        
        for _ in 0..update_interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    info!("✅ Learning bot stopped gracefully.");
}

// Helper struct for deserializing win records from database
#[derive(serde::Deserialize)]
struct WinRecordFromDb {
    round_id: i64,
    winner: String,
    winning_square: i16,
    amount_bet: i64,
    amount_won: i64,
    squares_bet: Vec<i32>,
    num_squares: i16,
    total_round_sol: i64,
    num_deployers: i32,
    is_motherlode: bool,
    is_full_ore: bool,
    ore_earned: f32,
}
