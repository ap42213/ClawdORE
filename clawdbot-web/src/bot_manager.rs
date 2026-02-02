use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct BotManager {
    running_bots: HashMap<String, Child>,
}

impl BotManager {
    pub fn new() -> Self {
        Self {
            running_bots: HashMap::new(),
        }
    }

    pub async fn start_bot(&mut self, bot_name: &str) -> Result<(), String> {
        if self.running_bots.contains_key(bot_name) {
            return Err(format!("Bot {} is already running", bot_name));
        }

        let bot_path = format!("../clawdbot/target/release/{}-bot", bot_name);
        
        // Check if bot binary exists
        if !std::path::Path::new(&bot_path).exists() {
            return Err(format!(
                "Bot binary not found. Please build first: cd ../clawdbot && cargo build --release"
            ));
        }

        let child = Command::new(&bot_path)
            .env("RUST_LOG", "info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start bot: {}", e))?;

        self.running_bots.insert(bot_name.to_string(), child);
        Ok(())
    }

    pub async fn stop_bot(&mut self, bot_name: &str) -> Result<(), String> {
        if let Some(mut child) = self.running_bots.remove(bot_name) {
            child
                .kill()
                .await
                .map_err(|e| format!("Failed to stop bot: {}", e))?;
            Ok(())
        } else {
            Err(format!("Bot {} is not running", bot_name))
        }
    }

    pub fn is_running(&self, bot_name: &str) -> bool {
        self.running_bots.contains_key(bot_name)
    }

    pub fn list_running(&self) -> Vec<String> {
        self.running_bots.keys().cloned().collect()
    }
}

impl Drop for BotManager {
    fn drop(&mut self) {
        // Kill all running bots on drop
        for (_, mut child) in self.running_bots.drain() {
            let _ = child.start_kill();
        }
    }
}
