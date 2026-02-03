use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced strategy engine for ORE betting
/// Analyzes historical data to find edges and opportunities

/// Historical round data for analysis
#[derive(Debug, Clone)]
pub struct RoundHistory {
    pub round_id: u64,
    pub winning_square: u8,
    pub deployed: [u64; 25],
    pub total_pot: u64,
    pub motherlode: bool,
    pub timestamp: Option<i64>,
}

/// Square statistics computed from history
#[derive(Debug, Clone, Default)]
pub struct SquareStats {
    pub wins: u32,
    pub total_rounds: u32,
    pub total_deployed_when_won: u64,
    pub total_pot_when_won: u64,
    pub avg_competition: f64,      // Average SOL deployed to this square
    pub win_rate: f64,             // Actual win rate
    pub expected_rate: f64,        // Expected rate (1/25 = 4%)
    pub edge: f64,                 // win_rate - expected_rate
    pub roi: f64,                  // Historical ROI
    pub recent_wins: u32,          // Wins in last 100 rounds
    pub streak: i32,               // Current win/loss streak (positive = wins)
}

/// Strategy recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecommendation {
    pub strategy_name: String,
    pub squares: Vec<usize>,
    pub weights: Vec<f64>,         // How much to allocate to each square (0.0 - 1.0)
    pub confidence: f64,           // 0.0 - 1.0
    pub expected_roi: f64,
    pub reasoning: String,
}

/// Main strategy engine
pub struct StrategyEngine {
    history: Vec<RoundHistory>,
    square_stats: [SquareStats; 25],
    whale_positions: HashMap<String, Vec<usize>>, // Whale address -> their favorite squares
    strategy_weights: HashMap<String, f64>,       // Learned strategy performance
}

impl StrategyEngine {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            square_stats: Default::default(),
            whale_positions: HashMap::new(),
            strategy_weights: HashMap::new(),
        }
    }

    /// Load persisted square stats from database
    pub fn load_square_stats_from_db(&mut self, stats: Vec<(i16, i32, i32, i64, f32, f32, i32, i64)>) {
        for (square_id, wins, rounds, deployed, win_rate, edge, streak, avg_comp) in stats {
            if (square_id as usize) < 25 {
                self.square_stats[square_id as usize] = SquareStats {
                    wins: wins as u32,
                    total_rounds: rounds as u32,
                    total_deployed_when_won: deployed as u64,
                    total_pot_when_won: 0,
                    avg_competition: avg_comp as f64,
                    win_rate: win_rate as f64,
                    expected_rate: 0.04,
                    edge: edge as f64,
                    roi: 0.0,
                    recent_wins: 0,
                    streak: streak,
                };
            }
        }
    }

    /// Load whale positions from database
    pub fn load_whales_from_db(&mut self, whales: Vec<(String, i64, Vec<i32>)>) {
        for (address, _deployed, squares) in whales {
            self.whale_positions.insert(
                address, 
                squares.into_iter().map(|s| s as usize).collect()
            );
        }
    }

    /// Load historical rounds from database
    pub fn load_rounds_from_db(&mut self, rounds: Vec<(i64, i16, Vec<i64>, i64, bool)>) {
        for (round_id, winning_square, deployed_vec, total, motherlode) in rounds {
            if winning_square >= 0 && deployed_vec.len() == 25 {
                let mut deployed = [0u64; 25];
                for (i, &d) in deployed_vec.iter().enumerate() {
                    deployed[i] = d as u64;
                }
                self.history.push(RoundHistory {
                    round_id: round_id as u64,
                    winning_square: winning_square as u8,
                    deployed,
                    total_pot: total as u64,
                    motherlode,
                    timestamp: None,
                });
            }
        }
        // Sort by round_id ascending (oldest first)
        self.history.sort_by_key(|r| r.round_id);
    }

    /// Load strategy performance weights
    pub fn load_strategy_weights(&mut self, perf: Vec<(String, i64, i64, f64)>) {
        for (name, _total, _hits, hit_rate) in perf {
            // Weight strategies by their historical hit rate
            self.strategy_weights.insert(name, hit_rate);
        }
    }

    /// Get loaded history count
    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    /// Get loaded whale count
    pub fn whale_count(&self) -> usize {
        self.whale_positions.len()
    }

    /// Add historical round data
    pub fn add_round(&mut self, round: RoundHistory) {
        self.history.push(round);
        self.recompute_stats();
    }

    /// Add multiple rounds at once
    pub fn load_history(&mut self, rounds: Vec<RoundHistory>) {
        self.history = rounds;
        self.recompute_stats();
    }

    /// Recompute all statistics from history
    fn recompute_stats(&mut self) {
        // Reset stats
        for stat in &mut self.square_stats {
            *stat = SquareStats::default();
        }

        let total_rounds = self.history.len() as u32;
        if total_rounds == 0 {
            return;
        }

        // Compute basic stats
        for round in &self.history {
            let winner = round.winning_square as usize;
            self.square_stats[winner].wins += 1;
            self.square_stats[winner].total_deployed_when_won += round.deployed[winner];
            self.square_stats[winner].total_pot_when_won += round.total_pot;

            for (i, &deployed) in round.deployed.iter().enumerate() {
                self.square_stats[i].total_rounds = total_rounds;
                self.square_stats[i].avg_competition += deployed as f64;
            }
        }

        // Compute derived stats
        let expected_rate = 1.0 / 25.0; // 4%
        
        for (i, stat) in self.square_stats.iter_mut().enumerate() {
            stat.avg_competition /= total_rounds as f64;
            stat.win_rate = stat.wins as f64 / total_rounds as f64;
            stat.expected_rate = expected_rate;
            stat.edge = stat.win_rate - expected_rate;
            
            // ROI: (winnings - cost) / cost
            if stat.total_deployed_when_won > 0 {
                stat.roi = (stat.total_pot_when_won as f64 - stat.total_deployed_when_won as f64) 
                    / stat.total_deployed_when_won as f64;
            }

            // Recent wins (last 100 rounds)
            let recent_start = self.history.len().saturating_sub(100);
            stat.recent_wins = self.history[recent_start..]
                .iter()
                .filter(|r| r.winning_square as usize == i)
                .count() as u32;

            // Compute streak
            stat.streak = 0;
            for round in self.history.iter().rev() {
                if round.winning_square as usize == i {
                    if stat.streak >= 0 {
                        stat.streak += 1;
                    } else {
                        break;
                    }
                } else {
                    if stat.streak <= 0 {
                        stat.streak -= 1;
                    } else {
                        break;
                    }
                }
                if stat.streak.abs() > 20 {
                    break; // Cap at 20
                }
            }
        }
    }

    /// Track a whale's deployment pattern
    pub fn track_whale(&mut self, address: String, squares: Vec<usize>) {
        self.whale_positions.insert(address, squares);
    }

    /// Get all strategy recommendations
    pub fn get_recommendations(&self, current_deployed: &[u64; 25]) -> Vec<StrategyRecommendation> {
        let mut recs = Vec::new();

        recs.push(self.momentum_strategy());
        recs.push(self.contrarian_value_strategy(current_deployed));
        recs.push(self.edge_hunting_strategy());
        recs.push(self.streak_reversal_strategy());
        recs.push(self.low_competition_strategy(current_deployed));
        recs.push(self.whale_following_strategy());
        recs.push(self.pattern_detection_strategy());
        recs.push(self.kelly_criterion_strategy(current_deployed));
        recs.push(self.quadrant_analysis_strategy());
        recs.push(self.mean_reversion_strategy());

        // Sort by confidence
        recs.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        recs
    }

    /// 1. MOMENTUM STRATEGY
    /// Bet on squares that have been winning recently
    fn momentum_strategy(&self) -> StrategyRecommendation {
        let mut scored: Vec<(usize, f64)> = self.square_stats
            .iter()
            .enumerate()
            .map(|(i, s)| {
                // Score based on recent wins with recency weighting
                let base_score = s.recent_wins as f64;
                let streak_bonus = if s.streak > 0 { s.streak as f64 * 0.5 } else { 0.0 };
                (i, base_score + streak_bonus)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let top_3: Vec<usize> = scored.iter().take(3).map(|(i, _)| *i).collect();
        let total_score: f64 = scored.iter().take(3).map(|(_, s)| s).sum();
        let weights: Vec<f64> = scored.iter().take(3).map(|(_, s)| s / total_score).collect();
        
        let confidence = if total_score > 15.0 { 0.7 } else if total_score > 10.0 { 0.5 } else { 0.3 };

        StrategyRecommendation {
            strategy_name: "Momentum".to_string(),
            squares: top_3,
            weights,
            confidence,
            expected_roi: 0.15,
            reasoning: "Betting on recently hot squares - momentum tends to persist short-term".to_string(),
        }
    }

    /// 2. CONTRARIAN VALUE STRATEGY
    /// Find squares with low current deployment but decent win rates
    fn contrarian_value_strategy(&self, current: &[u64; 25]) -> StrategyRecommendation {
        let total_deployed: u64 = current.iter().sum();
        if total_deployed == 0 {
            return StrategyRecommendation {
                strategy_name: "Contrarian Value".to_string(),
                squares: vec![0, 6, 12, 18, 24], // Diagonal
                weights: vec![0.2; 5],
                confidence: 0.3,
                expected_roi: 0.0,
                reasoning: "No current deployment data - using diagonal spread".to_string(),
            };
        }

        // Score each square: high win rate + low current competition = good value
        let mut scored: Vec<(usize, f64)> = self.square_stats
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let current_share = current[i] as f64 / total_deployed as f64;
                let expected_share = 1.0 / 25.0;
                
                // Value = positive edge + underweighted in current round
                let edge_score = s.edge * 10.0;
                let value_score = (expected_share - current_share) * 5.0;
                
                (i, edge_score + value_score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let top_5: Vec<usize> = scored.iter().take(5).filter(|(_, s)| *s > 0.0).map(|(i, _)| *i).collect();
        let total_score: f64 = scored.iter().take(5).map(|(_, s)| s.max(0.1)).sum();
        let weights: Vec<f64> = scored.iter().take(5).map(|(_, s)| s.max(0.1) / total_score).collect();

        StrategyRecommendation {
            strategy_name: "Contrarian Value".to_string(),
            squares: top_5,
            weights,
            confidence: 0.6,
            expected_roi: 0.25,
            reasoning: "Squares with historical edge but currently underbet - value play".to_string(),
        }
    }

    /// 3. EDGE HUNTING STRATEGY
    /// Pure statistical edge - squares that win more than 4%
    fn edge_hunting_strategy(&self) -> StrategyRecommendation {
        let mut with_edge: Vec<(usize, f64)> = self.square_stats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.edge > 0.005 && s.total_rounds > 50) // At least 0.5% edge with data
            .map(|(i, s)| (i, s.edge))
            .collect();

        with_edge.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if with_edge.is_empty() {
            return StrategyRecommendation {
                strategy_name: "Edge Hunting".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "No statistical edges detected (need more data or truly random)".to_string(),
            };
        }

        let total_edge: f64 = with_edge.iter().map(|(_, e)| e).sum();
        let squares: Vec<usize> = with_edge.iter().map(|(i, _)| *i).collect();
        let weights: Vec<f64> = with_edge.iter().map(|(_, e)| e / total_edge).collect();
        
        let confidence = (total_edge * 10.0).min(0.8);

        StrategyRecommendation {
            strategy_name: "Edge Hunting".to_string(),
            squares,
            weights,
            confidence,
            expected_roi: total_edge,
            reasoning: format!("Squares with proven statistical edge: {:.1}% total edge", total_edge * 100.0),
        }
    }

    /// 4. STREAK REVERSAL STRATEGY
    /// Bet on squares due for a win (long losing streaks)
    fn streak_reversal_strategy(&self) -> StrategyRecommendation {
        let mut cold_squares: Vec<(usize, i32)> = self.square_stats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.streak < -5) // At least 5 losses in a row
            .map(|(i, s)| (i, s.streak.abs()))
            .collect();

        cold_squares.sort_by(|a, b| b.1.cmp(&a.1));

        if cold_squares.is_empty() {
            return StrategyRecommendation {
                strategy_name: "Streak Reversal".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "No squares on significant losing streaks".to_string(),
            };
        }

        let squares: Vec<usize> = cold_squares.iter().take(5).map(|(i, _)| *i).collect();
        let total: i32 = cold_squares.iter().take(5).map(|(_, s)| s).sum();
        let weights: Vec<f64> = cold_squares.iter().take(5).map(|(_, s)| *s as f64 / total as f64).collect();

        // Gambler's fallacy warning - this doesn't actually work if truly random
        // But psychological patterns in human behavior might create edges
        StrategyRecommendation {
            strategy_name: "Streak Reversal".to_string(),
            squares,
            weights,
            confidence: 0.35, // Lower confidence - gambler's fallacy risk
            expected_roi: 0.1,
            reasoning: "Squares on long losing streaks - contrarian bet on mean reversion".to_string(),
        }
    }

    /// 5. LOW COMPETITION STRATEGY
    /// Find empty or nearly-empty squares for maximum payout
    fn low_competition_strategy(&self, current: &[u64; 25]) -> StrategyRecommendation {
        let total: u64 = current.iter().sum();
        if total == 0 {
            return StrategyRecommendation {
                strategy_name: "Low Competition".to_string(),
                squares: (0..25).collect(),
                weights: vec![0.04; 25],
                confidence: 0.5,
                expected_roi: 0.0,
                reasoning: "All squares empty - equal spread".to_string(),
            };
        }

        // Find squares with less than 2% of total pot
        let mut low_comp: Vec<(usize, u64)> = current
            .iter()
            .enumerate()
            .filter(|(_, &amt)| (amt as f64 / total as f64) < 0.02)
            .map(|(i, &amt)| (i, amt))
            .collect();

        // Sort by lowest competition
        low_comp.sort_by(|a, b| a.1.cmp(&b.1));

        let squares: Vec<usize> = low_comp.iter().take(5).map(|(i, _)| *i).collect();
        let weights = vec![0.2; squares.len().min(5)];

        StrategyRecommendation {
            strategy_name: "Low Competition".to_string(),
            squares,
            weights,
            confidence: 0.55,
            expected_roi: 0.5, // High payout if win
            reasoning: "Squares with minimal competition - high payout potential".to_string(),
        }
    }

    /// 6. WHALE FOLLOWING STRATEGY
    /// Track and follow successful whale deployers
    fn whale_following_strategy(&self) -> StrategyRecommendation {
        if self.whale_positions.is_empty() {
            return StrategyRecommendation {
                strategy_name: "Whale Following".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "No whale data tracked yet".to_string(),
            };
        }

        // Aggregate whale positions
        let mut square_counts: [u32; 25] = [0; 25];
        for squares in self.whale_positions.values() {
            for &sq in squares {
                if sq < 25 {
                    square_counts[sq] += 1;
                }
            }
        }

        let mut scored: Vec<(usize, u32)> = square_counts
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, &c)| (i, c))
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        let squares: Vec<usize> = scored.iter().take(5).map(|(i, _)| *i).collect();
        let total: u32 = scored.iter().take(5).map(|(_, c)| c).sum();
        let weights: Vec<f64> = scored.iter().take(5).map(|(_, c)| *c as f64 / total as f64).collect();

        StrategyRecommendation {
            strategy_name: "Whale Following".to_string(),
            squares,
            weights,
            confidence: 0.5,
            expected_roi: 0.15,
            reasoning: "Following positions favored by large deployers".to_string(),
        }
    }

    /// 7. PATTERN DETECTION STRATEGY
    /// Look for spatial patterns in winning squares
    fn pattern_detection_strategy(&self) -> StrategyRecommendation {
        if self.history.len() < 25 {
            return StrategyRecommendation {
                strategy_name: "Pattern Detection".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "Insufficient history for pattern detection".to_string(),
            };
        }

        // Analyze last N winners for spatial patterns
        let recent: Vec<u8> = self.history.iter().rev().take(50).map(|r| r.winning_square).collect();
        
        // Check adjacent square correlation
        let mut adjacent_wins: [u32; 25] = [0; 25];
        for i in 1..recent.len() {
            let prev = recent[i-1] as usize;
            let curr = recent[i] as usize;
            
            // Check if adjacent (including diagonals)
            let prev_row = prev / 5;
            let prev_col = prev % 5;
            let curr_row = curr / 5;
            let curr_col = curr % 5;
            
            if (prev_row as i32 - curr_row as i32).abs() <= 1 
                && (prev_col as i32 - curr_col as i32).abs() <= 1 {
                adjacent_wins[curr] += 1;
            }
        }

        let last_winner = recent[0] as usize;
        let row = last_winner / 5;
        let col = last_winner % 5;
        
        // Get adjacent squares to last winner
        let mut adjacent: Vec<usize> = Vec::new();
        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                if dr == 0 && dc == 0 { continue; }
                let nr = row as i32 + dr;
                let nc = col as i32 + dc;
                if nr >= 0 && nr < 5 && nc >= 0 && nc < 5 {
                    adjacent.push((nr as usize) * 5 + nc as usize);
                }
            }
        }

        let weights = vec![1.0 / adjacent.len() as f64; adjacent.len()];

        StrategyRecommendation {
            strategy_name: "Pattern Detection".to_string(),
            squares: adjacent,
            weights,
            confidence: 0.4,
            expected_roi: 0.1,
            reasoning: format!("Adjacent squares to last winner (square {})", last_winner),
        }
    }

    /// 8. KELLY CRITERION STRATEGY
    /// Optimal bet sizing based on edge and odds
    fn kelly_criterion_strategy(&self, current: &[u64; 25]) -> StrategyRecommendation {
        let total: u64 = current.iter().sum();
        if total == 0 {
            return StrategyRecommendation {
                strategy_name: "Kelly Criterion".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "Need current deployment data for Kelly calculation".to_string(),
            };
        }

        // Kelly: f* = (bp - q) / b
        // where b = odds, p = probability of winning, q = 1-p
        let mut kelly_scores: Vec<(usize, f64)> = Vec::new();
        
        for (i, stat) in self.square_stats.iter().enumerate() {
            if stat.total_rounds < 30 { continue; }
            
            let p = stat.win_rate;
            let q = 1.0 - p;
            
            // Odds = (total pot / square deployment) - 1
            let square_share = current[i] as f64 / total as f64;
            if square_share < 0.001 { continue; } // Avoid division issues
            
            let b = (1.0 / square_share) - 1.0;
            
            // Kelly fraction
            let f = (b * p - q) / b;
            
            if f > 0.0 {
                kelly_scores.push((i, f));
            }
        }

        kelly_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if kelly_scores.is_empty() {
            return StrategyRecommendation {
                strategy_name: "Kelly Criterion".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "No positive Kelly bets found - no edge detected".to_string(),
            };
        }

        // Use fractional Kelly (half Kelly for safety)
        let total_kelly: f64 = kelly_scores.iter().map(|(_, k)| k).sum();
        let squares: Vec<usize> = kelly_scores.iter().take(5).map(|(i, _)| *i).collect();
        let weights: Vec<f64> = kelly_scores.iter().take(5).map(|(_, k)| (k * 0.5) / total_kelly).collect();

        StrategyRecommendation {
            strategy_name: "Kelly Criterion".to_string(),
            squares,
            weights,
            confidence: 0.65,
            expected_roi: total_kelly * 0.5,
            reasoning: "Mathematically optimal bet sizing based on edge vs odds".to_string(),
        }
    }

    /// 9. QUADRANT ANALYSIS STRATEGY
    /// Analyze board by quadrants (corners, edges, center)
    fn quadrant_analysis_strategy(&self) -> StrategyRecommendation {
        // Corners: 0, 4, 20, 24
        // Edges: 1, 2, 3, 5, 9, 10, 14, 15, 19, 21, 22, 23
        // Center: 6, 7, 8, 11, 12, 13, 16, 17, 18
        
        let corners = [0, 4, 20, 24];
        let center = [6, 7, 8, 11, 12, 13, 16, 17, 18];
        
        let corner_wins: u32 = corners.iter()
            .map(|&i| self.square_stats[i].wins)
            .sum();
        let center_wins: u32 = center.iter()
            .map(|&i| self.square_stats[i].wins)
            .sum();
        
        let total_rounds = self.square_stats[0].total_rounds;
        if total_rounds == 0 {
            return StrategyRecommendation {
                strategy_name: "Quadrant Analysis".to_string(),
                squares: vec![12], // Center
                weights: vec![1.0],
                confidence: 0.2,
                expected_roi: 0.0,
                reasoning: "No data - defaulting to center".to_string(),
            };
        }

        let corner_rate = corner_wins as f64 / total_rounds as f64;
        let center_rate = center_wins as f64 / total_rounds as f64;
        
        let expected_corner = 4.0 / 25.0; // 16%
        let expected_center = 9.0 / 25.0; // 36%
        
        let corner_edge = corner_rate - expected_corner;
        let center_edge = center_rate - expected_center;

        let (squares, reasoning) = if corner_edge > center_edge && corner_edge > 0.02 {
            (corners.to_vec(), format!("Corners overperforming by {:.1}%", corner_edge * 100.0))
        } else if center_edge > 0.02 {
            (center.to_vec(), format!("Center overperforming by {:.1}%", center_edge * 100.0))
        } else {
            (vec![12], "No significant quadrant edge - defaulting to center".to_string())
        };

        let weights = vec![1.0 / squares.len() as f64; squares.len()];

        StrategyRecommendation {
            strategy_name: "Quadrant Analysis".to_string(),
            squares,
            weights,
            confidence: 0.45,
            expected_roi: 0.1,
            reasoning,
        }
    }

    /// 10. MEAN REVERSION STRATEGY
    /// Bet on squares that are statistically due
    fn mean_reversion_strategy(&self) -> StrategyRecommendation {
        if self.history.len() < 100 {
            return StrategyRecommendation {
                strategy_name: "Mean Reversion".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "Need at least 100 rounds for mean reversion analysis".to_string(),
            };
        }

        // Find squares significantly below expected win rate
        let expected = self.history.len() as f64 / 25.0;
        
        let mut underperformers: Vec<(usize, f64)> = self.square_stats
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let deviation = expected - s.wins as f64;
                (i, deviation)
            })
            .filter(|(_, d)| *d > 2.0) // At least 2 wins below expected
            .collect();

        underperformers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if underperformers.is_empty() {
            return StrategyRecommendation {
                strategy_name: "Mean Reversion".to_string(),
                squares: vec![],
                weights: vec![],
                confidence: 0.0,
                expected_roi: 0.0,
                reasoning: "All squares near expected values - no mean reversion opportunities".to_string(),
            };
        }

        let total: f64 = underperformers.iter().take(5).map(|(_, d)| d).sum();
        let squares: Vec<usize> = underperformers.iter().take(5).map(|(i, _)| *i).collect();
        let weights: Vec<f64> = underperformers.iter().take(5).map(|(_, d)| d / total).collect();

        StrategyRecommendation {
            strategy_name: "Mean Reversion".to_string(),
            squares,
            weights,
            confidence: 0.4,
            expected_roi: 0.15,
            reasoning: "Squares below expected win rate - betting on regression to mean".to_string(),
        }
    }

    /// Get best overall recommendation (consensus) with configurable square count
    pub fn get_consensus_recommendation(&self, current_deployed: &[u64; 25]) -> StrategyRecommendation {
        self.get_consensus_recommendation_n(current_deployed, 5)
    }
    
    /// Get consensus recommendation for N squares (1-25)
    pub fn get_consensus_recommendation_n(&self, current_deployed: &[u64; 25], num_squares: usize) -> StrategyRecommendation {
        let num_squares = num_squares.max(1).min(25);
        let recs = self.get_recommendations(current_deployed);
        
        // Weight squares by appearing in multiple strategies
        let mut square_scores: [f64; 25] = [0.0; 25];
        
        for rec in &recs {
            for (sq, weight) in rec.squares.iter().zip(&rec.weights) {
                square_scores[*sq] += weight * rec.confidence;
            }
        }

        let mut scored: Vec<(usize, f64)> = square_scores
            .iter()
            .enumerate()
            .filter(|(_, &s)| s > 0.0)
            .map(|(i, &s)| (i, s))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_squares: Vec<usize> = scored.iter().take(num_squares).map(|(i, _)| *i).collect();
        let total_score: f64 = scored.iter().take(num_squares).map(|(_, s)| s).sum();
        let weights: Vec<f64> = if total_score > 0.0 {
            scored.iter().take(num_squares).map(|(_, s)| s / total_score).collect()
        } else {
            vec![1.0 / num_squares as f64; top_squares.len()]
        };

        let confidence = (total_score / num_squares as f64).min(0.85);

        StrategyRecommendation {
            strategy_name: format!("Consensus-{}", num_squares),
            squares: top_squares,
            weights,
            confidence,
            expected_roi: 0.2,
            reasoning: format!("Weighted consensus across all strategies ({} squares)", num_squares),
        }
    }
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_engine_basic() {
        let mut engine = StrategyEngine::new();
        
        // Add some fake history
        for i in 0..100 {
            engine.add_round(RoundHistory {
                round_id: i,
                winning_square: (i % 25) as u8,
                deployed: [1_000_000_000; 25],
                total_pot: 25_000_000_000,
                motherlode: false,
                timestamp: Some(i as i64),
            });
        }

        let current = [1_000_000_000u64; 25];
        let recs = engine.get_recommendations(&current);
        
        assert!(recs.len() >= 5);
        for rec in &recs {
            println!("{}: {:?} (conf: {:.2})", rec.strategy_name, rec.squares, rec.confidence);
        }
    }
}
