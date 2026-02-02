use clawdbot::{
    bot::{Bot, BotRunner, BotStatus},
    client::OreClient,
    config::{BotConfig, MiningConfig},
    error::Result,
    strategy::MiningStrategy,
};
use log::{error, info};
use solana_sdk::signature::read_keypair_file;
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

struct MinerBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: MiningConfig,
    client: Arc<OreClient>,
    strategy: MiningStrategy,
}

impl MinerBot {
    fn new(config: MiningConfig, client: Arc<OreClient>) -> Self {
        let strategy = MiningStrategy::new(config.strategy.clone());
        
        Self {
            name: "Miner".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            strategy,
        }
    }

    async fn mining_loop(&self) -> Result<()> {
        info!("🔨 Miner bot started");

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

            // Check balance
            let balance = self.client.get_balance()?;
            let balance_sol = balance as f64 / 1_000_000_000.0;

            if balance_sol < self.config.min_sol_balance {
                error!("⚠️  Insufficient balance: {:.4} SOL (minimum: {:.2} SOL)", 
                    balance_sol, self.config.min_sol_balance);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Get current round
            let board = self.client.get_board()?;
            let round = self.client.get_round(board.round)?;

            info!("📊 Current round: {}, Total deployed: {}", 
                board.round, 
                round.deployed.iter().sum::<u64>()
            );

            // Get historical data for strategy
            let history = self.client.get_rounds(board.round, 10)?
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();

            // Select squares to deploy on
            let squares = self.strategy.select_squares(1, &round, &history)?;
            
            info!("🎯 Selected squares: {:?}", squares);

            // Here you would implement the actual deployment transaction
            // For now, we'll just log it
            info!("⛏️  Would deploy {:.4} SOL to squares {:?}", 
                self.config.deploy_amount_sol, squares);

            // Check if we should claim rewards
            if let Ok(Some(miner)) = self.client.get_miner() {
                let claimable_ore = miner.rewards_ore as f64 / 1e11; // Convert from grams to ORE
                
                if claimable_ore >= self.config.auto_claim_threshold_ore {
                    info!("💰 Claiming {:.2} ORE in rewards", claimable_ore);
                    // Implement claim transaction here
                }
            }

            // Wait before next iteration
            sleep(Duration::from_secs(30)).await;
        }

        info!("🛑 Miner bot stopped");
        Ok(())
    }
}

impl Bot for MinerBot {
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
            strategy: MiningStrategy::new(self.config.strategy.clone()),
        };

        tokio::spawn(async move {
            if let Err(e) = self_clone.mining_loop().await {
                error!("Miner bot error: {}", e);
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

    info!("🤖 Starting ClawdBot Miner");
    info!("📍 Wallet: {}", keypair.pubkey());
    info!("🌐 RPC: {}", config.rpc_url);

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);

    // Create miner bot
    let miner_bot = MinerBot::new(config.mining.clone(), Arc::new(client));

    // Create and start bot runner
    let client_for_runner = OreClient::new(
        config.rpc_url.clone(),
        read_keypair_file(&config.keypair_path).unwrap(),
    );
    let mut runner = BotRunner::new(config, client_for_runner);
    runner.add_bot(Box::new(miner_bot));

    // Run
    runner.run().await?;

    Ok(())
}
