use clawdbot::{
    bot::BotStatus,
    client::OreClient,
    config::{BotConfig, BettingConfig},
    error::Result,
    strategy::BettingStrategy,
};
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

/// Load keypair from file path or from environment variable
fn load_keypair(keypair_path: &str) -> std::result::Result<Keypair, String> {
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

struct BettingBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: BettingConfig,
    client: Arc<OreClient>,
    strategy: BettingStrategy,
    last_round: Arc<RwLock<u64>>,
}

impl BettingBot {
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

            // Get current round
            let board = self.client.get_board()?;
            let current_round_id = board.round_id;
            let mut last_round = self.last_round.write().unwrap();

            // Only bet on new rounds
            if current_round_id == *last_round {
                sleep(Duration::from_secs(10)).await;
                continue;
            }

            info!("🎲 New round detected: {}", current_round_id);
            *last_round = current_round_id;
            drop(last_round);

            // Check balance
            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            let bet_amount_sol = (balance_sol * self.config.bet_percentage)
                .clamp(self.config.min_bet_sol, self.config.max_bet_sol);

            if bet_amount_sol < self.config.min_bet_sol {
                warn!("⚠️  Insufficient balance for betting: {:.4} SOL", balance_sol);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Get round data and history
            let round = self.client.get_round(current_round_id)?;
            let history = self.client.get_rounds(current_round_id, 20)?
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();

            // Select squares to bet on
            let squares = self.strategy.select_squares(
                self.config.squares_to_bet,
                &history,
                &round,
            )?;

            // Calculate bet amounts for each square
            let bets = self.strategy.calculate_bet_amounts(
                &squares,
                bet_amount_sol,
                self.config.min_bet_sol,
                self.config.max_bet_sol,
            );

            info!("🎯 Placing bets:");
            for (square, amount) in &bets {
                info!("  Square #{}: {:.4} SOL", square, amount);
            }

            // Here you would implement the actual betting transactions
            // For now, we'll just log it
            let total_bet: f64 = bets.iter().map(|(_, amt)| amt).sum();
            info!("💰 Total bet: {:.4} SOL across {} squares", total_bet, bets.len());

            // Wait for next round
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
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("🎲 ORE Betting Bot Starting...");
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
    info!("🎯 Strategy: {}", config.betting.strategy);
    info!("📊 Risk Tolerance: {:.1}%", config.betting.risk_tolerance * 100.0);
    info!("═══════════════════════════════════════════════════════════════");

    // Check mode
    if config.mode != "live" {
        warn!("⚠️ Bot mode is '{}' - transactions will NOT be sent!", config.mode);
        warn!("   Set BOT_MODE=live to enable real transactions");
    }

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create and run betting bot
    let mut betting_bot = BettingBot::new(config.betting.clone(), Arc::new(client));
    if let Err(e) = betting_bot.start().await {
        error!("Betting bot error: {}", e);
    }
}
