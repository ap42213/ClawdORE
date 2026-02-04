use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotMode {
    Live,        // Real transactions on mainnet
    Simulation,  // Paper trading with real data
    Monitor,     // Read-only monitoring
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Bot operation mode
    pub mode: String, // "live", "simulation", or "monitor"
    
    /// RPC endpoint URL
    pub rpc_url: String,
    
    /// WebSocket URL for real-time updates
    pub ws_url: Option<String>,
    
    /// Bot wallet keypair path
    pub keypair_path: String,
    
    /// Mining configuration
    pub mining: MiningConfig,
    
    /// Betting configuration
    pub betting: BettingConfig,
    
    /// Analytics configuration
    pub analytics: AnalyticsConfig,
    
    /// Monitoring configuration
    pub monitor: MonitorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Enable mining
    pub enabled: bool,
    
    /// Amount of SOL to deploy per round
    pub deploy_amount_sol: f64,
    
    /// Automation enabled
    pub use_automation: bool,
    
    /// Maximum SOL to keep in automation
    pub max_automation_balance: f64,
    
    /// Minimum SOL balance to maintain
    pub min_sol_balance: f64,
    
    /// Auto-claim rewards when above threshold
    pub auto_claim_threshold_ore: f64,
    
    /// Strategy for square selection
    pub strategy: String, // "random", "weighted", "hot_squares", "contrarian"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingConfig {
    /// Enable betting
    pub enabled: bool,
    
    /// Fixed SOL amount per square (if set, overrides percentage-based calculation)
    #[serde(default)]
    pub sol_per_square: Option<f64>,
    
    /// Percentage of available balance to bet per round (used if sol_per_square is None)
    pub bet_percentage: f64,
    
    /// Maximum bet amount in SOL
    pub max_bet_sol: f64,
    
    /// Minimum bet amount in SOL
    pub min_bet_sol: f64,
    
    /// Risk tolerance (0.0 = conservative, 1.0 = aggressive)
    pub risk_tolerance: f64,
    
    /// Number of squares to bet on
    pub squares_to_bet: usize,
    
    /// Strategy for betting
    pub strategy: String, // "spread", "focused", "martingale", "kelly", "ml_based"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable analytics collection
    pub enabled: bool,
    
    /// Number of past rounds to analyze
    pub history_depth: usize,
    
    /// Update interval in seconds
    pub update_interval: u64,
    
    /// Store data to database
    pub use_database: bool,
    
    /// Database path (SQLite)
    pub database_path: Option<String>,
    
    /// Export analytics to file
    pub export_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreRoundConfig {
    /// Track round results (split vs full)
    pub track_rounds: bool,
    
    /// Round duration in seconds
    pub round_duration: u64,
    
    /// Monitor motherlode events
    pub track_motherlode: bool,
    
    /// Alert on motherlode
    pub motherlode_alert: bool,
}

impl Default for OreRoundConfig {
    fn default() -> Self {
        Self {
            track_rounds: true,
            round_duration: 60,
            track_motherlode: true,
            motherlode_alert: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Enable monitoring
    pub enabled: bool,
    
    /// Check interval in seconds
    pub check_interval: u64,
    
    /// Monitor balance changes
    pub track_balance: bool,
    
    /// Monitor round changes
    pub track_rounds: bool,
    
    /// Monitor other miners
    pub track_competition: bool,
    
    /// Alert thresholds
    pub alerts: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Alert if balance drops below this (SOL)
    pub min_balance_sol: f64,
    
    /// Alert if round ends soon (seconds remaining)
    pub round_ending_warning: u64,
    
    /// Alert on large wins (ORE)
    pub large_win_threshold: f64,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            mode: "simulation".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: Some("wss://api.mainnet-beta.solana.com".to_string()),
            keypair_path: "~/.config/solana/id.json".to_string(),
            mining: MiningConfig::default(),
            betting: BettingConfig::default(),
            analytics: AnalyticsConfig::default(),
            monitor: MonitorConfig::default(),
        }
    }
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deploy_amount_sol: 0.1,
            use_automation: true,
            max_automation_balance: 1.0,
            min_sol_balance: 0.5,
            auto_claim_threshold_ore: 10.0,
            strategy: "weighted".to_string(),
        }
    }
}

impl Default for BettingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sol_per_square: Some(0.001), // Default: 0.001 SOL per square
            bet_percentage: 0.05,
            max_bet_sol: 0.5,
            min_bet_sol: 0.001,
            risk_tolerance: 0.5,
            squares_to_bet: 3,
            strategy: "spread".to_string(),
        }
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            history_depth: 100,
            update_interval: 60,
            use_database: false,
            database_path: Some("./bot_data.db".to_string()),
            export_path: Some("./analytics.json".to_string()),
        }
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: 30,
            track_balance: true,
            track_rounds: true,
            track_competition: false,
            alerts: AlertConfig::default(),
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            min_balance_sol: 0.1,
            round_ending_warning: 300, // 5 minutes
            large_win_threshold: 100.0,
        }
    }
}

impl BotConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: BotConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn to_file(&self, path: &str) -> anyhow::Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Load config from environment variables (for Railway/cloud deployment)
    pub fn from_env() -> Self {
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let ws_url = std::env::var("WS_URL").ok();
        
        let mode = std::env::var("BOT_MODE")
            .unwrap_or_else(|_| "simulation".to_string());
        
        let keypair_path = std::env::var("KEYPAIR_PATH")
            .unwrap_or_else(|_| "/app/wallet.json".to_string());

        Self {
            mode,
            rpc_url,
            ws_url,
            keypair_path,
            mining: MiningConfig::from_env(),
            betting: BettingConfig::from_env(),
            analytics: AnalyticsConfig::from_env(),
            monitor: MonitorConfig::from_env(),
        }
    }
}

impl MiningConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("MINING_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
            deploy_amount_sol: std::env::var("DEPLOY_AMOUNT_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.1),
            use_automation: std::env::var("USE_AUTOMATION")
                .map(|v| v == "true")
                .unwrap_or(true),
            max_automation_balance: std::env::var("MAX_AUTOMATION_BALANCE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            min_sol_balance: std::env::var("MIN_SOL_BALANCE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
            auto_claim_threshold_ore: std::env::var("AUTO_CLAIM_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0),
            strategy: std::env::var("MINING_STRATEGY")
                .unwrap_or_else(|_| "weighted".to_string()),
        }
    }
}

impl BettingConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("BETTING_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
            sol_per_square: std::env::var("SOL_PER_SQUARE")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(Some(0.001)), // Default 0.001 SOL per square
            bet_percentage: std::env::var("BET_PERCENTAGE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.05),
            max_bet_sol: std::env::var("MAX_BET_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
            min_bet_sol: std::env::var("MIN_BET_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.001),
            risk_tolerance: std::env::var("RISK_TOLERANCE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
            squares_to_bet: std::env::var("SQUARES_TO_BET")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            strategy: std::env::var("BETTING_STRATEGY")
                .unwrap_or_else(|_| "spread".to_string()),
        }
    }
}

impl AnalyticsConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("ANALYTICS_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
            history_depth: std::env::var("HISTORY_DEPTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            update_interval: std::env::var("UPDATE_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            use_database: std::env::var("USE_DATABASE")
                .map(|v| v == "true")
                .unwrap_or(false),
            database_path: std::env::var("DATABASE_PATH").ok(),
            export_path: std::env::var("EXPORT_PATH").ok(),
        }
    }
}

impl MonitorConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: true,
            check_interval: std::env::var("CHECK_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            track_balance: true,
            track_rounds: true,
            track_competition: std::env::var("TRACK_COMPETITION")
                .map(|v| v == "true")
                .unwrap_or(false),
            alerts: AlertConfig::from_env(),
        }
    }
}

impl AlertConfig {
    pub fn from_env() -> Self {
        Self {
            min_balance_sol: std::env::var("MIN_BALANCE_ALERT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.1),
            round_ending_warning: std::env::var("ROUND_WARNING_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            large_win_threshold: std::env::var("LARGE_WIN_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
        }
    }
}
