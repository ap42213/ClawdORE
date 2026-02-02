use crate::{
    bot::BotStatus,
    client::OreClient,
    config::MonitorConfig,
    error::Result,
};
use colored::*;
use log::{info, warn};
use ore_api::state::Board;
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn status(&self) -> BotStatus {
        *self.status.read().unwrap()
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Running;
        self.monitor_loop().await
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping {} bot", self.name);
        *self.status.write().unwrap() = BotStatus::Stopped;
        Ok(())
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
                if let Err(e) = self.check_balance().await {
                    warn!("Balance check failed: {}", e);
                }
            }

            if self.config.track_rounds {
                if let Err(e) = self.check_round_status().await {
                    warn!("Round check failed: {}", e);
                }
            }

            if self.config.track_competition {
                if let Err(e) = self.check_competition().await {
                    warn!("Competition check failed: {}", e);
                }
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
            *last_round = board.round_id;
            info!("📊 Current round: {}", board.round_id);
            return Ok(());
        }

        if board.round_id != *last_round {
            info!("{}", format!("🎲 New round started: {} → {}", *last_round, board.round_id).cyan());
            *last_round = board.round_id;

            // Show round summary
            self.print_round_summary(&board).await?;
        }

        Ok(())
    }

    async fn print_round_summary(&self, board: &Board) -> Result<()> {
        println!("\n{}", "═══════════════════════════════════════".cyan());
        println!("{}", format!("Round #{}", board.round_id).cyan().bold());
        println!("{}", "═══════════════════════════════════════".cyan());

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
