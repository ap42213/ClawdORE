use clawdbot::{
    client::OreClient,
    config::BotConfig,
    error::Result,
    monitor::MonitorBot,
};
use log::info;
use solana_sdk::signature::{read_keypair_file, Signer};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Load configuration
    let config = BotConfig::default();
    
    // Load keypair
    let keypair = read_keypair_file(&config.keypair_path)
        .expect("Failed to load keypair");

    info!("🤖 Starting ClawdBot Monitor");
    info!("📍 Wallet: {}", keypair.pubkey());
    info!("🌐 RPC: {}", config.rpc_url);

    // Create client
    let client = OreClient::new(config.rpc_url.clone(), keypair);
    let client_arc = Arc::new(client);

    // Create and run monitor bot directly
    let mut monitor_bot = MonitorBot::new(config.monitor.clone(), Arc::clone(&client_arc));
    
    // Run monitor loop
    monitor_bot.start().await?;

    Ok(())
}
