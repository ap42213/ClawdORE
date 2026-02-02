use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ═══════════════════════════════════════════════════════════════════════════════
/// ORE LEARNING ENGINE - Deep Pattern Analysis for ORE Program
/// ═══════════════════════════════════════════════════════════════════════════════
/// 
/// SCOPE: ONLY wallets interacting with the ORE program
/// Program ID: OREdv7MP3vLxV9TveRrPDNLAbSYaGDM7KhSHRwAr2cz
///
/// We track EVERY wallet that:
///   - Deploys SOL to the ORE mining game (Deploy instruction)
///   - Wins rounds (detected via Reset transactions)
///   - Claims rewards (ClaimSOL, ClaimORE instructions)
///   - Uses automation (Automate instruction)
///
/// This engine analyzes on-chain ORE program activity to:
///   1. Build profiles for ALL ORE program wallets
///   2. Detect winning patterns and strategies
///   3. Find wallets with consistent success to copy
///   4. Track full ORE wins and motherlode performance
///
/// NOT tracked: Non-ORE program activity, other Solana programs
/// ═══════════════════════════════════════════════════════════════════════════════

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
pub const ORE_DECIMALS: f64 = 1e11;

/// ORE Round Win Record
/// Captures every winning event from an ORE program Reset transaction
/// The winner is the wallet whose Deploy landed on the winning square
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinRecord {
    /// ORE round number (from program state)
    pub round_id: u64,
    /// Wallet address that won (detected from Deploy on winning square)
    pub winner_address: String,
    /// The square (1-25) that won the round
    pub winning_square: u8,
    pub amount_bet: u64,           // Lamports bet by winner
    pub amount_won: u64,           // Lamports won
    pub squares_bet: Vec<u8>,      // All squares the winner bet on
    pub num_squares: u8,           // How many squares they bet
    pub total_round_sol: u64,      // Total SOL deployed in the round
    pub num_deployers: u32,        // How many people deployed
    pub is_motherlode: bool,       // Was this a motherlode round?
    pub is_full_ore: bool,         // Did they win a full ORE (+1.0)?
    pub ore_earned: f64,           // Actual ORE earned
    pub competition_on_square: u64, // SOL deployed on winning square
    pub winner_share_pct: f64,     // What % of winning square did winner have?
    pub slot: u64,
    pub timestamp: Option<i64>,
}

/// ORE Player Profile - Built from on-chain ORE program history
/// Each profile represents a wallet address that has interacted with ORE program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    /// Wallet address (Solana pubkey) that used the ORE program
    pub address: String,
    pub total_rounds: u32,
    pub wins: u32,
    pub total_deployed: u64,
    pub total_won: u64,
    pub total_ore_earned: f64,
    
    // Strategy fingerprint
    pub avg_squares_per_round: f64,
    pub preferred_square_count: u8,
    pub avg_bet_size: u64,
    pub favorite_squares: Vec<u8>,  // Most commonly bet squares
    
    // Performance metrics
    pub win_rate: f64,
    pub roi: f64,
    pub ore_per_sol: f64,
    pub full_ore_wins: u32,        // Times won full ORE
    
    // Behavioral patterns
    pub plays_motherlode: bool,     // Do they bet on motherlode rounds?
    pub motherlode_win_rate: f64,
    pub prefers_low_competition: bool,
    pub avg_round_competition: u64, // Avg total SOL in rounds they play
    
    // Timing patterns
    pub slots_before_round_end: Vec<u64>, // When do they deploy?
    pub last_seen_slot: u64,
}

/// ORE Mining Strategy - Detected pattern from successful wallets
/// Built by analyzing on-chain Deploy patterns of winning ORE program wallets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedStrategy {
    pub name: String,
    pub description: String,
    pub sample_size: u32,
    pub win_rate: f64,
    pub avg_roi: f64,
    pub avg_ore_per_round: f64,
    
    // Strategy parameters
    pub square_count: u8,           // Optimal squares
    pub bet_size_sol: f64,          // Optimal bet
    pub target_competition: String, // Low/Medium/High
    pub preferred_squares: Vec<u8>,
    pub play_motherlode: bool,
    
    // Confidence
    pub confidence: f64,
    pub consistent: bool,           // Has it worked consistently?
    pub examples: Vec<String>,      // Player addresses using this
}

/// Main Learning Engine
pub struct LearningEngine {
    // All wins we've observed
    win_history: Vec<WinRecord>,
    
    // Player profiles
    players: HashMap<String, PlayerProfile>,
    
    // Detected strategies
    detected_strategies: Vec<DetectedStrategy>,
    
    // Statistics
    pub total_rounds_analyzed: u32,
    pub total_wins_tracked: u32,
    pub full_ore_wins_tracked: u32,
    
    // Configuration
    pub min_samples_for_strategy: u32,
}

impl LearningEngine {
    pub fn new() -> Self {
        Self {
            win_history: Vec::new(),
            players: HashMap::new(),
            detected_strategies: Vec::new(),
            total_rounds_analyzed: 0,
            total_wins_tracked: 0,
            full_ore_wins_tracked: 0,
            min_samples_for_strategy: 20,
        }
    }

    /// Record a win with full context
    pub fn record_win(&mut self, win: WinRecord) {
        // Update player profile
        self.update_player_from_win(&win);
        
        // Track full ORE wins specially
        if win.is_full_ore {
            self.full_ore_wins_tracked += 1;
        }
        
        self.total_wins_tracked += 1;
        self.win_history.push(win);
        
        // Re-analyze strategies periodically
        if self.total_wins_tracked % 50 == 0 {
            self.analyze_and_detect_strategies();
        }
    }

    /// Record a player's deploy (even if they don't win)
    pub fn record_deploy(
        &mut self,
        address: &str,
        amount: u64,
        squares: &[u8],
        round_total_sol: u64,
        is_motherlode: bool,
        slot: u64,
    ) {
        let player = self.players.entry(address.to_string()).or_insert_with(|| {
            PlayerProfile {
                address: address.to_string(),
                total_rounds: 0,
                wins: 0,
                total_deployed: 0,
                total_won: 0,
                total_ore_earned: 0.0,
                avg_squares_per_round: 0.0,
                preferred_square_count: 5,
                avg_bet_size: 0,
                favorite_squares: Vec::new(),
                win_rate: 0.0,
                roi: 0.0,
                ore_per_sol: 0.0,
                full_ore_wins: 0,
                plays_motherlode: false,
                motherlode_win_rate: 0.0,
                prefers_low_competition: false,
                avg_round_competition: 0,
                slots_before_round_end: Vec::new(),
                last_seen_slot: 0,
            }
        });

        player.total_rounds += 1;
        player.total_deployed += amount;
        player.avg_bet_size = player.total_deployed / player.total_rounds as u64;
        
        // Update square preferences
        let n = player.total_rounds as f64;
        player.avg_squares_per_round = 
            ((player.avg_squares_per_round * (n - 1.0)) + squares.len() as f64) / n;
        
        // Track favorite squares
        for sq in squares {
            if !player.favorite_squares.contains(sq) && player.favorite_squares.len() < 10 {
                player.favorite_squares.push(*sq);
            }
        }
        
        // Track competition preference
        player.avg_round_competition = 
            (((player.avg_round_competition as f64 * (n - 1.0)) + round_total_sol as f64) / n) as u64;
        player.prefers_low_competition = 
            (player.avg_round_competition as f64 / LAMPORTS_PER_SOL as f64) < 5.0;
        
        // Track motherlode behavior
        if is_motherlode {
            player.plays_motherlode = true;
        }
        
        player.last_seen_slot = slot;
    }

    fn update_player_from_win(&mut self, win: &WinRecord) {
        let player = self.players.entry(win.winner_address.clone()).or_insert_with(|| {
            PlayerProfile {
                address: win.winner_address.clone(),
                total_rounds: 0,
                wins: 0,
                total_deployed: 0,
                total_won: 0,
                total_ore_earned: 0.0,
                avg_squares_per_round: 0.0,
                preferred_square_count: win.num_squares,
                avg_bet_size: 0,
                favorite_squares: Vec::new(),
                win_rate: 0.0,
                roi: 0.0,
                ore_per_sol: 0.0,
                full_ore_wins: 0,
                plays_motherlode: false,
                motherlode_win_rate: 0.0,
                prefers_low_competition: false,
                avg_round_competition: 0,
                slots_before_round_end: Vec::new(),
                last_seen_slot: 0,
            }
        });

        player.wins += 1;
        player.total_won += win.amount_won;
        player.total_ore_earned += win.ore_earned;
        
        if win.is_full_ore {
            player.full_ore_wins += 1;
        }
        
        // Update win rate
        if player.total_rounds > 0 {
            player.win_rate = player.wins as f64 / player.total_rounds as f64;
        }
        
        // Update ROI
        if player.total_deployed > 0 {
            player.roi = (player.total_won as f64 - player.total_deployed as f64) 
                / player.total_deployed as f64;
        }
        
        // Update ORE per SOL
        let total_sol = player.total_deployed as f64 / LAMPORTS_PER_SOL as f64;
        if total_sol > 0.0 {
            player.ore_per_sol = player.total_ore_earned / total_sol;
        }
        
        // Determine preferred square count from wins
        if player.wins > 3 {
            player.preferred_square_count = (player.avg_squares_per_round.round() as u8).max(1).min(25);
        }
    }

    /// Analyze patterns and detect winning strategies
    pub fn analyze_and_detect_strategies(&mut self) {
        self.detected_strategies.clear();
        
        // Strategy 1: Low Square Count Winners
        self.detect_low_square_strategy();
        
        // Strategy 2: High Square Count Winners
        self.detect_high_square_strategy();
        
        // Strategy 3: Motherlode Specialists
        self.detect_motherlode_strategy();
        
        // Strategy 4: Low Competition Hunters
        self.detect_low_competition_strategy();
        
        // Strategy 5: Full ORE Winners
        self.detect_full_ore_strategy();
        
        // Strategy 6: Copy Top Players
        self.detect_player_copy_strategies();
        
        // Sort by effectiveness
        self.detected_strategies.sort_by(|a, b| {
            let score_a = a.avg_roi * a.confidence;
            let score_b = b.avg_roi * b.confidence;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn detect_low_square_strategy(&mut self) {
        // Find wins with 1-3 squares
        let low_sq_wins: Vec<_> = self.win_history.iter()
            .filter(|w| w.num_squares <= 3)
            .collect();
        
        if low_sq_wins.len() >= self.min_samples_for_strategy as usize {
            let avg_ore: f64 = low_sq_wins.iter().map(|w| w.ore_earned).sum::<f64>() 
                / low_sq_wins.len() as f64;
            let full_ore_pct = low_sq_wins.iter().filter(|w| w.is_full_ore).count() as f64 
                / low_sq_wins.len() as f64;
            
            self.detected_strategies.push(DetectedStrategy {
                name: "Low Square Focus".to_string(),
                description: "Bet on 1-3 squares for higher ORE per win".to_string(),
                sample_size: low_sq_wins.len() as u32,
                win_rate: 0.0, // Would need loss data
                avg_roi: 0.0,
                avg_ore_per_round: avg_ore,
                square_count: 2,
                bet_size_sol: 0.01,
                target_competition: "Low".to_string(),
                preferred_squares: vec![],
                play_motherlode: false,
                confidence: (low_sq_wins.len() as f64 / 100.0).min(1.0),
                consistent: full_ore_pct > 0.3,
                examples: low_sq_wins.iter().take(5).map(|w| w.winner_address[..8].to_string()).collect(),
            });
        }
    }

    fn detect_high_square_strategy(&mut self) {
        // Find wins with 10+ squares
        let high_sq_wins: Vec<_> = self.win_history.iter()
            .filter(|w| w.num_squares >= 10)
            .collect();
        
        if high_sq_wins.len() >= self.min_samples_for_strategy as usize {
            let avg_ore: f64 = high_sq_wins.iter().map(|w| w.ore_earned).sum::<f64>() 
                / high_sq_wins.len() as f64;
            
            // Calculate win rate (10+ squares = 40%+ base chance)
            let implied_win_rate = 0.4 + (high_sq_wins.len() as f64 * 0.001);
            
            self.detected_strategies.push(DetectedStrategy {
                name: "High Coverage".to_string(),
                description: "Bet on 10+ squares for consistent wins".to_string(),
                sample_size: high_sq_wins.len() as u32,
                win_rate: implied_win_rate.min(0.8),
                avg_roi: 0.0,
                avg_ore_per_round: avg_ore,
                square_count: 12,
                bet_size_sol: 0.04 / 12.0,
                target_competition: "Any".to_string(),
                preferred_squares: vec![],
                play_motherlode: true,
                confidence: (high_sq_wins.len() as f64 / 100.0).min(1.0),
                consistent: true,
                examples: high_sq_wins.iter().take(5).map(|w| w.winner_address[..8].to_string()).collect(),
            });
        }
    }

    fn detect_motherlode_strategy(&mut self) {
        let motherlode_wins: Vec<_> = self.win_history.iter()
            .filter(|w| w.is_motherlode)
            .collect();
        
        if motherlode_wins.len() >= 5 {
            let avg_bet: f64 = motherlode_wins.iter()
                .map(|w| w.amount_bet as f64 / LAMPORTS_PER_SOL as f64)
                .sum::<f64>() / motherlode_wins.len() as f64;
            
            let avg_squares: f64 = motherlode_wins.iter()
                .map(|w| w.num_squares as f64)
                .sum::<f64>() / motherlode_wins.len() as f64;
            
            let avg_ore: f64 = motherlode_wins.iter()
                .map(|w| w.ore_earned)
                .sum::<f64>() / motherlode_wins.len() as f64;
            
            self.detected_strategies.push(DetectedStrategy {
                name: "Motherlode Hunter".to_string(),
                description: format!(
                    "Target motherlode rounds with {:.0} squares, {:.4} SOL",
                    avg_squares, avg_bet
                ),
                sample_size: motherlode_wins.len() as u32,
                win_rate: 0.0,
                avg_roi: 0.0,
                avg_ore_per_round: avg_ore,
                square_count: avg_squares.round() as u8,
                bet_size_sol: avg_bet,
                target_competition: "High".to_string(),
                preferred_squares: vec![],
                play_motherlode: true,
                confidence: (motherlode_wins.len() as f64 / 20.0).min(1.0),
                consistent: motherlode_wins.len() >= 10,
                examples: motherlode_wins.iter().take(5).map(|w| w.winner_address[..8].to_string()).collect(),
            });
        }
    }

    fn detect_low_competition_strategy(&mut self) {
        // Wins where total round SOL < 2
        let low_comp_wins: Vec<_> = self.win_history.iter()
            .filter(|w| (w.total_round_sol as f64 / LAMPORTS_PER_SOL as f64) < 2.0)
            .collect();
        
        if low_comp_wins.len() >= self.min_samples_for_strategy as usize {
            let avg_ore: f64 = low_comp_wins.iter()
                .map(|w| w.ore_earned)
                .sum::<f64>() / low_comp_wins.len() as f64;
            
            let full_ore_pct = low_comp_wins.iter()
                .filter(|w| w.is_full_ore)
                .count() as f64 / low_comp_wins.len() as f64;
            
            self.detected_strategies.push(DetectedStrategy {
                name: "Low Competition Hunter".to_string(),
                description: format!(
                    "Target rounds with <2 SOL total for {:.1}% full ORE rate",
                    full_ore_pct * 100.0
                ),
                sample_size: low_comp_wins.len() as u32,
                win_rate: 0.0,
                avg_roi: 0.0,
                avg_ore_per_round: avg_ore,
                square_count: 5,
                bet_size_sol: 0.02,
                target_competition: "VeryLow".to_string(),
                preferred_squares: vec![],
                play_motherlode: false,
                confidence: (low_comp_wins.len() as f64 / 100.0).min(1.0),
                consistent: full_ore_pct > 0.4,
                examples: low_comp_wins.iter().take(5).map(|w| w.winner_address[..8].to_string()).collect(),
            });
        }
    }

    fn detect_full_ore_strategy(&mut self) {
        // Analyze specifically what full ORE winners do
        let full_ore_wins: Vec<_> = self.win_history.iter()
            .filter(|w| w.is_full_ore)
            .collect();
        
        if full_ore_wins.len() >= 10 {
            let avg_squares: f64 = full_ore_wins.iter()
                .map(|w| w.num_squares as f64)
                .sum::<f64>() / full_ore_wins.len() as f64;
            
            let avg_competition: f64 = full_ore_wins.iter()
                .map(|w| w.total_round_sol as f64 / LAMPORTS_PER_SOL as f64)
                .sum::<f64>() / full_ore_wins.len() as f64;
            
            let avg_bet: f64 = full_ore_wins.iter()
                .map(|w| w.amount_bet as f64 / LAMPORTS_PER_SOL as f64)
                .sum::<f64>() / full_ore_wins.len() as f64;
            
            // Find most common square count
            let mut sq_counts: HashMap<u8, u32> = HashMap::new();
            for w in &full_ore_wins {
                *sq_counts.entry(w.num_squares).or_insert(0) += 1;
            }
            let best_sq_count = sq_counts.iter()
                .max_by_key(|(_, count)| *count)
                .map(|(sq, _)| *sq)
                .unwrap_or(5);
            
            self.detected_strategies.push(DetectedStrategy {
                name: "Full ORE Winner".to_string(),
                description: format!(
                    "PROVEN: {} squares, {:.4} SOL bet, in <{:.1} SOL rounds",
                    best_sq_count, avg_bet, avg_competition
                ),
                sample_size: full_ore_wins.len() as u32,
                win_rate: 0.0,
                avg_roi: 1.0, // Full ORE is always good ROI
                avg_ore_per_round: 1.0,
                square_count: best_sq_count,
                bet_size_sol: avg_bet,
                target_competition: if avg_competition < 2.0 { "VeryLow" } 
                    else if avg_competition < 5.0 { "Low" } 
                    else { "Medium" }.to_string(),
                preferred_squares: vec![],
                play_motherlode: false,
                confidence: (full_ore_wins.len() as f64 / 50.0).min(1.0),
                consistent: true,
                examples: full_ore_wins.iter().take(5).map(|w| w.winner_address[..8].to_string()).collect(),
            });
        }
    }

    fn detect_player_copy_strategies(&mut self) {
        // Find top players by ROI and create "copy" strategies
        let mut top_players: Vec<_> = self.players.values()
            .filter(|p| p.total_rounds >= 20 && p.wins >= 5)
            .collect();
        
        top_players.sort_by(|a, b| {
            let score_a = a.roi * a.win_rate * a.ore_per_sol;
            let score_b = b.roi * b.win_rate * b.ore_per_sol;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for player in top_players.iter().take(3) {
            self.detected_strategies.push(DetectedStrategy {
                name: format!("Copy {}", &player.address[..8]),
                description: format!(
                    "Copy player with {:.1}% win rate, {:.2} ORE/SOL over {} rounds",
                    player.win_rate * 100.0,
                    player.ore_per_sol,
                    player.total_rounds
                ),
                sample_size: player.total_rounds,
                win_rate: player.win_rate,
                avg_roi: player.roi,
                avg_ore_per_round: player.total_ore_earned / player.wins.max(1) as f64,
                square_count: player.preferred_square_count,
                bet_size_sol: player.avg_bet_size as f64 / LAMPORTS_PER_SOL as f64,
                target_competition: if player.prefers_low_competition { "Low" } else { "Any" }.to_string(),
                preferred_squares: player.favorite_squares.clone(),
                play_motherlode: player.plays_motherlode,
                confidence: (player.total_rounds as f64 / 100.0).min(1.0) * player.win_rate,
                consistent: player.wins >= 10,
                examples: vec![player.address[..12].to_string()],
            });
        }
    }

    /// Get the best current strategy to use
    pub fn get_best_strategy(&self) -> Option<&DetectedStrategy> {
        self.detected_strategies.first()
    }

    /// Get all detected strategies
    pub fn get_all_strategies(&self) -> &[DetectedStrategy] {
        &self.detected_strategies
    }

    /// Get top players to potentially copy
    pub fn get_players_to_copy(&self, limit: usize) -> Vec<&PlayerProfile> {
        let mut players: Vec<_> = self.players.values()
            .filter(|p| p.total_rounds >= 15 && p.wins >= 3)
            .collect();
        
        players.sort_by(|a, b| {
            // Score = ROI * win_rate * consistency
            let score_a = a.roi * a.win_rate * (a.wins as f64 / 10.0).min(1.0);
            let score_b = b.roi * b.win_rate * (b.wins as f64 / 10.0).min(1.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        players.into_iter().take(limit).collect()
    }

    /// Export learning summary
    pub fn get_summary(&self) -> serde_json::Value {
        let best_strategy = self.get_best_strategy();
        let top_players = self.get_players_to_copy(5);
        
        serde_json::json!({
            "total_wins_tracked": self.total_wins_tracked,
            "full_ore_wins": self.full_ore_wins_tracked,
            "players_tracked": self.players.len(),
            "strategies_detected": self.detected_strategies.len(),
            "best_strategy": best_strategy.map(|s| serde_json::json!({
                "name": s.name,
                "description": s.description,
                "square_count": s.square_count,
                "bet_size_sol": s.bet_size_sol,
                "target_competition": s.target_competition,
                "confidence": s.confidence,
            })),
            "top_players": top_players.iter().map(|p| serde_json::json!({
                "address": &p.address[..8],
                "win_rate": format!("{:.1}%", p.win_rate * 100.0),
                "roi": format!("{:.1}%", p.roi * 100.0),
                "ore_per_sol": format!("{:.2}", p.ore_per_sol),
                "preferred_squares": p.preferred_square_count,
                "full_ore_wins": p.full_ore_wins,
            })).collect::<Vec<_>>(),
        })
    }
}

impl Default for LearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_win() {
        let mut engine = LearningEngine::new();
        
        let win = WinRecord {
            round_id: 1000,
            winner_address: "ABC123".to_string(),
            winning_square: 12,
            amount_bet: 10_000_000, // 0.01 SOL
            amount_won: 50_000_000, // 0.05 SOL
            squares_bet: vec![10, 11, 12, 13, 14],
            num_squares: 5,
            total_round_sol: 500_000_000, // 0.5 SOL
            num_deployers: 10,
            is_motherlode: false,
            is_full_ore: true,
            ore_earned: 1.0,
            competition_on_square: 20_000_000,
            winner_share_pct: 0.5,
            slot: 123456,
            timestamp: None,
        };
        
        engine.record_win(win);
        
        assert_eq!(engine.total_wins_tracked, 1);
        assert_eq!(engine.full_ore_wins_tracked, 1);
        assert!(engine.players.contains_key("ABC123"));
    }
}
