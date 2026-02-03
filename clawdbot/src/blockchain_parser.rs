use crate::error::{BotError, Result};
use base64::Engine;
use log::{debug, info, warn};
use ore_api::state::{Board, Miner, Round, Treasury};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

/// ═══════════════════════════════════════════════════════════════════════════════
/// ORE BLOCKCHAIN PARSER
/// ═══════════════════════════════════════════════════════════════════════════════
/// 
/// This parser ONLY tracks transactions for the ORE mining program.
/// All wallet tracking is limited to wallets that interact with this program.
/// 
/// Tracked transaction types:
///   - Deploy: Wallet bets SOL on squares (1-25)
///   - Reset: Round ends, winning square revealed
///   - ClaimSOL/ClaimORE: Wallet claims rewards
///   - Automate: Wallet sets up automated mining
///
/// Every wallet we track is an ORE program user (miner).
/// ═══════════════════════════════════════════════════════════════════════════════

/// ORE Program ID - THE ONLY PROGRAM WE TRACK
/// All transactions must be to this program to be processed
pub const ORE_PROGRAM_ID: &str = "oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv";

/// ORE Instruction Types (from ore-api)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OreInstructionType {
    // Mining
    Automate = 0,
    Checkpoint = 2,
    ClaimSOL = 3,
    ClaimORE = 4,
    Close = 5,
    Deploy = 6,
    Log = 8,
    Reset = 9,
    ReloadSOL = 21,

    // Staking
    Deposit = 10,
    Withdraw = 11,
    ClaimYield = 12,
    CompoundYield = 22,

    // Admin
    Buyback = 13,
    Bury = 24,
    Wrap = 14,
    SetAdmin = 15,
    NewVar = 19,
    Liq = 25,

    // Unknown
    Unknown = 255,
}

impl From<u8> for OreInstructionType {
    fn from(value: u8) -> Self {
        match value {
            0 => OreInstructionType::Automate,
            2 => OreInstructionType::Checkpoint,
            3 => OreInstructionType::ClaimSOL,
            4 => OreInstructionType::ClaimORE,
            5 => OreInstructionType::Close,
            6 => OreInstructionType::Deploy,
            8 => OreInstructionType::Log,
            9 => OreInstructionType::Reset,
            21 => OreInstructionType::ReloadSOL,
            10 => OreInstructionType::Deposit,
            11 => OreInstructionType::Withdraw,
            12 => OreInstructionType::ClaimYield,
            22 => OreInstructionType::CompoundYield,
            13 => OreInstructionType::Buyback,
            24 => OreInstructionType::Bury,
            14 => OreInstructionType::Wrap,
            15 => OreInstructionType::SetAdmin,
            19 => OreInstructionType::NewVar,
            25 => OreInstructionType::Liq,
            _ => OreInstructionType::Unknown,
        }
    }
}

impl OreInstructionType {
    pub fn name(&self) -> &'static str {
        match self {
            OreInstructionType::Automate => "Automate",
            OreInstructionType::Checkpoint => "Checkpoint",
            OreInstructionType::ClaimSOL => "ClaimSOL",
            OreInstructionType::ClaimORE => "ClaimORE",
            OreInstructionType::Close => "Close",
            OreInstructionType::Deploy => "Deploy",
            OreInstructionType::Log => "Log",
            OreInstructionType::Reset => "Reset",
            OreInstructionType::ReloadSOL => "ReloadSOL",
            OreInstructionType::Deposit => "Deposit",
            OreInstructionType::Withdraw => "Withdraw",
            OreInstructionType::ClaimYield => "ClaimYield",
            OreInstructionType::CompoundYield => "CompoundYield",
            OreInstructionType::Buyback => "Buyback",
            OreInstructionType::Bury => "Bury",
            OreInstructionType::Wrap => "Wrap",
            OreInstructionType::SetAdmin => "SetAdmin",
            OreInstructionType::NewVar => "NewVar",
            OreInstructionType::Liq => "Liq",
            OreInstructionType::Unknown => "Unknown",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            OreInstructionType::Automate => "🤖",
            OreInstructionType::Checkpoint => "✅",
            OreInstructionType::ClaimSOL => "💰",
            OreInstructionType::ClaimORE => "⛏️",
            OreInstructionType::Close => "🔒",
            OreInstructionType::Deploy => "🚀",
            OreInstructionType::Log => "📝",
            OreInstructionType::Reset => "🔄",
            OreInstructionType::ReloadSOL => "🔃",
            OreInstructionType::Deposit => "📥",
            OreInstructionType::Withdraw => "📤",
            OreInstructionType::ClaimYield => "🌾",
            OreInstructionType::CompoundYield => "📈",
            OreInstructionType::Buyback => "🛒",
            OreInstructionType::Bury => "⚰️",
            OreInstructionType::Wrap => "🎁",
            OreInstructionType::SetAdmin => "👑",
            OreInstructionType::NewVar => "🎲",
            OreInstructionType::Liq => "💧",
            OreInstructionType::Unknown => "❓",
        }
    }
}

/// Parsed Deploy instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployData {
    pub amount_lamports: u64,
    pub amount_sol: f64,
    pub squares_mask: u32,
    pub squares: Vec<usize>,
    pub num_squares: usize,
}

/// Parsed Automate instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomateData {
    pub amount_lamports: u64,
    pub deposit_lamports: u64,
    pub fee_lamports: u64,
    pub mask: u64,
    pub strategy: u8,
    pub reload: bool,
}

/// Parsed Deposit instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositData {
    pub amount: u64,
    pub compound_fee: u64,
}

/// Parsed Withdraw instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawData {
    pub amount: u64,
}

/// Parsed ClaimYield instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimYieldData {
    pub amount: u64,
}

/// Parsed ORE Transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOreTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub instruction_type: OreInstructionType,
    pub signer: String,
    pub accounts: Vec<String>,
    pub success: bool,
    pub deploy_data: Option<DeployData>,
    pub automate_data: Option<AutomateData>,
    pub deposit_data: Option<DepositData>,
    pub withdraw_data: Option<WithdrawData>,
    pub claim_yield_data: Option<ClaimYieldData>,
    pub reset_data: Option<ResetData>,
}

/// Parsed Reset instruction data (round completion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetData {
    pub round_id: u64,
    pub winning_square: u8,
    pub motherlode: bool,
}

/// Deploy event from program logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployEvent {
    pub authority: String,
    pub signer: String,
    pub round_id: u64,
    pub amount: u64,
    pub mask: u64,
    pub strategy: u64,
    pub total_squares: u64,
    pub timestamp: i64,
}

/// Reset event (round completed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetEvent {
    pub round_id: u64,
    pub winning_square: u8,
    pub total_deployed: u64,
    pub total_winnings: u64,
    pub total_vaulted: u64,
    pub motherlode: bool,
    pub timestamp: i64,
}

/// Miner stats tracked by the parser
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackedMiner {
    pub address: String,
    pub total_deployed: u64,
    pub total_claimed_sol: u64,
    pub total_claimed_ore: u64,
    pub deploy_count: u64,
    pub claim_count: u64,
    pub last_seen: i64,
    pub squares_deployed: HashMap<usize, u64>, // square -> total deployed
    pub automation_enabled: bool,
}

/// Round stats tracked by the parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedRound {
    pub round_id: u64,
    pub total_deployed: u64,
    pub deployed_by_square: [u64; 25],
    pub num_deploys: u64,
    pub unique_miners: Vec<String>,
    pub winning_square: Option<u8>,
    pub motherlode: bool,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

impl TrackedRound {
    pub fn new(round_id: u64) -> Self {
        Self {
            round_id,
            total_deployed: 0,
            deployed_by_square: [0; 25],
            num_deploys: 0,
            unique_miners: Vec::new(),
            winning_square: None,
            motherlode: false,
            start_time: None,
            end_time: None,
        }
    }
}

/// Blockchain Parser for ORE program
pub struct BlockchainParser {
    rpc_client: Arc<RpcClient>,
    ore_program_id: Pubkey,
    tracked_miners: HashMap<String, TrackedMiner>,
    tracked_rounds: HashMap<u64, TrackedRound>,
    recent_transactions: Vec<ParsedOreTransaction>,
    instruction_counts: HashMap<OreInstructionType, u64>,
    total_sol_deployed: u64,
    total_ore_claimed: u64,
    total_sol_claimed: u64,
}

impl BlockchainParser {
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
            tracked_miners: HashMap::new(),
            tracked_rounds: HashMap::new(),
            recent_transactions: Vec::new(),
            instruction_counts: HashMap::new(),
            total_sol_deployed: 0,
            total_ore_claimed: 0,
            total_sol_claimed: 0,
        })
    }

    /// Get the ORE program ID
    pub fn program_id(&self) -> Pubkey {
        self.ore_program_id
    }

    /// Parse instruction data to determine the instruction type
    pub fn parse_instruction_type(&self, data: &[u8]) -> OreInstructionType {
        if data.is_empty() {
            return OreInstructionType::Unknown;
        }
        OreInstructionType::from(data[0])
    }

    /// Parse Deploy instruction data
    pub fn parse_deploy_data(&self, data: &[u8]) -> Option<DeployData> {
        // Deploy instruction: [discriminator (1 byte), amount (8 bytes), squares (4 bytes)]
        if data.len() < 13 {
            return None;
        }

        let amount_bytes: [u8; 8] = data[1..9].try_into().ok()?;
        let amount_lamports = u64::from_le_bytes(amount_bytes);

        let squares_bytes: [u8; 4] = data[9..13].try_into().ok()?;
        let squares_mask = u32::from_le_bytes(squares_bytes);

        // Convert mask to list of squares
        let mut squares = Vec::new();
        for i in 0..25 {
            if (squares_mask & (1 << i)) != 0 {
                squares.push(i);
            }
        }

        Some(DeployData {
            amount_lamports,
            amount_sol: amount_lamports as f64 / 1_000_000_000.0,
            squares_mask,
            squares: squares.clone(),
            num_squares: squares.len(),
        })
    }

    /// Parse Automate instruction data
    pub fn parse_automate_data(&self, data: &[u8]) -> Option<AutomateData> {
        // Automate: [disc (1), amount (8), deposit (8), fee (8), mask (8), strategy (1), reload (8)]
        if data.len() < 42 {
            return None;
        }

        let amount = u64::from_le_bytes(data[1..9].try_into().ok()?);
        let deposit = u64::from_le_bytes(data[9..17].try_into().ok()?);
        let fee = u64::from_le_bytes(data[17..25].try_into().ok()?);
        let mask = u64::from_le_bytes(data[25..33].try_into().ok()?);
        let strategy = data[33];
        let reload_val = u64::from_le_bytes(data[34..42].try_into().ok()?);

        Some(AutomateData {
            amount_lamports: amount,
            deposit_lamports: deposit,
            fee_lamports: fee,
            mask,
            strategy,
            reload: reload_val != 0,
        })
    }

    /// Parse Deposit instruction data
    pub fn parse_deposit_data(&self, data: &[u8]) -> Option<DepositData> {
        if data.len() < 17 {
            return None;
        }

        let amount = u64::from_le_bytes(data[1..9].try_into().ok()?);
        let compound_fee = u64::from_le_bytes(data[9..17].try_into().ok()?);

        Some(DepositData { amount, compound_fee })
    }

    /// Parse Withdraw instruction data
    pub fn parse_withdraw_data(&self, data: &[u8]) -> Option<WithdrawData> {
        if data.len() < 9 {
            return None;
        }

        let amount = u64::from_le_bytes(data[1..9].try_into().ok()?);
        Some(WithdrawData { amount })
    }

    /// Parse ClaimYield instruction data
    pub fn parse_claim_yield_data(&self, data: &[u8]) -> Option<ClaimYieldData> {
        if data.len() < 9 {
            return None;
        }

        let amount = u64::from_le_bytes(data[1..9].try_into().ok()?);
        Some(ClaimYieldData { amount })
    }

    /// Fetch and parse recent ORE transactions
    pub fn fetch_recent_transactions(&mut self, limit: usize) -> Result<Vec<ParsedOreTransaction>> {
        let signatures = self.rpc_client
            .get_signatures_for_address(&self.ore_program_id)
            .map_err(|e| BotError::RpcTimeout(format!("Failed to get signatures: {}", e)))?;

        let mut parsed = Vec::new();

        for sig_info in signatures.iter().take(limit) {
            let signature = Signature::from_str(&sig_info.signature)
                .map_err(|e| BotError::Other(format!("Invalid signature: {}", e)))?;

            match self.rpc_client.get_transaction(
                &signature,
                solana_transaction_status::UiTransactionEncoding::Base64,
            ) {
                Ok(tx) => {
                    if let Some(parsed_tx) = self.parse_transaction(&sig_info.signature, &tx, sig_info.slot, sig_info.block_time) {
                        self.process_parsed_transaction(&parsed_tx);
                        parsed.push(parsed_tx);
                    }
                }
                Err(e) => {
                    debug!("Failed to fetch tx {}: {}", sig_info.signature, e);
                }
            }
        }

        self.recent_transactions = parsed.clone();
        Ok(parsed)
    }

    /// Parse a single transaction
    fn parse_transaction(
        &self,
        signature: &str,
        tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        slot: u64,
        block_time: Option<i64>,
    ) -> Option<ParsedOreTransaction> {
        let tx_data = tx.transaction.transaction.decode()?;
        let message = tx_data.message;

        // Find ORE program instruction
        for (idx, instruction) in message.instructions().iter().enumerate() {
            let program_id = message.static_account_keys().get(instruction.program_id_index as usize)?;
            
            if *program_id == self.ore_program_id {
                let instruction_type = self.parse_instruction_type(&instruction.data);
                let signer = message.static_account_keys().first()?.to_string();

                let accounts: Vec<String> = instruction.accounts
                    .iter()
                    .filter_map(|&i| message.static_account_keys().get(i as usize).map(|p| p.to_string()))
                    .collect();

                let success = tx.transaction.meta
                    .as_ref()
                    .map(|m| m.err.is_none())
                    .unwrap_or(false);

                // Parse instruction-specific data
                let deploy_data = if instruction_type == OreInstructionType::Deploy {
                    self.parse_deploy_data(&instruction.data)
                } else {
                    None
                };

                let automate_data = if instruction_type == OreInstructionType::Automate {
                    self.parse_automate_data(&instruction.data)
                } else {
                    None
                };

                let deposit_data = if instruction_type == OreInstructionType::Deposit {
                    self.parse_deposit_data(&instruction.data)
                } else {
                    None
                };

                let withdraw_data = if instruction_type == OreInstructionType::Withdraw {
                    self.parse_withdraw_data(&instruction.data)
                } else {
                    None
                };

                let claim_yield_data = if instruction_type == OreInstructionType::ClaimYield {
                    self.parse_claim_yield_data(&instruction.data)
                } else {
                    None
                };

                let reset_data = if instruction_type == OreInstructionType::Reset {
                    // Reset instruction has no data - parse from logs/accounts instead
                    self.parse_reset_from_logs(tx, &accounts)
                } else {
                    None
                };

                return Some(ParsedOreTransaction {
                    signature: signature.to_string(),
                    slot,
                    block_time,
                    instruction_type,
                    signer,
                    accounts,
                    success,
                    deploy_data,
                    automate_data,
                    deposit_data,
                    withdraw_data,
                    claim_yield_data,
                    reset_data,
                });
            }
        }

        None
    }

    /// Parse Reset instruction data
    fn parse_reset_data(&self, data: &[u8]) -> Option<ResetData> {
        // Reset instruction format: [discriminator(1)] [round_id(8)] [winning_square(1)] [flags(1)]
        if data.len() < 11 {
            return None;
        }
        
        let round_id = u64::from_le_bytes(data[1..9].try_into().ok()?);
        let winning_square = data[9];
        let motherlode = if data.len() > 10 { data[10] != 0 } else { false };
        
        Some(ResetData {
            round_id,
            winning_square,
            motherlode,
        })
    }

    /// Parse Reset data from transaction logs (ResetEvent is emitted as program log)
    fn parse_reset_from_logs(
        &self,
        tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        accounts: &[String],
    ) -> Option<ResetData> {
        // The ResetEvent is emitted via program_log and contains:
        // disc(8), round_id(8), start_slot(8), end_slot(8), winning_square(8), ...
        // Total ResetEvent size: 8 + 8 + 8 + 8 + 8 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 = 144 bytes
        
        // Try to extract from inner instructions or return data
        if let Some(meta) = &tx.transaction.meta {
            // Check return data for ResetEvent
            if let Some(return_data) = &meta.return_data {
                if let solana_transaction_status::option_serializer::OptionSerializer::Some(data_str) = &return_data.data {
                    // Data is base64 encoded
                    if let Ok(data) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data_str.0) {
                        if data.len() >= 48 {
                            // ResetEvent: disc(8), round_id(8), start_slot(8), end_slot(8), winning_square(8)
                            let round_id = u64::from_le_bytes(data[8..16].try_into().ok()?);
                            let winning_square = u64::from_le_bytes(data[32..40].try_into().ok()?) as u8;
                            
                            // Check for motherlode in the event (offset 48 = motherlode amount, non-zero means hit)
                            let motherlode = if data.len() >= 56 {
                                u64::from_le_bytes(data[48..56].try_into().unwrap_or([0; 8])) > 0
                            } else {
                                false
                            };
                            
                            return Some(ResetData {
                                round_id,
                                winning_square,
                                motherlode,
                            });
                        }
                    }
                }
            }
            
            // Fallback: Parse from logs looking for "Winning square:" pattern
            if let solana_transaction_status::option_serializer::OptionSerializer::Some(logs) = &meta.log_messages {
                for log in logs {
                    // Look for patterns like "winning_square: X" in logs
                    if log.contains("winning_square") || log.contains("Winning square") {
                        // Try to extract number
                        for word in log.split_whitespace() {
                            if let Ok(num) = word.trim_matches(|c: char| !c.is_ascii_digit()).parse::<u8>() {
                                if num < 25 {
                                    // Extract round_id from the round account (6th account in reset)
                                    let round_id = if accounts.len() >= 6 {
                                        // We can't easily get round_id from account without RPC call
                                        // For now, return 0 and rely on board state
                                        0
                                    } else {
                                        0
                                    };
                                    
                                    let motherlode = log.contains("motherlode") || log.contains("MOTHERLODE");
                                    
                                    return Some(ResetData {
                                        round_id,
                                        winning_square: num,
                                        motherlode,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // If we can't parse the event, at least return something to indicate Reset happened
        // The coordinator can detect the round change and infer the winner from the Round account
        None
    }

    /// Process a parsed transaction and update internal state
    fn process_parsed_transaction(&mut self, tx: &ParsedOreTransaction) {
        if !tx.success {
            return;
        }

        // Update instruction counts
        *self.instruction_counts.entry(tx.instruction_type).or_insert(0) += 1;

        // Update miner tracking
        let miner = self.tracked_miners
            .entry(tx.signer.clone())
            .or_insert(TrackedMiner {
                address: tx.signer.clone(),
                ..Default::default()
            });

        miner.last_seen = tx.block_time.unwrap_or(0);

        match tx.instruction_type {
            OreInstructionType::Deploy => {
                if let Some(data) = &tx.deploy_data {
                    miner.total_deployed += data.amount_lamports;
                    miner.deploy_count += 1;
                    self.total_sol_deployed += data.amount_lamports;

                    for &square in &data.squares {
                        *miner.squares_deployed.entry(square).or_insert(0) += data.amount_lamports;
                    }
                }
            }
            OreInstructionType::ClaimSOL => {
                miner.claim_count += 1;
            }
            OreInstructionType::ClaimORE => {
                miner.claim_count += 1;
            }
            OreInstructionType::Automate => {
                miner.automation_enabled = true;
            }
            _ => {}
        }
    }

    /// Get current slot
    pub fn get_slot(&self) -> Result<u64> {
        Ok(self.rpc_client.get_slot()?)
    }

    /// Get current board state
    pub fn get_board(&self) -> Result<Board> {
        let (board_address, _) = ore_api::state::board_pda();
        let account = self.rpc_client.get_account(&board_address)?;
        
        let board = bytemuck::try_from_bytes::<Board>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Board: {:?}", e)))?;
        
        Ok(*board)
    }

    /// Get current round
    pub fn get_round(&self, round_id: u64) -> Result<Round> {
        let (round_address, _) = ore_api::state::round_pda(round_id);
        let account = self.rpc_client.get_account(&round_address)?;
        
        let round = bytemuck::try_from_bytes::<Round>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Round: {:?}", e)))?;
        
        Ok(*round)
    }

    /// Get treasury state
    pub fn get_treasury(&self) -> Result<Treasury> {
        let (treasury_address, _) = ore_api::state::treasury_pda();
        let account = self.rpc_client.get_account(&treasury_address)?;
        
        let treasury = bytemuck::try_from_bytes::<Treasury>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Treasury: {:?}", e)))?;
        
        Ok(*treasury)
    }

    /// Get miner account for a specific address
    pub fn get_miner(&self, authority: Pubkey) -> Result<Option<Miner>> {
        let (miner_address, _) = ore_api::state::miner_pda(authority);
        
        match self.rpc_client.get_account(&miner_address) {
            Ok(account) => {
                let miner = bytemuck::try_from_bytes::<Miner>(&account.data[8..])
                    .map_err(|e| BotError::Serialization(format!("Failed to deserialize Miner: {:?}", e)))?;
                Ok(Some(*miner))
            }
            Err(_) => Ok(None),
        }
    }

    /// Get statistics summary
    pub fn get_stats(&self) -> ParserStats {
        ParserStats {
            total_transactions: self.recent_transactions.len(),
            total_miners_tracked: self.tracked_miners.len(),
            total_rounds_tracked: self.tracked_rounds.len(),
            total_sol_deployed: self.total_sol_deployed as f64 / 1_000_000_000.0,
            instruction_counts: self.instruction_counts.clone(),
        }
    }

    /// Get tracked miners
    pub fn get_tracked_miners(&self) -> &HashMap<String, TrackedMiner> {
        &self.tracked_miners
    }

    /// Get recent transactions
    pub fn get_recent_transactions(&self) -> &[ParsedOreTransaction] {
        &self.recent_transactions
    }

    /// Get top deployers by total SOL deployed
    pub fn get_top_deployers(&self, limit: usize) -> Vec<(&String, &TrackedMiner)> {
        let mut miners: Vec<_> = self.tracked_miners.iter().collect();
        miners.sort_by(|a, b| b.1.total_deployed.cmp(&a.1.total_deployed));
        miners.into_iter().take(limit).collect()
    }

    /// Analyze square popularity from recent deploys
    pub fn analyze_square_popularity(&self) -> [u64; 25] {
        let mut square_counts = [0u64; 25];

        for tx in &self.recent_transactions {
            if let Some(data) = &tx.deploy_data {
                for &square in &data.squares {
                    if square < 25 {
                        square_counts[square] += data.amount_lamports;
                    }
                }
            }
        }

        square_counts
    }

    /// Format a transaction for display
    pub fn format_transaction(&self, tx: &ParsedOreTransaction) -> String {
        let time_str = tx.block_time
            .map(|t| chrono::DateTime::from_timestamp(t, 0)
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "?".to_string()))
            .unwrap_or_else(|| "?".to_string());

        let status = if tx.success { "✓" } else { "✗" };
        let short_sig = &tx.signature[..8];
        let short_signer = &tx.signer[..8];

        let details = match tx.instruction_type {
            OreInstructionType::Deploy => {
                if let Some(data) = &tx.deploy_data {
                    format!("{:.4} SOL → {} squares {:?}", 
                        data.amount_sol, data.num_squares, data.squares)
                } else {
                    String::new()
                }
            }
            OreInstructionType::Automate => {
                if let Some(data) = &tx.automate_data {
                    format!("strategy={} reload={}", data.strategy, data.reload)
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };

        format!(
            "[{}] {} {} {} {}...  by {}...  {}",
            time_str,
            status,
            tx.instruction_type.emoji(),
            tx.instruction_type.name(),
            short_sig,
            short_signer,
            details
        )
    }
}

/// Parser statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserStats {
    pub total_transactions: usize,
    pub total_miners_tracked: usize,
    pub total_rounds_tracked: usize,
    pub total_sol_deployed: f64,
    pub instruction_counts: HashMap<OreInstructionType, u64>,
}

impl Default for BlockchainParser {
    fn default() -> Self {
        Self::new("https://api.mainnet-beta.solana.com").expect("Failed to create default parser")
    }
}
