use clawdbot::{
    bot::BotStatus,
    client::OreClient,
    config::{BotConfig, MiningConfig},
    error::Result,
    strategy::MiningStrategy,
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

struct MinerBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: MiningConfig,
    client: Arc<OreClient>,
    strategy: MiningStrategy,
}

impl MinerBot {
    fn new(config: MiningConfig, client: Arc<OreClient>) -> Self {
        let strategy = MiningStrategy::new(config.strategy.clone());
        
        Self {
            name: "Miner".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            strategy,
        }
    }

    async fn mining_loop(&self) -> Result<()> {
        info!("🔨 Miner bot started");

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

            // Check balance
            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            if balance_sol < self.config.min_sol_balance {
                error!("⚠️  Insufficient balance: {:.4} SOL (minimum: {:.2} SOL)", 
                    balance_sol, self.config.min_sol_balance);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Get current round
            let board = self.client.get_board()?;
            let round = self.client.get_round(board.round_id)?;

            info!("📊 Current round: {}, Total deployed: {}", 
                board.round_id, 
                round.deployed.iter().sum::<u64>()
            );

            // Get historical data for strategy
            let history = self.client.get_rounds(board.round_id, 10)?
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();

            // Select squares to deploy on
            let squares = self.strategy.select_squares(1, &round, &history)?;
            
            info!("🎯 Selected squares: {:?}", squares);

            // Here you would implement the actual deployment transaction
            // For now, we'll just log it
            info!("⛏️  Would deploy {:.4} SOL to squares {:?}", 
                self.config.deploy_amount_sol, squares);

            // Check if we should claim rewards
            if let Ok(Some(miner)) = self.client.get_miner() {
                let claimable_ore = miner.rewards_ore as f64 / 1e11; // Convert from grams to ORE
                
                if claimable_ore >= self.config.auto_claim_threshold_ore {
                    info!("💰 Claiming {:.2} ORE in rewards", claimable_ore);
                    // Implement claim transaction here
                }
            }

            // Wait before next iteration
            sleep(Duration::from_secs(30)).await;
        }

        info!("🛑 Miner bot stopped");
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
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("⛏️ ORE Miner Bot Starting...");
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
    info!("⛏️ Strategy: {}", config.mining.strategy);
    info!("💰 Deploy Amount: {:.4} SOL", config.mining.deploy_amount_sol);
    info!("═══════════════════════════════════════════════════════════════");

    // Check mode
    if config.mode != "live" {
        warn!("⚠️ Bot mode is '{}' - transactions will NOT be sent!", config.mode);
        warn!("   Set BOT_MODE=live to enable real transactions");
    }

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create and run miner bot
    let mut miner_bot = MinerBot::new(config.mining.clone(), Arc::new(client));
    if let Err(e) = miner_bot.start().await {
        error!("Miner bot error: {}", e);
    }
}
