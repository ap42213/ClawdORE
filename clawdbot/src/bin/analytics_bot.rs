use clawdbot::{
    analytics::AnalyticsEngine,
    bot::BotStatus,
    client::OreClient,
    config::{AnalyticsConfig, BotConfig},
    error::Result,
};
use log::{error, info};
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

struct AnalyticsBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: AnalyticsConfig,
    client: Arc<OreClient>,
    engine: Arc<RwLock<AnalyticsEngine>>,
}

impl AnalyticsBot {
    fn new(config: AnalyticsConfig, client: Arc<OreClient>) -> Self {
        Self {
            name: "Analytics".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            engine: Arc::new(RwLock::new(AnalyticsEngine::new())),
        }
    }

    async fn analytics_loop(&self) -> Result<()> {
        info!("📊 Analytics bot started");

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

            // Collect round data
            let board = self.client.get_board()?;
            let current_round = board.round_id;

            info!("📈 Collecting analytics for round {}", current_round);

            // Fetch historical rounds
            let rounds = self.client.get_rounds(current_round, self.config.history_depth)?;

            // Update analytics engine
            {
                let mut engine = self.engine.write().unwrap();
                engine.clear_history();
                
                for (round_id, round) in rounds {
                    engine.add_round(round_id, round);
                }
            }

            // Generate analytics
            let analytics = {
                let engine = self.engine.read().unwrap();
                engine.get_overall_analytics()?
            };

            // Display analytics
            self.display_analytics(&analytics)?;

            // Export if configured
            if let Some(export_path) = &self.config.export_path {
                let engine = self.engine.read().unwrap();
                engine.export_to_json(export_path)?;
                info!("📁 Analytics exported to {}", export_path);
            }

            // Get predictions
            let predictions = {
                let engine = self.engine.read().unwrap();
                engine.predict_winning_squares(5)?
            };

            info!("🔮 Predicted top 5 squares for next round: {:?}", predictions);

            // Sleep before next update
            sleep(Duration::from_secs(self.config.update_interval)).await;
        }

        info!("🛑 Analytics bot stopped");
        Ok(())
    }

    fn display_analytics(&self, analytics: &clawdbot::analytics::OverallAnalytics) -> Result<()> {
        println!("\n╔═══════════════════════════════════════════════════════╗");
        println!("║              ORE ANALYTICS DASHBOARD                  ║");
        println!("╠═══════════════════════════════════════════════════════╣");
        println!("║ Rounds Analyzed: {:>38} ║", analytics.total_rounds_analyzed);
        println!("║ Total SOL Deployed: {:>35} ║", analytics.total_sol_deployed);
        println!("║ Most Winning Square: #{:<32} ║", analytics.most_winning_square);
        println!("║ Least Winning Square: #{:<31} ║", analytics.least_winning_square);
        println!("╠═══════════════════════════════════════════════════════╣");
        println!("║                  SQUARE STATISTICS                    ║");
        println!("╠═══════════════════════════════════════════════════════╣");

        // Sort squares by win frequency
        let mut sorted_stats = analytics.square_statistics.clone();
        sorted_stats.sort_by(|a, b| b.win_frequency.partial_cmp(&a.win_frequency).unwrap());

        // Display top 10 squares
        for (i, stats) in sorted_stats.iter().take(10).enumerate() {
            println!(
                "║ #{:2}. Square #{:2} | Wins: {:4} | Win%: {:5.2}% | Avg: {:8} ║",
                i + 1,
                stats.square_id,
                stats.times_won,
                stats.win_frequency * 100.0,
                stats.average_deployment as u64
            );
        }

        println!("╚═══════════════════════════════════════════════════════╝\n");

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;
        self.analytics_loop().await
    }
}

#[tokio::main]
async fn main() {
    // Initialize logger with RUST_LOG env var support
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("📊 ORE Analytics Bot Starting...");
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
    info!("📊 History Depth: {} rounds", config.analytics.history_depth);
    info!("⏱️ Update Interval: {} seconds", config.analytics.update_interval);
    info!("═══════════════════════════════════════════════════════════════");

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create and run analytics bot
    let mut analytics_bot = AnalyticsBot::new(config.analytics.clone(), Arc::new(client));
    if let Err(e) = analytics_bot.start().await {
        error!("Analytics bot error: {}", e);
    }
}
