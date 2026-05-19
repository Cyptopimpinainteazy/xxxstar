use serde::{Deserialize, Serialize};

/* ── User ────────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub avatar_url: String,
    pub headline: String,
    pub about_me: String,
    pub who_id_like_to_meet: String,
    pub interests: String,
    pub music_interests: String,
    pub movie_interests: String,
    pub hero_song_path: String,
    pub hero_song_title: String,
    pub profile_css: String,
    pub profile_bg_url: String,
    pub mood: String,
    pub gender: String,
    pub age: i32,
    pub location: String,
    pub orientation: String,
    pub status: String,
    pub body_type: String,
    pub ethnicity: String,
    pub zodiac_sign: String,
    pub smoke_drink: String,
    pub children: String,
    pub education: String,
    pub occupation: String,
    pub income: String,
    pub online_status: String,
    pub last_login: String,
    pub profile_views: i32,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterInput {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: String,
    pub team_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileInput {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub headline: Option<String>,
    pub about_me: Option<String>,
    pub who_id_like_to_meet: Option<String>,
    pub interests: Option<String>,
    pub music_interests: Option<String>,
    pub movie_interests: Option<String>,
    pub hero_song_path: Option<String>,
    pub hero_song_title: Option<String>,
    pub profile_css: Option<String>,
    pub profile_bg_url: Option<String>,
    pub mood: Option<String>,
    pub gender: Option<String>,
    pub age: Option<i32>,
    pub location: Option<String>,
    pub orientation: Option<String>,
    pub status: Option<String>,
    pub body_type: Option<String>,
    pub ethnicity: Option<String>,
    pub zodiac_sign: Option<String>,
    pub smoke_drink: Option<String>,
    pub children: Option<String>,
    pub education: Option<String>,
    pub occupation: Option<String>,
    pub income: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub user_id: String,
    pub username: String,
    pub token: String,
    pub role: String,
}

/* ── Friends ─────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequest {
    pub id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub from_display_name: String,
    pub from_avatar: String,
    pub to_user_id: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub headline: String,
    pub online_status: String,
    pub is_top_friend: bool,
    pub top_friend_rank: i32,
}

/* ── Messages ────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub from_display_name: String,
    pub from_avatar: String,
    pub to_user_id: String,
    pub subject: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    pub to_user_id: String,
    pub subject: String,
    pub body: String,
}

/* ── Bulletins ───────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bulletin {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostBulletinInput {
    pub title: String,
    pub body: String,
}

/* ── Profile Comments ────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileComment {
    pub id: String,
    pub profile_user_id: String,
    pub author_user_id: String,
    pub author_username: String,
    pub author_display_name: String,
    pub author_avatar: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostCommentInput {
    pub profile_user_id: String,
    pub body: String,
}

/* ── Blog ────────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlogPost {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub title: String,
    pub body: String,
    pub mood: String,
    pub comments: Vec<BlogComment>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlogComment {
    pub id: String,
    pub author_user_id: String,
    pub author_username: String,
    pub author_display_name: String,
    pub author_avatar: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBlogPostInput {
    pub title: String,
    pub body: String,
    pub mood: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostBlogCommentInput {
    pub blog_post_id: String,
    pub body: String,
}

/* ── Photos ──────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: String,
    pub user_id: String,
    pub album_name: String,
    pub file_path: String,
    pub caption: String,
    pub is_default: bool,
    pub created_at: String,
}

/* ── Music ───────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrack {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub artist: String,
    pub file_path: String,
    pub duration_secs: i32,
    pub play_count: i32,
    pub is_profile_song: bool,
    pub created_at: String,
}

/* ── Status Updates (feed) ───────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdate {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostStatusInput {
    pub body: String,
}

/* ── Kudos ───────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kudo {
    pub id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub from_avatar: String,
    pub kind: String,
    pub created_at: String,
}

/* ── Search ──────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub headline: String,
    pub location: String,
    pub online_status: String,
}

/* ── Groups ──────────────────────────────────────── */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_user_id: String,
    pub category: String,
    pub avatar_url: String,
    pub member_count: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupInput {
    pub name: String,
    pub description: String,
    pub category: Option<String>,
}
