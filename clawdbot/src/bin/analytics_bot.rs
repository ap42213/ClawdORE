use clawdbot::{
    analytics::AnalyticsEngine,
    bot::{Bot, BotRunner, BotStatus},
    client::OreClient,
    config::{AnalyticsConfig, BotConfig},
    error::Result,
};
use log::{error, info};
use solana_sdk::signature::read_keypair_file;
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

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
            let current_round = board.round;

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
}

impl Bot for AnalyticsBot {
    fn name(&self) -> &str {
        &self.name
    }

    fn status(&self) -> BotStatus {
        *self.status.read().unwrap()
    }

    async fn start(&mut self) -> Result<()> {
        info!("Starting {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;

        let self_clone = Self {
            name: self.name.clone(),
            status: Arc::clone(&self.status),
            config: self.config.clone(),
            client: Arc::clone(&self.client),
            engine: Arc::clone(&self.engine),
        };

        tokio::spawn(async move {
            if let Err(e) = self_clone.analytics_loop().await {
                error!("Analytics bot error: {}", e);
                *self_clone.status.write().unwrap() = BotStatus::Error;
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Stopping {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Stopped;
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        info!("Pausing {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<()> {
        info!("Resuming {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Load configuration
    let config = BotConfig::default();
    
    // Load keypair
    let keypair = read_keypair_file(&config.keypair_path)
        .expect("Failed to load keypair");

    info!("🤖 Starting ClawdBot Analytics Bot");
    info!("📍 Wallet: {}", keypair.pubkey());
    info!("🌐 RPC: {}", config.rpc_url);
    info!("📊 History Depth: {} rounds", config.analytics.history_depth);

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create analytics bot
    let analytics_bot = AnalyticsBot::new(config.analytics.clone(), Arc::new(client));

    // Create and start bot runner
    let client_for_runner = OreClient::new(
        config.rpc_url.clone(),
        read_keypair_file(&config.keypair_path).unwrap(),
    );
    let mut runner = BotRunner::new(config, client_for_runner);
    runner.add_bot(Box::new(analytics_bot));

    // Run
    runner.run().await?;

    Ok(())
}
