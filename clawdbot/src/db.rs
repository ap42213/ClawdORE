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
    
    // ALL player performance - learn from everyone, not just whales
    r#"CREATE TABLE IF NOT EXISTS player_performance (
        address TEXT PRIMARY KEY,
        total_deployed BIGINT DEFAULT 0,
        total_won BIGINT DEFAULT 0,
        total_rounds INTEGER DEFAULT 0,
        wins INTEGER DEFAULT 0,
        avg_squares_per_deploy REAL DEFAULT 0.0,
        preferred_square_count SMALLINT DEFAULT 5,
        avg_deploy_size BIGINT DEFAULT 0,
        roi REAL DEFAULT 0.0,
        ore_per_sol REAL DEFAULT 0.0,
        last_deploy_slot BIGINT,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Square count statistics - which counts work best
    r#"CREATE TABLE IF NOT EXISTS square_count_stats (
        square_count SMALLINT PRIMARY KEY,
        times_used INTEGER DEFAULT 0,
        times_won INTEGER DEFAULT 0,
        total_deployed BIGINT DEFAULT 0,
        total_won BIGINT DEFAULT 0,
        avg_ore_earned REAL DEFAULT 0.0,
        win_rate REAL DEFAULT 0.0,
        roi REAL DEFAULT 0.0,
        updated_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Round conditions history - learn optimal conditions
    r#"CREATE TABLE IF NOT EXISTS round_conditions (
        round_id BIGINT PRIMARY KEY,
        total_deployed BIGINT,
        num_deployers INTEGER,
        avg_deploy_size BIGINT,
        competition_level TEXT,
        expected_ore_multiplier REAL,
        squares_with_deploys SMALLINT,
        our_deployed BOOLEAN DEFAULT FALSE,
        our_won BOOLEAN DEFAULT FALSE,
        our_ore_earned REAL DEFAULT 0.0,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Comprehensive win records - THE KEY TABLE
    r#"CREATE TABLE IF NOT EXISTS win_records (
        id SERIAL PRIMARY KEY,
        round_id BIGINT NOT NULL,
        winner_address TEXT NOT NULL,
        winning_square SMALLINT NOT NULL,
        amount_bet BIGINT,
        amount_won BIGINT,
        squares_bet INTEGER[],
        num_squares SMALLINT,
        total_round_sol BIGINT,
        num_deployers INTEGER,
        is_motherlode BOOLEAN DEFAULT FALSE,
        is_full_ore BOOLEAN DEFAULT FALSE,
        ore_earned REAL,
        competition_on_square BIGINT,
        winner_share_pct REAL,
        slot BIGINT,
        block_time TIMESTAMPTZ,
        created_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Detected strategies - what patterns work
    r#"CREATE TABLE IF NOT EXISTS detected_strategies (
        id SERIAL PRIMARY KEY,
        name TEXT UNIQUE,
        description TEXT,
        sample_size INTEGER,
        win_rate REAL,
        avg_roi REAL,
        avg_ore_per_round REAL,
        square_count SMALLINT,
        bet_size_sol REAL,
        target_competition TEXT,
        preferred_squares INTEGER[],
        play_motherlode BOOLEAN,
        confidence REAL,
        consistent BOOLEAN,
        example_players TEXT[],
        updated_at TIMESTAMPTZ DEFAULT NOW()
    )"#,
    
    // Test-20 tracking: Server-side tracking of best 20 square picks
    r#"CREATE TABLE IF NOT EXISTS test_20_rounds (
        round_id BIGINT PRIMARY KEY,
        betting_squares INTEGER[] NOT NULL,
        skipping_squares INTEGER[] NOT NULL,
        winning_square SMALLINT,
        is_hit BOOLEAN,
        confidence REAL,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        completed_at TIMESTAMPTZ
    )"#,
    
    // Indexes
    "CREATE INDEX IF NOT EXISTS idx_test_20_completed ON test_20_rounds(completed_at) WHERE completed_at IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_transactions_signer ON transactions(signer)",
    "CREATE INDEX IF NOT EXISTS idx_transactions_round ON transactions(round_id)",
    "CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(instruction_type)",
    "CREATE INDEX IF NOT EXISTS idx_signals_unprocessed ON signals(processed, target_bot) WHERE NOT processed",
    "CREATE INDEX IF NOT EXISTS idx_rounds_completed ON rounds(completed_at) WHERE completed_at IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_strategy_performance_strategy ON strategy_performance(strategy_name)",
    "CREATE INDEX IF NOT EXISTS idx_whales_deployed ON whales(total_deployed DESC)",
    "CREATE INDEX IF NOT EXISTS idx_player_performance_roi ON player_performance(roi DESC)",
    "CREATE INDEX IF NOT EXISTS idx_player_performance_wins ON player_performance(wins DESC)",
    "CREATE INDEX IF NOT EXISTS idx_round_conditions_competition ON round_conditions(competition_level)",
    "CREATE INDEX IF NOT EXISTS idx_win_records_winner ON win_records(winner_address)",
    "CREATE INDEX IF NOT EXISTS idx_win_records_round ON win_records(round_id)",
    "CREATE INDEX IF NOT EXISTS idx_win_records_full_ore ON win_records(is_full_ore) WHERE is_full_ore",
    "CREATE INDEX IF NOT EXISTS idx_win_records_motherlode ON win_records(is_motherlode) WHERE is_motherlode",
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
    /// Bet/deploy transaction placed
    BetPlaced,
    /// Error occurred
    Error,
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
            SignalType::BetPlaced => write!(f, "bet_placed"),
            SignalType::Error => write!(f, "error"),
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

    // ==================== TEST-20 TRACKING METHODS ====================

    /// Lock test-20 picks at round start (pick best 20 squares to bet on)
    #[cfg(feature = "database")]
    pub async fn lock_test_20_round(&self, round_id: i64, betting_squares: &[i32], skipping_squares: &[i32], confidence: f32) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO test_20_rounds (round_id, betting_squares, skipping_squares, confidence)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (round_id) DO NOTHING
        "#)
        .bind(round_id)
        .bind(betting_squares)
        .bind(skipping_squares)
        .bind(confidence)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to lock test-20 round: {}", e)))?;
        
        Ok(())
    }

    /// Complete test-20 round with result
    #[cfg(feature = "database")]
    pub async fn complete_test_20_round(&self, round_id: i64, winning_square: i16) -> Result<bool> {
        // Get the locked round to check if hit
        let result = sqlx::query_as::<_, (Vec<i32>,)>(r#"
            SELECT betting_squares FROM test_20_rounds WHERE round_id = $1
        "#)
        .bind(round_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get test-20 round: {}", e)))?;
        
        if let Some((betting_squares,)) = result {
            let is_hit = betting_squares.contains(&(winning_square as i32));
            
            sqlx::query(r#"
                UPDATE test_20_rounds 
                SET winning_square = $2, is_hit = $3, completed_at = NOW()
                WHERE round_id = $1
            "#)
            .bind(round_id)
            .bind(winning_square)
            .bind(is_hit)
            .execute(&self.pool)
            .await
            .map_err(|e| BotError::Other(format!("Failed to complete test-20 round: {}", e)))?;
            
            Ok(is_hit)
        } else {
            // Round wasn't tracked
            Err(BotError::Other(format!("Test-20 round {} not found", round_id)))
        }
    }

    /// Get test-20 statistics
    #[cfg(feature = "database")]
    pub async fn get_test_20_stats(&self) -> Result<serde_json::Value> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM test_20_rounds WHERE is_hit IS NOT NULL"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,));
        
        let hits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM test_20_rounds WHERE is_hit = true"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,));
        
        let win_rate = if total.0 > 0 { 
            hits.0 as f64 / total.0 as f64 * 100.0 
        } else { 
            0.0 
        };
        
        // Get recent results
        let recent = sqlx::query_as::<_, (i64, i16, bool, Vec<i32>, Vec<i32>)>(r#"
            SELECT round_id, COALESCE(winning_square, 0), COALESCE(is_hit, false), 
                   betting_squares, skipping_squares
            FROM test_20_rounds 
            WHERE is_hit IS NOT NULL 
            ORDER BY completed_at DESC 
            LIMIT 50
        "#)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        
        let recent_results: Vec<serde_json::Value> = recent.iter().map(|(round_id, winning, is_hit, betting, skipping)| {
            serde_json::json!({
                "round_id": round_id,
                "winning_square": winning,
                "is_hit": is_hit,
                "betting_squares": betting,
                "skipping_squares": skipping
            })
        }).collect();
        
        Ok(serde_json::json!({
            "total": total.0,
            "hits": hits.0,
            "misses": total.0 - hits.0,
            "win_rate": win_rate,
            "baseline": 80.0,
            "edge": win_rate - 80.0,
            "recent_results": recent_results
        }))
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

    // ===== ALL PLAYER LEARNING METHODS =====

    /// Record a player's deploy for learning (ALL players, not just whales)
    #[cfg(feature = "database")]
    pub async fn record_player_deploy(
        &self,
        address: &str,
        amount_lamports: i64,
        square_count: i16,
        slot: i64,
    ) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO player_performance (address, total_deployed, total_rounds, avg_squares_per_deploy, avg_deploy_size, last_deploy_slot)
            VALUES ($1, $2, 1, $3, $2, $4)
            ON CONFLICT (address) DO UPDATE SET
                total_deployed = player_performance.total_deployed + $2,
                total_rounds = player_performance.total_rounds + 1,
                avg_squares_per_deploy = (player_performance.avg_squares_per_deploy * player_performance.total_rounds + $3) / (player_performance.total_rounds + 1),
                avg_deploy_size = (player_performance.total_deployed + $2) / (player_performance.total_rounds + 1),
                last_deploy_slot = $4,
                updated_at = NOW()
        "#)
        .bind(address)
        .bind(amount_lamports)
        .bind(square_count as f32)
        .bind(slot)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record player deploy: {}", e)))?;
        
        Ok(())
    }

    /// Record a player's win
    #[cfg(feature = "database")]
    pub async fn record_player_win(
        &self,
        address: &str,
        amount_won_lamports: i64,
    ) -> Result<()> {
        sqlx::query(r#"
            UPDATE player_performance SET
                total_won = total_won + $2,
                wins = wins + 1,
                roi = CASE WHEN total_deployed > 0 
                    THEN (total_won + $2 - total_deployed)::REAL / total_deployed::REAL 
                    ELSE 0.0 END,
                updated_at = NOW()
            WHERE address = $1
        "#)
        .bind(address)
        .bind(amount_won_lamports)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record player win: {}", e)))?;
        
        Ok(())
    }

    /// Load all tracked players
    #[cfg(feature = "database")]
    pub async fn load_all_players(&self) -> Result<Vec<(String, i64, i64, i32, i32, f32, i16, i64, f32)>> {
        let players = sqlx::query_as::<_, (String, i64, i64, i32, i32, f32, i16, i64, f32)>(r#"
            SELECT address, total_deployed, total_won, total_rounds, wins, 
                   avg_squares_per_deploy, preferred_square_count, avg_deploy_size, roi
            FROM player_performance
            WHERE total_rounds >= 5
            ORDER BY total_rounds DESC
            LIMIT 1000
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load players: {}", e)))?;
        
        Ok(players)
    }

    /// Get top performing players by ROI
    #[cfg(feature = "database")]
    pub async fn get_top_performers(&self, limit: i32) -> Result<Vec<(String, f32, f32, i32, f32)>> {
        // Returns: address, roi, win_rate, rounds, avg_squares
        let performers = sqlx::query_as::<_, (String, f32, f32, i32, f32)>(r#"
            SELECT 
                address, 
                roi,
                CASE WHEN total_rounds > 0 THEN wins::REAL / total_rounds::REAL ELSE 0.0 END as win_rate,
                total_rounds,
                avg_squares_per_deploy
            FROM player_performance
            WHERE total_rounds >= 10
            ORDER BY roi DESC, win_rate DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get top performers: {}", e)))?;
        
        Ok(performers)
    }

    /// Update square count statistics
    #[cfg(feature = "database")]
    pub async fn record_square_count_deploy(&self, square_count: i16, amount_lamports: i64) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO square_count_stats (square_count, times_used, total_deployed)
            VALUES ($1, 1, $2)
            ON CONFLICT (square_count) DO UPDATE SET
                times_used = square_count_stats.times_used + 1,
                total_deployed = square_count_stats.total_deployed + $2,
                updated_at = NOW()
        "#)
        .bind(square_count)
        .bind(amount_lamports)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record square count deploy: {}", e)))?;
        
        Ok(())
    }

    /// Record a win for a square count
    #[cfg(feature = "database")]
    pub async fn record_square_count_win(&self, square_count: i16, amount_won_lamports: i64) -> Result<()> {
        sqlx::query(r#"
            UPDATE square_count_stats SET
                times_won = times_won + 1,
                total_won = total_won + $2,
                win_rate = CASE WHEN times_used > 0 THEN (times_won + 1)::REAL / times_used::REAL ELSE 0.0 END,
                roi = CASE WHEN total_deployed > 0 
                    THEN (total_won + $2 - total_deployed)::REAL / total_deployed::REAL 
                    ELSE 0.0 END,
                updated_at = NOW()
            WHERE square_count = $1
        "#)
        .bind(square_count)
        .bind(amount_won_lamports)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record square count win: {}", e)))?;
        
        Ok(())
    }

    /// Load square count statistics
    #[cfg(feature = "database")]
    pub async fn load_square_count_stats(&self) -> Result<Vec<(i16, i32, i32, i64, i64, f32, f32)>> {
        // Returns: square_count, times_used, times_won, total_deployed, total_won, win_rate, roi
        let stats = sqlx::query_as::<_, (i16, i32, i32, i64, i64, f32, f32)>(r#"
            SELECT square_count, times_used, times_won, total_deployed, total_won, win_rate, roi
            FROM square_count_stats
            WHERE times_used >= 5
            ORDER BY roi DESC
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load square count stats: {}", e)))?;
        
        Ok(stats)
    }

    /// Record round conditions for learning
    #[cfg(feature = "database")]
    pub async fn record_round_conditions(
        &self,
        round_id: i64,
        total_deployed: i64,
        num_deployers: i32,
        competition_level: &str,
        expected_ore_multiplier: f32,
        squares_with_deploys: i16,
    ) -> Result<()> {
        let avg_deploy = if num_deployers > 0 { total_deployed / num_deployers as i64 } else { 0 };
        
        sqlx::query(r#"
            INSERT INTO round_conditions 
                (round_id, total_deployed, num_deployers, avg_deploy_size, competition_level, 
                 expected_ore_multiplier, squares_with_deploys)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (round_id) DO UPDATE SET
                total_deployed = $2,
                num_deployers = $3,
                avg_deploy_size = $4,
                competition_level = $5,
                expected_ore_multiplier = $6,
                squares_with_deploys = $7
        "#)
        .bind(round_id)
        .bind(total_deployed)
        .bind(num_deployers)
        .bind(avg_deploy)
        .bind(competition_level)
        .bind(expected_ore_multiplier)
        .bind(squares_with_deploys)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record round conditions: {}", e)))?;
        
        Ok(())
    }

    /// Record our bot's result in a round
    #[cfg(feature = "database")]
    pub async fn record_our_round_result(
        &self,
        round_id: i64,
        won: bool,
        ore_earned: f32,
    ) -> Result<()> {
        sqlx::query(r#"
            UPDATE round_conditions SET
                our_deployed = TRUE,
                our_won = $2,
                our_ore_earned = $3
            WHERE round_id = $1
        "#)
        .bind(round_id)
        .bind(won)
        .bind(ore_earned)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record our round result: {}", e)))?;
        
        Ok(())
    }

    /// Get best performing competition levels
    #[cfg(feature = "database")]
    pub async fn get_best_conditions(&self) -> Result<Vec<(String, i64, f32, f32)>> {
        // Returns: competition_level, count, our_win_rate, avg_ore
        let conditions = sqlx::query_as::<_, (String, i64, f32, f32)>(r#"
            SELECT 
                competition_level,
                COUNT(*) as count,
                AVG(CASE WHEN our_won THEN 1.0 ELSE 0.0 END) as our_win_rate,
                AVG(our_ore_earned) as avg_ore
            FROM round_conditions
            WHERE our_deployed = TRUE
            GROUP BY competition_level
            ORDER BY avg_ore DESC
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get best conditions: {}", e)))?;
        
        Ok(conditions)
    }

    /// Get comprehensive learning summary
    #[cfg(feature = "database")]
    pub async fn get_comprehensive_learning_summary(&self) -> Result<serde_json::Value> {
        let total_players: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM player_performance")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let active_players: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM player_performance WHERE total_rounds >= 10"
        )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let best_square_count: Option<(i16,)> = sqlx::query_as(
            "SELECT square_count FROM square_count_stats ORDER BY roi DESC LIMIT 1"
        )
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

        let avg_winner_squares: (f32,) = sqlx::query_as(
            "SELECT COALESCE(AVG(avg_squares_per_deploy), 5.0) FROM player_performance WHERE wins > 0"
        )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((5.0,));

        Ok(serde_json::json!({
            "total_players_tracked": total_players.0,
            "active_players": active_players.0,
            "best_square_count": best_square_count.map(|x| x.0).unwrap_or(5),
            "avg_winner_square_count": avg_winner_squares.0,
        }))
    }

    // ===== COMPREHENSIVE WIN TRACKING =====

    /// Record a complete win with all context - THE KEY LEARNING DATA
    #[cfg(feature = "database")]
    pub async fn record_win(
        &self,
        round_id: i64,
        winner_address: &str,
        winning_square: i16,
        amount_bet: i64,
        amount_won: i64,
        squares_bet: &[i32],
        num_squares: i16,
        total_round_sol: i64,
        num_deployers: i32,
        is_motherlode: bool,
        is_full_ore: bool,
        ore_earned: f32,
        competition_on_square: i64,
        winner_share_pct: f32,
        slot: i64,
    ) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO win_records 
                (round_id, winner_address, winning_square, amount_bet, amount_won,
                 squares_bet, num_squares, total_round_sol, num_deployers,
                 is_motherlode, is_full_ore, ore_earned, competition_on_square,
                 winner_share_pct, slot)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT DO NOTHING
        "#)
        .bind(round_id)
        .bind(winner_address)
        .bind(winning_square)
        .bind(amount_bet)
        .bind(amount_won)
        .bind(squares_bet)
        .bind(num_squares)
        .bind(total_round_sol)
        .bind(num_deployers)
        .bind(is_motherlode)
        .bind(is_full_ore)
        .bind(ore_earned)
        .bind(competition_on_square)
        .bind(winner_share_pct)
        .bind(slot)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to record win: {}", e)))?;
        
        Ok(())
    }

    /// Load all wins for learning
    #[cfg(feature = "database")]
    pub async fn load_wins(&self, limit: i32) -> Result<Vec<serde_json::Value>> {
        let wins = sqlx::query_as::<_, (i64, String, i16, i64, i64, Vec<i32>, i16, i64, i32, bool, bool, f32)>(r#"
            SELECT round_id, winner_address, winning_square, amount_bet, amount_won,
                   squares_bet, num_squares, total_round_sol, num_deployers,
                   is_motherlode, is_full_ore, ore_earned
            FROM win_records
            ORDER BY round_id DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load wins: {}", e)))?;
        
        Ok(wins.into_iter().map(|(round_id, winner, winning_sq, bet, won, squares, num_sq, total_sol, deployers, motherlode, full_ore, ore)| {
            serde_json::json!({
                "round_id": round_id,
                "winner": winner,
                "winning_square": winning_sq,
                "amount_bet": bet,
                "amount_won": won,
                "squares_bet": squares,
                "num_squares": num_sq,
                "total_round_sol": total_sol,
                "num_deployers": deployers,
                "is_motherlode": motherlode,
                "is_full_ore": full_ore,
                "ore_earned": ore,
            })
        }).collect())
    }

    /// Get full ORE wins specifically - the most valuable learning data
    #[cfg(feature = "database")]
    pub async fn get_full_ore_wins(&self, limit: i32) -> Result<Vec<serde_json::Value>> {
        let wins = sqlx::query_as::<_, (i64, String, i16, i64, Vec<i32>, i16, i64, i32)>(r#"
            SELECT round_id, winner_address, winning_square, amount_bet,
                   squares_bet, num_squares, total_round_sol, num_deployers
            FROM win_records
            WHERE is_full_ore = TRUE
            ORDER BY round_id DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get full ORE wins: {}", e)))?;
        
        Ok(wins.into_iter().map(|(round_id, winner, winning_sq, bet, squares, num_sq, total_sol, deployers)| {
            serde_json::json!({
                "round_id": round_id,
                "winner": winner,
                "winning_square": winning_sq,
                "amount_bet_sol": bet as f64 / 1_000_000_000.0,
                "squares_bet": squares,
                "num_squares": num_sq,
                "total_round_sol": total_sol as f64 / 1_000_000_000.0,
                "num_deployers": deployers,
            })
        }).collect())
    }

    /// Get motherlode wins
    #[cfg(feature = "database")]
    pub async fn get_motherlode_wins(&self, limit: i32) -> Result<Vec<serde_json::Value>> {
        let wins = sqlx::query_as::<_, (i64, String, i16, i64, i64, i16, i64, f32)>(r#"
            SELECT round_id, winner_address, winning_square, amount_bet, amount_won,
                   num_squares, total_round_sol, ore_earned
            FROM win_records
            WHERE is_motherlode = TRUE
            ORDER BY ore_earned DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to get motherlode wins: {}", e)))?;
        
        Ok(wins.into_iter().map(|(round_id, winner, winning_sq, bet, won, num_sq, total_sol, ore)| {
            serde_json::json!({
                "round_id": round_id,
                "winner": winner,
                "winning_square": winning_sq,
                "bet_sol": bet as f64 / 1_000_000_000.0,
                "won_sol": won as f64 / 1_000_000_000.0,
                "num_squares": num_sq,
                "total_round_sol": total_sol as f64 / 1_000_000_000.0,
                "ore_earned": ore,
            })
        }).collect())
    }

    /// Save a detected strategy
    #[cfg(feature = "database")]
    pub async fn save_detected_strategy(
        &self,
        name: &str,
        description: &str,
        sample_size: i32,
        win_rate: f32,
        avg_roi: f32,
        avg_ore_per_round: f32,
        square_count: i16,
        bet_size_sol: f32,
        target_competition: &str,
        preferred_squares: &[i32],
        play_motherlode: bool,
        confidence: f32,
        consistent: bool,
        example_players: &[String],
    ) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO detected_strategies 
                (name, description, sample_size, win_rate, avg_roi, avg_ore_per_round,
                 square_count, bet_size_sol, target_competition, preferred_squares,
                 play_motherlode, confidence, consistent, example_players, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())
            ON CONFLICT (name) DO UPDATE SET
                description = $2,
                sample_size = $3,
                win_rate = $4,
                avg_roi = $5,
                avg_ore_per_round = $6,
                square_count = $7,
                bet_size_sol = $8,
                target_competition = $9,
                preferred_squares = $10,
                play_motherlode = $11,
                confidence = $12,
                consistent = $13,
                example_players = $14,
                updated_at = NOW()
        "#)
        .bind(name)
        .bind(description)
        .bind(sample_size)
        .bind(win_rate)
        .bind(avg_roi)
        .bind(avg_ore_per_round)
        .bind(square_count)
        .bind(bet_size_sol)
        .bind(target_competition)
        .bind(preferred_squares)
        .bind(play_motherlode)
        .bind(confidence)
        .bind(consistent)
        .bind(example_players)
        .execute(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to save strategy: {}", e)))?;
        
        Ok(())
    }

    /// Load detected strategies
    #[cfg(feature = "database")]
    pub async fn load_detected_strategies(&self) -> Result<Vec<serde_json::Value>> {
        let strategies = sqlx::query_as::<_, (String, String, i32, f32, f32, f32, i16, f32, String, Vec<i32>, bool, f32, bool)>(r#"
            SELECT name, description, sample_size, win_rate, avg_roi, avg_ore_per_round,
                   square_count, bet_size_sol, target_competition, preferred_squares,
                   play_motherlode, confidence, consistent
            FROM detected_strategies
            ORDER BY confidence DESC, avg_roi DESC
        "#)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BotError::Other(format!("Failed to load strategies: {}", e)))?;
        
        Ok(strategies.into_iter().map(|(name, desc, samples, win_rate, roi, ore, sq_count, bet, comp, pref_sq, motherlode, conf, consistent)| {
            serde_json::json!({
                "name": name,
                "description": desc,
                "sample_size": samples,
                "win_rate": win_rate,
                "avg_roi": roi,
                "avg_ore_per_round": ore,
                "square_count": sq_count,
                "bet_size_sol": bet,
                "target_competition": comp,
                "preferred_squares": pref_sq,
                "play_motherlode": motherlode,
                "confidence": conf,
                "consistent": consistent,
            })
        }).collect())
    }

    /// Get winning stats summary
    #[cfg(feature = "database")]
    pub async fn get_win_stats(&self) -> Result<serde_json::Value> {
        let total_wins: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM win_records")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let full_ore_wins: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM win_records WHERE is_full_ore")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        
        let motherlode_wins: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM win_records WHERE is_motherlode")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));

        // Most common winning square count
        let best_sq_count: Option<(i16, i64)> = sqlx::query_as(r#"
            SELECT num_squares, COUNT(*) as cnt 
            FROM win_records 
            GROUP BY num_squares 
            ORDER BY cnt DESC 
            LIMIT 1
        "#)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

        // Average stats for full ORE wins
        let full_ore_avg: Option<(f64, f64, f64)> = sqlx::query_as(r#"
            SELECT 
                AVG(num_squares)::FLOAT8,
                AVG(amount_bet)::FLOAT8 / 1000000000.0,
                AVG(total_round_sol)::FLOAT8 / 1000000000.0
            FROM win_records 
            WHERE is_full_ore = TRUE
        "#)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

        Ok(serde_json::json!({
            "total_wins_tracked": total_wins.0,
            "full_ore_wins": full_ore_wins.0,
            "motherlode_wins": motherlode_wins.0,
            "most_common_winning_squares": best_sq_count.map(|(sq, _)| sq).unwrap_or(5),
            "full_ore_avg_squares": full_ore_avg.as_ref().map(|(sq, _, _)| *sq).unwrap_or(5.0),
            "full_ore_avg_bet_sol": full_ore_avg.as_ref().map(|(_, bet, _)| *bet).unwrap_or(0.02),
            "full_ore_avg_round_sol": full_ore_avg.as_ref().map(|(_, _, round)| *round).unwrap_or(1.0),
        }))
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
