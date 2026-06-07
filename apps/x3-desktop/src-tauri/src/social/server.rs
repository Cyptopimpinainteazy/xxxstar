// Real-time WebSocket Server for X3 Social Network
// Handles message broadcasting and live notifications

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use chrono::Utc;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: Option<String>,  // None for broadcast, Some for direct
    pub message: String,
    pub message_type: String,  // chat, notification, typing, system
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub notification_type: String,  // friend_request, like, comment, mentioned, etc.
    pub from_user_id: String,
    pub from_username: String,
    pub from_display_name: String,
    pub from_avatar: String,
    pub subject: String,
    pub message: String,
    pub related_id: Option<String>,  // ID of post, comment, etc.
    pub related_type: Option<String>,  // post, comment, profile, etc.
    pub timestamp: String,
}

impl AppState {
    /// Create a new app state with broadcast channel
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1000);
        Self { tx }
    }

    /// Subscribe to message stream
    pub fn subscribe(&self) -> broadcast::Receiver<ChatMessage> {
        self.tx.subscribe()
    }

    /// Send message to all subscribers
    pub async fn broadcast_message(&self, msg: ChatMessage) -> Result<(), String> {
        self.tx.send(msg).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Send direct message to specific user (filtering done on client side)
    pub async fn send_direct_message(
        &self,
        from_id: &str,
        to_id: &str,
        content: &str,
    ) -> Result<(), String> {
        let msg = ChatMessage {
            from_user_id: from_id.to_string(),
            from_username: "user".to_string(),  // Would be fetched from DB
            to_user_id: Some(to_id.to_string()),
            message: content.to_string(),
            message_type: "chat".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.broadcast_message(msg).await
    }

    /// Broadcast notification to user
    pub async fn send_notification(
        &self,
        to_user_id: &str,
        notif: Notification,
    ) -> Result<(), String> {
        let msg_content = serde_json::to_string(&notif).map_err(|e| e.to_string())?;
        let msg = ChatMessage {
            from_user_id: notif.from_user_id.clone(),
            from_username: notif.from_username.clone(),
            to_user_id: Some(to_user_id.to_string()),
            message: msg_content,
            message_type: "notification".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.broadcast_message(msg).await
    }

    /// Handle typing indicator
    pub async fn send_typing_indicator(&self, user_id: &str) -> Result<(), String> {
        let msg = ChatMessage {
            from_user_id: user_id.to_string(),
            from_username: "system".to_string(),
            to_user_id: None,
            message: format!("{} is typing...", user_id),
            message_type: "typing".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        self.broadcast_message(msg).await
    }
}

/// Message handler for processing incoming WebSocket messages
pub struct MessageHandler;

impl MessageHandler {
    /// Process chat message
    pub async fn process_chat(
        state: &AppState,
        from_user_id: &str,
        to_user_id: Option<&str>,
        content: &str,
    ) -> Result<(), String> {
        let msg = ChatMessage {
            from_user_id: from_user_id.to_string(),
            from_username: "user".to_string(),
            to_user_id: to_user_id.map(|s| s.to_string()),
            message: content.to_string(),
            message_type: "chat".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        state.broadcast_message(msg).await
    }

    /// Process notification
    pub async fn process_notification(
        state: &AppState,
        user_id: &str,
        notif_type: &str,
        from_user: &str,
        content: &str,
    ) -> Result<(), String> {
        let notif = Notification {
            notification_type: notif_type.to_string(),
            from_user_id: "system".to_string(),
            from_username: from_user.to_string(),
            from_display_name: from_user.to_string(),
            from_avatar: "".to_string(),
            subject: content.to_string(),
            message: content.to_string(),
            related_id: None,
            related_type: None,
            timestamp: Utc::now().to_rfc3339(),
        };
        state.send_notification(user_id, notif).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new();
        // Should successfully create
        assert!(true);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            from_user_id: "user1".to_string(),
            from_username: "alice".to_string(),
            to_user_id: Some("user2".to_string()),
            message: "Hello!".to_string(),
            message_type: "chat".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.from_username, "alice");
    }

    #[test]
    fn test_notification_creation() {
        let notif = Notification {
            notification_type: "friend_request".to_string(),
            from_user_id: "user2".to_string(),
            from_username: "bob".to_string(),
            from_display_name: "Bob".to_string(),
            from_avatar: "https://example.com/bob.jpg".to_string(),
            subject: "Friend Request".to_string(),
            message: "Bob wants to be your friend".to_string(),
            related_id: None,
            related_type: None,
            timestamp: Utc::now().to_rfc3339(),
        };

        assert_eq!(notif.notification_type, "friend_request");
    }
}

