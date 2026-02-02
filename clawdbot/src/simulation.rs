use crate::{
    client::OreClient,
    config::BotConfig,
    error::{BotError, Result},
    ore_round::{OreRound, OreRoundTracker, RoundOutcome},
};
use log::{info, warn};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Simulation engine for paper trading
pub struct SimulationEngine {
    client: Arc<OreClient>,
    config: BotConfig,
    round_tracker: OreRoundTracker,
    simulated_balance: f64,
    simulated_ore_balance: f64,
}

impl SimulationEngine {
    pub fn new(client: Arc<OreClient>, config: BotConfig, initial_balance: f64) -> Self {
        Self {
            client,
            config,
            round_tracker: OreRoundTracker::new(),
            simulated_balance: initial_balance,
            simulated_ore_balance: 0.0,
        }
    }

    /// Monitor mainnet and simulate participation
    pub async fn run(&mut self) -> Result<()> {
        info!("🎮 Starting ORE simulation engine");
        info!("💰 Initial simulated balance: {:.4} SOL", self.simulated_balance);

        loop {
            // Fetch current round from mainnet
            match self.fetch_current_round().await {
                Ok(round) => {
                    self.process_round(round).await?;
                }
                Err(e) => {
                    warn!("Failed to fetch round: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }

            // Wait for next round (60 seconds)
            sleep(Duration::from_secs(60)).await;
        }
    }

    /// Fetch current round data from mainnet
    async fn fetch_current_round(&self) -> Result<OreRound> {
        // Get board and current round
        let board = self.client.get_board()?;
        let round = self.client.get_round(board.round)?;

        // Determine outcome type
        let outcome = self.determine_outcome(&round)?;

        Ok(OreRound {
            round_id: board.round,
            timestamp: chrono::Utc::now().timestamp(),
            outcome,
            total_participants: 0, // Would need to fetch from chain
            total_deployed: round.deployed.iter().sum(),
            duration_seconds: 60,
        })
    }

    /// Determine if round was split or full ORE
    fn determine_outcome(&self, round: &ore_api::state::Round) -> Result<RoundOutcome> {
        // This is simplified - actual logic would check the reward distribution
        // You'd need to examine the treasury or reward account changes
        
        // For now, use a heuristic based on deployment patterns
        let total_deployed: u64 = round.deployed.iter().sum();
        let max_deployed = round.deployed.iter().max().unwrap_or(&0);
        
        // If one square has >50% deployment, likely full ore winner
        if *max_deployed > total_deployed / 2 {
            Ok(RoundOutcome::FullOre {
                winner: "unknown".to_string(),
                amount: 1.0,
            })
        } else {
            Ok(RoundOutcome::Split {
                total_ore: 1.0,
                participants: round.deployed.iter().filter(|&&d| d > 0).count(),
            })
        }
    }

    /// Process a round and simulate participation
    async fn process_round(&mut self, round: OreRound) -> Result<()> {
        info!("📊 Round {} - {:?}", round.round_id, round.outcome);

        // Track the round
        self.round_tracker.add_round(round.clone());

        // Simulate our participation
        self.simulate_participation(&round).await?;

        // Check for motherlode likelihood
        let (is_likely, prob) = self.round_tracker.is_motherlode_likely();
        if is_likely {
            info!("🎰 MOTHERLODE LIKELY! Probability: {:.1}%", prob * 100.0);
        }

        // Display statistics every 10 rounds
        if round.round_id % 10 == 0 {
            self.display_stats();
        }

        Ok(())
    }

    /// Simulate our participation in a round
    async fn simulate_participation(&mut self, round: &OreRound) -> Result<()> {
        // Decide how much to bet (simulated)
        let bet_amount = self.calculate_bet_amount();

        if bet_amount <= 0.0 || bet_amount > self.simulated_balance {
            info!("⏸️  Skipping round (insufficient balance)");
            return Ok(());
        }

        // Deduct bet from balance
        self.simulated_balance -= bet_amount;
        info!("💸 Simulated bet: {:.4} SOL", bet_amount);

        // Simulate outcome
        match &round.outcome {
            RoundOutcome::Split { total_ore, participants } => {
                // We would get a share
                let our_share = total_ore / (*participants as f64);
                self.simulated_ore_balance += our_share;
                info!("✅ Split! Earned {:.4} ORE", our_share);
            }
            RoundOutcome::FullOre { amount, .. } => {
                // Random chance we won
                let participants = round.total_participants.max(1);
                let win_chance = 1.0 / participants as f64;
                let random_roll: f64 = rand::random();
                
                if random_roll < win_chance {
                    self.simulated_ore_balance += amount;
                    info!("🎉 WON FULL ORE! +{:.4} ORE", amount);
                } else {
                    info!("❌ Lost (full ore went to another wallet)");
                }
            }
            RoundOutcome::Motherlode { amount, .. } => {
                let participants = round.total_participants.max(1);
                let win_chance = 1.0 / participants as f64;
                let random_roll: f64 = rand::random();
                
                if random_roll < win_chance {
                    self.simulated_ore_balance += amount;
                    info!("💎 WON MOTHERLODE!!! +{:.4} ORE", amount);
                } else {
                    info!("❌ Motherlode went to another wallet");
                }
            }
        }

        Ok(())
    }

    /// Calculate how much to bet this round
    fn calculate_bet_amount(&self) -> f64 {
        // Simple Kelly-inspired sizing
        let max_bet = self.simulated_balance * 0.05; // 5% of bankroll
        let min_bet = 0.01;

        max_bet.max(min_bet)
    }

    /// Display current statistics
    fn display_stats(&self) {
        let stats = self.round_tracker.calculate_stats();
        
        info!("📊 SIMULATION STATISTICS");
        info!("═══════════════════════════════");
        info!("💰 SOL Balance:    {:.4}", self.simulated_balance);
        info!("⛏️  ORE Balance:    {:.4}", self.simulated_ore_balance);
        info!("📈 Rounds Tracked: {}", stats.total_rounds);
        info!("🔀 Split Rounds:   {} ({:.1}%)", stats.split_rounds, stats.split_percentage);
        info!("🎯 Full ORE:       {} ({:.1}%)", stats.full_ore_rounds, stats.full_ore_percentage);
        info!("💎 Motherlode:     {} ({:.1}%)", stats.motherlode_rounds, stats.motherlode_percentage);
        info!("👥 Avg Players:    {:.1}", stats.average_participants);
        info!("⛏️  Total ORE:      {:.4}", stats.total_ore_distributed);
        info!("═══════════════════════════════");
    }

    /// Export results
    pub fn export_results(&self, path: &str) -> Result<()> {
        let json = self.round_tracker.export_to_json()?;
        std::fs::write(path, json)?;
        info!("💾 Exported results to {}", path);
        Ok(())
    }

    /// Get current balances
    pub fn get_balances(&self) -> (f64, f64) {
        (self.simulated_balance, self.simulated_ore_balance)
    }

    /// Get round tracker
    pub fn get_round_tracker(&self) -> &OreRoundTracker {
        &self.round_tracker
    }
}
