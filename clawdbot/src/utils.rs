use crate::{client::OreClient, error::Result};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_instruction,
    transaction::Transaction,
};
use std::sync::Arc;

/// Transaction builder helper for ORE operations
pub struct TransactionBuilder {
    client: Arc<OreClient>,
    instructions: Vec<Instruction>,
}

impl TransactionBuilder {
    pub fn new(client: Arc<OreClient>) -> Self {
        Self {
            client,
            instructions: Vec::new(),
        }
    }

    /// Add a custom instruction
    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add multiple instructions
    pub fn add_instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }

    /// Add a SOL transfer instruction
    pub fn add_transfer(mut self, to: Pubkey, lamports: u64) -> Self {
        let from = self.client.pubkey();
        self.instructions.push(system_instruction::transfer(&from, &to, lamports));
        self
    }

    /// Build and sign the transaction
    pub fn build(self) -> Result<Transaction> {
        let recent_blockhash = self.client.rpc_client.get_latest_blockhash()?;
        
        let mut transaction = Transaction::new_with_payer(
            &self.instructions,
            Some(&self.client.pubkey()),
        );
        
        transaction.sign(&[&*self.client.keypair], recent_blockhash);
        
        Ok(transaction)
    }

    /// Build and send the transaction
    pub fn send(self) -> Result<String> {
        let transaction = self.build()?;
        self.client.send_transaction(transaction)
    }

    /// Build transaction with simulation first
    pub async fn simulate_and_send(self) -> Result<String> {
        let transaction = self.build()?;
        
        // Simulate first
        let result = self.client.rpc_client.simulate_transaction(&transaction)?;
        
        if result.value.err.is_some() {
            return Err(crate::error::BotError::TransactionFailed(
                format!("Simulation failed: {:?}", result.value.err)
            ));
        }
        
        // Send if simulation passed
        self.client.send_transaction(transaction)
    }
}

/// Helper to build ORE-specific transactions
pub mod ore_transactions {
    use super::*;
    use ore_api::instruction::*;

    /// Build a mine instruction transaction
    pub fn build_mine_tx(
        client: Arc<OreClient>,
        square: usize,
        amount: u64,
    ) -> Result<Transaction> {
        // This is a placeholder - actual implementation would use ore-api
        TransactionBuilder::new(client)
            .build()
    }

    /// Build a claim rewards transaction
    pub fn build_claim_tx(
        client: Arc<OreClient>,
    ) -> Result<Transaction> {
        // Placeholder for claim transaction
        TransactionBuilder::new(client)
            .build()
    }

    /// Build a bet transaction
    pub fn build_bet_tx(
        client: Arc<OreClient>,
        squares: &[usize],
        amounts: &[u64],
    ) -> Result<Transaction> {
        // Placeholder for betting transaction
        TransactionBuilder::new(client)
            .build()
    }
}

/// Rate limiter for RPC calls
pub struct RateLimiter {
    calls_per_second: u32,
    last_call: std::sync::Mutex<std::time::Instant>,
}

impl RateLimiter {
    pub fn new(calls_per_second: u32) -> Self {
        Self {
            calls_per_second,
            last_call: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub async fn acquire(&self) {
        let min_interval = std::time::Duration::from_millis(1000 / self.calls_per_second as u64);
        
        loop {
            let mut last = self.last_call.lock().unwrap();
            let elapsed = last.elapsed();
            
            if elapsed >= min_interval {
                *last = std::time::Instant::now();
                break;
            }
            
            drop(last);
            tokio::time::sleep(min_interval - elapsed).await;
        }
    }
}
