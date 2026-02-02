use crate::error::{BotError, Result};
use ore_api::state::{Board, Miner, Round, Treasury};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::sync::Arc;
use backoff::{ExponentialBackoff, future::retry};
use std::time::Duration;

pub struct OreClient {
    rpc_client: Arc<RpcClient>,
    keypair: Arc<Keypair>,
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
        let board_address = ore_api::consts::BOARD_ADDRESS;
        let account = self.rpc_client.get_account(&board_address)?;
        
        // Deserialize the board data
        let board = bytemuck::try_from_bytes::<Board>(&account.data)
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Board: {:?}", e)))?;
        
        Ok(*board)
    }

    pub fn get_miner(&self) -> Result<Option<Miner>> {
        let (miner_address, _) = ore_api::state::miner_pda(self.keypair.pubkey());
        
        match self.rpc_client.get_account(&miner_address) {
            Ok(account) => {
                let miner = bytemuck::try_from_bytes::<Miner>(&account.data)
                    .map_err(|e| BotError::Serialization(format!("Failed to deserialize Miner: {:?}", e)))?;
                Ok(Some(*miner))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn get_round(&self, round_id: u64) -> Result<Round> {
        let (round_address, _) = ore_api::state::round_pda(round_id);
        let account = self.rpc_client.get_account(&round_address)?;
        
        let round = bytemuck::try_from_bytes::<Round>(&account.data)
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Round: {:?}", e)))?;
        
        Ok(*round)
    }

    pub fn get_treasury(&self) -> Result<Treasury> {
        let treasury_address = ore_api::consts::TREASURY_ADDRESS;
        let account = self.rpc_client.get_account(&treasury_address)?;
        
        let treasury = bytemuck::try_from_bytes::<Treasury>(&account.data)
            .map_err(|e| BotError::Serialization(format!("Failed to deserialize Treasury: {:?}", e)))?;
        
        Ok(*treasury)
    }

    pub fn get_current_round(&self) -> Result<Round> {
        let board = self.get_board()?;
        self.get_round(board.round)
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
        Ok(self.rpc_client.get_block_time(slot)?)
    }

    /// Get multiple miner accounts for analysis
    pub fn get_miners(&self, addresses: &[Pubkey]) -> Result<Vec<(Pubkey, Miner)>> {
        let mut miners = Vec::new();
        
        for address in addresses {
            if let Ok(account) = self.rpc_client.get_account(address) {
                if let Ok(miner) = bytemuck::try_from_bytes::<Miner>(&account.data) {
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
}
