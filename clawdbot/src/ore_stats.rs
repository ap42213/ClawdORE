use crate::blockchain_parser::BlockchainParser;
use crate::error::{BotError, Result};
use log::{debug, info, warn};
use ore_api::state::{Board, Miner, Round, Treasury};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ═══════════════════════════════════════════════════════════════════════════════
/// ORE STATS SERVICE
/// ═══════════════════════════════════════════════════════════════════════════════
/// 
/// Provides comprehensive ORE mining statistics fetched directly from Solana.
/// Similar to ore-stats.com but integrated with ClawdORE bots.
/// 
/// Features:
///   - Live round data (grid deployments, miner counts, timings)
///   - Historical round data with caching
///   - Global protocol stats (treasury, motherlode, staking)
///   - Miner leaderboards and performance tracking
///   - Square analysis and pattern detection
/// ═══════════════════════════════════════════════════════════════════════════════

pub const ORE_PROGRAM_ID: &str = "oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv";
pub const ORE_MINT: &str = "oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp";
pub const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

/// Live round data for the 5x5 grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveRoundData {
    pub round_id: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub current_slot: u64,
    pub slots_remaining: u64,
    pub time_remaining_secs: u64,
    pub is_intermission: bool,
    
    /// Per-square data (indices 0-24 map to squares 1-25)
    pub squares: Vec<SquareData>,
    
    /// Aggregate stats
    pub total_deployed_lamports: u64,
    pub total_deployed_sol: f64,
    pub total_miners: u64,
    pub total_vaulted_lamports: u64,
    pub total_vaulted_sol: f64,
    
    /// Top miner (if round completed)
    pub top_miner: Option<String>,
    pub top_miner_reward: Option<f64>,
    
    /// Motherlode
    pub motherlode_lamports: u64,
    pub motherlode_sol: f64,
}

/// Data for a single square in the 5x5 grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareData {
    pub square_num: u8,         // 1-25 (display)
    pub index: u8,              // 0-24 (internal)
    pub deployed_lamports: u64,
    pub deployed_sol: f64,
    pub miner_count: u64,
    pub is_winning: bool,
    pub percentage_of_total: f64,
}

/// Completed round history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundHistory {
    pub round_id: u64,
    pub total_deployed_sol: f64,
    pub total_vaulted_sol: f64,
    pub total_miners: u64,
    pub winning_square: u8,     // 1-25
    pub is_motherlode: bool,
    pub top_miner: String,
    pub top_miner_reward_ore: f64,
    pub timestamp: Option<i64>,
}

/// Global protocol statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    pub treasury_balance_lamports: u64,
    pub treasury_balance_sol: f64,
    pub motherlode_lamports: u64,
    pub motherlode_sol: f64,
    pub total_staked_ore: u64,
    pub total_refined_ore: u64,
    pub total_unclaimed_ore: u64,
    pub ore_price_usd: Option<f64>,
    pub sol_price_usd: Option<f64>,
}

/// Miner leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub address: String,
    pub rank: usize,
    pub total_deployed_sol: f64,
    pub total_earned_sol: f64,
    pub total_earned_ore: f64,
    pub net_profit_sol: f64,
    pub rounds_participated: u64,
    pub win_rate: f64,
    pub favorite_squares: Vec<u8>,
}

/// Square pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareAnalysis {
    pub square_num: u8,
    pub total_deployed_sol: f64,
    pub times_won: u64,
    pub win_rate: f64,
    pub average_deployment: f64,
    pub expected_value: f64,
    pub recommendation: String,
}

/// Comprehensive stats response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OreStatsResponse {
    pub live_round: LiveRoundData,
    pub protocol: ProtocolStats,
    pub recent_rounds: Vec<RoundHistory>,
    pub timestamp: i64,
}

/// ORE Stats Service
pub struct OreStatsService {
    rpc_client: Arc<RpcClient>,
    ore_program_id: Pubkey,
    round_cache: Arc<RwLock<HashMap<u64, RoundHistory>>>,
    last_fetch: Arc<RwLock<Option<i64>>>,
}

impl OreStatsService {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));

        let ore_program_id = Pubkey::from_str(ORE_PROGRAM_ID)
            .map_err(|e| BotError::Other(format!("Invalid ORE program ID: {}", e)))?;

        Ok(Self {
            rpc_client,
            ore_program_id,
            round_cache: Arc::new(RwLock::new(HashMap::new())),
            last_fetch: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Create from an existing RPC client
    pub fn with_client(rpc_client: Arc<RpcClient>) -> Result<Self> {
        let ore_program_id = Pubkey::from_str(ORE_PROGRAM_ID)
            .map_err(|e| BotError::Other(format!("Invalid ORE program ID: {}", e)))?;

        Ok(Self {
            rpc_client,
            ore_program_id,
            round_cache: Arc::new(RwLock::new(HashMap::new())),
            last_fetch: Arc::new(RwLock::new(None)),
        })
    }

    /// Get current slot
    pub fn get_current_slot(&self) -> Result<u64> {
        Ok(self.rpc_client.get_slot()?)
    }

    /// Get Board PDA (singleton with current round info)
    pub fn get_board(&self) -> Result<Board> {
        let (board_pda, _) = ore_api::state::board_pda();
        let account = self.rpc_client.get_account(&board_pda)?;
        
        let board = bytemuck::try_from_bytes::<Board>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Board: {:?}", e)))?;
        
        Ok(*board)
    }

    /// Get Round data by ID
    pub fn get_round(&self, round_id: u64) -> Result<Round> {
        let (round_pda, _) = ore_api::state::round_pda(round_id);
        let account = self.rpc_client.get_account(&round_pda)?;
        
        let round = bytemuck::try_from_bytes::<Round>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Round: {:?}", e)))?;
        
        Ok(*round)
    }

    /// Get Treasury data
    pub fn get_treasury(&self) -> Result<Treasury> {
        let (treasury_pda, _) = ore_api::state::treasury_pda();
        let account = self.rpc_client.get_account(&treasury_pda)?;
        
        let treasury = bytemuck::try_from_bytes::<Treasury>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Treasury: {:?}", e)))?;
        
        Ok(*treasury)
    }

    /// Get Miner account for a wallet
    pub fn get_miner(&self, authority: &Pubkey) -> Result<Option<Miner>> {
        let (miner_pda, _) = ore_api::state::miner_pda(*authority);
        
        match self.rpc_client.get_account(&miner_pda) {
            Ok(account) => {
                let miner = bytemuck::try_from_bytes::<Miner>(&account.data[8..])
                    .map_err(|e| BotError::Serialization(format!("Failed to deserialize Miner: {:?}", e)))?;
                Ok(Some(*miner))
            }
            Err(_) => Ok(None),
        }
    }

    /// Fetch live round data
    pub fn get_live_round(&self) -> Result<LiveRoundData> {
        let board = self.get_board()?;
        let current_slot = self.get_current_slot()?;
        let round = self.get_round(board.round_id)?;
        
        // Calculate timing
        let slots_remaining = if current_slot < board.end_slot {
            board.end_slot - current_slot
        } else {
            0
        };
        
        // Check if in intermission (round ended, waiting for reset)
        let is_intermission = current_slot >= board.end_slot;
        
        // Estimate time remaining (~370ms per slot on average)
        let time_remaining_secs = (slots_remaining as f64 * 0.37) as u64;
        
        // Build square data
        let mut squares = Vec::with_capacity(25);
        let mut total_deployed: u64 = 0;
        let mut total_miners: u64 = 0;
        
        for i in 0..25 {
            let deployed = round.deployed[i];
            let miners = round.count[i];
            total_deployed += deployed;
            total_miners += miners;
            
            squares.push(SquareData {
                square_num: (i + 1) as u8,
                index: i as u8,
                deployed_lamports: deployed,
                deployed_sol: deployed as f64 / LAMPORTS_PER_SOL,
                miner_count: miners,
                is_winning: false, // Will be set later for completed rounds
                percentage_of_total: 0.0, // Calculate after sum
            });
        }
        
        // Calculate percentages
        for sq in squares.iter_mut() {
            if total_deployed > 0 {
                sq.percentage_of_total = (sq.deployed_lamports as f64 / total_deployed as f64) * 100.0;
            }
        }
        
        // Get winning square if round is completed
        let winning_square = if is_intermission {
            if let Some(rng) = round.rng() {
                let winner = round.winning_square(rng) as usize;
                if winner < 25 {
                    squares[winner].is_winning = true;
                }
                Some(winner as u8 + 1) // Convert to 1-25
            } else {
                None
            }
        } else {
            None
        };
        
        // Top miner
        let top_miner = if round.top_miner != Pubkey::default() {
            Some(round.top_miner.to_string())
        } else {
            None
        };
        
        Ok(LiveRoundData {
            round_id: board.round_id,
            start_slot: board.start_slot,
            end_slot: board.end_slot,
            current_slot,
            slots_remaining,
            time_remaining_secs,
            is_intermission,
            squares,
            total_deployed_lamports: total_deployed,
            total_deployed_sol: total_deployed as f64 / LAMPORTS_PER_SOL,
            total_miners,
            total_vaulted_lamports: round.total_vaulted,
            total_vaulted_sol: round.total_vaulted as f64 / LAMPORTS_PER_SOL,
            top_miner,
            top_miner_reward: Some(round.top_miner_reward as f64 / 1e11), // ORE has 11 decimals
            motherlode_lamports: round.motherlode,
            motherlode_sol: round.motherlode as f64 / LAMPORTS_PER_SOL,
        })
    }

    /// Fetch protocol-wide stats
    pub fn get_protocol_stats(&self) -> Result<ProtocolStats> {
        let treasury = self.get_treasury()?;
        
        Ok(ProtocolStats {
            treasury_balance_lamports: treasury.balance,
            treasury_balance_sol: treasury.balance as f64 / LAMPORTS_PER_SOL,
            motherlode_lamports: treasury.motherlode,
            motherlode_sol: treasury.motherlode as f64 / LAMPORTS_PER_SOL,
            total_staked_ore: treasury.total_staked,
            total_refined_ore: treasury.total_refined,
            total_unclaimed_ore: treasury.total_unclaimed,
            ore_price_usd: None, // Would need external price feed
            sol_price_usd: None,
        })
    }

    /// Fetch historical rounds
    pub fn get_round_history(&self, count: usize) -> Result<Vec<RoundHistory>> {
        let board = self.get_board()?;
        let current_round_id = board.round_id;
        
        let mut history = Vec::with_capacity(count);
        
        // Fetch last N completed rounds
        for offset in 1..=count {
            if current_round_id < offset as u64 {
                break;
            }
            
            let round_id = current_round_id - offset as u64;
            
            // Check cache first
            {
                let cache = self.round_cache.blocking_read();
                if let Some(cached) = cache.get(&round_id) {
                    history.push(cached.clone());
                    continue;
                }
            }
            
            // Fetch from chain
            match self.get_round(round_id) {
                Ok(round) => {
                    // Get winning square
                    let (winning_square, is_motherlode) = if let Some(rng) = round.rng() {
                        let sq = round.winning_square(rng) as u8 + 1; // Convert to 1-25
                        let ml = round.did_hit_motherlode(rng);
                        (sq, ml)
                    } else {
                        (0, false) // Should not happen for completed rounds
                    };
                    
                    let round_history = RoundHistory {
                        round_id,
                        total_deployed_sol: round.total_deployed as f64 / LAMPORTS_PER_SOL,
                        total_vaulted_sol: round.total_vaulted as f64 / LAMPORTS_PER_SOL,
                        total_miners: round.total_miners,
                        winning_square,
                        is_motherlode,
                        top_miner: round.top_miner.to_string(),
                        top_miner_reward_ore: round.top_miner_reward as f64 / 1e11,
                        timestamp: None, // Would need block time lookup
                    };
                    
                    // Cache it
                    {
                        let mut cache = self.round_cache.blocking_write();
                        cache.insert(round_id, round_history.clone());
                    }
                    
                    history.push(round_history);
                }
                Err(e) => {
                    debug!("Failed to fetch round {}: {}", round_id, e);
                    break;
                }
            }
        }
        
        Ok(history)
    }

    /// Analyze square performance across historical rounds
    pub fn analyze_squares(&self, num_rounds: usize) -> Result<Vec<SquareAnalysis>> {
        let history = self.get_round_history(num_rounds)?;
        
        let mut square_wins = [0u64; 25];
        let mut square_deployments = [0.0f64; 25];
        
        for round in &history {
            if round.winning_square > 0 && round.winning_square <= 25 {
                square_wins[(round.winning_square - 1) as usize] += 1;
            }
        }
        
        let total_rounds = history.len() as f64;
        let expected_win_rate = 1.0 / 25.0; // 4% for random
        
        let mut analysis = Vec::with_capacity(25);
        
        for i in 0..25 {
            let wins = square_wins[i];
            let win_rate = if total_rounds > 0.0 {
                wins as f64 / total_rounds
            } else {
                0.0
            };
            
            let recommendation = if win_rate > expected_win_rate * 1.5 {
                "🔥 HOT - Above expected".to_string()
            } else if win_rate < expected_win_rate * 0.5 {
                "❄️ COLD - Below expected".to_string()
            } else {
                "➖ NEUTRAL".to_string()
            };
            
            analysis.push(SquareAnalysis {
                square_num: (i + 1) as u8,
                total_deployed_sol: square_deployments[i],
                times_won: wins,
                win_rate,
                average_deployment: 0.0, // Would need more data
                expected_value: 0.0,     // Would need economic calculation
                recommendation,
            });
        }
        
        Ok(analysis)
    }

    /// Get comprehensive stats for API response
    pub fn get_full_stats(&self) -> Result<OreStatsResponse> {
        let live_round = self.get_live_round()?;
        let protocol = self.get_protocol_stats()?;
        let recent_rounds = self.get_round_history(20)?;
        
        Ok(OreStatsResponse {
            live_round,
            protocol,
            recent_rounds,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
    
    /// Get stats formatted for bot decision making
    pub fn get_bot_recommendations(&self) -> Result<BotRecommendations> {
        let live = self.get_live_round()?;
        let analysis = self.analyze_squares(100)?;
        
        // Find best squares based on analysis
        let mut hot_squares: Vec<u8> = analysis.iter()
            .filter(|a| a.recommendation.contains("HOT"))
            .map(|a| a.square_num)
            .collect();
        
        // Find underweight squares (low deployment but average win rate)
        let avg_deployment = live.total_deployed_sol / 25.0;
        let underweight: Vec<u8> = live.squares.iter()
            .filter(|s| s.deployed_sol < avg_deployment * 0.5)
            .map(|s| s.square_num)
            .collect();
        
        // Calculate optimal squares (balance of low competition + good history)
        let mut scored: Vec<(u8, f64)> = Vec::new();
        for sq in &live.squares {
            let win_history = analysis.get(sq.index as usize)
                .map(|a| a.win_rate)
                .unwrap_or(0.04);
            
            // Lower deployment = better odds, higher win rate = better square
            let competition_factor = if live.total_deployed_sol > 0.0 {
                1.0 - (sq.deployed_sol / live.total_deployed_sol)
            } else {
                1.0
            };
            
            let score = competition_factor * 0.5 + (win_history * 12.5) * 0.5;
            scored.push((sq.square_num, score));
        }
        
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let recommended: Vec<u8> = scored.iter().take(5).map(|(sq, _)| *sq).collect();
        
        Ok(BotRecommendations {
            round_id: live.round_id,
            time_remaining_secs: live.time_remaining_secs,
            is_intermission: live.is_intermission,
            recommended_squares: recommended,
            hot_squares,
            underweight_squares: underweight,
            total_deployed: live.total_deployed_sol,
            total_miners: live.total_miners,
            motherlode: live.motherlode_sol,
        })
    }
}

/// Recommendations for bot decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotRecommendations {
    pub round_id: u64,
    pub time_remaining_secs: u64,
    pub is_intermission: bool,
    pub recommended_squares: Vec<u8>,
    pub hot_squares: Vec<u8>,
    pub underweight_squares: Vec<u8>,
    pub total_deployed: f64,
    pub total_miners: u64,
    pub motherlode: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_data_creation() {
        let sq = SquareData {
            square_num: 13,
            index: 12,
            deployed_lamports: 1_000_000_000,
            deployed_sol: 1.0,
            miner_count: 50,
            is_winning: false,
            percentage_of_total: 4.0,
        };
        
        assert_eq!(sq.square_num, 13);
        assert_eq!(sq.index, 12);
        assert_eq!(sq.deployed_sol, 1.0);
    }
}
