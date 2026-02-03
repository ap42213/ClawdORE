use clawdbot::{
    blockchain_parser::{BlockchainParser, OreInstructionType},
    config::BotConfig,
    db::{is_database_available, Signal, SignalType},
    strategies::{StrategyEngine, RoundHistory, StrategyRecommendation},
    ore_strategy::{OreStrategyEngine, CompetitionLevel},
    learning_engine::{LearningEngine, WinRecord},
};
use colored::*;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, DbRound, DbTransaction};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

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
    
    // Initialize strategy engine
    let mut strategy_engine = StrategyEngine::new();
    info!("🧠 Strategy Engine initialized with 10 strategies:");
    info!("   • Momentum, Contrarian Value, Edge Hunting");
    info!("   • Streak Reversal, Low Competition, Whale Following");
    info!("   • Pattern Detection, Kelly Criterion, Quadrant Analysis");
    info!("   • Mean Reversion, Consensus (weighted combination)");

    // Initialize ORE-specific strategy engine for ALL player learning
    let mut ore_strategy = OreStrategyEngine::new();
    info!("\n🎯 ORE Strategy Engine initialized:");
    info!("   • Tracking ALL players (not just whales)");
    info!("   • Learning optimal square counts (1-25)");
    info!("   • Analyzing competition levels for ORE splits");
    info!("   • Min wallet: {:.4} SOL, Max bet: {:.4} SOL", 
        ore_strategy.min_wallet_sol, ore_strategy.max_bet_per_round_sol);

    // Load persisted learning data from database
    #[cfg(feature = "database")]
    if let Some(ref db) = db {
        info!("\n📚 Loading learned data from database...");
        
        // Load square statistics
        if let Ok(stats) = db.load_square_stats().await {
            if !stats.is_empty() {
                strategy_engine.load_square_stats_from_db(stats);
                info!("   ✅ Loaded square statistics for 25 squares");
            }
        }
        
        // Load whale data
        if let Ok(whales) = db.load_whales(1_000_000_000).await { // 1+ SOL deployed
            if !whales.is_empty() {
                let count = whales.len();
                strategy_engine.load_whales_from_db(whales);
                info!("   ✅ Loaded {} whale deployers", count);
            }
        }
        
        // Load historical rounds
        if let Ok(rounds) = db.load_round_history(500).await {
            if !rounds.is_empty() {
                let count = rounds.len();
                strategy_engine.load_rounds_from_db(rounds);
                info!("   ✅ Loaded {} historical rounds", count);
            }
        }
        
        // Load strategy performance weights
        if let Ok(perf) = db.get_strategy_performance().await {
            if !perf.is_empty() {
                info!("   📊 Strategy Performance:");
                for (name, total, hits, rate) in &perf {
                    info!("      • {}: {:.1}% hit rate ({}/{} rounds)", 
                        name, rate * 100.0, hits, total);
                }
                strategy_engine.load_strategy_weights(perf);
            }
        }
        
        // Get learning summary
        if let Ok(summary) = db.get_learning_summary().await {
            info!("\n📈 Learning Summary:");
            info!("   • Completed rounds analyzed: {}", summary["completed_rounds"]);
            info!("   • Whales tracked: {}", summary["tracked_whales"]);
            info!("   • Transactions processed: {}", summary["transactions_analyzed"]);
        }
        
        // Load ALL player performance data for ore_strategy
        if let Ok(players) = db.load_all_players().await {
            if !players.is_empty() {
                info!("\n👥 Loaded {} tracked players for learning", players.len());
                for (addr, deployed, won, rounds, wins, avg_sq, _, _, roi) in players.iter().take(5) {
                    info!("   • {}: {} rounds, {:.1}% win rate, {:.1}% ROI, avg {:.1} squares",
                        &addr[..8], rounds,
                        (*wins as f64 / *rounds.max(&1) as f64) * 100.0,
                        roi * 100.0,
                        avg_sq);
                }
                // Load into ore_strategy engine
                let perf_data: Vec<clawdbot::ore_strategy::PlayerPerformance> = players.iter()
                    .map(|(addr, deployed, won, rounds, wins, avg_sq, pref_sq, avg_dep, roi)| {
                        clawdbot::ore_strategy::PlayerPerformance {
                            address: addr.clone(),
                            total_deployed: *deployed as u64,
                            total_won: *won as u64,
                            total_rounds: *rounds as u32,
                            wins: *wins as u32,
                            avg_squares_per_deploy: *avg_sq as f64,
                            preferred_square_count: *pref_sq as u8,
                            avg_deploy_size: *avg_dep as u64,
                            roi: *roi as f64,
                            ore_per_sol: 0.0,
                        }
                    })
                    .collect();
                ore_strategy.load_player_stats(perf_data);
            }
        }
        
        // Load square count statistics
        if let Ok(sq_stats) = db.load_square_count_stats().await {
            if !sq_stats.is_empty() {
                info!("\n🎲 Square count performance:");
                for (count, used, won, _, _, win_rate, roi) in sq_stats.iter().take(5) {
                    info!("   • {} squares: {:.1}% win rate, {:.1}% ROI ({} samples)",
                        count, win_rate * 100.0, roi * 100.0, used);
                }
                // Load into ore_strategy engine
                let sq_data: Vec<clawdbot::ore_strategy::SquareCountStats> = sq_stats.iter()
                    .map(|(count, used, won, deployed, total_won, win_rate, roi)| {
                        clawdbot::ore_strategy::SquareCountStats {
                            count: *count as u8,
                            times_used: *used as u32,
                            times_won: *won as u32,
                            total_deployed: *deployed as u64,
                            total_won: *total_won as u64,
                            avg_ore_earned: 0.0,
                            win_rate: *win_rate as f64,
                            roi: *roi as f64,
                        }
                    })
                    .collect();
                ore_strategy.load_square_count_stats(sq_data);
            }
        }
        
        // Get best conditions from history
        if let Ok(conditions) = db.get_best_conditions().await {
            if !conditions.is_empty() {
                info!("\n🎯 Best conditions for ORE earnings:");
                for (level, count, win_rate, avg_ore) in &conditions {
                    info!("   • {}: {:.1}% win rate, {:.2} avg ORE ({} rounds)",
                        level, win_rate * 100.0, avg_ore, count);
                }
            }
        }
        
        // Get comprehensive summary
        if let Ok(summary) = db.get_comprehensive_learning_summary().await {
            info!("\n📊 All-Player Learning Summary:");
            info!("   • Total players tracked: {}", summary["total_players_tracked"]);
            info!("   • Active players (10+ rounds): {}", summary["active_players"]);
            info!("   • Best square count: {}", summary["best_square_count"]);
            info!("   • Avg winner squares: {:.1}", summary["avg_winner_square_count"]);
        }
        
        info!("");
    }

    // Initialize learning engine for deep analysis
    let mut learning_engine = LearningEngine::new();
    info!("🧠 Learning Engine initialized for deep pattern analysis");

    // Track deploys per round for win detection
    // We keep both current and previous round deploys so we can detect wins
    // when the Reset transaction comes (which happens AFTER new round starts)
    let mut round_deploys: HashMap<String, (u64, Vec<u8>)> = HashMap::new();
    let mut previous_round_deploys: HashMap<String, (u64, Vec<u8>)> = HashMap::new();
    let mut pending_round_clear = false;

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
                    
                    // IMPORTANT: Save current deploys for win detection
                    // The Reset transaction will come in this cycle's transactions
                    // so we need to preserve the deploys until after we process it
                    previous_round_deploys = round_deploys.clone();
                    pending_round_clear = true;
                    info!("📋 Saved {} deploys from round {} for win detection", 
                        previous_round_deploys.len(), last_round_id);
                    
                    // Try to get the completed round data and add to strategy engine
                    // Get the winning square from the completed round's slot_hash
                    let winning_result = parser.get_round_result(last_round_id);
                    if let Ok(completed) = parser.get_round(last_round_id) {
                        let (winning_square, motherlode) = match winning_result {
                            Ok(Some((sq, ml))) => {
                                info!("🎯 Round {} RESULT: Winning square {} {}", 
                                    last_round_id, sq, if ml { "🎰 MOTHERLODE!" } else { "" });
                                (sq, ml)
                            },
                            _ => {
                                warn!("⚠️ Could not determine winning square for round {}", last_round_id);
                                (0, false)
                            }
                        };
                        
                        let round_history = RoundHistory {
                            round_id: last_round_id,
                            winning_square,
                            deployed: completed.deployed,
                            total_pot: completed.deployed.iter().sum(),
                            motherlode,
                            timestamp: None,
                        };
                        strategy_engine.add_round(round_history.clone());
                        ore_strategy.record_round(&round_history.deployed, round_history.winning_square);
                        info!("📚 Added round {} to strategy history (winning square: {})", 
                            last_round_id, winning_square);
                        
                        // LEARNING: Check which tracked deploys hit the winning square
                        let winning_sq_usize = winning_square as usize;
                        let total_deployed: u64 = completed.deployed.iter().sum();
                        let is_full_ore = (total_deployed as f64 / 1_000_000_000.0) < 2.0;
                        
                        let mut winners_found = 0;
                        for (address, (deploy_amount, squares)) in &previous_round_deploys {
                            if squares.contains(&winning_square) {
                                // This player won!
                                winners_found += 1;
                                let num_squares = squares.len() as u8;
                                
                                // Record win in learning engine
                                let competition_on_sq = if winning_sq_usize < 25 { completed.deployed[winning_sq_usize] } else { 0 };
                                let winner_share: f64 = if competition_on_sq > 0 { *deploy_amount as f64 / competition_on_sq as f64 } else { 1.0 };
                                learning_engine.record_win(WinRecord {
                                    round_id: last_round_id,
                                    winner_address: address.clone(),
                                    winning_square,
                                    squares_bet: squares.clone(),
                                    amount_bet: *deploy_amount,
                                    amount_won: if winning_sq_usize < 25 { total_deployed.saturating_sub(completed.deployed[winning_sq_usize]) * (*deploy_amount) / competition_on_sq.max(1) } else { 0 },
                                    num_squares: num_squares,
                                    total_round_sol: total_deployed,
                                    num_deployers: previous_round_deploys.len() as u32,
                                    is_motherlode: motherlode,
                                    is_full_ore,
                                    ore_earned: if is_full_ore { 1.0 } else { 0.5 },
                                    competition_on_square: competition_on_sq,
                                    winner_share_pct: winner_share,
                                    slot: 0,
                                    timestamp: Some(std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64),
                                });
                                
                                // Record in ore_strategy
                                let winnings = if winning_sq_usize < 25 {
                                    total_deployed.saturating_sub(completed.deployed[winning_sq_usize])
                                } else { 0 };
                                ore_strategy.record_win(
                                    address, 
                                    winnings * (*deploy_amount) / completed.deployed[winning_sq_usize].max(1),
                                    if is_full_ore { 1.0 } else { 0.5 },
                                    num_squares
                                );
                                
                                // Record in database
                                #[cfg(feature = "database")]
                                if let Some(ref db) = db {
                                    let squares_i32: Vec<i32> = squares.iter().map(|s| *s as i32).collect();
                                    let amount_won_calc = (winnings * (*deploy_amount) / competition_on_sq.max(1)) as i64;
                                    db.record_win(
                                        last_round_id as i64,
                                        address,
                                        winning_square as i16,
                                        *deploy_amount as i64,
                                        amount_won_calc,
                                        &squares_i32,
                                        num_squares as i16,
                                        total_deployed as i64,
                                        previous_round_deploys.len() as i32,
                                        motherlode,
                                        is_full_ore,
                                        if is_full_ore { 1.0 } else { 0.5 },
                                        competition_on_sq as i64,
                                        winner_share as f32,
                                        0_i64,
                                    ).await.ok();
                                }
                            }
                        }
                        
                        if winners_found > 0 {
                            info!("🏆 Detected {} winners on square {} (full ORE: {})", 
                                winners_found, winning_square, is_full_ore);
                        }
                    }
                    
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
                        
                        // Update monitor_status with timing info for frontend
                        let slots_per_second: f64 = 2.5; // ~400ms per slot
                        let current_slot = parser.get_slot().unwrap_or(board.start_slot);
                        let round_duration_slots = board.end_slot.saturating_sub(board.start_slot);
                        let round_duration_secs = (round_duration_slots as f64 / slots_per_second) as u64;
                        let slots_remaining = board.end_slot.saturating_sub(current_slot);
                        let time_remaining_secs = (slots_remaining as f64 / slots_per_second) as u64;
                        
                        db.set_state("monitor_status", serde_json::json!({
                            "round_id": current_round,
                            "total_deployed": total_deployed,
                            "active_squares": current.deployed.iter().filter(|&&d| d > 0).count(),
                            "start_slot": board.start_slot,
                            "end_slot": board.end_slot,
                            "current_slot": current_slot,
                            "round_duration_secs": round_duration_secs,
                            "time_remaining_secs": time_remaining_secs,
                            "slots_remaining": slots_remaining,
                            "deployed_squares": current.deployed.iter().map(|&d| d).collect::<Vec<_>>(),
                            "updated_at": chrono::Utc::now().to_rfc3339(),
                        })).await.ok();
                    }

                    // Run strategy analysis
                    let recommendations = strategy_engine.get_recommendations(&current.deployed);
                    let consensus = strategy_engine.get_consensus_recommendation(&current.deployed);
                    
                    // Display top strategies
                    info!("\n{}", "═══ STRATEGY ANALYSIS ═══".yellow().bold());
                    
                    for (i, rec) in recommendations.iter().take(5).enumerate() {
                        if rec.confidence > 0.2 && !rec.squares.is_empty() {
                            let emoji = match i {
                                0 => "🥇",
                                1 => "🥈",
                                2 => "🥉",
                                _ => "📊",
                            };
                            info!("{} {} (conf: {:.0}%): {:?}", 
                                emoji, 
                                rec.strategy_name.bright_white(),
                                rec.confidence * 100.0,
                                rec.squares);
                            info!("   └─ {}", rec.reasoning.dimmed());
                        }
                    }
                    
                    // Display consensus recommendation
                    if !consensus.squares.is_empty() {
                        info!("\n{}", "🎯 CONSENSUS RECOMMENDATION".green().bold());
                        info!("   Squares: {:?}", consensus.squares);
                        info!("   Weights: {:?}", consensus.weights.iter()
                            .map(|w| format!("{:.1}%", w * 100.0))
                            .collect::<Vec<_>>());
                        info!("   Confidence: {:.0}%", consensus.confidence * 100.0);
                    }

                    // Send strategy signals to database
                    #[cfg(feature = "database")]
                    if let Some(ref db) = db {
                        // Send consensus recommendation as deploy opportunity
                        if consensus.confidence > 0.4 && !consensus.squares.is_empty() {
                            let signal = Signal::deploy_opportunity(
                                BOT_NAME, 
                                consensus.squares.clone(),
                                &format!("Consensus ({:.0}% confidence) - {}", 
                                    consensus.confidence * 100.0,
                                    consensus.reasoning)
                            );
                            db.send_signal(&signal).await.ok();
                        }
                        
                        // Send top strategy as separate signal
                        if let Some(top) = recommendations.first() {
                            if top.confidence > 0.5 {
                                let signal = Signal::new(
                                    SignalType::DeployOpportunity,
                                    BOT_NAME,
                                    serde_json::json!({
                                        "strategy": top.strategy_name,
                                        "squares": top.squares,
                                        "weights": top.weights,
                                        "confidence": top.confidence,
                                        "expected_roi": top.expected_roi,
                                        "reasoning": top.reasoning
                                    }),
                                );
                                db.send_signal(&signal).await.ok();
                            }
                        }
                        
                        // Store all strategy recommendations as state
                        let strategies_json: Vec<serde_json::Value> = recommendations.iter()
                            .filter(|r| r.confidence > 0.2)
                            .map(|r| serde_json::json!({
                                "name": r.strategy_name,
                                "squares": r.squares,
                                "weights": r.weights,
                                "confidence": r.confidence,
                                "expected_roi": r.expected_roi,
                                "reasoning": r.reasoning
                            }))
                            .collect();
                        
                        db.set_state("current_strategies", serde_json::json!(strategies_json)).await.ok();
                        db.set_state("consensus_recommendation", serde_json::json!({
                            "squares": consensus.squares,
                            "weights": consensus.weights,
                            "confidence": consensus.confidence
                        })).await.ok();
                    }
                    
                    info!("");
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
                
                // TRACK ALL PLAYERS (not just whales!) - this is key for learning
                let mut deploy_count = 0;
                for tx in &transactions {
                    if let Some(ref deploy) = tx.deploy_data {
                        let square_count = deploy.squares.len() as u8;
                        let squares_u8: Vec<u8> = deploy.squares.iter().map(|&s| s as u8).collect();
                        
                        // Track in ore_strategy engine (in-memory)
                        ore_strategy.record_deploy(
                            &tx.signer,
                            deploy.amount_lamports,
                            square_count,
                        );
                        
                        // Track in learning engine with more context
                        learning_engine.record_deploy(
                            &tx.signer,
                            deploy.amount_lamports,
                            &squares_u8,
                            0, // Will get total from round data
                            false, // Will detect motherlode from reset
                            tx.slot,
                        );
                        
                        // Track for win detection
                        round_deploys.insert(
                            tx.signer.clone(),
                            (deploy.amount_lamports, squares_u8.clone())
                        );
                        
                        deploy_count += 1;
                        
                        // Persist ALL player deploys to database
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            db.record_player_deploy(
                                &tx.signer,
                                deploy.amount_lamports as i64,
                                square_count as i16,
                                tx.slot as i64,
                            ).await.ok();
                            
                            // Also track square count stats
                            db.record_square_count_deploy(
                                square_count as i16,
                                deploy.amount_lamports as i64,
                            ).await.ok();
                        }
                        
                        // Still track whales separately for whale-following strategy
                        if deploy.amount_lamports > 1_000_000_000 { // > 1 SOL = whale
                            strategy_engine.track_whale(
                                tx.signer.clone(),
                                deploy.squares.iter().map(|&s| s as usize).collect()
                            );
                            
                            #[cfg(feature = "database")]
                            if let Some(ref db) = db {
                                let squares: Vec<i32> = deploy.squares.iter().map(|&s| s as i32).collect();
                                db.track_whale(
                                    &tx.signer, 
                                    deploy.amount_lamports as i64,
                                    &squares
                                ).await.ok();
                            }
                            
                            info!("🐋 Whale: {} → {:.2} SOL on {:?}",
                                &tx.signer[..8],
                                deploy.amount_lamports as f64 / 1_000_000_000.0,
                                deploy.squares);
                        }
                    }
                    
                    // Detect Reset transactions (round completions with winning squares)
                    if let Some(ref reset) = tx.reset_data {
                        info!("{}", format!(
                            "🎯 ROUND {} COMPLETED! Winning square: {} {}",
                            reset.round_id,
                            reset.winning_square,
                            if reset.motherlode { "🎰 MOTHERLODE!" } else { "" }
                        ).yellow().bold());
                        
                        // Update learning - this is the key data!
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            // Mark round as completed with winning square
                            db.complete_round(
                                reset.round_id as i64,
                                reset.winning_square as i16,
                                reset.motherlode
                            ).await.ok();
                            
                            // Try to get the round's deployment data for learning
                            if let Ok(round) = parser.get_round(reset.round_id) {
                                let deployed: [i64; 25] = round.deployed.map(|d| d as i64);
                                db.update_square_stats(reset.winning_square as i16, &deployed).await.ok();
                                
                                // Record round conditions for competition analysis
                                let total_deployed: i64 = deployed.iter().sum();
                                let num_deployers = deployed.iter().filter(|&&d| d > 0).count() as i32;
                                let squares_with_deploys = num_deployers as i16;
                                let competition = CompetitionLevel::from_deployed(total_deployed as u64);
                                let winning_sq = reset.winning_square as usize;
                                let competition_on_square = if winning_sq < 25 { deployed[winning_sq] } else { 0 };
                                
                                // Determine if this could be a full ORE win
                                let is_full_ore = (total_deployed as f64 / LAMPORTS_PER_SOL as f64) < 2.0;
                                let ore_earned = if is_full_ore { 1.0 } else {
                                    1.0 / (num_deployers.max(1) as f64 / 2.0)
                                };
                                
                                db.record_round_conditions(
                                    reset.round_id as i64,
                                    total_deployed,
                                    num_deployers,
                                    &format!("{:?}", competition),
                                    competition.ore_multiplier() as f32,
                                    squares_with_deploys,
                                ).await.ok();
                                
                                info!("📊 Round {} Analysis:", reset.round_id);
                                info!("   • Total deployed: {:.4} SOL ({:?})", 
                                    total_deployed as f64 / LAMPORTS_PER_SOL as f64, competition);
                                info!("   • Competition on square {}: {:.4} SOL", 
                                    reset.winning_square, competition_on_square as f64 / LAMPORTS_PER_SOL as f64);
                                info!("   • Full ORE: {} | Est. ORE: {:.2}", 
                                    if is_full_ore { "YES ✅" } else { "No" }, ore_earned);
                                
                                // FIND AND RECORD ALL WINNERS
                                // Use previous_round_deploys since round_deploys may have been 
                                // cleared or started accumulating for the new round
                                let deploys_to_check = if previous_round_deploys.is_empty() {
                                    &round_deploys
                                } else {
                                    &previous_round_deploys
                                };
                                
                                info!("   📋 Checking {} tracked deploys for winners", deploys_to_check.len());
                                
                                let mut winners_found = 0;
                                for (address, (amount, squares)) in deploys_to_check {
                                    if squares.contains(&(reset.winning_square as u8)) {
                                        let num_squares = squares.len() as u8;
                                        let winner_share = if competition_on_square > 0 {
                                            *amount as f64 / competition_on_square as f64
                                        } else {
                                            1.0
                                        };
                                        let amount_won = (competition_on_square as f64 * winner_share) as i64;
                                        
                                        info!("   🏆 Winner: {} bet {:.4} SOL on {} squares → won {:.4} SOL ({:.1}% share)",
                                            &address[..8],
                                            *amount as f64 / LAMPORTS_PER_SOL as f64,
                                            num_squares,
                                            amount_won as f64 / LAMPORTS_PER_SOL as f64,
                                            winner_share * 100.0);
                                        
                                        // Record comprehensive win to database
                                        db.record_win(
                                            reset.round_id as i64,
                                            address,
                                            reset.winning_square as i16,
                                            *amount as i64,
                                            amount_won,
                                            &squares.iter().map(|&s| s as i32).collect::<Vec<_>>(),
                                            num_squares as i16,
                                            total_deployed,
                                            num_deployers,
                                            reset.motherlode,
                                            is_full_ore,
                                            ore_earned as f32,
                                            competition_on_square,
                                            winner_share as f32,
                                            tx.slot as i64,
                                        ).await.ok();
                                        
                                        // Update player win record
                                        db.record_player_win(address, amount_won).await.ok();
                                        
                                        // Record in learning engine
                                        learning_engine.record_win(WinRecord {
                                            round_id: reset.round_id,
                                            winner_address: address.clone(),
                                            winning_square: reset.winning_square as u8,
                                            amount_bet: *amount,
                                            amount_won: amount_won as u64,
                                            squares_bet: squares.clone(),
                                            num_squares,
                                            total_round_sol: total_deployed as u64,
                                            num_deployers: num_deployers as u32,
                                            is_motherlode: reset.motherlode,
                                            is_full_ore,
                                            ore_earned,
                                            competition_on_square: competition_on_square as u64,
                                            winner_share_pct: winner_share,
                                            slot: tx.slot,
                                            timestamp: tx.block_time,
                                        });
                                        
                                        // Record square count win
                                        db.record_square_count_win(num_squares as i16, amount_won).await.ok();
                                        
                                        winners_found += 1;
                                    }
                                }
                                
                                if winners_found > 0 {
                                    info!("   ✅ Recorded {} winner(s) for learning", winners_found);
                                }
                            }
                            
                            // Record strategy performance for each strategy
                            if let Ok(state) = db.get_state("current_strategies").await {
                                if let Some(strategies) = state {
                                    if let Some(arr) = strategies.as_array() {
                                        for strat in arr {
                                            if let (Some(name), Some(squares), Some(conf)) = (
                                                strat["name"].as_str(),
                                                strat["squares"].as_array(),
                                                strat["confidence"].as_f64()
                                            ) {
                                                let sq: Vec<i32> = squares.iter()
                                                    .filter_map(|s| s.as_i64().map(|n| n as i32))
                                                    .collect();
                                                db.record_strategy_performance(
                                                    name,
                                                    reset.round_id as i64,
                                                    &sq,
                                                    reset.winning_square as i16,
                                                    conf as f32
                                                ).await.ok();
                                            }
                                        }
                                        info!("📊 Recorded strategy performance for {} strategies", arr.len());
                                    }
                                }
                            }
                        }
                    }
                }
                
                if deploy_count > 0 {
                    info!("👥 Tracked {} player deploys for learning", deploy_count);
                }
                
                // Periodically run strategy detection in learning engine
                if learning_engine.total_wins_tracked > 0 && learning_engine.total_wins_tracked % 20 == 0 {
                    learning_engine.analyze_and_detect_strategies();
                    let strategies = learning_engine.get_all_strategies();
                    if let Some(best) = strategies.first() {
                        info!("{}", format!(
                            "🎯 Best detected strategy: {} ({} squares, {:.4} SOL, {} competition)",
                            best.name, best.square_count, best.bet_size_sol, best.target_competition
                        ).green());
                        
                        // Save to database
                        #[cfg(feature = "database")]
                        if let Some(ref db) = db {
                            db.save_detected_strategy(
                                &best.name,
                                &best.description,
                                best.sample_size as i32,
                                best.win_rate as f32,
                                best.avg_roi as f32,
                                best.avg_ore_per_round as f32,
                                best.square_count as i16,
                                best.bet_size_sol as f32,
                                &best.target_competition,
                                &best.preferred_squares.iter().map(|&s| s as i32).collect::<Vec<_>>(),
                                best.play_motherlode,
                                best.confidence as f32,
                                best.consistent,
                                &best.examples,
                            ).await.ok();
                        }
                    }
                }

                // Count instruction types
                let stats = parser.get_stats();
                let total_deploys = stats.instruction_counts.get(&OreInstructionType::Deploy).unwrap_or(&0);
                let claim_count = stats.instruction_counts.get(&OreInstructionType::ClaimSOL).unwrap_or(&0)
                    + stats.instruction_counts.get(&OreInstructionType::ClaimORE).unwrap_or(&0);
                
                // Show ore_strategy learning summary periodically
                let ore_summary = ore_strategy.get_learning_summary();
                info!("📈 Stats: {} total deploys | {} claims | {} players tracked", 
                    total_deploys, claim_count, ore_summary["total_players_tracked"]);
                info!("🧠 Learning: {} wins tracked | {} full ORE wins",
                    learning_engine.total_wins_tracked, learning_engine.full_ore_wins_tracked);
                if let Some(optimal) = ore_summary["optimal_square_count"].as_u64() {
                    info!("🎯 Optimal squares: {} ({})", optimal, 
                        ore_summary["optimal_reasoning"].as_str().unwrap_or(""));
                }
            }
            Err(e) => {
                warn!("Failed to fetch transactions: {}", e);
            }
        }

        // Now that we've processed transactions (including Reset), clear if needed
        if pending_round_clear {
            round_deploys.clear();
            previous_round_deploys.clear();
            pending_round_clear = false;
            info!("🗑️ Cleared deploy tracking for new round");
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
