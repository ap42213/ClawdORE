use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ORE round outcome types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoundOutcome {
    Split { total_ore: f64, participants: usize },
    FullOre { winner: String, amount: f64 },
    Motherlode { winner: String, amount: f64 },
}

/// ORE round data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreRound {
    pub round_id: u64,
    pub timestamp: i64,
    pub outcome: RoundOutcome,
    pub total_participants: usize,
    pub total_deployed: u64,
    pub duration_seconds: u64,
}

/// Statistics for ORE rounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreRoundStats {
    pub total_rounds: usize,
    pub split_rounds: usize,
    pub full_ore_rounds: usize,
    pub motherlode_rounds: usize,
    pub split_percentage: f64,
    pub full_ore_percentage: f64,
    pub motherlode_percentage: f64,
    pub average_participants: f64,
    pub total_ore_distributed: f64,
}

/// Wallet performance in ORE rounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRoundPerformance {
    pub wallet: String,
    pub rounds_participated: usize,
    pub full_ore_wins: usize,
    pub split_earnings: f64,
    pub full_ore_earnings: f64,
    pub motherlode_earnings: f64,
    pub total_earnings: f64,
    pub total_spent: f64,
    pub net_profit: f64,
}

/// ORE round tracker
pub struct OreRoundTracker {
    rounds: Vec<OreRound>,
    wallet_performance: HashMap<String, WalletRoundPerformance>,
}

impl OreRoundTracker {
    pub fn new() -> Self {
        Self {
            rounds: Vec::new(),
            wallet_performance: HashMap::new(),
        }
    }

    /// Add a round to tracking
    pub fn add_round(&mut self, round: OreRound) {
        // Update wallet performance based on outcome
        match &round.outcome {
            RoundOutcome::Split { total_ore, participants } => {
                let ore_per_participant = total_ore / (*participants as f64);
                // Would need actual participant list to update
            }
            RoundOutcome::FullOre { winner, amount } => {
                self.update_wallet_win(winner, *amount, false);
            }
            RoundOutcome::Motherlode { winner, amount } => {
                self.update_wallet_win(winner, *amount, true);
            }
        }
        
        self.rounds.push(round);
    }

    /// Update wallet performance for a win
    fn update_wallet_win(&mut self, wallet: &str, amount: f64, is_motherlode: bool) {
        let perf = self.wallet_performance
            .entry(wallet.to_string())
            .or_insert(WalletRoundPerformance {
                wallet: wallet.to_string(),
                rounds_participated: 0,
                full_ore_wins: 0,
                split_earnings: 0.0,
                full_ore_earnings: 0.0,
                motherlode_earnings: 0.0,
                total_earnings: 0.0,
                total_spent: 0.0,
                net_profit: 0.0,
            });

        if is_motherlode {
            perf.motherlode_earnings += amount;
        } else {
            perf.full_ore_wins += 1;
            perf.full_ore_earnings += amount;
        }
        perf.total_earnings += amount;
        perf.net_profit = perf.total_earnings - perf.total_spent;
    }

    /// Calculate overall statistics
    pub fn calculate_stats(&self) -> OreRoundStats {
        let total_rounds = self.rounds.len();
        let mut split_rounds = 0;
        let mut full_ore_rounds = 0;
        let mut motherlode_rounds = 0;
        let mut total_participants = 0;
        let mut total_ore = 0.0;

        for round in &self.rounds {
            total_participants += round.total_participants;
            
            match &round.outcome {
                RoundOutcome::Split { total_ore: ore, .. } => {
                    split_rounds += 1;
                    total_ore += ore;
                }
                RoundOutcome::FullOre { amount, .. } => {
                    full_ore_rounds += 1;
                    total_ore += amount;
                }
                RoundOutcome::Motherlode { amount, .. } => {
                    motherlode_rounds += 1;
                    total_ore += amount;
                }
            }
        }

        let avg_participants = if total_rounds > 0 {
            total_participants as f64 / total_rounds as f64
        } else {
            0.0
        };

        OreRoundStats {
            total_rounds,
            split_rounds,
            full_ore_rounds,
            motherlode_rounds,
            split_percentage: (split_rounds as f64 / total_rounds as f64) * 100.0,
            full_ore_percentage: (full_ore_rounds as f64 / total_rounds as f64) * 100.0,
            motherlode_percentage: (motherlode_rounds as f64 / total_rounds as f64) * 100.0,
            average_participants: avg_participants,
            total_ore_distributed: total_ore,
        }
    }

    /// Get rounds in time range
    pub fn get_rounds_in_range(&self, start: i64, end: i64) -> Vec<&OreRound> {
        self.rounds
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .collect()
    }

    /// Get recent rounds
    pub fn get_recent_rounds(&self, count: usize) -> Vec<&OreRound> {
        let start = self.rounds.len().saturating_sub(count);
        self.rounds[start..].iter().collect()
    }

    /// Predict next round outcome based on patterns
    pub fn predict_next_outcome(&self) -> Result<(RoundOutcome, f64)> {
        let recent = self.get_recent_rounds(100);
        
        let mut split_count = 0;
        let mut full_ore_count = 0;
        
        for round in recent {
            match round.outcome {
                RoundOutcome::Split { .. } => split_count += 1,
                RoundOutcome::FullOre { .. } => full_ore_count += 1,
                RoundOutcome::Motherlode { .. } => {} // Rare, don't predict
            }
        }
        
        let total = split_count + full_ore_count;
        let split_prob = split_count as f64 / total as f64;
        
        if split_prob > 0.5 {
            Ok((RoundOutcome::Split { 
                total_ore: 1.0, 
                participants: 0 
            }, split_prob))
        } else {
            Ok((RoundOutcome::FullOre { 
                winner: "unknown".to_string(), 
                amount: 1.0 
            }, 1.0 - split_prob))
        }
    }

    /// Get top performing wallets
    pub fn get_top_wallets(&self, limit: usize) -> Vec<&WalletRoundPerformance> {
        let mut wallets: Vec<_> = self.wallet_performance.values().collect();
        wallets.sort_by(|a, b| b.net_profit.partial_cmp(&a.net_profit).unwrap());
        wallets.into_iter().take(limit).collect()
    }

    /// Check if motherlode is likely based on patterns
    pub fn is_motherlode_likely(&self) -> (bool, f64) {
        let recent = self.get_recent_rounds(500);
        
        let motherlode_count = recent.iter()
            .filter(|r| matches!(r.outcome, RoundOutcome::Motherlode { .. }))
            .count();
        
        let rounds_since_last = recent.iter()
            .rev()
            .position(|r| matches!(r.outcome, RoundOutcome::Motherlode { .. }))
            .unwrap_or(recent.len());
        
        // Simple heuristic: if it's been a long time, more likely
        let avg_rounds_between = if motherlode_count > 0 {
            recent.len() as f64 / motherlode_count as f64
        } else {
            1000.0 // Default if we haven't seen one
        };
        
        let likelihood = (rounds_since_last as f64 / avg_rounds_between).min(1.0);
        
        (rounds_since_last > avg_rounds_between as usize, likelihood)
    }

    /// Get wallet performance
    pub fn get_wallet_performance(&self, wallet: &str) -> Option<&WalletRoundPerformance> {
        self.wallet_performance.get(wallet)
    }

    /// Get all rounds
    pub fn get_all_rounds(&self) -> &[OreRound] {
        &self.rounds
    }

    /// Export data for analysis
    pub fn export_to_json(&self) -> Result<String> {
        let data = serde_json::json!({
            "rounds": self.rounds,
            "stats": self.calculate_stats(),
            "top_wallets": self.get_top_wallets(10),
        });
        
        Ok(serde_json::to_string_pretty(&data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_tracking() {
        let mut tracker = OreRoundTracker::new();
        
        tracker.add_round(OreRound {
            round_id: 1,
            timestamp: 1000,
            outcome: RoundOutcome::Split { total_ore: 10.0, participants: 5 },
            total_participants: 5,
            total_deployed: 100,
            duration_seconds: 60,
        });
        
        tracker.add_round(OreRound {
            round_id: 2,
            timestamp: 1060,
            outcome: RoundOutcome::FullOre { 
                winner: "wallet1".to_string(), 
                amount: 1.0 
            },
            total_participants: 10,
            total_deployed: 200,
            duration_seconds: 60,
        });
        
        let stats = tracker.calculate_stats();
        assert_eq!(stats.total_rounds, 2);
        assert_eq!(stats.split_rounds, 1);
        assert_eq!(stats.full_ore_rounds, 1);
    }
}
