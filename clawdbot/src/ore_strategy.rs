use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ORE Mining Strategy Engine
/// Learns optimal play from ALL on-chain players (not just whales)
/// Key objectives:
/// 1. Find optimal number of squares (1-25) for ROI
/// 2. Target low-competition rounds for better ORE splits
/// 3. Maximize rounds played with limited SOL
/// 4. Extract maximum ORE rewards

pub const BOARD_SIZE: usize = 25;
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Player performance data learned from on-chain activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPerformance {
    pub address: String,
    pub total_deployed: u64,
    pub total_won: u64,
    pub total_rounds: u32,
    pub wins: u32,
    pub avg_squares_per_deploy: f64,
    pub preferred_square_count: u8,
    pub avg_deploy_size: u64,
    pub roi: f64,  // Return on investment
    pub ore_per_sol: f64,  // ORE earned per SOL spent
}

/// Round conditions analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundConditions {
    pub round_id: u64,
    pub total_deployed: u64,
    pub num_deployers: u32,
    pub avg_deploy_size: u64,
    pub competition_level: CompetitionLevel,
    pub expected_ore_multiplier: f64,
    pub squares_with_deploys: u8,
    pub empty_squares: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CompetitionLevel {
    VeryLow,   // < 0.5 SOL total - best for ORE
    Low,       // 0.5 - 2 SOL
    Medium,    // 2 - 10 SOL
    High,      // 10 - 50 SOL
    VeryHigh,  // > 50 SOL - worst for ORE splits
}

impl CompetitionLevel {
    pub fn from_deployed(lamports: u64) -> Self {
        let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
        if sol < 0.5 {
            Self::VeryLow
        } else if sol < 2.0 {
            Self::Low
        } else if sol < 10.0 {
            Self::Medium
        } else if sol < 50.0 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    /// Expected ORE multiplier based on competition
    /// Lower competition = higher ORE per winner
    pub fn ore_multiplier(&self) -> f64 {
        match self {
            Self::VeryLow => 2.0,   // Can get +2 or higher ORE
            Self::Low => 1.5,       // +1.5 ORE typical
            Self::Medium => 1.0,    // +1 ORE typical
            Self::High => 0.5,      // Split rewards
            Self::VeryHigh => 0.25, // Heavy splits
        }
    }
}

/// Learned optimal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalConfig {
    pub squares_to_play: u8,
    pub deploy_per_square_lamports: u64,
    pub total_deploy_lamports: u64,
    pub target_competition: CompetitionLevel,
    pub confidence: f64,
    pub reasoning: String,
}

/// Deployment decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployDecision {
    pub should_deploy: bool,
    pub squares: Vec<usize>,
    pub total_amount_lamports: u64,
    pub per_square_lamports: u64,
    pub expected_ore: f64,
    pub reasoning: String,
    pub skip_reason: Option<String>,
}

/// Main ORE Strategy Engine
pub struct OreStrategyEngine {
    // Learned from all players
    player_stats: HashMap<String, PlayerPerformance>,
    
    // Learned optimal square counts
    square_count_performance: [SquareCountStats; 26], // Index 0 unused, 1-25 valid
    
    // Round history for pattern detection
    round_history: Vec<RoundConditions>,
    
    // Configuration limits
    pub min_wallet_sol: f64,
    pub max_bet_per_round_sol: f64,
    pub target_rounds_per_session: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SquareCountStats {
    pub count: u8,
    pub times_used: u32,
    pub times_won: u32,
    pub total_deployed: u64,
    pub total_won: u64,
    pub avg_ore_earned: f64,
    pub win_rate: f64,
    pub roi: f64,
}

impl OreStrategyEngine {
    pub fn new() -> Self {
        let mut square_count_performance: [SquareCountStats; 26] = Default::default();
        for i in 0..26 {
            square_count_performance[i].count = i as u8;
        }
        
        Self {
            player_stats: HashMap::new(),
            square_count_performance,
            round_history: Vec::new(),
            min_wallet_sol: 0.05,        // Keep at least 0.05 SOL
            max_bet_per_round_sol: 0.04, // Max 0.04 SOL per round total
            target_rounds_per_session: 100, // Try to play 100 rounds
        }
    }

    /// Load learned data from database
    pub fn load_player_stats(&mut self, stats: Vec<PlayerPerformance>) {
        for stat in stats {
            self.player_stats.insert(stat.address.clone(), stat);
        }
    }

    /// Load square count performance from database
    pub fn load_square_count_stats(&mut self, stats: Vec<SquareCountStats>) {
        for stat in stats {
            if stat.count > 0 && stat.count <= 25 {
                self.square_count_performance[stat.count as usize] = stat.clone();
            }
        }
    }

    /// Record a player's deploy for learning
    pub fn record_deploy(
        &mut self,
        address: &str,
        amount_lamports: u64,
        square_count: u8,
    ) {
        let player = self.player_stats.entry(address.to_string()).or_insert_with(|| {
            PlayerPerformance {
                address: address.to_string(),
                total_deployed: 0,
                total_won: 0,
                total_rounds: 0,
                wins: 0,
                avg_squares_per_deploy: 0.0,
                preferred_square_count: 0,
                avg_deploy_size: 0,
                roi: 0.0,
                ore_per_sol: 0.0,
            }
        });

        player.total_deployed += amount_lamports;
        player.total_rounds += 1;
        
        // Update rolling average of squares used
        let n = player.total_rounds as f64;
        player.avg_squares_per_deploy = 
            ((player.avg_squares_per_deploy * (n - 1.0)) + square_count as f64) / n;
        
        player.avg_deploy_size = player.total_deployed / player.total_rounds as u64;

        // Update square count stats
        if square_count > 0 && square_count <= 25 {
            self.square_count_performance[square_count as usize].times_used += 1;
            self.square_count_performance[square_count as usize].total_deployed += amount_lamports;
        }
    }

    /// Record a win for learning
    pub fn record_win(
        &mut self,
        address: &str,
        amount_won_lamports: u64,
        ore_earned: f64,
        square_count: u8,
    ) {
        if let Some(player) = self.player_stats.get_mut(address) {
            player.total_won += amount_won_lamports;
            player.wins += 1;
            player.roi = if player.total_deployed > 0 {
                (player.total_won as f64 - player.total_deployed as f64) / player.total_deployed as f64
            } else {
                0.0
            };
            
            // Calculate ORE per SOL
            let total_sol = player.total_deployed as f64 / LAMPORTS_PER_SOL as f64;
            if total_sol > 0.0 {
                // This would need actual ORE tracking, simplified here
                player.ore_per_sol = ore_earned / total_sol;
            }
        }

        // Update square count stats
        if square_count > 0 && square_count <= 25 {
            let stats = &mut self.square_count_performance[square_count as usize];
            stats.times_won += 1;
            stats.total_won += amount_won_lamports;
            stats.win_rate = stats.times_won as f64 / stats.times_used.max(1) as f64;
            stats.roi = if stats.total_deployed > 0 {
                (stats.total_won as f64 - stats.total_deployed as f64) / stats.total_deployed as f64
            } else {
                0.0
            };
        }
    }

    /// Record a completed round for learning (update square stats based on winning square)
    pub fn record_round(&mut self, deployed: &[u64; 25], winning_square: u8) {
        // Count how many squares were deployed to
        let squares_with_deploys: Vec<u8> = deployed.iter()
            .enumerate()
            .filter(|(_, &d)| d > 0)
            .map(|(i, _)| i as u8)
            .collect();
        
        let num_squares = squares_with_deploys.len() as u8;
        
        // Update square count performance based on whether this count would have won
        // This is aggregate learning - not player-specific
        if num_squares > 0 && num_squares <= 25 {
            let stats = &mut self.square_count_performance[num_squares as usize];
            stats.times_used += 1;
            
            // If betting on this many squares would catch the winning square,
            // that's a statistical win for this strategy
            // (This is a simplified model - real wins depend on which specific squares)
        }
        
        // Track winning square frequency for pattern detection
        // Note: empty_squares stored as 1-25 to match ORE UI
        self.round_history.push(RoundConditions {
            round_id: 0, // Set externally if needed
            total_deployed: deployed.iter().sum(),
            num_deployers: squares_with_deploys.len() as u32,
            avg_deploy_size: deployed.iter().sum::<u64>() / squares_with_deploys.len().max(1) as u64,
            competition_level: CompetitionLevel::from_deployed(deployed.iter().sum()),
            expected_ore_multiplier: 1.0,
            squares_with_deploys: num_squares,
            empty_squares: deployed.iter().enumerate().filter(|(_, &d)| d == 0).map(|(i, _)| i + 1).collect(),
        });
        
        // Keep only last 1000 rounds
        if self.round_history.len() > 1000 {
            self.round_history.remove(0);
        }
    }

    /// Analyze current round conditions
    /// Note: empty_squares returned as 1-25 (not 0-24) to match ORE UI
    pub fn analyze_round(&self, deployed: &[u64; 25], num_deployers: u32) -> RoundConditions {
        let total_deployed: u64 = deployed.iter().sum();
        let squares_with_deploys = deployed.iter().filter(|&&d| d > 0).count() as u8;
        // Convert 0-24 indices to 1-25 for output
        let empty_squares: Vec<usize> = deployed.iter()
            .enumerate()
            .filter(|(_, &d)| d == 0)
            .map(|(i, _)| i + 1)  // +1 to convert to 1-25
            .collect();

        let avg_deploy_size = if num_deployers > 0 {
            total_deployed / num_deployers as u64
        } else {
            0
        };

        let competition = CompetitionLevel::from_deployed(total_deployed);

        RoundConditions {
            round_id: 0, // Set externally
            total_deployed,
            num_deployers,
            avg_deploy_size,
            competition_level: competition,
            expected_ore_multiplier: competition.ore_multiplier(),
            squares_with_deploys,
            empty_squares,
        }
    }

    /// Find optimal number of squares based on learned data
    /// Uses PURE learning - no preset defaults, explores when no data
    pub fn get_optimal_square_count(&self) -> (u8, f64, String) {
        let mut best_count = 0u8;
        let mut best_ev = f64::NEG_INFINITY;
        let mut reasoning = String::new();

        // Find which counts have enough data AND wins (minimum 5 samples to start learning)
        let mut counts_with_data = 0;
        let mut counts_with_wins = 0;
        let min_samples = 5; // Lowered from 10 to learn faster
        
        // First pass: check if ANY count has wins
        for count in 1..=25u8 {
            let stats = &self.square_count_performance[count as usize];
            if stats.times_used >= min_samples as u32 {
                counts_with_data += 1;
                if stats.times_won > 0 {
                    counts_with_wins += 1;
                }
            }
        }
        
        // Calculate EV for each square count (1-25)
        // EV = (win_probability * expected_reward) - cost
        // With no data, use theoretical probabilities
        if counts_with_wins == 0 {
            // Use pure EV calculation - can pick ANY count from 1-25
            // Win probability = count/25, Reward scales with 1/sqrt(count), Cost = count
            for count in 1..=25u8 {
                let win_prob = count as f64 / 25.0;
                // Assume 1 ORE reward, shared proportionally if multiple winners
                // More squares = better odds but smaller share
                let expected_ore = 1.0 / (count as f64).sqrt();
                let cost = count as f64 * 0.001; // ~0.001 SOL per square
                
                let ev = (win_prob * expected_ore) - cost;
                
                if ev > best_ev {
                    best_ev = ev;
                    best_count = count;
                }
            }
            
            return (best_count, best_ev, format!(
                "EV-OPTIMAL: {} squares (EV={:.4}) - higher count = better odds, exploring full range 1-25",
                best_count, best_ev
            ));
        }
        
        // Score each square count by EV using learned data
        // Can pick ANY count from 1-25 if EV is positive
        for count in 1..=25u8 {
            let stats = &self.square_count_performance[count as usize];
            
            // Use actual data if available, otherwise theoretical
            let win_prob = if stats.times_used >= min_samples as u32 && stats.times_won > 0 {
                stats.win_rate as f64
            } else if stats.times_used >= min_samples as u32 {
                // Tried but no wins - use lower estimate
                0.5 * (count as f64 / 25.0)
            } else {
                // No data - use theoretical
                count as f64 / 25.0
            };
            
            // Expected ORE when winning (from actual data if available)
            let expected_ore = if stats.times_won > 0 && stats.avg_ore_earned > 0.0 {
                stats.avg_ore_earned as f64
            } else {
                1.0 / (count as f64).sqrt() // Theoretical
            };
            
            let cost = count as f64 * 0.001; // SOL cost
            let ev = (win_prob * expected_ore) - cost;

            if ev > best_ev {
                best_ev = ev;
                best_count = count;
                if stats.times_won > 0 {
                    reasoning = format!(
                        "LEARNED: {} squares, EV={:.4}, {:.1}% win rate ({} wins), {:.3} avg ORE",
                        count, ev, stats.win_rate * 100.0, stats.times_won, stats.avg_ore_earned
                    );
                } else {
                    reasoning = format!(
                        "EV-OPTIMAL: {} squares (EV={:.4}), theoretical estimate",
                        count, ev
                    );
                }
            }
        }

        // If somehow no positive EV found, pick based on exploration
        if best_count == 0 || best_ev <= 0.0 {
            best_count = self.pick_exploration_count();
            best_ev = 0.0;
            reasoning = format!(
                "EXPLORING: {} squares - gathering data across 1-25 range",
                best_count
            );
        }

        (best_count, best_ev, reasoning)
    }
    
    /// Pick a square count to explore (one we have less data on)
    /// Can explore ANY count from 1-25, prioritizes least-sampled
    fn pick_exploration_count(&self) -> u8 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // All counts from 1-25 are valid exploration targets
        let mut exploration_candidates: Vec<(u8, u32)> = (1..=25u8)
            .map(|count| {
                let samples = self.square_count_performance[count as usize].times_used;
                (count, samples)
            })
            .collect();
        
        // Sort by fewest samples first (explore the unknown)
        exploration_candidates.sort_by_key(|(_count, samples)| *samples);
        
        // Use microseconds for better randomness
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        
        // Pick from ALL 25 counts, weighted by exploration need
        // Lower samples = picked more often
        let total_inverse: u32 = exploration_candidates.iter()
            .map(|(_, samples)| 1000 / (samples + 1)) // +1 to avoid division by zero
            .sum();
        
        // Add round counter to break determinism across calls
        let round_offset = self.round_history.len() as u128;
        let random_val = ((now + round_offset * 12345) as u32) % total_inverse.max(1);
        let mut cumulative = 0u32;
        
        for (count, samples) in &exploration_candidates {
            cumulative += 1000 / (samples + 1);
            if cumulative >= random_val {
                return *count;
            }
        }
        
        // Fallback: return the least explored
        exploration_candidates[0].0
    }
    
    /// Get win rates by square count for AI context
    pub fn get_square_count_win_rates(&self) -> Vec<(u8, f64)> {
        self.square_count_performance.iter()
            .filter(|s| s.times_used >= 5)  // Only counts with enough data
            .map(|s| (s.count, s.win_rate))
            .collect()
    }

    /// Get top performing players to learn from
    pub fn get_top_performers(&self, limit: usize) -> Vec<&PlayerPerformance> {
        let mut players: Vec<_> = self.player_stats.values()
            .filter(|p| p.total_rounds >= 10) // Minimum activity
            .collect();
        
        // Sort by ROI * win_rate (combined performance)
        players.sort_by(|a, b| {
            let score_a = a.roi * (a.wins as f64 / a.total_rounds.max(1) as f64);
            let score_b = b.roi * (b.wins as f64 / b.total_rounds.max(1) as f64);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        players.into_iter().take(limit).collect()
    }

    /// Main decision: should we deploy this round?
    pub fn make_deploy_decision(
        &self,
        wallet_balance_lamports: u64,
        current_round_deployed: &[u64; 25],
        num_deployers: u32,
        consensus_squares: &[usize],
        consensus_confidence: f64,
    ) -> DeployDecision {
        let wallet_sol = wallet_balance_lamports as f64 / LAMPORTS_PER_SOL as f64;
        let conditions = self.analyze_round(current_round_deployed, num_deployers);

        // Check if we have enough balance
        if wallet_sol < self.min_wallet_sol {
            return DeployDecision {
                should_deploy: false,
                squares: vec![],
                total_amount_lamports: 0,
                per_square_lamports: 0,
                expected_ore: 0.0,
                reasoning: String::new(),
                skip_reason: Some(format!(
                    "Wallet balance {:.4} SOL below minimum {:.4} SOL",
                    wallet_sol, self.min_wallet_sol
                )),
            };
        }

        // Calculate available budget (leave min_wallet_sol)
        let available_sol = wallet_sol - self.min_wallet_sol;
        let max_this_round = available_sol.min(self.max_bet_per_round_sol);

        // Decide based on competition level
        let (should_play, ore_multiplier, skip_reason) = match conditions.competition_level {
            CompetitionLevel::VeryLow => (true, 2.0, None),
            CompetitionLevel::Low => (true, 1.5, None),
            CompetitionLevel::Medium => (true, 1.0, None),
            CompetitionLevel::High => {
                // Only play if high confidence
                if consensus_confidence > 0.6 {
                    (true, 0.5, None)
                } else {
                    (false, 0.0, Some("High competition, low confidence - skipping".to_string()))
                }
            }
            CompetitionLevel::VeryHigh => {
                (false, 0.0, Some("Very high competition - skip for better ORE splits".to_string()))
            }
        };

        if !should_play {
            return DeployDecision {
                should_deploy: false,
                squares: vec![],
                total_amount_lamports: 0,
                per_square_lamports: 0,
                expected_ore: 0.0,
                reasoning: String::new(),
                skip_reason,
            };
        }

        // Get optimal square count
        let (optimal_count, _, square_reasoning) = self.get_optimal_square_count();
        
        // Use consensus squares if available, otherwise pick based on empty squares
        // All squares are 1-25 range
        let squares: Vec<usize> = if !consensus_squares.is_empty() && consensus_confidence > 0.4 {
            consensus_squares.iter()
                .take(optimal_count as usize)
                .copied()
                .collect()
        } else if !conditions.empty_squares.is_empty() {
            // Prefer empty squares (less competition) - already 1-25
            conditions.empty_squares.iter()
                .take(optimal_count as usize)
                .copied()
                .collect()
        } else {
            // Random fallback - use 1-25 range
            (1..=optimal_count as usize).collect()
        };

        let num_squares = squares.len();
        
        // Total amount is max_this_round, divided across squares
        let total_amount_lamports = (max_this_round * LAMPORTS_PER_SOL as f64) as u64;
        let per_square_lamports = total_amount_lamports / num_squares as u64;

        // Expected ORE calculation
        let win_probability = num_squares as f64 / 25.0;
        let expected_ore = win_probability * ore_multiplier;

        DeployDecision {
            should_deploy: true,
            squares,
            total_amount_lamports,
            per_square_lamports,
            expected_ore,
            reasoning: format!(
                "Competition: {:?} ({}x ORE), {} squares ({}), {:.4} SOL total",
                conditions.competition_level,
                ore_multiplier,
                num_squares,
                square_reasoning,
                total_amount_lamports as f64 / LAMPORTS_PER_SOL as f64
            ),
            skip_reason: None,
        }
    }

    /// Calculate how many rounds we can play with current balance
    pub fn estimate_rounds_remaining(&self, wallet_balance_lamports: u64) -> u32 {
        let wallet_sol = wallet_balance_lamports as f64 / LAMPORTS_PER_SOL as f64;
        let playable_sol = (wallet_sol - self.min_wallet_sol).max(0.0);
        
        if self.max_bet_per_round_sol > 0.0 {
            (playable_sol / self.max_bet_per_round_sol) as u32
        } else {
            0
        }
    }

    /// Get summary of what we've learned
    pub fn get_learning_summary(&self) -> serde_json::Value {
        let (optimal_squares, _, reasoning) = self.get_optimal_square_count();
        let top_performers = self.get_top_performers(5);
        
        let top_players: Vec<serde_json::Value> = top_performers.iter()
            .map(|p| serde_json::json!({
                "address": &p.address[..8],
                "rounds": p.total_rounds,
                "win_rate": format!("{:.1}%", (p.wins as f64 / p.total_rounds.max(1) as f64) * 100.0),
                "avg_squares": format!("{:.1}", p.avg_squares_per_deploy),
                "roi": format!("{:.1}%", p.roi * 100.0),
            }))
            .collect();

        // Best square counts by win rate
        let mut square_counts: Vec<_> = self.square_count_performance.iter()
            .filter(|s| s.times_used >= 5)
            .collect();
        square_counts.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap_or(std::cmp::Ordering::Equal));

        let best_counts: Vec<serde_json::Value> = square_counts.iter().take(5)
            .map(|s| serde_json::json!({
                "squares": s.count,
                "win_rate": format!("{:.1}%", s.win_rate * 100.0),
                "roi": format!("{:.1}%", s.roi * 100.0),
                "samples": s.times_used,
            }))
            .collect();

        serde_json::json!({
            "total_players_tracked": self.player_stats.len(),
            "optimal_square_count": optimal_squares,
            "optimal_reasoning": reasoning,
            "top_performers": top_players,
            "best_square_counts": best_counts,
            "config": {
                "min_wallet_sol": self.min_wallet_sol,
                "max_bet_per_round_sol": self.max_bet_per_round_sol,
            }
        })
    }
    
    /// Apply a detected strategy from the learning engine
    /// This allows copying strategies that work for successful players
    pub fn apply_detected_strategy(&mut self, strategy: &serde_json::Value) {
        // Extract and apply strategy parameters
        if let Some(square_count) = strategy["square_count"].as_u64() {
            // Update the square count stats to reflect this as optimal
            let count = (square_count as u8).min(25).max(1);
            let stats = &mut self.square_count_performance[count as usize];
            // Boost the confidence in this square count
            stats.times_used += 100; // Add significant weight
            stats.win_rate = strategy["confidence"].as_f64().unwrap_or(0.5);
            log::info!("📊 Applied detected strategy: {} squares", count);
        }
        
        if let Some(bet_size) = strategy["bet_size_sol"].as_f64() {
            // Adjust max bet per round based on detected successful patterns
            if bet_size > 0.001 && bet_size <= 0.1 {
                self.max_bet_per_round_sol = bet_size.min(0.04); // Still cap at our limit
                log::info!("💰 Adjusted bet size to {:.4} SOL", self.max_bet_per_round_sol);
            }
        }
        
        if let Some(target) = strategy["target_competition"].as_str() {
            // Log the target competition level for decision making
            log::info!("🎯 Target competition: {}", target);
        }
        
        log::info!("🧠 Strategy applied: {} (confidence: {:.0}%)",
            strategy["name"].as_str().unwrap_or("Unknown"),
            strategy["confidence"].as_f64().unwrap_or(0.0) * 100.0);
    }
    
    /// Apply the best detected strategy from a list
    pub fn apply_best_strategy(&mut self, strategies: &[serde_json::Value]) {
        // Find the strategy with highest confidence
        let best = strategies.iter()
            .max_by(|a, b| {
                let conf_a = a["confidence"].as_f64().unwrap_or(0.0);
                let conf_b = b["confidence"].as_f64().unwrap_or(0.0);
                conf_a.partial_cmp(&conf_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        
        if let Some(strategy) = best {
            if strategy["confidence"].as_f64().unwrap_or(0.0) > 0.5 {
                self.apply_detected_strategy(strategy);
            } else {
                log::info!("🔍 Best strategy confidence too low ({:.0}%), will explore instead",
                    strategy["confidence"].as_f64().unwrap_or(0.0) * 100.0);
            }
        }
    }
}

impl Default for OreStrategyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_competition_levels() {
        assert_eq!(CompetitionLevel::from_deployed(100_000_000), CompetitionLevel::VeryLow);
        assert_eq!(CompetitionLevel::from_deployed(1_000_000_000), CompetitionLevel::Low);
        assert_eq!(CompetitionLevel::from_deployed(5_000_000_000), CompetitionLevel::Medium);
        assert_eq!(CompetitionLevel::from_deployed(20_000_000_000), CompetitionLevel::High);
        assert_eq!(CompetitionLevel::from_deployed(100_000_000_000), CompetitionLevel::VeryHigh);
    }

    #[test]
    fn test_deploy_decision() {
        let engine = OreStrategyEngine::new();
        let deployed = [0u64; 25]; // Empty round
        
        let decision = engine.make_deploy_decision(
            100_000_000, // 0.1 SOL
            &deployed,
            0,
            &[5, 10, 15],
            0.7,
        );

        assert!(decision.should_deploy);
        assert!(!decision.squares.is_empty());
        assert!(decision.total_amount_lamports > 0);
    }

    #[test]
    fn test_skip_high_competition() {
        let engine = OreStrategyEngine::new();
        let mut deployed = [0u64; 25];
        // Make it very high competition (100 SOL)
        for d in &mut deployed {
            *d = 4_000_000_000; // 4 SOL each = 100 SOL total
        }
        
        let decision = engine.make_deploy_decision(
            100_000_000,
            &deployed,
            50,
            &[5],
            0.3, // Low confidence
        );

        assert!(!decision.should_deploy);
        assert!(decision.skip_reason.is_some());
    }
}
