use clawdbot::{
    bot::BotStatus,
    client::OreClient,
    config::{BotConfig, BettingConfig},
    db::is_database_available,
    error::Result,
    strategy::BettingStrategy,
};
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, Signal, SignalType};

const BOT_NAME: &str = "betting-bot";

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

struct BettingBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: BettingConfig,
    client: Arc<OreClient>,
    strategy: BettingStrategy,
    last_round: Arc<RwLock<u64>>,
    #[cfg(feature = "database")]
    db: Option<SharedDb>,
}

impl BettingBot {
    #[cfg(feature = "database")]
    fn new(config: BettingConfig, client: Arc<OreClient>, db: Option<SharedDb>) -> Self {
        let strategy = BettingStrategy::new(
            config.strategy.clone(),
            config.risk_tolerance,
        );
        
        Self {
            name: "Betting".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            strategy,
            last_round: Arc::new(RwLock::new(0)),
            db,
        }
    }

    #[cfg(not(feature = "database"))]
    fn new(config: BettingConfig, client: Arc<OreClient>) -> Self {
        let strategy = BettingStrategy::new(
            config.strategy.clone(),
            config.risk_tolerance,
        );
        
        Self {
            name: "Betting".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            strategy,
            last_round: Arc::new(RwLock::new(0)),
        }
    }

    async fn betting_loop(&self) -> Result<()> {
        info!("🎲 Betting bot started");

        loop {
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

            // Send heartbeat
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                let signal = Signal::new(
                    SignalType::Heartbeat,
                    BOT_NAME,
                    serde_json::json!({
                        "status": "running",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                );
                db.send_signal(&signal).await.ok();
            }

            let board = self.client.get_board()?;
            let current_round_id = board.round_id;
            let mut last_round = self.last_round.write().unwrap();

            if current_round_id == *last_round {
                sleep(Duration::from_secs(10)).await;
                continue;
            }

            info!("🎲 New round detected: {}", current_round_id);
            *last_round = current_round_id;
            drop(last_round);

            // Check if we need to checkpoint from a previous round before deploying
            match self.client.needs_checkpoint() {
                Ok(Some(old_round_id)) => {
                    info!("⚠️  Need to checkpoint round {} before deploying", old_round_id);
                    match self.client.checkpoint(old_round_id) {
                        Ok(sig) => {
                            info!("✅ Checkpointed round {}: {}", old_round_id, sig);
                            // Small delay to let tx confirm
                            sleep(Duration::from_millis(500)).await;
                        }
                        Err(e) => {
                            warn!("⚠️  Checkpoint failed (may already be done): {}", e);
                        }
                    }
                }
                Ok(None) => {
                    // No checkpoint needed
                }
                Err(e) => {
                    warn!("⚠️  Could not check checkpoint status: {}", e);
                }
            }

            // Get squares from coordinator recommendations (database) or fall back to local strategy
            let mut squares: Vec<usize> = Vec::new();
            let mut confidence = 0.0;
            
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                if let Ok(Some(rec)) = db.get_state("consensus_recommendation").await {
                    confidence = rec["confidence"].as_f64().unwrap_or(0.0);
                    if let Some(sq_array) = rec["squares"].as_array() {
                        squares = sq_array.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as usize))
                            .filter(|&s| s < 25)
                            .collect();
                    }
                    if !squares.is_empty() {
                        info!("📊 Using coordinator recommendation: {:?} (conf: {:.0}%)", 
                              squares, confidence * 100.0);
                    }
                }
            }

            // Fall back to local strategy if no coordinator recommendation
            if squares.is_empty() {
                let round = self.client.get_round(current_round_id)?;
                let history = self.client.get_rounds(current_round_id, 20)?
                    .into_iter()
                    .map(|(_, r)| r)
                    .collect::<Vec<_>>();

                squares = self.strategy.select_squares(
                    self.config.squares_to_bet,
                    &history,
                    &round,
                )?;
                info!("🎲 Using local strategy (no coordinator): {:?}", squares);
            }

            // Skip if no squares to bet on
            if squares.is_empty() {
                warn!("⚠️  No squares selected, skipping round");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            // Use fixed sol_per_square 
            let sol_per_square = self.config.sol_per_square.unwrap_or(0.001);

            // Check we have enough balance
            let required_balance = sol_per_square * squares.len() as f64;
            if balance_sol < required_balance {
                warn!("⚠️  Insufficient balance: {:.4} SOL (need {:.4} SOL for {} squares @ {:.4} SOL each)", 
                      balance_sol, required_balance, squares.len(), sol_per_square);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            info!("🎯 Betting on {} squares @ {:.4} SOL each (1-indexed for ORE UI):", squares.len(), sol_per_square);
            for square in &squares {
                info!("  Square #{} (UI) → internal index {}", square, square.saturating_sub(1));
            }

            let total_bet = sol_per_square * squares.len() as f64;
            info!("💰 Total: {:.4} SOL across {} squares", total_bet, squares.len());

            // Convert squares to boolean array
            // Note: Coordinator sends 1-25 (1-indexed), but ore-api needs 0-24 (0-indexed)
            let mut squares_array = [false; 25];
            for square in &squares {
                // Convert 1-indexed (1-25) to 0-indexed (0-24)
                let idx = if *square > 0 { square - 1 } else { *square };
                if idx < 25 {
                    squares_array[idx] = true;
                }
            }
            
            // Convert SOL to lamports
            let lamports_per_square = (sol_per_square * 1_000_000_000.0) as u64;

            // Execute deploy transaction immediately!
            if lamports_per_square > 0 {
                info!("🚀 Deploying {} lamports per square to {} squares...", 
                      lamports_per_square, squares.len());
                
                match self.client.deploy(lamports_per_square, squares_array) {
                    Ok(signature) => {
                        info!("✅ Deploy successful! Tx: {}", signature);
                        
                        // Log to database
                        #[cfg(feature = "database")]
                        if let Some(ref db) = self.db {
                            let signal = Signal::new(
                                SignalType::BetPlaced,
                                BOT_NAME,
                                serde_json::json!({
                                    "round": current_round_id,
                                    "squares": squares,
                                    "sol_per_square": sol_per_square,
                                    "total_bet_sol": total_bet,
                                    "lamports_per_square": lamports_per_square,
                                    "signature": signature.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }),
                            );
                            db.send_signal(&signal).await.ok();
                            
                            db.set_state("last_betting_decision", serde_json::json!({
                                "round": current_round_id,
                                "squares": squares,
                                "sol_per_square": sol_per_square,
                                "total_bet": total_bet,
                                "signature": signature.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })).await.ok();
                        }
                    }
                    Err(e) => {
                        error!("❌ Deploy failed: {}", e);
                        
                        #[cfg(feature = "database")]
                        if let Some(ref db) = self.db {
                            let signal = Signal::new(
                                SignalType::Error,
                                BOT_NAME,
                                serde_json::json!({
                                    "error": format!("{}", e),
                                    "round": current_round_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }),
                            );
                            db.send_signal(&signal).await.ok();
                        }
                    }
                }
            }

            // Wait for next round detection (60-second rounds, poll every 5 seconds)
            sleep(Duration::from_secs(5)).await;
        }

        info!("🛑 Betting bot stopped");
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;
        self.betting_loop().await
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!(r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║   ██████╗ ███████╗████████╗████████╗██╗███╗   ██╗ ██████╗             ║
    ║   ██╔══██╗██╔════╝╚══██╔══╝╚══██╔══╝██║████╗  ██║██╔════╝             ║
    ║   ██████╔╝█████╗     ██║      ██║   ██║██╔██╗ ██║██║  ███╗            ║
    ║   ██╔══██╗██╔══╝     ██║      ██║   ██║██║╚██╗██║██║   ██║            ║
    ║   ██████╔╝███████╗   ██║      ██║   ██║██║ ╚████║╚██████╔╝            ║
    ║   ╚═════╝ ╚══════╝   ╚═╝      ╚═╝   ╚═╝╚═╝  ╚═══╝ ╚═════╝             ║
    ║                                                                       ║
    ║                  ORE Betting Bot - Strategy Executor                  ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#);

    info!("🎲 ORE Betting Bot Starting...");
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

    let keypair = match load_keypair(&config.keypair_path) {
        Ok(kp) => kp,
        Err(e) => {
            error!("Failed to load keypair: {}", e);
            return;
        }
    };

    info!("📡 RPC: {}", config.rpc_url);
    info!("🔑 Wallet: {}", keypair.pubkey());
    info!("═══════════════════════════════════════════════════════════════");

    let client = OreClient::new(config.rpc_url.clone(), keypair);

    #[cfg(feature = "database")]
    let mut betting_bot = BettingBot::new(config.betting.clone(), Arc::new(client), db);
    
    #[cfg(not(feature = "database"))]
    let mut betting_bot = BettingBot::new(config.betting.clone(), Arc::new(client));

    if let Err(e) = betting_bot.start().await {
        error!("Betting bot error: {}", e);
    }
}
