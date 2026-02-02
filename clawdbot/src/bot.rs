use crate::{client::OreClient, config::BotConfig, error::Result};
use log::{debug, error, info, warn};
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

pub trait Bot: Send + Sync {
    fn name(&self) -> &str;
    fn status(&self) -> BotStatus;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn pause(&mut self) -> Result<()>;
    async fn resume(&mut self) -> Result<()>;
}

pub struct BotRunner {
    bots: Vec<Box<dyn Bot>>,
    config: Arc<BotConfig>,
    client: Arc<OreClient>,
}

impl BotRunner {
    pub fn new(config: BotConfig, client: OreClient) -> Self {
        Self {
            bots: Vec::new(),
            config: Arc::new(config),
            client: Arc::new(client),
        }
    }

    pub fn add_bot(&mut self, bot: Box<dyn Bot>) {
        self.bots.push(bot);
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting ClawdBot system...");
        
        // Start all bots
        for bot in &mut self.bots {
            info!("Starting bot: {}", bot.name());
            if let Err(e) = bot.start().await {
                error!("Failed to start bot {}: {}", bot.name(), e);
            }
        }

        // Main monitoring loop
        loop {
            sleep(Duration::from_secs(10)).await;
            
            // Check bot statuses
            for bot in &self.bots {
                debug!("Bot {} status: {:?}", bot.name(), bot.status());
            }
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down ClawdBot system...");
        
        for bot in &mut self.bots {
            info!("Stopping bot: {}", bot.name());
            if let Err(e) = bot.stop().await {
                error!("Failed to stop bot {}: {}", bot.name(), e);
            }
        }
        
        Ok(())
    }

    pub fn get_config(&self) -> &BotConfig {
        &self.config
    }

    pub fn get_client(&self) -> &OreClient {
        &self.client
    }
}
