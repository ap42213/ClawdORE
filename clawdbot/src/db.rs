use crate::error::{BotError, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::env;

#[cfg(feature = "database")]
use sqlx::FromRow;

/// Shared database for bot coordination
/// Uses PostgreSQL on Railway for persistent shared state

// Database schema statements (executed one at a time)
pub const SCHEMA_STATEMENTS: &[&str] = &[
    // Rounds table
    r#"CREATE TABLE IF NOT EXISTS rounds (
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
    )"#,
    
    // Miners table
    r#"CREATE TABLE IF NOT EXISTS miners (
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
    )"#,
    
    // Transactions table
    r#"CREATE TABLE IF NOT EXISTS transactions (
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
    )"#,
    
    // Bot state table
    r#"CREATE TABLE IF NOT EXISTS bot_state (
        key TEXT PRIMARY KEY,
        value JSONB,
        updated_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Analytics snapshots
    r#"CREATE TABLE IF NOT EXISTS analytics_snapshots (
        id SERIAL PRIMARY KEY,
        snapshot_type TEXT,
        data JSONB,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Signals table
    r#"CREATE TABLE IF NOT EXISTS signals (
        id SERIAL PRIMARY KEY,
        signal_type TEXT NOT NULL,
        source_bot TEXT NOT NULL,
        target_bot TEXT,
        payload JSONB,
        processed BOOLEAN DEFAULT FALSE,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Square statistics - learned patterns for each square
    r#"CREATE TABLE IF NOT EXISTS square_stats (
        square_id SMALLINT PRIMARY KEY,
        total_wins INTEGER DEFAULT 0,
        total_rounds INTEGER DEFAULT 0,
        total_deployed BIGINT DEFAULT 0,
        win_rate REAL DEFAULT 0.04,
        edge REAL DEFAULT 0.0,
        recent_wins INTEGER DEFAULT 0,
        streak INTEGER DEFAULT 0,
        avg_competition BIGINT DEFAULT 0,
        updated_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Whale tracking - learned whale behavior
    r#"CREATE TABLE IF NOT EXISTS whales (
        address TEXT PRIMARY KEY,
        total_deployed BIGINT DEFAULT 0,
        deploy_count INTEGER DEFAULT 0,
        favorite_squares INTEGER[] DEFAULT ARRAY[]::INTEGER[],
        avg_deploy_size BIGINT DEFAULT 0,
        last_seen TIMESTAMPTZ DEFAULT NOW(),
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Strategy performance - track which strategies work
    r#"CREATE TABLE IF NOT EXISTS strategy_performance (
        id SERIAL PRIMARY KEY,
        strategy_name TEXT NOT NULL,
        round_id BIGINT,
        recommended_squares INTEGER[],
        winning_square SMALLINT,
        hit BOOLEAN DEFAULT FALSE,
        confidence REAL,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Indexes
    "CREATE INDEX IF NOT EXISTS idx_transactions_signer ON transactions(signer)",
    "CREATE INDEX IF NOT EXISTS idx_transactions_round ON transactions(round_id)",
    "CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(instruction_type)",
    "CREATE INDEX IF NOT EXISTS idx_signals_unprocessed ON signals(processed, target_bot) WHERE NOT processed",
    "CREATE INDEX IF NOT EXISTS idx_rounds_completed ON rounds(completed_at) WHERE completed_at IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_strategy_performance_strategy ON strategy_performance(strategy_name)",
    "CREATE INDEX IF NOT EXISTS idx_whales_deployed ON whales(total_deployed DESC)",
];

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
        
        for (i, statement) in SCHEMA_STATEMENTS.iter().enumerate() {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|e| BotError::Other(format!("Schema statement {} failed: {}", i + 1, e)))?;
        }
        
        info!("✅ Database schema ready ({} tables/indexes created)", SCHEMA_STATEMENTS.len());
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

    // ==================== LEARNING METHODS ====================

    /// Update square statistics from a completed round
    #[cfg(feature = "database")]
    pub async fn update_square_stats(&self, winning_square: i16, deployed: &[i64; 25]) -> Result<()> {
        for (i, &amount) in deployed.iter().enumerate() {
            let is_winner = i as i16 == winning_square;
            let streak_update = if is_winner { 1 } else { -1 };
            
            sqlx::query(r#"
                INSERT INTO square_stats (square_id, total_wins, total_rounds, total_deployed, streak, updated_at)
                VALUES ($1, $2, 1, $3, $4, NOW())
                ON CONFLICT (square_id) DO UPDATE SET
                    total_wins = square_stats.total_wins + $2,
                    total_rounds = square_stats.total_rounds + 1,
                    total_deployed = square_stats.total_deployed + $3,
                    win_rate = (square_stats.total_wins + $2)::REAL / (square_stats.total_rounds + 1)::REAL,
                    edge = (square_stats.total_wins + $2)::REAL / (square_stats.total_rounds + 1)::REAL - 0.04,
                    streak = CASE 
                        WHEN $2 = 1 AND square_stats.streak >= 0 THEN square_stats.streak + 1
                        WHEN $2 = 0 AND square_stats.streak <= 0 THEN square_stats.streak - 1
                        ELSE $4 
                    END,
                    avg_competition = (square_stats.total_deployed + $3) / (square_stats.total_rounds + 1),
                    updated_at = NOW()
            "#)
            .bind(i as i16)
            .bind(if is_winner { 1i32 } else { 0i32 })
            .bind(amount)
            .bind(streak_update)
            .execute(&self.pool)
            .await
            .ok();
        }
        Ok(())
    }

    /// Load persisted square stats for strategy engine
    #[cfg(feature = "database")]
    pub async fn load_square_stats(&self) -> Result<Vec<(i16, i32, i32, i64, f32, f32, i32, i64)>> {
        let stats = sqlx::query_as::<_, (i16, i32, i32, i64, f32, f32, i32, i64)>(r#"
            SELECT square_id, total_wins, total_rounds, total_deployed, 
                   win_rate, edge, streak, avg_competition
            FROM square_stats
            ORDER BY square_id
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load square stats: {}", e)))?;
        
        Ok(stats)
    }

    /// Track/update a whale deployer
    #[cfg(feature = "database")]
    pub async fn track_whale(&self, address: &str, amount: i64, squares: &[i32]) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO whales (address, total_deployed, deploy_count, favorite_squares, avg_deploy_size, last_seen)
            VALUES ($1, $2, 1, $3, $2, NOW())
            ON CONFLICT (address) DO UPDATE SET
                total_deployed = whales.total_deployed + $2,
                deploy_count = whales.deploy_count + 1,
                favorite_squares = (
                    SELECT ARRAY(
                        SELECT DISTINCT unnest(whales.favorite_squares || $3)
                        LIMIT 10
                    )
                ),
                avg_deploy_size = (whales.total_deployed + $2) / (whales.deploy_count + 1),
                last_seen = NOW()
        "#)
        .bind(address)
        .bind(amount)
        .bind(squares)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to track whale: {}", e)))?;
        
        Ok(())
    }

    /// Load whale data for strategy engine
    #[cfg(feature = "database")]
    pub async fn load_whales(&self, min_deployed: i64) -> Result<Vec<(String, i64, Vec<i32>)>> {
        let whales = sqlx::query_as::<_, (String, i64, Vec<i32>)>(r#"
            SELECT address, total_deployed, favorite_squares
            FROM whales
            WHERE total_deployed >= $1
            ORDER BY total_deployed DESC
            LIMIT 50
        "#)
        .bind(min_deployed)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load whales: {}", e)))?;
        
        Ok(whales)
    }

    /// Record strategy performance for learning
    #[cfg(feature = "database")]
    pub async fn record_strategy_performance(
        &self, 
        strategy_name: &str, 
        round_id: i64, 
        recommended_squares: &[i32],
        winning_square: i16,
        confidence: f32
    ) -> Result<()> {
        let hit = recommended_squares.contains(&(winning_square as i32));
        
        sqlx::query(r#"
            INSERT INTO strategy_performance 
                (strategy_name, round_id, recommended_squares, winning_square, hit, confidence)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#)
        .bind(strategy_name)
        .bind(round_id)
        .bind(recommended_squares)
        .bind(winning_square)
        .bind(hit)
        .bind(confidence)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record strategy: {}", e)))?;
        
        Ok(())
    }

    /// Get strategy success rates
    #[cfg(feature = "database")]
    pub async fn get_strategy_performance(&self) -> Result<Vec<(String, i64, i64, f64)>> {
        let perf = sqlx::query_as::<_, (String, i64, i64, f64)>(r#"
            SELECT 
                strategy_name,
                COUNT(*) as total_predictions,
                SUM(CASE WHEN hit THEN 1 ELSE 0 END) as hits,
                AVG(CASE WHEN hit THEN 1.0 ELSE 0.0 END) as hit_rate
            FROM strategy_performance
            GROUP BY strategy_name
            ORDER BY hit_rate DESC
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get strategy performance: {}", e)))?;
        
        Ok(perf)
    }

    /// Load historical rounds for strategy engine initialization
    #[cfg(feature = "database")]
    pub async fn load_round_history(&self, limit: i32) -> Result<Vec<(i64, i16, Vec<i64>, i64, bool)>> {
        let rounds = sqlx::query_as::<_, (i64, i16, Vec<i64>, i64, bool)>(r#"
            SELECT round_id, COALESCE(winning_square, -1), deployed_squares, total_deployed, motherlode
            FROM rounds
            WHERE winning_square IS NOT NULL
            ORDER BY round_id DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load round history: {}", e)))?;
        
        Ok(rounds)
    }

    /// Update round with winning square (when round completes)
    #[cfg(feature = "database")]
    pub async fn complete_round(&self, round_id: i64, winning_square: i16, motherlode: bool) -> Result<()> {
        sqlx::query(r#"
            UPDATE rounds 
            SET winning_square = $2, motherlode = $3, completed_at = NOW()
            WHERE round_id = $1
        "#)
        .bind(round_id)
        .bind(winning_square)
        .bind(motherlode)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to complete round: {}", e)))?;
        
        Ok(())
    }

    /// Get learning summary
    #[cfg(feature = "database")]
    pub async fn get_learning_summary(&self) -> Result<serde_json::Value> {
        let total_rounds: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rounds WHERE winning_square IS NOT NULL")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let total_whales: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM whales")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let total_txs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transactions")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));

        Ok(serde_json::json!({
            "completed_rounds": total_rounds.0,
            "tracked_whales": total_whales.0,
            "transactions_analyzed": total_txs.0
        }))
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
