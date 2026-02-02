use crate::{client::OreClient, config::BotConfig, error::Result};
use log::{debug, error, info};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotStatus {
    Idle,
    Running,
    Paused,
    Stopped,
    Error,
}

pub struct BotRunner {
    config: Arc<BotConfig>,
    client: Arc<OreClient>,
}

impl BotRunner {
    pub fn new(config: BotConfig, client: OreClient) -> Self {
        Self {
            config: Arc::new(config),
            client: Arc::new(client),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting ClawdBot system...");
        
        // Main monitoring loop
        loop {
            sleep(Duration::from_secs(10)).await;
            debug!("Bot running...");
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down ClawdBot system...");
        Ok(())
    }

    pub fn get_config(&self) -> &BotConfig {
        &self.config
    }

    pub fn get_client(&self) -> &OreClient {
        &self.client
    }
}
