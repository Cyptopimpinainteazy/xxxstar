// crates/gpu-swarm/src/advanced/social_agents.rs
// Social agent integration for live actions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, error, span, Level};

/// Supported social platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocialPlatform {
    Twitter,
    Telegram,
    Discord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialAgent {
    pub id: String,
    pub platform: SocialPlatform,
    pub enabled: bool,
    pub auth_token: String,
    pub chat_id: Option<String>,
    pub channel_id: Option<String>,
    pub webhook_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialMessage {
    pub agent_id: String,
    pub platform: SocialPlatform,
    pub content: String,
    pub embeds: Option<Vec<String>>,
    pub priority: u32,
}

pub struct SocialAgentManager {
    agents: HashMap<String, SocialAgent>,
    feature_flags: HashMap<String, bool>,
    message_queue: parking_lot::Mutex<Vec<SocialMessage>>,
}

impl SocialAgentManager {
    pub fn new() -> Self {
        SocialAgentManager {
            agents: HashMap::new(),
            feature_flags: HashMap::new(),
            message_queue: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Register a social agent
    pub fn register_agent(&mut self, agent: SocialAgent) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "register_agent", agent_id = &agent.id);
        let _enter = span.enter();

        if self.agents.contains_key(&agent.id) {
            return Err(format!("Agent {} already registered", agent.id).into());
        }

        self.agents.insert(agent.id.clone(), agent);
        debug!("✅ Registered social agent");
        Ok(())
    }

    /// Enable/disable a feature flag
    pub fn set_feature_flag(&mut self, flag: &str, enabled: bool) {
        self.feature_flags.insert(flag.to_string(), enabled);
        debug!("🚩 Feature flag '{}' set to {}", flag, enabled);
    }

    /// Check if feature is enabled for agent
    pub fn is_feature_enabled(&self, agent_id: &str, feature: &str) -> bool {
        let flag = format!("{}:{}", agent_id, feature);
        self.feature_flags.get(&flag).copied().unwrap_or(false)
    }

    /// Queue a message for sending
    pub fn queue_message(&self, message: SocialMessage) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "queue_message", agent_id = &message.agent_id);
        let _enter = span.enter();

        let agent = self
            .agents
            .get(&message.agent_id)
            .ok_or(format!("Agent {} not found", message.agent_id))?;

        if !agent.enabled {
            return Err(format!("Agent {} is disabled", message.agent_id).into());
        }

        let mut queue = self.message_queue.lock();
        queue.push(message.clone());
        
        // Sort by priority (descending)
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        debug!("📨 Message queued for {}", message.agent_id);
        Ok(())
    }

    /// Process queued messages
    pub async fn process_messages(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "process_messages");
        let _enter = span.enter();

        let mut queue = self.message_queue.lock();
        let mut processed = 0;

        while let Some(message) = queue.first() {
            if let Err(e) = self.send_message(message).await {
                error!("Failed to send message: {}", e);
                break;
            }
            queue.remove(0);
            processed += 1;
        }

        debug!("✅ Processed {} messages", processed);
        Ok(processed)
    }

    /// Send a message via the appropriate platform
    async fn send_message(&self, message: &SocialMessage) -> Result<(), Box<dyn std::error::Error>> {
        let agent = self
            .agents
            .get(&message.agent_id)
            .ok_or("Agent not found")?;

        match agent.platform {
            SocialPlatform::Twitter => self.send_twitter_message(agent, message).await?,
            SocialPlatform::Telegram => self.send_telegram_message(agent, message).await?,
            SocialPlatform::Discord => self.send_discord_message(agent, message).await?,
        }

        Ok(())
    }

    /// Send via Twitter/X API
    async fn send_twitter_message(
        &self,
        agent: &SocialAgent,
        message: &SocialMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "send_twitter");
        let _enter = span.enter();

        // Twitter API v2 endpoint
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.twitter.com/2/tweets")
            .header("Authorization", format!("Bearer {}", agent.auth_token))
            .json(&serde_json::json!({
                "text": message.content,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Twitter API error: {}", response.status());
            return Err("Twitter send failed".into());
        }

        info!("✅ Twitter message sent");
        Ok(())
    }

    /// Send via Telegram API
    async fn send_telegram_message(
        &self,
        agent: &SocialAgent,
        message: &SocialMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "send_telegram");
        let _enter = span.enter();

        let chat_id = agent.chat_id.as_ref().ok_or("Telegram chat_id not configured")?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "https://api.telegram.org/bot{}/sendMessage",
                agent.auth_token
            ))
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": message.content,
                "parse_mode": "HTML",
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Telegram API error: {}", response.status());
            return Err("Telegram send failed".into());
        }

        info!("✅ Telegram message sent");
        Ok(())
    }

    /// Send via Discord webhook
    async fn send_discord_message(
        &self,
        agent: &SocialAgent,
        message: &SocialMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "send_discord");
        let _enter = span.enter();

        let webhook_url = agent.webhook_url.as_ref().ok_or("Discord webhook not configured")?;

        let client = reqwest::Client::new();
        let response = client
            .post(webhook_url)
            .json(&serde_json::json!({
                "content": message.content,
                "embeds": message.embeds.as_ref().unwrap_or(&Vec::new()),
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Discord API error: {}", response.status());
            return Err("Discord send failed".into());
        }

        info!("✅ Discord message sent");
        Ok(())
    }

    /// Get agent list
    pub fn list_agents(&self) -> Vec<SocialAgent> {
        self.agents.values().cloned().collect()
    }

    /// Get pending message count
    pub fn pending_message_count(&self) -> usize {
        self.message_queue.lock().len()
    }
}

impl Default for SocialAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_registration() {
        let mut manager = SocialAgentManager::new();

        let agent = SocialAgent {
            id: "twitter-monitor".to_string(),
            platform: SocialPlatform::Twitter,
            enabled: true,
            auth_token: "test-token".to_string(),
            chat_id: None,
            channel_id: None,
            webhook_url: None,
        };

        assert!(manager.register_agent(agent).is_ok());
        assert_eq!(manager.list_agents().len(), 1);
    }

    #[test]
    fn test_message_queueing() {
        let mut manager = SocialAgentManager::new();

        let agent = SocialAgent {
            id: "test-agent".to_string(),
            platform: SocialPlatform::Twitter,
            enabled: true,
            auth_token: "token".to_string(),
            chat_id: None,
            channel_id: None,
            webhook_url: None,
        };

        manager.register_agent(agent).unwrap();

        let message = SocialMessage {
            agent_id: "test-agent".to_string(),
            platform: SocialPlatform::Twitter,
            content: "Test message".to_string(),
            embeds: None,
            priority: 1,
        };

        assert!(manager.queue_message(message).is_ok());
        assert_eq!(manager.pending_message_count(), 1);
    }
}
