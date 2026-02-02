use clawdbot::{
    analytics::AnalyticsEngine,
    bot::BotStatus,
    client::OreClient,
    config::{AnalyticsConfig, BotConfig},
    db::is_database_available,
    error::Result,
};
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

#[cfg(feature = "database")]
use clawdbot::db::{SharedDb, Signal, SignalType};

const BOT_NAME: &str = "analytics-bot";

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

struct AnalyticsBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: AnalyticsConfig,
    client: Arc<OreClient>,
    engine: Arc<RwLock<AnalyticsEngine>>,
    #[cfg(feature = "database")]
    db: Option<SharedDb>,
}

impl AnalyticsBot {
    #[cfg(feature = "database")]
    fn new(config: AnalyticsConfig, client: Arc<OreClient>, db: Option<SharedDb>) -> Self {
        Self {
            name: "Analytics".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            engine: Arc::new(RwLock::new(AnalyticsEngine::new())),
            db,
        }
    }

    #[cfg(not(feature = "database"))]
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
                        "status": "analyzing",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                );
                db.send_signal(&signal).await.ok();
            }

            let board = match self.client.get_board() {
                Ok(b) => b,
                Err(e) => {
                    warn!("Failed to get board: {} - retrying in 30s", e);
                    sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };
            let current_round = board.round_id;

            info!("📈 Collecting analytics for round {}", current_round);

            let rounds = match self.client.get_rounds(current_round, self.config.history_depth) {
                Ok(r) => r,
                Err(e) => {
                    warn!("Failed to get rounds: {} - retrying in 30s", e);
                    sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

            {
                let mut engine = self.engine.write().unwrap();
                engine.clear_history();
                
                for (round_id, round) in rounds {
                    engine.add_round(round_id, round);
                }
            }

            let analytics = match {
                let engine = self.engine.read().unwrap();
                engine.get_overall_analytics()
            } {
                Ok(a) => a,
                Err(e) => {
                    warn!("Failed to get analytics: {} - retrying in 30s", e);
                    sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

            if let Err(e) = self.display_analytics(&analytics) {
                warn!("Failed to display analytics: {}", e);
            }

            // Store analytics in database
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                db.set_state("analytics_summary", serde_json::json!({
                    "total_rounds_analyzed": analytics.total_rounds_analyzed,
                    "total_sol_deployed": analytics.total_sol_deployed,
                    "most_winning_square": analytics.most_winning_square,
                    "least_winning_square": analytics.least_winning_square,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await.ok();
            }

            let predictions = match {
                let engine = self.engine.read().unwrap();
                engine.predict_winning_squares(5)
            } {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to get predictions: {}", e);
                    vec![]
                }
            };

            info!("🔮 Predicted top 5 squares: {:?}", predictions);

            // Store predictions
            #[cfg(feature = "database")]
            if let Some(ref db) = self.db {
                db.set_state("analytics_predictions", serde_json::json!({
                    "top_squares": predictions,
                    "confidence": 0.5,
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })).await.ok();
            }

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

        let mut sorted_stats = analytics.square_statistics.clone();
        sorted_stats.sort_by(|a, b| b.win_frequency.partial_cmp(&a.win_frequency).unwrap());

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
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!(r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                                                                       ║
    ║    █████╗ ███╗   ██╗ █████╗ ██╗  ██╗   ██╗████████╗██╗ ██████╗███████╗║
    ║   ██╔══██╗████╗  ██║██╔══██╗██║  ╚██╗ ██╔╝╚══██╔══╝██║██╔════╝██╔════╝║
    ║   ███████║██╔██╗ ██║███████║██║   ╚████╔╝    ██║   ██║██║     ███████╗║
    ║   ██╔══██║██║╚██╗██║██╔══██║██║    ╚██╔╝     ██║   ██║██║     ╚════██║║
    ║   ██║  ██║██║ ╚████║██║  ██║███████╗██║      ██║   ██║╚██████╗███████║║
    ║   ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝      ╚═╝   ╚═╝ ╚═════╝╚══════╝║
    ║                                                                       ║
    ║                ORE Analytics Bot - Pattern Analysis                   ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#);

    info!("📊 ORE Analytics Bot Starting...");
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
    let mut analytics_bot = AnalyticsBot::new(config.analytics.clone(), Arc::new(client), db);
    
    #[cfg(not(feature = "database"))]
    let mut analytics_bot = AnalyticsBot::new(config.analytics.clone(), Arc::new(client));

    if let Err(e) = analytics_bot.start().await {
        error!("Analytics bot error: {}", e);
    }
}
