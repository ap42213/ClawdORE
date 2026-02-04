use crate::error::{BotError, Result};
use ore_api::state::{Board, Miner, Round, Treasury, board_pda, miner_pda, round_pda, treasury_pda};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use backoff::{ExponentialBackoff, future::retry};
use std::time::Duration;
use log::info;

pub struct OreClient {
    pub rpc_client: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
}

impl OreClient {
    pub fn new(rpc_url: String, keypair: Keypair) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        ));

        Self {
            rpc_client,
            keypair: Arc::new(keypair),
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    pub fn get_balance(&self) -> Result<u64> {
        let balance = self
            .rpc_client
            .get_balance(&self.keypair.pubkey())
            .map_err(|e| BotError::RpcTimeout(format!("Failed to get balance: {}", e)))?;
        Ok(balance)
    }

    /// Get balance with automatic retry on failure
    pub async fn get_balance_with_retry(&self) -> Result<u64> {
        let pubkey = self.keypair.pubkey();
        let rpc = self.rpc_client.clone();
        
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };
        
        let balance = retry(backoff, || async {
            rpc.get_balance(&pubkey)
                .map_err(|e| backoff::Error::transient(BotError::RpcTimeout(format!("Get balance failed: {}", e))))
        }).await?;
        
        Ok(balance)
    }

    pub fn get_board(&self) -> Result<Board> {
        let (board_address, _) = board_pda();
        let account = self.rpc_client.get_account(&board_address)?;
        
        // Skip discriminator (8 bytes)
        let board = bytemuck::try_from_bytes::<Board>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Board: {:?}", e)))?;
        
        Ok(*board)
    }

    pub fn get_miner(&self) -> Result<Option<Miner>> {
        let (miner_address, _) = miner_pda(self.keypair.pubkey());
        
        match self.rpc_client.get_account(&miner_address) {
            Ok(account) => {
                // Skip discriminator (8 bytes)
                let miner = bytemuck::try_from_bytes::<Miner>(&account.data[8..])
                    .map_err(|e| BotError::Serialization(format!("Failed to deserialize Miner: {:?}", e)))?;
                Ok(Some(*miner))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn get_round(&self, round_id: u64) -> Result<Round> {
        let (round_address, _) = round_pda(round_id);
        let account = self.rpc_client.get_account(&round_address)?;
        
        // Skip discriminator (8 bytes)
        let round = bytemuck::try_from_bytes::<Round>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Round: {:?}", e)))?;
        
        Ok(*round)
    }

    pub fn get_treasury(&self) -> Result<Treasury> {
        let (treasury_address, _) = treasury_pda();
        let account = self.rpc_client.get_account(&treasury_address)?;
        
        // Skip discriminator (8 bytes)
        let treasury = bytemuck::try_from_bytes::<Treasury>(&account.data[8..])
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Treasury: {:?}", e)))?;
        
        Ok(*treasury)
    }

    pub fn get_current_round(&self) -> Result<Round> {
        let board = self.get_board()?;
        self.get_round(board.round_id)
    }

    pub fn send_transaction(&self, transaction: Transaction) -> Result<String> {
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&transaction)?;
        Ok(signature.to_string())
    }

    pub fn get_slot(&self) -> Result<u64> {
        Ok(self.rpc_client.get_slot()?)
    }

    pub fn get_block_time(&self, slot: u64) -> Result<Option<i64>> {
        Ok(self.rpc_client.get_block_time(slot).ok())
    }

    /// Get multiple miner accounts for analysis
    pub fn get_miners(&self, addresses: &[Pubkey]) -> Result<Vec<(Pubkey, Miner)>> {
        let mut miners = Vec::new();
        
        for address in addresses {
            if let Ok(account) = self.rpc_client.get_account(address) {
                if let Ok(miner) = bytemuck::try_from_bytes::<Miner>(&account.data[8..]) {
                    miners.push((*address, *miner));
                }
            }
        }
        
        Ok(miners)
    }

    /// Get historical round data
    pub fn get_rounds(&self, start_round: u64, count: usize) -> Result<Vec<(u64, Round)>> {
        let mut rounds = Vec::new();
        
        for i in 0..count {
            let round_id = start_round.saturating_sub(i as u64);
            if let Ok(round) = self.get_round(round_id) {
                rounds.push((round_id, round));
            }
        }
        
        Ok(rounds)
    }

    /// Deploy SOL to ORE squares
    /// amount_lamports: amount per square in lamports
    /// squares: array of 25 booleans, true = deploy to that square
    /// Returns transaction signature
    pub fn deploy(&self, amount_lamports: u64, squares: [bool; 25]) -> Result<Signature> {
        let board = self.get_board()?;
        let round_id = board.round_id;
        
        info!("🎲 Building deploy tx for round {} with {} lamports per square", 
              round_id, amount_lamports);
        
        // Build the deploy instruction using ore-api SDK
        let deploy_ix = ore_api::sdk::deploy(
            self.keypair.pubkey(),  // signer
            self.keypair.pubkey(),  // authority (same as signer for manual deploy)
            amount_lamports,         // amount per square
            round_id,                // current round
            squares,                 // which squares to deploy to
        );
        
        // Add compute budget instructions for priority
        let compute_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
        let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(1_000_000);
        
        // Build transaction
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[compute_limit_ix, compute_price_ix, deploy_ix],
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            recent_blockhash,
        );
        
        // Send without waiting for confirmation (speed is critical)
        let signature = self.rpc_client.send_transaction(&transaction)?;
        
        info!("🚀 Deploy tx sent: {}", signature);
        Ok(signature)
    }

    /// Deploy with retry on failure
    pub async fn deploy_with_retry(&self, amount_lamports: u64, squares: [bool; 25]) -> Result<Signature> {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(15)),
            initial_interval: Duration::from_millis(200),
            max_interval: Duration::from_secs(2),
            ..Default::default()
        };
        
        // Clone what we need for the closure
        let keypair = self.keypair.clone();
        let rpc = self.rpc_client.clone();
        
        // We need to re-fetch blockhash on each retry
        let signature = retry(backoff, || async {
            let board = rpc.get_account(&board_pda().0)
                .map_err(|e| backoff::Error::transient(BotError::RpcTimeout(format!("Get board failed: {}", e))))?;
            
            let board_data = bytemuck::try_from_bytes::<Board>(&board.data[8..])
                .map_err(|e| backoff::Error::permanent(BotError::Serialization(format!("{:?}", e))))?;
            
            let deploy_ix = ore_api::sdk::deploy(
                keypair.pubkey(),
                keypair.pubkey(),
                amount_lamports,
                board_data.round_id,
                squares,
            );
            
            let compute_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
            let compute_price_ix = ComputeBudgetInstruction::set_compute_unit_price(1_000_000);
            
            let blockhash = rpc.get_latest_blockhash()
                .map_err(|e| backoff::Error::transient(BotError::RpcTimeout(format!("Get blockhash failed: {}", e))))?;
            
            let tx = Transaction::new_signed_with_payer(
                &[compute_limit_ix, compute_price_ix, deploy_ix],
                Some(&keypair.pubkey()),
                &[&*keypair],
                blockhash,
            );
            
            rpc.send_transaction(&tx)
                .map_err(|e| backoff::Error::transient(BotError::RpcTimeout(format!("Send tx failed: {}", e))))
        }).await?;
        
        Ok(signature)
    }
}
