// ActivityPub Federation for X3 Social Network
// Enables federation with Mastodon, Pixelfed, and other ActivityPub-compatible services

use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPubActor {
    pub id: String,
    #[serde(rename = "type")]
    pub actor_type: String,
    pub name: String,
    pub preferred_username: String,
    pub summary: Option<String>,
    pub url: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    pub public_key: PublicKey,
    pub icon: Option<Image>,
    pub image: Option<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    #[serde(rename = "type")]
    pub image_type: String,
    pub url: String,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCreate {
    pub id: String,
    #[serde(rename = "type")]
    pub activity_type: String,  // "Create"
    pub actor: String,
    pub published: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub object: CreateObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateObject {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,  // "Note", "Article"
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub content: String,
    pub published: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub replies: Option<String>,
    pub attachment: Option<Vec<ActivityMedia>>,
    pub tag: Option<Vec<ActivityTag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityMedia {
    #[serde(rename = "type")]
    pub media_type: String,  // "Image", "Video"
    pub media_type_attr: String,  // MIME type
    pub url: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityTag {
    #[serde(rename = "type")]
    pub tag_type: String,  // "Hashtag", "Mention"
    pub href: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Follow {
    pub id: String,
    #[serde(rename = "type")]
    pub follow_type: String,  // "Follow"
    pub actor: String,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accept {
    pub id: String,
    #[serde(rename = "type")]
    pub accept_type: String,  // "Accept"
    pub actor: String,
    pub object: Follow,
}

pub struct ActivityPubHandler {
    pub instance_domain: String,
    pub instance_name: String,
    pub instance_admin: String,
}

impl ActivityPubHandler {
    pub fn new(domain: String, name: String, admin: String) -> Self {
        Self {
            instance_domain: domain,
            instance_name: name,
            instance_admin: admin,
        }
    }

    /// Generate ActivityPub Actor representation for a user
    pub fn user_to_actor(&self, user_id: &str, username: &str, display_name: &str, summary: &str, avatar_url: &str) -> ActivityPubActor {
        let base_url = format!("https://{}/ap", self.instance_domain);
        let user_url = format!("{}/users/{}", base_url, user_id);
        
        ActivityPubActor {
            id: user_url.clone(),
            actor_type: "Person".to_string(),
            name: display_name.to_string(),
            preferred_username: username.to_string(),
            summary: Some(summary.to_string()),
            url: user_url.clone(),
            inbox: format!("{}/inbox", user_url),
            outbox: format!("{}/outbox", user_url),
            followers: format!("{}/followers", user_url),
            following: format!("{}/following", user_url),
            public_key: PublicKey {
                id: format!("{}/public-key", user_url),
                owner: user_url.clone(),
                public_key_pem: "[PEM key would go here]".to_string(),
            },
            icon: Some(Image {
                image_type: "Image".to_string(),
                url: avatar_url.to_string(),
                media_type: Some("image/jpeg".to_string()),
            }),
            image: None,
        }
    }

    /// Generate ActivityPub Create activity from a post
    pub fn post_to_activity(&self, post_id: &str, user_id: &str, content: &str, attachments: Vec<(&str, &str)>) -> ActivityCreate {
        let base_url = format!("https://{}/ap", self.instance_domain);
        let user_url = format!("{}/users/{}", base_url, user_id);
        let activity_id = format!("{}/activities/{}", base_url, uuid::Uuid::new_v4());
        let object_id = format!("{}/posts/{}", base_url, post_id);

        let attachment = if !attachments.is_empty() {
            Some(
                attachments
                    .iter()
                    .map(|(url, media_type)| ActivityMedia {
                        media_type: if media_type.starts_with("video") { "Video" } else { "Image" }.to_string(),
                        media_type_attr: media_type.to_string(),
                        url: url.to_string(),
                        name: None,
                    })
                    .collect()
            )
        } else {
            None
        };

        ActivityCreate {
            id: activity_id,
            activity_type: "Create".to_string(),
            actor: user_url.clone(),
            published: Utc::now().to_rfc3339(),
            to: vec!["https://www.w3.org/ns/activitystreams#Public".to_string()],
            cc: vec![format!("{}/followers", user_url)],
            object: CreateObject {
                id: object_id,
                object_type: "Note".to_string(),
                attributed_to: user_url,
                in_reply_to: None,
                content: content.to_string(),
                published: Utc::now().to_rfc3339(),
                to: vec!["https://www.w3.org/ns/activitystreams#Public".to_string()],
                cc: vec![],
                replies: None,
                attachment,
                tag: None,
            },
        }
    }

    /// Generate follow acceptance activity
    pub fn accept_follow(&self, follow_activity: &Follow) -> Accept {
        let base_url = format!("https://{}/ap", self.instance_domain);
        
        Accept {
            id: format!("{}/activities/{}", base_url, uuid::Uuid::new_v4()),
            accept_type: "Accept".to_string(),
            actor: follow_activity.object.clone(),
            object: follow_activity.clone(),
        }
    }
}

// Temporary UUID generation until we import uuid crate
mod uuid {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            let mut hasher = RandomState::new().build_hasher();
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().hash(&mut hasher);
            format!("{:x}", hasher.finish())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_generation() {
        let handler = ActivityPubHandler::new(
            "x3.network".to_string(),
            "X3 Social".to_string(),
            "admin@x3.network".to_string(),
        );

        let actor = handler.user_to_actor(
            "user123",
            "alice",
            "Alice Wonder",
            "A passionate creator",
            "https://example.com/avatar.jpg",
        );

        assert_eq!(actor.preferred_username, "alice");
        assert!(actor.id.contains("user123"));
    }

    #[test]
    fn test_activity_creation() {
        let handler = ActivityPubHandler::new(
            "x3.network".to_string(),
            "X3 Social".to_string(),
            "admin@x3.network".to_string(),
        );

        let activity = handler.post_to_activity(
            "post123",
            "user123",
            "Hello Fediverse!",
            vec![],
        );

        assert_eq!(activity.activity_type, "Create");
        assert_eq!(activity.object.content, "Hello Fediverse!");
    }
}
