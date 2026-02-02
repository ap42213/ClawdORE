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
    
    /// Percentage of available balance to bet per round
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
            bet_percentage: 0.05,
            max_bet_sol: 0.5,
            min_bet_sol: 0.01,
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
}
