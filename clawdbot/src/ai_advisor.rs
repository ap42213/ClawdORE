//! AI Advisor using OpenRouter GLM 4.7 for enhanced mining decisions
//! 
//! Provides LLM-powered analysis of:
//! - Current board state and competition
//! - Historical patterns and win rates
//! - Optimal square selection strategy

use log::{info, warn, debug};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const MODEL: &str = "zhipu-ai/glm-4-plus";  // GLM 4.7/4-plus

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone)]
pub struct AIAdvisor {
    client: Client,
    api_key: String,
    enabled: bool,
}

/// AI-enhanced decision recommendation
#[derive(Debug, Clone, Default)]
pub struct AIRecommendation {
    pub suggested_squares: Vec<usize>,
    pub confidence: f64,
    pub reasoning: String,
    pub risk_assessment: String,
    pub expected_roi_modifier: f64,  // Multiplier for expected ROI (1.0 = no change)
}

impl AIAdvisor {
    pub fn new() -> Self {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .unwrap_or_default();
        
        let enabled = !api_key.is_empty();
        
        if enabled {
            info!("🤖 AI Advisor enabled (GLM 4.7 via OpenRouter)");
        } else {
            info!("🤖 AI Advisor disabled (no OPENROUTER_API_KEY)");
        }
        
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            api_key,
            enabled,
        }
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Get AI-powered recommendation for the current round
    pub async fn get_recommendation(
        &self,
        round_id: u64,
        deployed: &[u64; 25],
        total_deployed_sol: f64,
        num_deployers: u32,
        time_remaining: f64,
        consensus_squares: &[usize],
        consensus_confidence: f64,
        win_rate_by_count: &[(u8, f64)],  // (square_count, win_rate)
        balance_sol: f64,
    ) -> Option<AIRecommendation> {
        if !self.enabled {
            return None;
        }
        
        // Build context for the AI
        let board_state = self.format_board_state(deployed);
        let win_stats = self.format_win_stats(win_rate_by_count);
        
        let prompt = format!(
            r#"You are an expert ORE blockchain game strategist. Analyze this round and recommend optimal squares.

GAME RULES:
- 5x5 grid (squares 1-25)
- Players deploy SOL to squares
- One random square wins each round
- Winners split the ORE reward proportionally by their deployment
- Lower total deployment = more ORE per winner
- More squares = higher win chance but diluted reward

CURRENT ROUND #{round_id}:
- Time remaining: {time_remaining:.1}s
- Total deployed: {total_deployed_sol:.4} SOL
- Active deployers: {num_deployers}
- My balance: {balance_sol:.4} SOL

BOARD STATE (deployment per square in SOL):
{board_state}

HISTORICAL WIN RATES BY SQUARE COUNT:
{win_stats}

CONSENSUS RECOMMENDATION:
- Squares: {consensus_squares:?}
- Confidence: {consensus_confidence:.0}%

Based on this data, recommend:
1. Which squares to deploy to (list 1-25)
2. Confidence level (0-100%)
3. Brief reasoning (1-2 sentences)
4. Risk level (low/medium/high)

Respond in JSON format:
{{"squares": [1, 7, 13], "confidence": 75, "reasoning": "...", "risk": "medium"}}"#,
            round_id = round_id,
            time_remaining = time_remaining,
            total_deployed_sol = total_deployed_sol,
            num_deployers = num_deployers,
            balance_sol = balance_sol,
            board_state = board_state,
            win_stats = win_stats,
            consensus_squares = consensus_squares,
            consensus_confidence = consensus_confidence * 100.0,
        );
        
        debug!("🤖 Querying AI advisor...");
        
        let request = ChatRequest {
            model: MODEL.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.3,  // Lower = more deterministic
            max_tokens: 256,
        };
        
        let response = match self.client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://clawdore.app")
            .header("X-Title", "ClawdORE Mining Bot")
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("AI request failed: {}", e);
                return None;
            }
        };
        
        if !response.status().is_success() {
            warn!("AI API error: {}", response.status());
            return None;
        }
        
        let chat_response: ChatResponse = match response.json().await {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to parse AI response: {}", e);
                return None;
            }
        };
        
        let content = chat_response.choices.first()?.message.content.clone();
        
        // Parse the JSON response
        self.parse_ai_response(&content)
    }
    
    fn format_board_state(&self, deployed: &[u64; 25]) -> String {
        let lamports_per_sol = 1_000_000_000u64;
        let mut rows = Vec::new();
        
        for row in 0..5 {
            let mut cells = Vec::new();
            for col in 0..5 {
                let idx = row * 5 + col;
                let sol = deployed[idx] as f64 / lamports_per_sol as f64;
                cells.push(format!("{:2}: {:.3}", idx + 1, sol));
            }
            rows.push(cells.join(" | "));
        }
        
        rows.join("\n")
    }
    
    fn format_win_stats(&self, win_rate_by_count: &[(u8, f64)]) -> String {
        if win_rate_by_count.is_empty() {
            return "No historical data available".to_string();
        }
        
        win_rate_by_count.iter()
            .map(|(count, rate)| format!("{} squares: {:.1}% win rate", count, rate * 100.0))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn parse_ai_response(&self, content: &str) -> Option<AIRecommendation> {
        // Try to extract JSON from the response
        let json_start = content.find('{')?;
        let json_end = content.rfind('}')? + 1;
        let json_str = &content[json_start..json_end];
        
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        
        let squares: Vec<usize> = parsed["squares"]
            .as_array()?
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as usize))
            .filter(|&s| s >= 1 && s <= 25)
            .collect();
        
        if squares.is_empty() {
            return None;
        }
        
        let confidence = parsed["confidence"].as_f64().unwrap_or(50.0) / 100.0;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("AI analysis").to_string();
        let risk = parsed["risk"].as_str().unwrap_or("medium").to_lowercase();
        
        // Convert risk to ROI modifier
        let roi_modifier = match risk.as_str() {
            "low" => 1.1,
            "high" => 0.9,
            _ => 1.0,
        };
        
        info!("🤖 AI recommends: {:?} (conf: {:.0}%, risk: {})", 
            squares, confidence * 100.0, risk);
        info!("   Reasoning: {}", reasoning);
        
        Some(AIRecommendation {
            suggested_squares: squares,
            confidence,
            reasoning,
            risk_assessment: risk,
            expected_roi_modifier: roi_modifier,
        })
    }
}

impl Default for AIAdvisor {
    fn default() -> Self {
        Self::new()
    }
}
