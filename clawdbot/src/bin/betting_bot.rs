use clawdbot::{
    bot::{Bot, BotRunner, BotStatus},
    client::OreClient,
    config::{BotConfig, BettingConfig},
    error::Result,
    strategy::BettingStrategy,
};
use log::{error, info, warn};
use solana_sdk::signature::read_keypair_file;
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

struct BettingBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: BettingConfig,
    client: Arc<OreClient>,
    strategy: BettingStrategy,
    last_round: Arc<RwLock<u64>>,
}

impl BettingBot {
    fn new(config: BettingConfig, client: Arc<OreClient>) -> Self {
        let strategy = BettingStrategy::new(
            config.strategy.clone(),
            config.risk_tolerance,
        );
        
        Self {
            name: "Betting".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            strategy,
            last_round: Arc::new(RwLock::new(0)),
        }
    }

    async fn betting_loop(&self) -> Result<()> {
        info!("🎲 Betting bot started");

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

            // Get current round
            let board = self.client.get_board()?;
            let current_round_id = board.round;
            let mut last_round = self.last_round.write().unwrap();

            // Only bet on new rounds
            if current_round_id == *last_round {
                sleep(Duration::from_secs(10)).await;
                continue;
            }

            info!("🎲 New round detected: {}", current_round_id);
            *last_round = current_round_id;
            drop(last_round);

            // Check balance
            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            let bet_amount_sol = (balance_sol * self.config.bet_percentage)
                .clamp(self.config.min_bet_sol, self.config.max_bet_sol);

            if bet_amount_sol < self.config.min_bet_sol {
                warn!("⚠️  Insufficient balance for betting: {:.4} SOL", balance_sol);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Get round data and history
            let round = self.client.get_round(current_round_id)?;
            let history = self.client.get_rounds(current_round_id, 20)?
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();

            // Select squares to bet on
            let squares = self.strategy.select_squares(
                self.config.squares_to_bet,
                &history,
                &round,
            )?;

            // Calculate bet amounts for each square
            let bets = self.strategy.calculate_bet_amounts(
                &squares,
                bet_amount_sol,
                self.config.min_bet_sol,
                self.config.max_bet_sol,
            );

            info!("🎯 Placing bets:");
            for (square, amount) in &bets {
                info!("  Square #{}: {:.4} SOL", square, amount);
            }

            // Here you would implement the actual betting transactions
            // For now, we'll just log it
            let total_bet: f64 = bets.iter().map(|(_, amt)| amt).sum();
            info!("💰 Total bet: {:.4} SOL across {} squares", total_bet, bets.len());

            // Wait for next round
            sleep(Duration::from_secs(30)).await;
        }

        info!("🛑 Betting bot stopped");
        Ok(())
    }
}

impl Bot for BettingBot {
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
            strategy: BettingStrategy::new(
                self.config.strategy.clone(),
                self.config.risk_tolerance,
            ),
            last_round: Arc::clone(&self.last_round),
        };

        tokio::spawn(async move {
            if let Err(e) = self_clone.betting_loop().await {
                error!("Betting bot error: {}", e);
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

    info!("🤖 Starting ClawdBot Betting Bot");
    info!("📍 Wallet: {}", keypair.pubkey());
    info!("🌐 RPC: {}", config.rpc_url);
    info!("🎯 Strategy: {}", config.betting.strategy);
    info!("📊 Risk Tolerance: {:.1}%", config.betting.risk_tolerance * 100.0);

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create betting bot
    let betting_bot = BettingBot::new(config.betting.clone(), Arc::new(client));

    // Create and start bot runner
    let client_for_runner = OreClient::new(
        config.rpc_url.clone(),
        read_keypair_file(&config.keypair_path).unwrap(),
    );
    let mut runner = BotRunner::new(config, client_for_runner);
    runner.add_bot(Box::new(betting_bot));

    // Run
    runner.run().await?;

    Ok(())
}
