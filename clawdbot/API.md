# ClawdBot API Documentation

## Core Components

### OreClient

The `OreClient` is the main interface for interacting with the ORE protocol.

```rust
use clawdbot::client::OreClient;
use solana_sdk::signature::Keypair;

let client = OreClient::new(
    "https://api.mainnet-beta.solana.com".to_string(),
    keypair
);
```

#### Methods

##### `get_balance() -> Result<u64>`
Get the SOL balance of the bot's wallet in lamports.

##### `get_board() -> Result<Board>`
Get the current board state containing round number and timestamps.

##### `get_miner() -> Result<Option<Miner>>`
Get the miner account for the bot's wallet, if it exists.

##### `get_round(round_id: u64) -> Result<Round>`
Get data for a specific round.

##### `get_current_round() -> Result<Round>`
Get the current active round data.

##### `get_treasury() -> Result<Treasury>`
Get the treasury state with staking and rewards info.

##### `get_rounds(start_round: u64, count: usize) -> Result<Vec<(u64, Round)>>`
Get multiple historical rounds for analysis.

---

## Bot Trait

All bots must implement the `Bot` trait:

```rust
use clawdbot::bot::{Bot, BotStatus};
use clawdbot::error::Result;

#[async_trait]
pub trait Bot: Send + Sync {
    fn name(&self) -> &str;
    fn status(&self) -> BotStatus;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn pause(&mut self) -> Result<()>;
    async fn resume(&mut self) -> Result<()>;
}
```

### Creating a Custom Bot

```rust
use clawdbot::bot::{Bot, BotStatus};
use std::sync::{Arc, RwLock};

struct MyCustomBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    client: Arc<OreClient>,
}

impl MyCustomBot {
    pub fn new(client: Arc<OreClient>) -> Self {
        Self {
            name: "MyCustom".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            client,
        }
    }

    async fn bot_loop(&self) -> Result<()> {
        loop {
            // Check status
            {
                let status = self.status.read().unwrap();
                if *status == BotStatus::Stopped {
                    break;
                }
            }

            // Your bot logic here
            // ...

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }
}

impl Bot for MyCustomBot {
    fn name(&self) -> &str {
        &self.name
    }

    fn status(&self) -> BotStatus {
        *self.status.read().unwrap()
    }

    async fn start(&mut self) -> Result<()> {
        *self.status.write().unwrap() = BotStatus::Running;
        
        let self_clone = /* clone self */;
        tokio::spawn(async move {
            if let Err(e) = self_clone.bot_loop().await {
                eprintln!("Bot error: {}", e);
            }
        });
        
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.status.write().unwrap() = BotStatus::Stopped;
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        *self.status.write().unwrap() = BotStatus::Paused;
        Ok(())
    }

    async fn resume(&mut self) -> Result<()> {
        *self.status.write().unwrap() = BotStatus::Running;
        Ok(())
    }
}
```

---

## Strategies

### BettingStrategy

```rust
use clawdbot::strategy::BettingStrategy;

let strategy = BettingStrategy::new(
    "weighted".to_string(),  // strategy type
    0.5                      // risk tolerance (0.0-1.0)
);

// Select squares to bet on
let squares = strategy.select_squares(
    3,              // number of squares
    &round_history, // Vec<Round>
    &current_round  // &Round
)?;

// Calculate bet amounts
let bets = strategy.calculate_bet_amounts(
    &squares,     // &[usize]
    1.0,          // total budget (SOL)
    0.01,         // min bet (SOL)
    0.5           // max bet (SOL)
);
```

#### Available Strategies
- `"random"` - Random selection
- `"weighted"` - Based on historical win rates
- `"hot_squares"` - Recent winners
- `"contrarian"` - Opposite of crowd
- `"spread"` - Evenly distributed
- `"focused"` - Concentrated on best odds

### MiningStrategy

```rust
use clawdbot::strategy::MiningStrategy;

let strategy = MiningStrategy::new("weighted".to_string());

let squares = strategy.select_squares(
    1,               // number of squares
    &current_round,  // &Round
    &round_history   // &[Round]
)?;
```

#### Available Strategies
- `"random"` - Baseline
- `"weighted"` - Lower deployment = better odds
- `"balanced"` - Mixed approach

---

## Analytics Engine

```rust
use clawdbot::analytics::AnalyticsEngine;

let mut engine = AnalyticsEngine::new();

// Add round data
for (round_id, round) in historical_rounds {
    engine.add_round(round_id, round);
}

// Analyze
let analytics = engine.get_overall_analytics()?;
let square_stats = engine.analyze_squares()?;
let predictions = engine.predict_winning_squares(5)?;

// Export
engine.export_to_json("analytics.json")?;
```

### AnalyticsEngine Methods

##### `add_round(round_id: u64, round: Round)`
Add a round to the history.

##### `analyze_rounds() -> Result<Vec<RoundAnalytics>>`
Analyze all rounds and return statistics.

##### `analyze_squares() -> Result<Vec<SquareStatistics>>`
Get statistics for each square on the board.

##### `get_overall_analytics() -> Result<OverallAnalytics>`
Get comprehensive analytics summary.

##### `predict_winning_squares(top_n: usize) -> Result<Vec<usize>>`
Predict the most likely winning squares.

##### `export_to_json(path: &str) -> Result<()>`
Export analytics to a JSON file.

##### `get_recent_trends(last_n_rounds: usize) -> Result<Vec<SquareStatistics>>`
Analyze recent trends.

---

## Configuration

### Loading Config

```rust
use clawdbot::config::BotConfig;

// From file
let config = BotConfig::from_file("config.json")?;

// Default
let config = BotConfig::default();

// Save
config.to_file("config.json")?;
```

### Config Structure

```rust
pub struct BotConfig {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub keypair_path: String,
    pub mining: MiningConfig,
    pub betting: BettingConfig,
    pub analytics: AnalyticsConfig,
    pub monitor: MonitorConfig,
}
```

---

## Error Handling

All bot operations return `Result<T>` where the error type is `BotError`.

```rust
use clawdbot::error::{BotError, Result};

fn my_function() -> Result<()> {
    // Handle Solana errors
    let balance = client.get_balance()
        .map_err(|e| BotError::SolanaClient(e))?;
    
    // Custom errors
    if balance < min_balance {
        return Err(BotError::InsufficientBalance(
            format!("Need {} lamports", min_balance)
        ));
    }
    
    Ok(())
}
```

### Error Types

- `SolanaClient` - RPC/client errors
- `SolanaProgram` - On-chain program errors
- `Anchor` - Anchor framework errors
- `Request` - HTTP request errors
- `Serialization` - Data serialization errors
- `Config` - Configuration errors
- `InsufficientBalance` - Not enough funds
- `Mining` - Mining-specific errors
- `Betting` - Betting-specific errors
- `Analytics` - Analytics errors
- `Strategy` - Strategy errors
- `Other` - General errors

---

## Data Types

### Board

```rust
pub struct Board {
    pub round: u64,              // Current round number
    pub created_at: i64,         // Creation timestamp
    pub last_reset_at: i64,      // Last reset timestamp
    // ... other fields
}
```

### Round

```rust
pub struct Round {
    pub deployed: [u64; 25],     // SOL deployed on each square
    // ... other fields
}
```

### Miner

```rust
pub struct Miner {
    pub authority: Pubkey,
    pub deployed: [u64; 25],
    pub cumulative: [u64; 25],
    pub rewards_sol: u64,
    pub rewards_ore: u64,
    pub lifetime_rewards_sol: u64,
    pub lifetime_rewards_ore: u64,
    pub lifetime_deployed: u64,
    // ... other fields
}
```

### Treasury

```rust
pub struct Treasury {
    pub balance: u64,
    pub motherlode: u64,
    pub total_staked: u64,
    pub total_refined: u64,
    // ... other fields
}
```

---

## Advanced Usage

### Custom Strategy Implementation

```rust
pub trait Strategy {
    fn select_squares(
        &self,
        num_squares: usize,
        round_history: &[Round],
        current_round: &Round,
    ) -> Result<Vec<usize>>;
}

pub struct MyStrategy {
    // Your strategy state
}

impl Strategy for MyStrategy {
    fn select_squares(
        &self,
        num_squares: usize,
        round_history: &[Round],
        current_round: &Round,
    ) -> Result<Vec<usize>> {
        // Your custom logic here
        let squares = vec![0, 5, 12]; // example
        Ok(squares)
    }
}
```

### Multiple Wallets

```rust
let clients: Vec<OreClient> = keypairs
    .iter()
    .map(|kp| OreClient::new(rpc_url.clone(), kp.clone()))
    .collect();

for client in clients {
    // Deploy mining operations across multiple wallets
}
```

### Database Integration

```rust
#[cfg(feature = "database")]
use sqlx::SqlitePool;

async fn store_analytics(pool: &SqlitePool, analytics: &OverallAnalytics) {
    sqlx::query!(
        "INSERT INTO analytics (rounds, sol_deployed) VALUES (?, ?)",
        analytics.total_rounds_analyzed,
        analytics.total_sol_deployed
    )
    .execute(pool)
    .await?;
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_selection() {
        let strategy = BettingStrategy::new("weighted".to_string(), 0.5);
        // Add test logic
    }

    #[tokio::test]
    async fn test_bot_lifecycle() {
        let mut bot = MyCustomBot::new(client);
        bot.start().await.unwrap();
        assert_eq!(bot.status(), BotStatus::Running);
        bot.stop().await.unwrap();
        assert_eq!(bot.status(), BotStatus::Stopped);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_mining_workflow() {
    let config = BotConfig::default();
    let client = OreClient::new(config.rpc_url.clone(), keypair);
    
    // Test full mining workflow
    let board = client.get_board().unwrap();
    let round = client.get_round(board.round).unwrap();
    
    let strategy = MiningStrategy::new("weighted".to_string());
    let squares = strategy.select_squares(1, &round, &[]).unwrap();
    
    assert!(!squares.is_empty());
}
```

---

## Best Practices

1. **Error Handling**: Always handle `Result` types properly
2. **Logging**: Use appropriate log levels (debug, info, warn, error)
3. **Rate Limiting**: Don't overwhelm the RPC with requests
4. **Balance Checks**: Always verify sufficient balance before transactions
5. **Testing**: Test on devnet before mainnet
6. **Monitoring**: Use the monitor bot to track operations
7. **Graceful Shutdown**: Implement proper cleanup in `stop()`
8. **State Management**: Use Arc<RwLock<T>> for shared mutable state

---

## Examples

See the bot implementations in `src/bin/` for complete examples:
- `miner_bot.rs` - Mining bot implementation
- `betting_bot.rs` - Betting bot implementation
- `analytics_bot.rs` - Analytics bot implementation
- `monitor_bot.rs` - Monitor bot implementation

---

## Contributing

To add a new feature:

1. Define your bot struct
2. Implement the `Bot` trait
3. Add configuration options to `BotConfig`
4. Create a binary in `src/bin/`
5. Update documentation
6. Add tests
7. Submit PR

---

## Support

For API questions:
- Read the source code documentation
- Check existing bot implementations
- Open an issue with specific questions
- Join the community discussions
