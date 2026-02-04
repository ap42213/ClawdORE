use crate::error::Result;
use ore_api::state::Round;
use rand::{thread_rng, Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

pub const BOARD_SIZE: usize = 25; // 5x5 grid
pub const BOARD_WIDTH: usize = 5;
pub const BOARD_HEIGHT: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareAnalysis {
    pub square_id: usize,
    pub historical_wins: usize,
    pub total_deployed: u64,
    pub average_deployed: f64,
    pub win_rate: f64,
    pub last_win_rounds_ago: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BettingStrategy {
    strategy_type: String,
    risk_tolerance: f64,
}

impl BettingStrategy {
    pub fn new(strategy_type: String, risk_tolerance: f64) -> Self {
        Self {
            strategy_type,
            risk_tolerance,
        }
    }

    /// Select squares to bet on based on the strategy
    pub fn select_squares(
        &self,
        num_squares: usize,
        round_history: &[Round],
        current_round: &Round,
    ) -> Result<Vec<usize>> {
        match self.strategy_type.as_str() {
            "random" => self.random_selection(num_squares),
            "weighted" => self.weighted_selection(num_squares, round_history),
            "hot_squares" => self.hot_squares_selection(num_squares, round_history),
            "contrarian" => self.contrarian_selection(num_squares, current_round),
            "spread" => self.spread_selection(num_squares),
            "focused" => self.focused_selection(num_squares, round_history),
            _ => self.random_selection(num_squares),
        }
    }

    /// Random square selection
    /// Returns 1-indexed squares (1-25) to match ORE UI and coordinator output
    fn random_selection(&self, num_squares: usize) -> Result<Vec<usize>> {
        let mut rng = thread_rng();
        let mut squares = Vec::new();
        
        while squares.len() < num_squares {
            // Generate 0-indexed internally, but store as 1-indexed
            let square = rng.gen_range(0..BOARD_SIZE) + 1; // 1-25
            if !squares.contains(&square) {
                squares.push(square);
            }
        }
        
        Ok(squares)
    }

    /// Cryptographically secure random selection using entropy
    /// Use this for high-stakes betting to ensure true randomness
    /// Returns 1-indexed squares (1-25) to match ORE UI
    pub fn secure_random_selection(&self, num_squares: usize, seed: Option<[u8; 32]>) -> Result<Vec<usize>> {
        let mut rng = if let Some(s) = seed {
            StdRng::from_seed(s)
        } else {
            StdRng::from_entropy()
        };
        
        let mut squares = Vec::new();
        while squares.len() < num_squares {
            // Generate 0-indexed internally, but store as 1-indexed (1-25)
            let square = rng.gen_range(0..BOARD_SIZE) + 1;
            if !squares.contains(&square) {
                squares.push(square);
            }
        }
        
        Ok(squares)
    }

    /// Weighted selection based on historical performance
    fn weighted_selection(&self, num_squares: usize, history: &[Round]) -> Result<Vec<usize>> {
        let analysis = self.analyze_squares(history);
        
        // Sort by win rate and recency
        let mut sorted: Vec<_> = analysis.into_iter().collect();
        sorted.sort_by(|a, b| {
            let score_a = self.calculate_square_score(&a);
            let score_b = self.calculate_square_score(&b);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        Ok(sorted.iter().take(num_squares).map(|s| s.square_id).collect())
    }

    /// Select "hot" squares that have won recently
    fn hot_squares_selection(&self, num_squares: usize, history: &[Round]) -> Result<Vec<usize>> {
        let analysis = self.analyze_squares(history);
        
        // Prioritize squares that won recently
        let mut sorted: Vec<_> = analysis.into_iter().collect();
        sorted.sort_by_key(|s| s.last_win_rounds_ago.unwrap_or(usize::MAX));
        
        Ok(sorted.iter().take(num_squares).map(|s| s.square_id).collect())
    }

    /// Contrarian strategy - bet against the crowd
    fn contrarian_selection(&self, num_squares: usize, current_round: &Round) -> Result<Vec<usize>> {
        // Find squares with lowest deployment
        let mut deployment: Vec<(usize, u64)> = current_round
            .deployed
            .iter()
            .enumerate()
            .map(|(i, &d)| (i, d))
            .collect();
        
        deployment.sort_by_key(|(_, d)| *d);
        
        Ok(deployment.iter().take(num_squares).map(|(i, _)| *i).collect())
    }

    /// Spread bets across the board
    fn spread_selection(&self, num_squares: usize) -> Result<Vec<usize>> {
        let step = BOARD_SIZE / num_squares;
        Ok((0..num_squares).map(|i| (i * step) % BOARD_SIZE).collect())
    }

    /// Get adjacent squares for a given square on 5x5 grid
    pub fn get_adjacent_squares(square_id: usize) -> Vec<usize> {
        let row = square_id / BOARD_WIDTH;
        let col = square_id % BOARD_WIDTH;
        let mut adjacent = Vec::new();
        
        // Check all 8 directions (including diagonals)
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 { continue; }
                
                let new_row = row as i32 + dr;
                let new_col = col as i32 + dc;
                
                if new_row >= 0 && new_row < BOARD_HEIGHT as i32 
                    && new_col >= 0 && new_col < BOARD_WIDTH as i32 {
                    let adj_square = (new_row as usize * BOARD_WIDTH) + new_col as usize;
                    adjacent.push(adj_square);
                }
            }
        }
        
        adjacent
    }

    /// Cluster strategy - bet on squares near winning squares
    pub fn cluster_selection(&self, num_squares: usize, history: &[Round]) -> Result<Vec<usize>> {
        if history.is_empty() {
            return self.random_selection(num_squares);
        }
        
        // Find squares that won recently and their neighbors
        let mut hot_zones = Vec::new();
        for (_, _round) in history.iter().enumerate().take(10) {
            // TODO: Identify winning square from round data
            // For now, use a placeholder
            // hot_zones.extend(Self::get_adjacent_squares(winning_square));
        }
        
        // Fallback to weighted if no hot zones
        if hot_zones.is_empty() {
            return self.weighted_selection(num_squares, history);
        }
        
        Ok(hot_zones.into_iter().take(num_squares).collect())
    }

    /// Focused selection on highest probability squares
    fn focused_selection(&self, num_squares: usize, history: &[Round]) -> Result<Vec<usize>> {
        let analysis = self.analyze_squares(history);
        
        // Sort by win rate only
        let mut sorted: Vec<_> = analysis.into_iter().collect();
        sorted.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());
        
        Ok(sorted.iter().take(num_squares).map(|s| s.square_id).collect())
    }

    /// Analyze historical square performance
    fn analyze_squares(&self, history: &[Round]) -> Vec<SquareAnalysis> {
        let mut analyses = Vec::new();
        
        for square_id in 0..BOARD_SIZE {
            let mut wins = 0;
            let mut total_deployed = 0u64;
            let mut last_win = None;
            
            for (rounds_ago, round) in history.iter().enumerate() {
                // Check if this square won (you'd need to implement win detection)
                // For now, we'll use a simple heuristic
                total_deployed += round.deployed[square_id];
                
                // Placeholder for win detection logic
                // if square_won(round, square_id) {
                //     wins += 1;
                //     if last_win.is_none() {
                //         last_win = Some(rounds_ago);
                //     }
                // }
            }
            
            let win_rate = if history.len() > 0 {
                wins as f64 / history.len() as f64
            } else {
                0.0
            };
            
            let average_deployed = if history.len() > 0 {
                total_deployed as f64 / history.len() as f64
            } else {
                0.0
            };
            
            analyses.push(SquareAnalysis {
                square_id,
                historical_wins: wins,
                total_deployed,
                average_deployed,
                win_rate,
                last_win_rounds_ago: last_win,
            });
        }
        
        analyses
    }

    /// Calculate a score for a square based on various factors
    fn calculate_square_score(&self, analysis: &SquareAnalysis) -> f64 {
        let mut score = analysis.win_rate * 100.0;
        
        // Bonus for recent wins
        if let Some(rounds_ago) = analysis.last_win_rounds_ago {
            score += 10.0 / (rounds_ago as f64 + 1.0);
        }
        
        // Adjust by risk tolerance
        score *= 1.0 + self.risk_tolerance;
        
        score
    }

    /// Calculate bet amounts for selected squares
    pub fn calculate_bet_amounts(
        &self,
        squares: &[usize],
        total_budget: f64,
        min_bet: f64,
        max_bet: f64,
    ) -> Vec<(usize, f64)> {
        let per_square = total_budget / squares.len() as f64;
        
        squares
            .iter()
            .map(|&square| {
                let amount = per_square.clamp(min_bet, max_bet);
                (square, amount)
            })
            .collect()
    }

    /// Kelly Criterion based bet sizing
    /// win_probability: estimated probability of winning (0.0 to 1.0)
    /// odds: payout odds (e.g., 2.0 for doubling your money)
    pub fn kelly_bet_size(
        &self,
        bankroll: f64,
        win_probability: f64,
        odds: f64,
    ) -> f64 {
        // Kelly Criterion: f = (bp - q) / b
        // where f = fraction to bet, p = win probability, q = lose probability, b = odds - 1
        let q = 1.0 - win_probability;
        let b = odds - 1.0;
        
        let kelly_fraction = ((b * win_probability) - q) / b;
        
        // Use fractional Kelly (half Kelly) for safety
        let fractional_kelly = kelly_fraction * 0.5 * self.risk_tolerance;
        
        // Clamp to reasonable limits
        let fraction = fractional_kelly.clamp(0.0, 0.25);
        
        bankroll * fraction
    }

    /// Calculate optimal bet distribution across multiple squares
    pub fn calculate_optimal_bets(
        &self,
        squares: &[usize],
        total_budget: f64,
        square_probabilities: &[f64],
        min_bet: f64,
        max_bet: f64,
    ) -> Vec<(usize, f64)> {
        let mut bets = Vec::new();
        let total_prob: f64 = square_probabilities.iter().sum();
        
        if total_prob == 0.0 {
            return self.calculate_bet_amounts(squares, total_budget, min_bet, max_bet);
        }
        
        // Distribute budget proportional to probabilities
        for (i, &square) in squares.iter().enumerate() {
            let prob = square_probabilities.get(i).unwrap_or(&0.0);
            let fraction = prob / total_prob;
            let amount = (total_budget * fraction).clamp(min_bet, max_bet);
            bets.push((square, amount));
        }
        
        bets
    }
}

/// Mining strategy for square selection
pub struct MiningStrategy {
    strategy_type: String,
}

impl MiningStrategy {
    pub fn new(strategy_type: String) -> Self {
        Self { strategy_type }
    }

    pub fn select_squares(
        &self,
        num_squares: usize,
        current_round: &Round,
        round_history: &[Round],
    ) -> Result<Vec<usize>> {
        match self.strategy_type.as_str() {
            "random" => self.random_selection(num_squares),
            "weighted" => self.weighted_selection(num_squares, current_round),
            "balanced" => self.balanced_selection(num_squares, current_round),
            _ => self.random_selection(num_squares),
        }
    }

    fn random_selection(&self, num_squares: usize) -> Result<Vec<usize>> {
        let mut rng = thread_rng();
        let mut squares = Vec::new();
        
        while squares.len() < num_squares {
            let square = rng.gen_range(0..BOARD_SIZE);
            if !squares.contains(&square) {
                squares.push(square);
            }
        }
        
        Ok(squares)
    }

    fn weighted_selection(&self, num_squares: usize, current_round: &Round) -> Result<Vec<usize>> {
        // Select squares with lower deployment for better odds
        let mut deployment: Vec<(usize, u64)> = current_round
            .deployed
            .iter()
            .enumerate()
            .map(|(i, &d)| (i, d))
            .collect();
        
        deployment.sort_by_key(|(_, d)| *d);
        
        Ok(deployment.iter().take(num_squares).map(|(i, _)| *i).collect())
    }

    fn balanced_selection(&self, num_squares: usize, current_round: &Round) -> Result<Vec<usize>> {
        // Mix of low and medium deployment squares
        let mut deployment: Vec<(usize, u64)> = current_round
            .deployed
            .iter()
            .enumerate()
            .map(|(i, &d)| (i, d))
            .collect();
        
        deployment.sort_by_key(|(_, d)| *d);
        
        let mut selected = Vec::new();
        let step = deployment.len() / num_squares;
        
        for i in 0..num_squares {
            selected.push(deployment[i * step].0);
        }
        
        Ok(selected)
    }
}
