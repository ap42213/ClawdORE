//! Setup Automation Account for Fast Executor Deploys
//! 
//! This tool creates an automation account that allows our miner bot
//! to execute deploys on your behalf with much faster timing.
//!
//! Usage:
//!   KEYPAIR_B58=<your_key> RPC_URL=<rpc> EXECUTOR_PUBKEY=<miner_bot_pubkey> \
//!   DEPOSIT_SOL=1.0 STRATEGY=discretionary SQUARES=5 cargo run --bin setup_automation
//!
//! Environment Variables:
//!   KEYPAIR_B58 or KEYPAIR_JSON - Your wallet's private key
//!   RPC_URL - Solana RPC endpoint
//!   EXECUTOR_PUBKEY - The miner bot's public key (will execute deploys for you)
//!   DEPOSIT_SOL - How much SOL to pre-fund (default: 1.0)
//!   AMOUNT_SOL - SOL per round (default: 0.04)
//!   STRATEGY - "random", "preferred", or "discretionary" (default: discretionary)
//!   SQUARES - Number of squares for random, or comma-separated for preferred (default: 5)
//!   RELOAD - "true" to auto-reload winnings (default: true)
//!   FEE_SOL - Executor fee per deploy (default: 0.0001)

use colored::*;
use log::{error, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

fn load_keypair(keypair_path: &str) -> Result<Keypair, String> {
    if let Ok(keypair_b58) = std::env::var("KEYPAIR_B58") {
        let bytes = bs58::decode(&keypair_b58)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58 keypair: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    if let Ok(keypair_json) = std::env::var("KEYPAIR_JSON") {
        let bytes: Vec<u8> = serde_json::from_str(&keypair_json)
            .map_err(|e| format!("Failed to parse keypair JSON: {}", e))?;
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to create keypair from bytes: {}", e));
    }
    
    read_keypair_file(keypair_path)
        .map_err(|e| format!("Failed to read keypair file '{}': {}", keypair_path, e))
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    println!("{}", r#"
    ╔═══════════════════════════════════════════════════════════════════════╗
    ║                    AUTOMATION SETUP                                   ║
    ║              Fast Executor Deploys for ORE Mining                     ║
    ╚═══════════════════════════════════════════════════════════════════════╝
    "#.bright_cyan());

    // Get RPC URL
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    
    // Load your keypair (the authority)
    let keypair_path = std::env::var("KEYPAIR_PATH").unwrap_or_else(|_| "keypair.json".to_string());
    let keypair = match load_keypair(&keypair_path) {
        Ok(kp) => kp,
        Err(e) => {
            error!("Failed to load keypair: {}", e);
            error!("");
            error!("Set one of:");
            error!("  - KEYPAIR_B58 (base58 encoded private key)");
            error!("  - KEYPAIR_JSON (JSON array of bytes)");
            error!("  - KEYPAIR_PATH pointing to a keypair file");
            return;
        }
    };

    // Get executor pubkey (the miner bot that will execute deploys)
    let executor_pubkey = match std::env::var("EXECUTOR_PUBKEY") {
        Ok(s) => match Pubkey::from_str(&s) {
            Ok(pk) => pk,
            Err(e) => {
                error!("Invalid EXECUTOR_PUBKEY: {}", e);
                return;
            }
        },
        Err(_) => {
            error!("EXECUTOR_PUBKEY is required");
            error!("This should be the public key of your miner bot");
            return;
        }
    };

    // Parse configuration
    let deposit_sol: f64 = std::env::var("DEPOSIT_SOL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);
    
    let amount_sol: f64 = std::env::var("AMOUNT_SOL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.04);
    
    let fee_sol: f64 = std::env::var("FEE_SOL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0001);
    
    let reload: bool = std::env::var("RELOAD")
        .ok()
        .map(|s| s.to_lowercase() == "true")
        .unwrap_or(true);
    
    let strategy_str = std::env::var("STRATEGY")
        .unwrap_or_else(|_| "discretionary".to_string())
        .to_lowercase();
    
    let squares_str = std::env::var("SQUARES").unwrap_or_else(|_| "5".to_string());
    
    // Parse strategy and mask
    let (strategy, mask): (u8, u64) = match strategy_str.as_str() {
        "random" => {
            // For random, mask is just the number of squares
            let num_squares: u64 = squares_str.parse().unwrap_or(5);
            (0, num_squares)
        }
        "preferred" => {
            // For preferred, mask is a bitmask of specific squares
            let squares: Vec<usize> = squares_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .filter(|&sq| sq < 25)
                .collect();
            let mut mask: u64 = 0;
            for sq in squares {
                mask |= 1 << sq;
            }
            (1, mask)
        }
        "discretionary" => {
            // For discretionary, executor chooses - mask not used
            (2, 0)
        }
        _ => {
            error!("Invalid STRATEGY: {}. Use 'random', 'preferred', or 'discretionary'", strategy_str);
            return;
        }
    };

    // Convert to lamports
    let deposit = (deposit_sol * LAMPORTS_PER_SOL as f64) as u64;
    let amount = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
    let fee = (fee_sol * LAMPORTS_PER_SOL as f64) as u64;

    info!("═══════════════════════════════════════════════════════════════");
    info!("📋 Automation Setup:");
    info!("   RPC: {}", rpc_url);
    info!("   Authority (you): {}", keypair.pubkey());
    info!("   Executor (miner bot): {}", executor_pubkey);
    info!("");
    info!("💰 Funding:");
    info!("   Deposit: {:.4} SOL", deposit_sol);
    info!("   Amount per round: {:.4} SOL", amount_sol);
    info!("   Executor fee: {:.6} SOL", fee_sol);
    info!("   Est. rounds: {}", (deposit_sol / (amount_sol + fee_sol)) as u32);
    info!("");
    info!("🎯 Strategy:");
    info!("   Type: {}", strategy_str);
    info!("   Mask: {}", mask);
    info!("   Auto-reload: {}", reload);
    info!("═══════════════════════════════════════════════════════════════");

    // Create RPC client
    let rpc_client = RpcClient::new_with_commitment(
        rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Check balance
    let balance = match rpc_client.get_balance(&keypair.pubkey()) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to get balance: {}", e);
            return;
        }
    };

    let balance_sol = balance as f64 / LAMPORTS_PER_SOL as f64;
    info!("💳 Current balance: {:.4} SOL", balance_sol);

    if balance < deposit + 10_000_000 { // Need extra for tx fees
        error!("Insufficient balance! Need at least {:.4} SOL", deposit_sol + 0.01);
        return;
    }

    // Build the automate instruction
    let ix = ore_api::sdk::automate(
        keypair.pubkey(),  // signer (you)
        amount,            // SOL per square per round
        deposit,           // total SOL to deposit
        executor_pubkey,   // who can trigger deploys
        fee,               // fee per deploy
        mask,              // strategy mask
        strategy,          // strategy type
        reload,            // auto-reload winnings
    );

    // Get blockhash
    let blockhash = match rpc_client.get_latest_blockhash() {
        Ok(bh) => bh,
        Err(e) => {
            error!("Failed to get blockhash: {}", e);
            return;
        }
    };

    // Create and sign transaction
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    info!("");
    info!("{}", "📤 Sending automation setup transaction...".yellow());

    // Send and confirm
    match rpc_client.send_and_confirm_transaction(&tx) {
        Ok(sig) => {
            info!("{}", format!("✅ Automation created successfully!").green().bold());
            info!("   Signature: {}", sig);
            info!("");
            info!("📋 Next Steps:");
            info!("   1. Set BOT_MODE=executor on your miner bot");
            info!("   2. Set AUTHORITY_PUBKEY={}", keypair.pubkey());
            info!("   3. Restart the miner bot");
            info!("");
            info!("   The miner bot will now deploy at ~0.8s before round end!");
        }
        Err(e) => {
            error!("❌ Transaction failed: {}", e);
            error!("");
            error!("If this is your first automation, you may need to:");
            error!("  - Ensure you have enough SOL");
            error!("  - Check the executor pubkey is correct");
        }
    }
}
