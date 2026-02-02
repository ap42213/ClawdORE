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

            // Check for coordinator recommendations
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                if let Ok(Some(rec)) = db.get_state("consensus_recommendation").await {
                    info!("📊 Coordinator recommends: {:?} (conf: {}%)",
                        rec["squares"],
                        rec["confidence"].as_f64().unwrap_or(0.0) * 100.0);
                }
            }

            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            let bet_amount_sol = (balance_sol * self.config.bet_percentage)
                .clamp(self.config.min_bet_sol, self.config.max_bet_sol);

            if bet_amount_sol < self.config.min_bet_sol {
                warn!("⚠️  Insufficient balance: {:.4} SOL", balance_sol);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            let round = self.client.get_round(current_round_id)?;
            let history = self.client.get_rounds(current_round_id, 20)?
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();

            let squares = self.strategy.select_squares(
                self.config.squares_to_bet,
                &history,
                &round,
            )?;

            let bets = self.strategy.calculate_bet_amounts(
                &squares,
                bet_amount_sol,
                self.config.min_bet_sol,
                self.config.max_bet_sol,
            );

            info!("🎯 Planned bets:");
            for (square, amount) in &bets {
                info!("  Square #{}: {:.4} SOL", square, amount);
            }

            let total_bet: f64 = bets.iter().map(|(_, amt)| amt).sum();
            info!("💰 Total: {:.4} SOL across {} squares", total_bet, bets.len());

            // Log bets to database
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                db.set_state("last_betting_decision", serde_json::json!({
                    "round": current_round_id,
                    "squares": squares,
                    "bets": bets.iter().map(|(s, a)| serde_json::json!({"square": s, "amount": a})).collect::<Vec<_>>(),
                    "total_bet": total_bet,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })).await.ok();
            }

            sleep(Duration::from_secs(30)).await;
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
