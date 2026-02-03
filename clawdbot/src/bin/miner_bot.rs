use clawdbot::{
    blockchain_parser::BlockchainParser,
    bot::BotStatus,
    client::OreClient,
    config::BotConfig,
    db::is_database_available,
    error::Result,
    ore_strategy::{OreStrategyEngine, DeployDecision, CompetitionLevel, PlayerPerformance, SquareCountStats},
};
use colored::*;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::SharedDb;

/// ORE Game Configuration
/// Key rules from user:
/// - Can pick 1-25 squares per round
/// - Min wallet: 0.05 SOL
/// - Max bet per round: 0.04 SOL total (divided by squares)
/// - Target low deployed rounds for better ORE splits
/// - Maximize rounds played while extracting max ORE

const MIN_WALLET_SOL: f64 = 0.05;
const MAX_BET_PER_ROUND_SOL: f64 = 0.04;
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Timing thresholds for MANUAL mode (slower, we sign ourselves)
const MANUAL_DECISION_TIME: f64 = 5.0;
const MANUAL_SIGN_DEADLINE: f64 = 3.0;
const MANUAL_TOO_LATE: f64 = 1.5;

/// Timing thresholds for EXECUTOR mode (fast, automation pre-funded)
/// We can push much closer to 0 since we sign instantly
const EXECUTOR_DECISION_TIME: f64 = 2.0;   // Start analyzing
const EXECUTOR_SIGN_DEADLINE: f64 = 0.8;   // Execute here - max intel, still safe
const EXECUTOR_TOO_LATE: f64 = 0.4;        // ~1 slot, too risky

/// Load keypair from file path or from environment variable
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

/// Smart ORE Miner Bot
/// Learns from ALL on-chain players to optimize:
/// 1. Number of squares to play
/// 2. When to play (competition level)
/// 3. Which squares to pick
/// 4. Budget per round
///
/// Supports two execution modes:
/// - Manual: Signs deploys directly (slower, 3-5s buffer needed)
/// - Executor: Triggers automation deploys (fast, 0.5-1s buffer)
struct SmartMinerBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    ore_strategy: OreStrategyEngine,
    parser: BlockchainParser,
    keypair: Keypair,
    rpc_url: String,
    mode: String,           // "simulation", "live", or "executor"
    authority: Option<Pubkey>,  // For executor mode: whose automation to trigger
    
    // Tracking
    rounds_played: u32,
    rounds_won: u32,
    total_deployed: u64,
    total_won: u64,
    ore_earned: f64,
}

impl SmartMinerBot {
    async fn new(
        rpc_url: String, 
        keypair: Keypair, 
        mode: String,
        authority: Option<Pubkey>,
    ) -> Result<Self> {
        let parser = BlockchainParser::new(&rpc_url)?;
        
        let mut ore_strategy = OreStrategyEngine::new();
        ore_strategy.min_wallet_sol = MIN_WALLET_SOL;
        ore_strategy.max_bet_per_round_sol = MAX_BET_PER_ROUND_SOL;
        
        Ok(Self {
            name: "SmartMiner".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            ore_strategy,
            parser,
            keypair,
            rpc_url,
            mode,
            authority,
            rounds_played: 0,
            rounds_won: 0,
            total_deployed: 0,
            total_won: 0,
            ore_earned: 0.0,
        })
    }
    
    /// Get timing thresholds based on mode
    fn get_timing(&self) -> (f64, f64, f64) {
        if self.mode == "executor" {
            (EXECUTOR_DECISION_TIME, EXECUTOR_SIGN_DEADLINE, EXECUTOR_TOO_LATE)
        } else {
            (MANUAL_DECISION_TIME, MANUAL_SIGN_DEADLINE, MANUAL_TOO_LATE)
        }
    }

    /// Load learned strategies from database
    #[cfg(feature = "database")]
    async fn load_learned_data(&mut self, db: &SharedDb) {
        info!("📚 Loading learned strategies from database...");
        
        // Load all player performance data
        if let Ok(players) = db.load_all_players().await {
            if !players.is_empty() {
                let count = players.len();
                let perf_data: Vec<PlayerPerformance> = players.iter()
                    .map(|(addr, deployed, won, rounds, wins, avg_sq, pref_sq, avg_dep, roi)| {
                        PlayerPerformance {
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
                self.ore_strategy.load_player_stats(perf_data);
                info!("   ✅ Loaded {} player strategies", count);
            }
        }
        
        // Load square count statistics
        if let Ok(sq_stats) = db.load_square_count_stats().await {
            if !sq_stats.is_empty() {
                let sq_data: Vec<SquareCountStats> = sq_stats.iter()
                    .map(|(count, used, won, deployed, total_won, win_rate, roi)| {
                        SquareCountStats {
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
                self.ore_strategy.load_square_count_stats(sq_data);
                info!("   ✅ Loaded square count statistics");
            }
        }
        
        // Get consensus recommendation
        if let Ok(state) = db.get_state("consensus_recommendation").await {
            if let Some(rec) = state {
                info!("   📊 Current consensus: {:?} (conf: {}%)", 
                    rec["squares"],
                    rec["confidence"].as_f64().unwrap_or(0.0) * 100.0);
            }
        }
        
        // Load detected strategies (the key learning!)
        if let Ok(strategies) = db.load_detected_strategies().await {
            if !strategies.is_empty() {
                info!("   🧠 Detected strategies:");
                for s in &strategies {
                    info!("      • {}: {} squares, {:.4} SOL, {} competition (conf: {:.0}%)",
                        s["name"].as_str().unwrap_or("?"),
                        s["square_count"],
                        s["bet_size_sol"].as_f64().unwrap_or(0.0),
                        s["target_competition"].as_str().unwrap_or("?"),
                        s["confidence"].as_f64().unwrap_or(0.0) * 100.0);
                }
                
                // Apply the best learned strategy to our ore_strategy!
                info!("   🎯 Applying best learned strategy...");
                self.ore_strategy.apply_best_strategy(&strategies);
            }
        }
        
        // Load win stats to understand what works
        if let Ok(stats) = db.get_win_stats().await {
            info!("   📈 Win Statistics:");
            info!("      • Total wins tracked: {}", stats["total_wins_tracked"]);
            info!("      • Full ORE wins: {}", stats["full_ore_wins"]);
            if let Some(avg_sq) = stats["full_ore_avg_squares"].as_f64() {
                info!("      • Full ORE winners avg: {:.1} squares, {:.4} SOL bet",
                    avg_sq, stats["full_ore_avg_bet_sol"].as_f64().unwrap_or(0.0));
            }
        }
    }

    #[cfg(not(feature = "database"))]
    async fn load_learned_data(&mut self, _db: &()) {
        info!("📚 No database connected, bot will learn through exploration");
    }

    /// Get wallet balance
    fn get_balance(&self) -> Result<u64> {
        let client = OreClient::new(self.rpc_url.clone(), Keypair::from_bytes(&self.keypair.to_bytes()).unwrap());
        client.get_balance()
    }

    /// Execute a deploy transaction on-chain (MANUAL mode)
    /// Returns the transaction signature on success
    async fn execute_deploy(&self, decision: &DeployDecision, round_id: u64) -> Result<String> {
        info!("{}", "⚡ EXECUTING MANUAL DEPLOY...".green().bold());
        
        // Convert squares Vec to [bool; 25] array
        let mut squares_arr = [false; 25];
        for &sq in &decision.squares {
            if sq < 25 {
                squares_arr[sq] = true;
            }
        }
        
        // Build the deploy instruction using ore_api
        let ix = ore_api::sdk::deploy(
            self.keypair.pubkey(),  // signer
            self.keypair.pubkey(),  // authority (same for manual deploy)
            decision.total_amount_lamports,
            round_id,
            squares_arr,
        );
        
        // Create RPC client
        let rpc_client = RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );
        
        // Get recent blockhash
        let blockhash = rpc_client.get_latest_blockhash()
            .map_err(|e| clawdbot::error::BotError::RpcTimeout(format!("Failed to get blockhash: {}", e)))?;
        
        // Create and sign transaction
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            blockhash,
        );
        
        // Send and confirm
        info!("   📤 Sending transaction...");
        let signature = rpc_client.send_and_confirm_transaction(&tx)
            .map_err(|e| clawdbot::error::BotError::RpcTimeout(format!("Transaction failed: {}", e)))?;
        
        info!("{}", format!("   ✅ Transaction confirmed: {}", signature).green());
        
        Ok(signature.to_string())
    }

    /// Execute a deploy via automation account (EXECUTOR mode)
    /// This is FAST - we sign with our keypair, SOL comes from pre-funded automation
    async fn execute_executor_deploy(&self, decision: &DeployDecision, round_id: u64) -> Result<String> {
        let authority = self.authority.ok_or_else(|| {
            clawdbot::error::BotError::Config("Executor mode requires AUTHORITY_PUBKEY".into())
        })?;
        
        info!("{}", "⚡ EXECUTOR DEPLOY (FAST MODE)...".yellow().bold());
        info!("   Authority: {}", authority);
        info!("   Squares: {:?}", decision.squares);
        
        // Convert squares Vec to [bool; 25] array
        let mut squares_arr = [false; 25];
        for &sq in &decision.squares {
            if sq < 25 {
                squares_arr[sq] = true;
            }
        }
        
        // Build the deploy instruction - WE are signer, AUTHORITY owns the automation
        // In Discretionary mode, we (executor) choose the squares via the mask
        let ix = ore_api::sdk::deploy(
            self.keypair.pubkey(),  // signer (executor - US)
            authority,              // authority (whose automation account)
            decision.total_amount_lamports,
            round_id,
            squares_arr,
        );
        
        // Create RPC client with confirmed commitment for speed
        let rpc_client = RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );
        
        // Get recent blockhash
        let blockhash = rpc_client.get_latest_blockhash()
            .map_err(|e| clawdbot::error::BotError::RpcTimeout(format!("Failed to get blockhash: {}", e)))?;
        
        // Create and sign transaction - WE sign, not the authority!
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            blockhash,
        );
        
        // Send transaction (don't wait for full confirmation for speed)
        info!("   📤 Sending executor transaction...");
        let signature = rpc_client.send_transaction(&tx)
            .map_err(|e| clawdbot::error::BotError::RpcTimeout(format!("Transaction failed: {}", e)))?;
        
        info!("{}", format!("   ✅ Transaction sent: {}", signature).green());
        info!("   ⏱️  Deployed at ~{:.2}s before round end", self.get_time_remaining(&self.parser.get_board()?));
        
        Ok(signature.to_string())
    }

    /// Calculate time remaining in current round
    fn get_time_remaining(&self, board: &ore_api::state::Board) -> f64 {
        let current_slot = match self.parser.get_slot() {
            Ok(s) => s,
            Err(_) => return 60.0, // Default to full round on error
        };
        
        if current_slot >= board.end_slot {
            return 0.0;
        }
        
        let slots_remaining = board.end_slot.saturating_sub(current_slot);
        // ~400ms per slot = 2.5 slots per second
        slots_remaining as f64 / 2.5
    }

    /// Main mining loop
    async fn mining_loop(&mut self) -> Result<()> {
        info!("⛏️  Smart Miner started!");
        info!("   Min wallet: {:.4} SOL", self.ore_strategy.min_wallet_sol);
        info!("   Max bet/round: {:.4} SOL", self.ore_strategy.max_bet_per_round_sol);
        
        let mut last_round_id: u64 = 0;
        let update_interval = 10; // Check every 10 seconds
        
        loop {
            // Check status
            {
                let status = self.status.read().unwrap();
                if *status == BotStatus::Stopped {
                    break;
                }
                if *status == BotStatus::Paused {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }

            // Get wallet balance
            let balance = match self.get_balance() {
                Ok(b) => b,
                Err(e) => {
                    warn!("Failed to get balance: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            let balance_sol = balance as f64 / LAMPORTS_PER_SOL as f64;
            let rounds_remaining = self.ore_strategy.estimate_rounds_remaining(balance);

            // Get current round
            let board = match self.parser.get_board() {
                Ok(b) => b,
                Err(e) => {
                    warn!("Failed to get board: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            let round = match self.parser.get_round(board.round_id) {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to get round: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let current_round_id = board.round_id;
            let total_deployed: u64 = round.deployed.iter().sum();
            let competition = CompetitionLevel::from_deployed(total_deployed);
            let num_deployers = round.deployed.iter().filter(|&&d| d > 0).count() as u32;

            // Display status
            let time_remaining = self.get_time_remaining(&board);
            info!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
            info!("💰 Balance: {:.4} SOL | Est. rounds: {}", balance_sol, rounds_remaining);
            info!("📊 Round {} | Deployed: {:.4} SOL | Competition: {:?}", 
                current_round_id,
                total_deployed as f64 / LAMPORTS_PER_SOL as f64,
                competition);
            info!("⏱️  Time remaining: {:.1}s", time_remaining);
            
            // Get consensus recommendation from coordinator (via database)
            let mut consensus_squares: Vec<usize> = Vec::new();
            let mut consensus_confidence: f64 = 0.0;
            
            #[cfg(feature = "database")]
            if is_database_available() {
                if let Ok(db) = SharedDb::connect().await {
                    if let Ok(Some(rec)) = db.get_state("consensus_recommendation").await {
                        if let Some(squares) = rec["squares"].as_array() {
                            consensus_squares = squares.iter()
                                .filter_map(|s| s.as_u64().map(|n| n as usize))
                                .collect();
                        }
                        consensus_confidence = rec["confidence"].as_f64().unwrap_or(0.0);
                    }
                }
            }

            // Make deploy decision using learned strategy
            let decision = self.ore_strategy.make_deploy_decision(
                balance,
                &round.deployed,
                num_deployers,
                &consensus_squares,
                consensus_confidence,
            );

            if decision.should_deploy {
                info!("{}", format!("🎯 DEPLOY DECISION: YES").green().bold());
                info!("   Squares: {:?} ({} total)", decision.squares, decision.squares.len());
                info!("   Amount: {:.4} SOL ({:.6} per square)", 
                    decision.total_amount_lamports as f64 / LAMPORTS_PER_SOL as f64,
                    decision.per_square_lamports as f64 / LAMPORTS_PER_SOL as f64);
                info!("   Expected ORE: {:.2}", decision.expected_ore);
                info!("   Reasoning: {}", decision.reasoning);
                
                // Get timing thresholds based on mode
                let (decision_time, sign_deadline, too_late) = self.get_timing();
                let time_remaining = self.get_time_remaining(&board);
                
                info!("   Mode: {} | Timing: decide@{:.1}s, sign@{:.1}s, late@{:.1}s", 
                    self.mode, decision_time, sign_deadline, too_late);
                
                if time_remaining <= too_late {
                    // Too late - skip this round
                    warn!("   💀 TOO LATE ({:.1}s remaining) - waiting for next round", time_remaining);
                } else if time_remaining <= sign_deadline {
                    // In the signing window - execute immediately!
                    let result = match self.mode.as_str() {
                        "executor" => self.execute_executor_deploy(&decision, current_round_id).await,
                        "live" => self.execute_deploy(&decision, current_round_id).await,
                        _ => {
                            info!("   📋 SIMULATION MODE - would execute at {:.1}s", time_remaining);
                            self.rounds_played += 1;
                            self.total_deployed += decision.total_amount_lamports;
                            Ok("simulation".to_string())
                        }
                    };
                    
                    match result {
                        Ok(sig) if sig != "simulation" => {
                            info!("   🎉 Deploy successful! Signature: {}", sig);
                            self.rounds_played += 1;
                            self.total_deployed += decision.total_amount_lamports;
                            
                            // Log to database
                            #[cfg(feature = "database")]
                            if is_database_available() {
                                if let Ok(db) = SharedDb::connect().await {
                                    db.set_state("last_deploy", serde_json::json!({
                                        "round_id": current_round_id,
                                        "squares": decision.squares,
                                        "amount_lamports": decision.total_amount_lamports,
                                        "signature": sig,
                                        "mode": self.mode,
                                        "time_remaining": time_remaining,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })).await.ok();
                                }
                            }
                        }
                        Err(e) => {
                            error!("   ❌ Deploy failed: {}", e);
                        }
                        _ => {}
                    }
                } else if time_remaining <= decision_time {
                    // In decision window - wait for optimal timing
                    let wait_time = (time_remaining - sign_deadline).max(0.1);
                    info!("   ⏳ Waiting {:.1}s for optimal timing ({:.1}s target)...", 
                        wait_time, sign_deadline);
                    sleep(Duration::from_secs_f64(wait_time)).await;
                    
                    // Now execute
                    let result = match self.mode.as_str() {
                        "executor" => self.execute_executor_deploy(&decision, current_round_id).await,
                        "live" => self.execute_deploy(&decision, current_round_id).await,
                        _ => {
                            info!("   📋 SIMULATION MODE - no transaction sent");
                            self.rounds_played += 1;
                            self.total_deployed += decision.total_amount_lamports;
                            Ok("simulation".to_string())
                        }
                    };
                    
                    match result {
                        Ok(sig) if sig != "simulation" => {
                            info!("   🎉 Deploy successful! Signature: {}", sig);
                            self.rounds_played += 1;
                            self.total_deployed += decision.total_amount_lamports;
                        }
                        Err(e) => {
                            error!("   ❌ Deploy failed: {}", e);
                        }
                        _ => {}
                    }
                } else {
                    // Too early - wait for decision window
                    let wait_time = (time_remaining - decision_time).max(0.1);
                    info!("   ⏳ Too early ({:.1}s remaining) - waiting {:.1}s for decision window...", 
                        time_remaining, wait_time);
                }
                
            } else {
                info!("{}", format!("⏸️  SKIP THIS ROUND").yellow());
                if let Some(reason) = decision.skip_reason {
                    info!("   Reason: {}", reason);
                }
            }

            // Check for new round (learning opportunity)
            if current_round_id != last_round_id && last_round_id != 0 {
                info!("{}", format!("🆕 New round: {} → {}", last_round_id, current_round_id).green());
                
                // Check if we won the previous round
                if let Ok(Some((winning_square, motherlode))) = self.parser.get_round_result(last_round_id) {
                    info!("🎯 Round {} result: square {} won {}", 
                        last_round_id, winning_square, if motherlode { "🎰 MOTHERLODE!" } else { "" });
                    
                    // Update strategy with round result
                    if let Ok(completed_round) = self.parser.get_round(last_round_id) {
                        self.ore_strategy.record_round(&completed_round.deployed, winning_square);
                        
                        // Check if WE won (if we played)
                        if self.rounds_played > 0 {
                            let last_decision = self.ore_strategy.get_optimal_square_count();
                            // This is a simplified check - ideally track our actual squares
                            info!("   Checking our squares against winner...");
                        }
                    }
                }
            }
            last_round_id = current_round_id;

            // Display learning stats
            let summary = self.ore_strategy.get_learning_summary();
            let (optimal_count, _, reasoning) = self.ore_strategy.get_optimal_square_count();
            info!("\n📈 Learning Stats:");
            info!("   Players tracked: {}", summary["total_players_tracked"]);
            info!("   Optimal squares: {} ({})", optimal_count, reasoning);
            info!("   My stats: {} rounds, {} won, {:.4} SOL deployed", 
                self.rounds_played, self.rounds_won, 
                self.total_deployed as f64 / LAMPORTS_PER_SOL as f64);
            
            info!("\n⏳ Next check in {} seconds...\n", update_interval);
            sleep(Duration::from_secs(update_interval)).await;
        }

        info!("🛑 Smart Miner stopped");
        info!("📊 Final Stats: {} rounds, {} won ({:.1}% win rate)", 
            self.rounds_played, 
            self.rounds_won,
            if self.rounds_played > 0 { 
                self.rounds_won as f64 / self.rounds_played as f64 * 100.0 
            } else { 0.0 });
        
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;
        self.mining_loop().await
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ███████╗███╗   ███╗ █████╗ ██████╗ ████████╗                        ║
    ║   ██╔════╝████╗ ████║██╔══██╗██╔══██╗╚══██╔══╝                        ║
    ║   ███████╗██╔████╔██║███████║██████╔╝   ██║                           ║
    ║   ╚════██║██║╚██╔╝██║██╔══██║██╔══██╗   ██║                           ║
    ║   ███████║██║ ╚═╝ ██║██║  ██║██║  ██║   ██║                           ║
    ║   ╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝                           ║
    ║                                                                       ║
    ║   ███╗   ███╗██╗███╗   ██╗███████╗██████╗                             ║
    ║   ████╗ ████║██║████╗  ██║██╔════╝██╔══██╗                            ║
    ║   ██╔████╔██║██║██╔██╗ ██║█████╗  ██████╔╝                            ║
    ║   ██║╚██╔╝██║██║██║╚██╗██║██╔══╝  ██╔══██╗                            ║
    ║   ██║ ╚═╝ ██║██║██║ ╚████║███████╗██║  ██║                            ║
    ║   ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝                            ║
    ║                                                                       ║
    ║          Learns from ALL players • Optimizes for ORE splits           ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.bright_cyan());

    info!("⛏️ Smart ORE Miner Starting...");
    info!("═══════════════════════════════════════════════════════════════");
    info!("📋 ORE Game Rules:");
    info!("   • 5x5 grid (25 squares)");
    info!("   • Can bet on 1-25 squares per round");
    info!("   • Min wallet balance: {:.4} SOL", MIN_WALLET_SOL);
    info!("   • Max bet per round: {:.4} SOL (divided across squares)", MAX_BET_PER_ROUND_SOL);
    info!("   • Low competition = better ORE splits!");
    info!("═══════════════════════════════════════════════════════════════");

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

    // Get mode and authority (for executor mode)
    let mode = config.mode.clone();
    let authority: Option<Pubkey> = std::env::var("AUTHORITY_PUBKEY")
        .ok()
        .and_then(|s| s.parse().ok());
    
    match mode.as_str() {
        "executor" => {
            if let Some(auth) = &authority {
                info!("{}", "🚀 EXECUTOR MODE - fast automation deploys!".yellow().bold());
                info!("   Authority: {}", auth);
                info!("   Timing: deploy at ~0.8s before round end");
            } else {
                error!("❌ EXECUTOR mode requires AUTHORITY_PUBKEY environment variable");
                error!("   This is the pubkey of the wallet that created the automation account");
                return;
            }
        }
        "live" => {
            info!("{}", "🟢 LIVE MODE - manual deploys (slower timing)".green().bold());
            info!("   Timing: deploy at ~3s before round end");
        }
        _ => {
            warn!("{}", "⚠️ SIMULATION MODE - no transactions will be sent".yellow());
            warn!("   Set BOT_MODE=live or BOT_MODE=executor to enable");
        }
    }

    // Create bot
    let mut bot = match SmartMinerBot::new(config.rpc_url.clone(), keypair, mode, authority).await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create bot: {}", e);
            return;
        }
    };

    // Load learned data from database
    #[cfg(feature = "database")]
    if is_database_available() {
        if let Ok(db) = SharedDb::connect().await {
            bot.load_learned_data(&db).await;
        }
    }

    // Set up Ctrl+C handler
    let status = bot.status.clone();
    ctrlc::set_handler(move || {
        println!("\n🛑 Stopping miner...");
        *status.write().unwrap() = BotStatus::Stopped;
    }).ok();

    // Run the bot
    if let Err(e) = bot.start().await {
        error!("Miner bot error: {}", e);
    }
}
