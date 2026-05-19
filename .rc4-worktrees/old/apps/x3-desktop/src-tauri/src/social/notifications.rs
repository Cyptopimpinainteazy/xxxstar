// Real-time Notifications for X3 Social Network

use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub from_user_id: String,
    pub from_username: String,
    pub from_display_name: String,
    pub from_avatar_url: String,
    pub subject: String,
    pub body: String,
    pub related_id: Option<String>,
    pub related_type: Option<String>, // post, comment, profile, message
    pub is_read: bool,
    pub action_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    FriendRequest,
    FriendRequestAccepted,
    PostLiked,
    PostCommented,
    CommentReplied,
    Mentioned,
    NewFollower,
    ProfileVisited,
    NewMessage,
    EventInvite,
    BlogCommented,
    MusicShared,
    PhotoCommented,
    TipReceived,
    CreatorSubscribed,
    ProofOfHumanVerified,
    NftMinted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferences {
    pub user_id: String,
    pub friend_request: bool,
    pub post_likes: bool,
    pub post_comments: bool,
    pub mentions: bool,
    pub new_followers: bool,
    pub direct_messages: bool,
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub frequency: NotificationFrequency,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: Option<String>, // HH:MM
    pub quiet_hours_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationFrequency {
    Instant,
    Daily,
    Weekly,
    Never,
}

pub struct NotificationManager;

impl NotificationManager {
    /// Create friend request notification
    pub fn friend_request(
        user_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::FriendRequest,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} sent you a friend request", from_display_name),
            body: format!("{} (@{}) wants to be your friend", from_display_name, from_username),
            related_id: None,
            related_type: None,
            is_read: false,
            action_url: Some(format!("/profile/{}", from_user_id)),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create post liked notification
    pub fn post_liked(
        user_id: &str,
        post_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::PostLiked,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} liked your post", from_display_name),
            body: format!("{} (@{}) liked your post", from_display_name, from_username),
            related_id: Some(post_id.to_string()),
            related_type: Some("post".to_string()),
            is_read: false,
            action_url: Some(format!("/post/{}", post_id)),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create post commented notification
    pub fn post_commented(
        user_id: &str,
        post_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
        comment_text: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::PostCommented,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} commented on your post", from_display_name),
            body: format!("{}: \"{}\"", from_display_name, truncate(comment_text, 100)),
            related_id: Some(post_id.to_string()),
            related_type: Some("comment".to_string()),
            is_read: false,
            action_url: Some(format!("/post/{}", post_id)),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create mentioned notification
    pub fn mentioned(
        user_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
        post_id: &str,
        context: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::Mentioned,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} mentioned you", from_display_name),
            body: format!("{} mentioned you in a post: \"{}\"", from_display_name, truncate(context, 100)),
            related_id: Some(post_id.to_string()),
            related_type: Some("mention".to_string()),
            is_read: false,
            action_url: Some(format!("/post/{}", post_id)),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create new follower notification
    pub fn new_follower(
        user_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::NewFollower,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} started following you", from_display_name),
            body: format!("{} (@{}) is now following you", from_display_name, from_username),
            related_id: None,
            related_type: None,
            is_read: false,
            action_url: Some(format!("/profile/{}", from_user_id)),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Create tip received notification
    pub fn tip_received(
        user_id: &str,
        from_user_id: &str,
        from_username: &str,
        from_display_name: &str,
        from_avatar_url: &str,
        amount: f64,
        currency: &str,
    ) -> Notification {
        Notification {
            id: uuid(),
            user_id: user_id.to_string(),
            notification_type: NotificationType::TipReceived,
            from_user_id: from_user_id.to_string(),
            from_username: from_username.to_string(),
            from_display_name: from_display_name.to_string(),
            from_avatar_url: from_avatar_url.to_string(),
            subject: format!("{} sent you a tip!", from_display_name),
            body: format!("{} sent you a tip of {:.2} {}", from_display_name, amount, currency),
            related_id: None,
            related_type: None,
            is_read: false,
            action_url: Some(format!("/earnings")),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

fn uuid() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let mut hasher = RandomState::new().build_hasher();
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friend_request_notification() {
        let notif = NotificationManager::friend_request(
            "user1",
            "user2",
            "alice",
            "Alice",
            "https://example.com/alice.jpg",
        );

        assert_eq!(notif.notification_type, NotificationType::FriendRequest);
        assert_eq!(notif.user_id, "user1");
        assert!(!notif.is_read);
    }

    #[test]
    fn test_post_liked_notification() {
        let notif = NotificationManager::post_liked(
            "user1",
            "post123",
            "user2",
            "alice",
            "Alice",
            "https://example.com/alice.jpg",
        );

        assert_eq!(notif.notification_type, NotificationType::PostLiked);
        assert!(notif.related_id.is_some());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5), "Hello...");
        assert_eq!(truncate("Hi", 10), "Hi");
    }
}
