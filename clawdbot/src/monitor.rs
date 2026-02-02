use crate::{
    bot::{Bot, BotStatus},
    client::OreClient,
    config::MonitorConfig,
    error::Result,
};
use colored::*;
use log::{info, warn};
use ore_api::state::{Board, Miner, Treasury};
use std::sync::{Arc, RwLock};
use tokio::time::{sleep, Duration};

pub struct MonitorBot {
    name: String,
    status: Arc<RwLock<BotStatus>>,
    config: MonitorConfig,
    client: Arc<OreClient>,
    last_balance: Arc<RwLock<u64>>,
    last_round: Arc<RwLock<u64>>,
}

impl MonitorBot {
    pub fn new(config: MonitorConfig, client: Arc<OreClient>) -> Self {
        Self {
            name: "Monitor".to_string(),
            status: Arc::new(RwLock::new(BotStatus::Idle)),
            config,
            client,
            last_balance: Arc::new(RwLock::new(0)),
            last_round: Arc::new(RwLock::new(0)),
        }
    }

    async fn monitor_loop(&self) -> Result<()> {
        loop {
            // Check if bot should stop
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

            // Perform monitoring tasks
            if self.config.track_balance {
                self.check_balance().await?;
            }

            if self.config.track_rounds {
                self.check_round_status().await?;
            }

            if self.config.track_competition {
                self.check_competition().await?;
            }

            // Sleep before next check
            sleep(Duration::from_secs(self.config.check_interval)).await;
        }

        Ok(())
    }

    async fn check_balance(&self) -> Result<()> {
        let balance = self.client.get_balance()?;
        let mut last_balance = self.last_balance.write().unwrap();

        if *last_balance == 0 {
            *last_balance = balance;
            return Ok(());
        }

        let balance_sol = balance as f64 / 1_000_000_000.0;
        let last_balance_sol = *last_balance as f64 / 1_000_000_000.0;

        if balance != *last_balance {
            let diff = balance_sol - last_balance_sol;
            if diff > 0.0 {
                info!("{}", format!("💰 Balance increased by {:.4} SOL (now {:.4} SOL)", diff, balance_sol).green());
            } else {
                info!("{}", format!("📉 Balance decreased by {:.4} SOL (now {:.4} SOL)", diff.abs(), balance_sol).yellow());
            }
        }

        // Alert if balance is too low
        if balance_sol < self.config.alerts.min_balance_sol {
            warn!(
                "{}",
                format!(
                    "⚠️  Low balance alert: {:.4} SOL (minimum: {:.2} SOL)",
                    balance_sol, self.config.alerts.min_balance_sol
                )
                .red()
            );
        }

        *last_balance = balance;
        Ok(())
    }

    async fn check_round_status(&self) -> Result<()> {
        let board = self.client.get_board()?;
        let mut last_round = self.last_round.write().unwrap();

        if *last_round == 0 {
            *last_round = board.round;
            info!("📊 Current round: {}", board.round);
            return Ok(());
        }

        if board.round != *last_round {
            info!("{}", format!("🎲 New round started: {} → {}", *last_round, board.round).cyan());
            *last_round = board.round;

            // Show round summary
            self.print_round_summary(&board).await?;
        }

        Ok(())
    }

    async fn print_round_summary(&self, board: &Board) -> Result<()> {
        println!("\n{}", "═══════════════════════════════════════".cyan());
        println!("{}", format!("Round #{}", board.round).cyan().bold());
        println!("{}", "═══════════════════════════════════════".cyan());

        // Get treasury info
        if let Ok(treasury) = self.client.get_treasury() {
            println!("💎 Total Staked: {} ORE", treasury.total_staked);
            println!("🔥 Motherlode Pool: {} ORE", treasury.motherlode);
        }

        // Get miner info
        if let Ok(Some(miner)) = self.client.get_miner() {
            println!("\n{}", "Your Stats:".yellow());
            println!("  Lifetime SOL Rewards: {}", miner.lifetime_rewards_sol);
            println!("  Lifetime ORE Rewards: {}", miner.lifetime_rewards_ore);
            println!("  Lifetime Deployed: {}", miner.lifetime_deployed);
            println!("  Claimable SOL: {}", miner.rewards_sol);
            println!("  Claimable ORE: {}", miner.rewards_ore);
        }

        println!("{}", "═══════════════════════════════════════\n".cyan());

        Ok(())
    }

    async fn check_competition(&self) -> Result<()> {
        // Get current round data
        let round = self.client.get_current_round()?;
        
        let total_deployed: u64 = round.deployed.iter().sum();
        
        info!("🎯 Total deployed this round: {} lamports", total_deployed);
        
        // Find most and least deployed squares
        let mut max_deployed = 0u64;
        let mut min_deployed = u64::MAX;
        let mut max_square = 0;
        let mut min_square = 0;

        for (i, &deployed) in round.deployed.iter().enumerate() {
            if deployed > max_deployed {
                max_deployed = deployed;
                max_square = i;
            }
            if deployed < min_deployed && deployed > 0 {
                min_deployed = deployed;
                min_square = i;
            }
        }

        info!("🔥 Hottest square: #{} with {} lamports", max_square, max_deployed);
        info!("❄️  Coldest square: #{} with {} lamports", min_square, min_deployed);

        Ok(())
    }
}

impl Bot for MonitorBot {
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
            last_balance: Arc::clone(&self.last_balance),
            last_round: Arc::clone(&self.last_round),
        };

        tokio::spawn(async move {
            if let Err(e) = self_clone.monitor_loop().await {
                log::error!("Monitor bot error: {}", e);
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
