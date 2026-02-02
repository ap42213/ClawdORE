use crate::error::Result;
use ore_api::state::{Miner, Round};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundAnalytics {
    pub round_id: u64,
    pub total_deployed: u64,
    pub total_miners: usize,
    pub winning_square: Option<usize>,
    pub winning_amount: u64,
    pub total_winners: usize,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerPerformance {
    pub address: String,
    pub total_deployed: u64,
    pub total_winnings: u64,
    pub rounds_participated: usize,
    pub win_rate: f64,
    pub roi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareStatistics {
    pub square_id: usize,
    pub times_won: usize,
    pub total_deployed: u64,
    pub average_deployment: f64,
    pub win_frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallAnalytics {
    pub total_rounds_analyzed: usize,
    pub total_ore_minted: u64,
    pub total_sol_deployed: u64,
    pub average_round_duration: i64,
    pub most_winning_square: usize,
    pub least_winning_square: usize,
    pub square_statistics: Vec<SquareStatistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_profit_loss: f64,
    pub win_rate: f64,
    pub average_bet_size: f64,
    pub roi: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub total_bets: usize,
    pub winning_bets: usize,
}

pub struct AnalyticsEngine {
    round_history: Vec<(u64, Round)>,
    miner_stats: HashMap<String, MinerPerformance>,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        Self {
            round_history: Vec::new(),
            miner_stats: HashMap::new(),
        }
    }

    pub fn add_round(&mut self, round_id: u64, round: Round) {
        self.round_history.push((round_id, round));
    }

    pub fn analyze_rounds(&self) -> Result<Vec<RoundAnalytics>> {
        let mut analytics = Vec::new();

        for (round_id, round) in &self.round_history {
            let total_deployed: u64 = round.deployed.iter().sum();
            
            analytics.push(RoundAnalytics {
                round_id: *round_id,
                total_deployed,
                total_miners: 0, // Would need to count from miner data
                winning_square: None, // Would need to determine from round data
                winning_amount: 0,
                total_winners: 0,
                timestamp: 0, // Would need block time
            });
        }

        Ok(analytics)
    }

    pub fn analyze_squares(&self) -> Result<Vec<SquareStatistics>> {
        let mut square_stats: HashMap<usize, SquareStatistics> = HashMap::new();

        // Initialize all squares
        for i in 0..25 {
            square_stats.insert(
                i,
                SquareStatistics {
                    square_id: i,
                    times_won: 0,
                    total_deployed: 0,
                    average_deployment: 0.0,
                    win_frequency: 0.0,
                },
            );
        }

        // Analyze each round
        for (_, round) in &self.round_history {
            for (square_id, &deployed) in round.deployed.iter().enumerate() {
                if let Some(stats) = square_stats.get_mut(&square_id) {
                    stats.total_deployed += deployed;
                }
            }
        }

        // Calculate averages
        let total_rounds = self.round_history.len();
        for stats in square_stats.values_mut() {
            if total_rounds > 0 {
                stats.average_deployment = stats.total_deployed as f64 / total_rounds as f64;
                stats.win_frequency = stats.times_won as f64 / total_rounds as f64;
            }
        }

        Ok(square_stats.into_values().collect())
    }

    pub fn get_overall_analytics(&self) -> Result<OverallAnalytics> {
        let square_stats = self.analyze_squares()?;
        
        let mut most_winning = 0;
        let mut least_winning = 0;
        let mut max_wins = 0;
        let mut min_wins = usize::MAX;

        for stats in &square_stats {
            if stats.times_won > max_wins {
                max_wins = stats.times_won;
                most_winning = stats.square_id;
            }
            if stats.times_won < min_wins {
                min_wins = stats.times_won;
                least_winning = stats.square_id;
            }
        }

        let total_sol_deployed: u64 = self
            .round_history
            .iter()
            .map(|(_, round)| round.deployed.iter().sum::<u64>())
            .sum();

        Ok(OverallAnalytics {
            total_rounds_analyzed: self.round_history.len(),
            total_ore_minted: 0, // Would calculate from treasury data
            total_sol_deployed,
            average_round_duration: 0, // Would calculate from timestamps
            most_winning_square: most_winning,
            least_winning_square: least_winning,
            square_statistics: square_stats,
        })
    }
    pub fn calculate_performance_metrics(
        &self,
        bets: &[(u64, usize, f64)], // (round_id, square, amount)
        wins: &[(u64, f64)], // (round_id, payout)
    ) -> Result<PerformanceMetrics> {
        let total_bets = bets.len();
        let winning_bets = wins.len();
        
        let total_wagered: f64 = bets.iter().map(|(_, _, amt)| amt).sum();
        let total_winnings: f64 = wins.iter().map(|(_, payout)| payout).sum();
        let profit_loss = total_winnings - total_wagered;
        
        let win_rate = if total_bets > 0 {
            winning_bets as f64 / total_bets as f64
        } else {
            0.0
        };
        
        let avg_bet_size = if total_bets > 0 {
            total_wagered / total_bets as f64
        } else {
            0.0
        };
        
        let roi = if total_wagered > 0.0 {
            (profit_loss / total_wagered) * 100.0
        } else {
            0.0
        };
        
        // Calculate Sharpe ratio (simplified)
        let returns: Vec<f64> = bets.iter().map(|(round_id, _, amt)| {
            let win = wins.iter().find(|(r, _)| r == round_id);
            if let Some((_, payout)) = win {
                (payout - amt) / amt
            } else {
                -1.0
            }
        }).collect();
        
        let mean_return = if !returns.is_empty() {
            returns.iter().sum::<f64>() / returns.len() as f64
        } else {
            0.0
        };
        
        let std_dev = if returns.len() > 1 {
            let variance: f64 = returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / (returns.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        let sharpe_ratio = if std_dev > 0.0 {
            mean_return / std_dev
        } else {
            0.0
        };
        
        // Calculate max drawdown
        let mut peak = 0.0;
        let mut max_dd = 0.0;
        let mut running_pnl = 0.0;
        
        for (round_id, _, amt) in bets {
            running_pnl -= amt;
            if let Some((_, payout)) = wins.iter().find(|(r, _)| r == round_id) {
                running_pnl += payout;
            }
            
            if running_pnl > peak {
                peak = running_pnl;
            }
            
            let drawdown = peak - running_pnl;
            if drawdown > max_dd {
                max_dd = drawdown;
            }
        }
        
        Ok(PerformanceMetrics {
            total_profit_loss: profit_loss,
            win_rate,
            average_bet_size: avg_bet_size,
            roi,
            sharpe_ratio,
            max_drawdown: max_dd,
            total_bets,
            winning_bets,
        })
    }
    pub fn analyze_miner(&self, miner: &Miner) -> MinerPerformance {
        MinerPerformance {
            address: "".to_string(), // Would use actual address
            total_deployed: miner.lifetime_deployed,
            total_winnings: miner.lifetime_rewards_sol,
            rounds_participated: miner.round_id as usize,
            win_rate: 0.0, // Calculate from history
            roi: if miner.lifetime_deployed > 0 {
                (miner.lifetime_rewards_sol as f64 / miner.lifetime_deployed as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    pub fn predict_winning_squares(&self, top_n: usize) -> Result<Vec<usize>> {
        let square_stats = self.analyze_squares()?;
        
        let mut sorted = square_stats;
        sorted.sort_by(|a, b| b.win_frequency.partial_cmp(&a.win_frequency).unwrap());
        
        Ok(sorted.iter().take(top_n).map(|s| s.square_id).collect())
    }

    pub fn export_to_json(&self, path: &str) -> Result<()> {
        let analytics = self.get_overall_analytics()?;
        let json = serde_json::to_string_pretty(&analytics)
            .map_err(|e| crate::error::BotError::Serialization(e.to_string()))?;
        
        std::fs::write(path, json)
            .map_err(|e| crate::error::BotError::Other(e.to_string()))?;
        
        Ok(())
    }

    pub fn clear_history(&mut self) {
        self.round_history.clear();
        self.miner_stats.clear();
    }

    pub fn get_recent_trends(&self, last_n_rounds: usize) -> Result<Vec<SquareStatistics>> {
        let recent_rounds = self
            .round_history
            .iter()
            .rev()
            .take(last_n_rounds)
            .collect::<Vec<_>>();

        let mut square_stats: HashMap<usize, SquareStatistics> = HashMap::new();

        for i in 0..25 {
            square_stats.insert(
                i,
                SquareStatistics {
                    square_id: i,
                    times_won: 0,
                    total_deployed: 0,
                    average_deployment: 0.0,
                    win_frequency: 0.0,
                },
            );
        }

        for (_, round) in recent_rounds.iter() {
            for (square_id, &deployed) in round.deployed.iter().enumerate() {
                if let Some(stats) = square_stats.get_mut(&square_id) {
                    stats.total_deployed += deployed;
                }
            }
        }

        for stats in square_stats.values_mut() {
            if last_n_rounds > 0 {
                stats.average_deployment = stats.total_deployed as f64 / last_n_rounds as f64;
            }
        }

        Ok(square_stats.into_values().collect())
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}
