use crate::error::{BotError, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::env;

#[cfg(feature = "database")]
use sqlx::FromRow;

/// Shared database for bot coordination
/// Uses PostgreSQL on Railway for persistent shared state

// Database schema (will be created automatically)
pub const SCHEMA: &str = r#"
-- Rounds table: stores all round data
CREATE TABLE IF NOT EXISTS rounds (
    round_id BIGINT PRIMARY KEY,
    start_slot BIGINT,
    end_slot BIGINT,
    winning_square SMALLINT,
    total_deployed BIGINT DEFAULT 0,
    deployed_squares BIGINT[] DEFAULT ARRAY[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]::BIGINT[],
    total_winnings BIGINT DEFAULT 0,
    total_vaulted BIGINT DEFAULT 0,
    motherlode BOOLEAN DEFAULT FALSE,
    num_deploys INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Miners table: tracks all miner accounts
CREATE TABLE IF NOT EXISTS miners (
    address TEXT PRIMARY KEY,
    total_deployed BIGINT DEFAULT 0,
    total_claimed_sol BIGINT DEFAULT 0,
    total_claimed_ore BIGINT DEFAULT 0,
    deploy_count INTEGER DEFAULT 0,
    claim_count INTEGER DEFAULT 0,
    automation_enabled BOOLEAN DEFAULT FALSE,
    favorite_squares INTEGER[] DEFAULT ARRAY[]::INTEGER[],
    win_rate REAL DEFAULT 0.0,
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Transactions table: recent ORE transactions
CREATE TABLE IF NOT EXISTS transactions (
    signature TEXT PRIMARY KEY,
    slot BIGINT,
    block_time TIMESTAMPTZ,
    instruction_type TEXT,
    signer TEXT,
    round_id BIGINT,
    amount_lamports BIGINT,
    squares INTEGER[],
    success BOOLEAN,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Bot state table: coordination signals between bots
CREATE TABLE IF NOT EXISTS bot_state (
    key TEXT PRIMARY KEY,
    value JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Analytics snapshots
CREATE TABLE IF NOT EXISTS analytics_snapshots (
    id SERIAL PRIMARY KEY,
    snapshot_type TEXT,
    data JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Signals table: for bot-to-bot communication
CREATE TABLE IF NOT EXISTS signals (
    id SERIAL PRIMARY KEY,
    signal_type TEXT NOT NULL,
    source_bot TEXT NOT NULL,
    target_bot TEXT,  -- NULL means broadcast to all
    payload JSONB,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_transactions_signer ON transactions(signer);
CREATE INDEX IF NOT EXISTS idx_transactions_round ON transactions(round_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(instruction_type);
CREATE INDEX IF NOT EXISTS idx_signals_unprocessed ON signals(processed, target_bot) WHERE NOT processed;
CREATE INDEX IF NOT EXISTS idx_rounds_completed ON rounds(completed_at) WHERE completed_at IS NOT NULL;
"#;

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
}

impl DbConfig {
    pub fn from_env() -> Option<Self> {
        // Railway provides DATABASE_URL automatically when you add PostgreSQL
        env::var("DATABASE_URL").ok().map(|url| Self { url })
    }
}

/// Round data stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "database", derive(FromRow))]
pub struct DbRound {
    pub round_id: i64,
    pub start_slot: Option<i64>,
    pub end_slot: Option<i64>,
    pub winning_square: Option<i16>,
    pub total_deployed: i64,
    pub deployed_squares: Vec<i64>,
    pub total_winnings: i64,
    pub total_vaulted: i64,
    pub motherlode: bool,
    pub num_deploys: i32,
    #[cfg_attr(feature = "database", sqlx(skip))]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Miner data stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "database", derive(FromRow))]
pub struct DbMiner {
    pub address: String,
    pub total_deployed: i64,
    pub total_claimed_sol: i64,
    pub total_claimed_ore: i64,
    pub deploy_count: i32,
    pub claim_count: i32,
    pub automation_enabled: bool,
    pub favorite_squares: Vec<i32>,
    pub win_rate: f32,
}

/// Transaction data stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTransaction {
    pub signature: String,
    pub slot: i64,
    pub block_time: Option<chrono::DateTime<chrono::Utc>>,
    pub instruction_type: String,
    pub signer: String,
    pub round_id: Option<i64>,
    pub amount_lamports: Option<i64>,
    pub squares: Vec<i32>,
    pub success: bool,
}

/// Signal for bot-to-bot communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: Option<i32>,
    pub signal_type: SignalType,
    pub source_bot: String,
    pub target_bot: Option<String>,
    pub payload: serde_json::Value,
}

/// Signal types for bot coordination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// New round started
    RoundStarted,
    /// Round is ending soon (< 10 seconds)
    RoundEndingSoon,
    /// Round completed with results
    RoundCompleted,
    /// Motherlode detected
    MotherlodeAlert,
    /// Deploy opportunity identified
    DeployOpportunity,
    /// Claim rewards recommended
    ClaimRecommended,
    /// Hot square identified (high activity)
    HotSquare,
    /// Cold square identified (low activity, contrarian opportunity)
    ColdSquare,
    /// Price movement alert
    PriceAlert,
    /// Bot health check
    Heartbeat,
    /// Custom signal
    Custom,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalType::RoundStarted => write!(f, "round_started"),
            SignalType::RoundEndingSoon => write!(f, "round_ending_soon"),
            SignalType::RoundCompleted => write!(f, "round_completed"),
            SignalType::MotherlodeAlert => write!(f, "motherlode_alert"),
            SignalType::DeployOpportunity => write!(f, "deploy_opportunity"),
            SignalType::ClaimRecommended => write!(f, "claim_recommended"),
            SignalType::HotSquare => write!(f, "hot_square"),
            SignalType::ColdSquare => write!(f, "cold_square"),
            SignalType::PriceAlert => write!(f, "price_alert"),
            SignalType::Heartbeat => write!(f, "heartbeat"),
            SignalType::Custom => write!(f, "custom"),
        }
    }
}

/// Shared database client using raw SQL (no heavy ORM dependencies)
/// In production, you'd use sqlx with the database feature enabled
pub struct SharedDb {
    config: DbConfig,
    #[cfg(feature = "database")]
    pool: sqlx::PgPool,
}

impl SharedDb {
    /// Create new database connection
    #[cfg(feature = "database")]
    pub async fn connect() -> Result<Self> {
        let config = DbConfig::from_env()
            .ok_or_else(|| BotError::Other("DATABASE_URL not set".to_string()))?;
        
        info!("🔌 Connecting to database...");
        
        let pool = sqlx::PgPool::connect(&config.url)
            .await
            .map_err(|e| BotError::Other(format!("Database connection failed: {}", e)))?;
        
        info!("✅ Database connected");
        
        let db = Self { config, pool };
        db.init_schema().await?;
        
        Ok(db)
    }

    /// Initialize database schema
    #[cfg(feature = "database")]
    pub async fn init_schema(&self) -> Result<()> {
        info!("📋 Initializing database schema...");
        
        sqlx::query(SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| BotError::Other(format!("Schema init failed: {}", e)))?;
        
        info!("✅ Database schema ready");
        Ok(())
    }

    /// Store a round
    #[cfg(feature = "database")]
    pub async fn upsert_round(&self, round: &DbRound) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO rounds (round_id, start_slot, end_slot, winning_square, total_deployed, 
                               deployed_squares, total_winnings, total_vaulted, motherlode, 
                               num_deploys, completed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (round_id) DO UPDATE SET
                start_slot = EXCLUDED.start_slot,
                end_slot = EXCLUDED.end_slot,
                winning_square = EXCLUDED.winning_square,
                total_deployed = EXCLUDED.total_deployed,
                deployed_squares = EXCLUDED.deployed_squares,
                total_winnings = EXCLUDED.total_winnings,
                total_vaulted = EXCLUDED.total_vaulted,
                motherlode = EXCLUDED.motherlode,
                num_deploys = EXCLUDED.num_deploys,
                completed_at = EXCLUDED.completed_at
        "#)
        .bind(round.round_id)
        .bind(round.start_slot)
        .bind(round.end_slot)
        .bind(round.winning_square)
        .bind(round.total_deployed)
        .bind(&round.deployed_squares)
        .bind(round.total_winnings)
        .bind(round.total_vaulted)
        .bind(round.motherlode)
        .bind(round.num_deploys)
        .bind(round.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to upsert round: {}", e)))?;
        
        Ok(())
    }

    /// Get recent rounds
    #[cfg(feature = "database")]
    pub async fn get_recent_rounds(&self, limit: i32) -> Result<Vec<i64>> {
        let round_ids: Vec<(i64,)> = sqlx::query_as(
            "SELECT round_id FROM rounds ORDER BY round_id DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get rounds: {}", e)))?;
        
        Ok(round_ids.into_iter().map(|(id,)| id).collect())
    }

    /// Store/update a miner
    #[cfg(feature = "database")]
    pub async fn upsert_miner(&self, miner: &DbMiner) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO miners (address, total_deployed, total_claimed_sol, total_claimed_ore,
                               deploy_count, claim_count, automation_enabled, favorite_squares, 
                               win_rate, last_seen)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            ON CONFLICT (address) DO UPDATE SET
                total_deployed = miners.total_deployed + EXCLUDED.total_deployed,
                total_claimed_sol = miners.total_claimed_sol + EXCLUDED.total_claimed_sol,
                total_claimed_ore = miners.total_claimed_ore + EXCLUDED.total_claimed_ore,
                deploy_count = miners.deploy_count + 1,
                claim_count = CASE WHEN EXCLUDED.total_claimed_sol > 0 OR EXCLUDED.total_claimed_ore > 0 
                              THEN miners.claim_count + 1 ELSE miners.claim_count END,
                automation_enabled = EXCLUDED.automation_enabled,
                last_seen = NOW()
        "#)
        .bind(&miner.address)
        .bind(miner.total_deployed)
        .bind(miner.total_claimed_sol)
        .bind(miner.total_claimed_ore)
        .bind(miner.deploy_count)
        .bind(miner.claim_count)
        .bind(miner.automation_enabled)
        .bind(&miner.favorite_squares)
        .bind(miner.win_rate)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to upsert miner: {}", e)))?;
        
        Ok(())
    }

    /// Store a transaction
    #[cfg(feature = "database")]
    pub async fn insert_transaction(&self, tx: &DbTransaction) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO transactions (signature, slot, block_time, instruction_type, signer,
                                     round_id, amount_lamports, squares, success)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (signature) DO NOTHING
        "#)
        .bind(&tx.signature)
        .bind(tx.slot)
        .bind(tx.block_time)
        .bind(&tx.instruction_type)
        .bind(&tx.signer)
        .bind(tx.round_id)
        .bind(tx.amount_lamports)
        .bind(&tx.squares)
        .bind(tx.success)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to insert transaction: {}", e)))?;
        
        Ok(())
    }

    /// Send a signal to other bots
    #[cfg(feature = "database")]
    pub async fn send_signal(&self, signal: &Signal) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO signals (signal_type, source_bot, target_bot, payload)
            VALUES ($1, $2, $3, $4)
        "#)
        .bind(signal.signal_type.to_string())
        .bind(&signal.source_bot)
        .bind(&signal.target_bot)
        .bind(&signal.payload)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to send signal: {}", e)))?;
        
        Ok(())
    }

    /// Get pending signals for a bot
    #[cfg(feature = "database")]
    pub async fn get_pending_signals(&self, bot_name: &str) -> Result<Vec<(i32, String, String, Option<String>, serde_json::Value)>> {
        let signals = sqlx::query_as::<_, (i32, String, String, Option<String>, serde_json::Value)>(r#"
            SELECT id, signal_type, source_bot, target_bot, payload
            FROM signals 
            WHERE NOT processed AND (target_bot IS NULL OR target_bot = $1)
            ORDER BY created_at ASC
        "#)
        .bind(bot_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get signals: {}", e)))?;
        
        Ok(signals)
    }

    /// Mark signals as processed
    #[cfg(feature = "database")]
    pub async fn mark_signals_processed(&self, signal_ids: &[i32]) -> Result<()> {
        sqlx::query("UPDATE signals SET processed = TRUE WHERE id = ANY($1)")
            .bind(signal_ids)
            .execute(&self.pool)
            .await
            .map_err(|e| BotError::Other(format!("Failed to mark signals: {}", e)))?;
        
        Ok(())
    }

    /// Store bot state (key-value)
    #[cfg(feature = "database")]
    pub async fn set_state(&self, key: &str, value: serde_json::Value) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO bot_state (key, value, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()
        "#)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to set state: {}", e)))?;
        
        Ok(())
    }

    /// Get bot state
    #[cfg(feature = "database")]
    pub async fn get_state(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let result = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT value FROM bot_state WHERE key = $1"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get state: {}", e)))?;
        
        Ok(result)
    }

    /// Get square statistics from historical data
    #[cfg(feature = "database")]
    pub async fn get_square_stats(&self) -> Result<Vec<(i64, i64, i64)>> {
        let stats = sqlx::query_as::<_, (i64, i64, i64)>(r#"
            SELECT 
                sq.square_id::bigint,
                COALESCE(SUM(sq.deployed), 0)::bigint as total_deployed,
                COUNT(CASE WHEN r.winning_square = sq.square_id THEN 1 END)::bigint as wins
            FROM rounds r
            CROSS JOIN LATERAL unnest(r.deployed_squares) WITH ORDINALITY AS sq(deployed, square_id)
            GROUP BY sq.square_id
            ORDER BY sq.square_id
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get square stats: {}", e)))?;
        
        Ok(stats)
    }

    /// Get top miners (returns address and total deployed)
    #[cfg(feature = "database")]
    pub async fn get_top_miners(&self, limit: i32) -> Result<Vec<(String, i64)>> {
        let miners = sqlx::query_as::<_, (String, i64)>(
            "SELECT address, total_deployed FROM miners ORDER BY total_deployed DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get top miners: {}", e)))?;
        
        Ok(miners)
    }

    /// Clean up old data
    #[cfg(feature = "database")]
    pub async fn cleanup_old_data(&self, days: i32) -> Result<()> {
        // Clean old transactions
        sqlx::query("DELETE FROM transactions WHERE created_at < NOW() - INTERVAL '$1 days'")
            .bind(days)
            .execute(&self.pool)
            .await
            .ok();
        
        // Clean processed signals older than 1 day
        sqlx::query("DELETE FROM signals WHERE processed AND created_at < NOW() - INTERVAL '1 day'")
            .execute(&self.pool)
            .await
            .ok();
        
        Ok(())
    }
}

// Fallback implementation when database feature is not enabled
#[cfg(not(feature = "database"))]
impl SharedDb {
    pub async fn connect() -> Result<Self> {
        Err(BotError::Other("Database feature not enabled. Rebuild with --features database".to_string()))
    }
}

/// Check if database is available
pub fn is_database_available() -> bool {
    env::var("DATABASE_URL").is_ok()
}

/// Helper to create signals
impl Signal {
    pub fn new(signal_type: SignalType, source: &str, payload: serde_json::Value) -> Self {
        Self {
            id: None,
            signal_type,
            source_bot: source.to_string(),
            target_bot: None,
            payload,
        }
    }

    pub fn to_bot(mut self, target: &str) -> Self {
        self.target_bot = Some(target.to_string());
        self
    }

    pub fn round_started(source: &str, round_id: u64) -> Self {
        Self::new(
            SignalType::RoundStarted,
            source,
            serde_json::json!({ "round_id": round_id }),
        )
    }

    pub fn round_completed(source: &str, round_id: u64, winning_square: u8, motherlode: bool) -> Self {
        Self::new(
            SignalType::RoundCompleted,
            source,
            serde_json::json!({
                "round_id": round_id,
                "winning_square": winning_square,
                "motherlode": motherlode
            }),
        )
    }

    pub fn deploy_opportunity(source: &str, squares: Vec<usize>, reason: &str) -> Self {
        Self::new(
            SignalType::DeployOpportunity,
            source,
            serde_json::json!({
                "squares": squares,
                "reason": reason
            }),
        )
    }

    pub fn hot_square(source: &str, square: usize, amount: u64) -> Self {
        Self::new(
            SignalType::HotSquare,
            source,
            serde_json::json!({
                "square": square,
                "amount_lamports": amount
            }),
        )
    }

    pub fn cold_square(source: &str, square: usize) -> Self {
        Self::new(
            SignalType::ColdSquare,
            source,
            serde_json::json!({ "square": square }),
        )
    }
}
