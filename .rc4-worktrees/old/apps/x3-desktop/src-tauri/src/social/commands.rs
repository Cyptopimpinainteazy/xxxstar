use crate::social::db::SocialDb;
use crate::social::models::*;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use ring::{pbkdf2, rand::{SecureRandom, SystemRandom}};
use rusqlite::params;
use serde::Serialize;
use std::num::NonZeroU32;
use tauri::State;
use uuid::Uuid;

static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const PBKDF2_ITERS: u32 = 100_000;
const PBKDF2_LEN: usize = 32;

fn pw_hash(password: &str) -> Result<String, String> {
    let rng = SystemRandom::new();
    let mut salt = [0u8; 16];
    rng.fill(&mut salt).map_err(|e| e.to_string())?;
    let iters = NonZeroU32::new(PBKDF2_ITERS).unwrap();
    let mut hash = [0u8; PBKDF2_LEN];
    pbkdf2::derive(PBKDF2_ALG, iters, &salt, password.as_bytes(), &mut hash);
    Ok(format!("pbkdf2:{}:{}:{}", PBKDF2_ITERS, hex::encode(salt), hex::encode(hash)))
}

fn pw_verify(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.splitn(4, ':').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2" { return false; }
    let iters = match parts[1].parse::<u32>().ok().and_then(NonZeroU32::new) { Some(n) => n, None => return false };
    let salt = match hex::decode(parts[2]) { Ok(s) => s, Err(_) => return false };
    let hash = match hex::decode(parts[3]) { Ok(h) => h, Err(_) => return false };
    pbkdf2::verify(PBKDF2_ALG, iters, &salt, password.as_bytes(), &hash).is_ok()
}

type CmdResult<T> = Result<T, String>;

fn now() -> String { Utc::now().to_rfc3339() }
fn uid() -> String { Uuid::new_v4().to_string() }
fn e(err: impl std::fmt::Display) -> String { err.to_string() }

/* ══════════════════════════════════════════════════════
   AUTH
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_register(db: State<'_, SocialDb>, input: RegisterInput) -> CmdResult<AuthSession> {
    let conn = db.conn.lock().map_err(e)?;
    let hash = pw_hash(&input.password)?;
    let id = uid();
    let ts = now();

    // Resolve role from team code
    let role = if let Some(ref code) = input.team_code {
        if code.trim().is_empty() {
            "user".to_string()
        } else {
            let trimmed = code.trim();
            let result: Result<(String, String, i32, i32), _> = conn.query_row(
                "SELECT id, role, max_uses, use_count FROM team_codes WHERE code = ?1 AND active = 1",
                params![trimmed],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            );
            match result {
                Ok((tc_id, tc_role, max_uses, use_count)) => {
                    if max_uses > 0 && use_count >= max_uses {
                        return Err("Team code has been fully redeemed".into());
                    }
                    conn.execute(
                        "UPDATE team_codes SET use_count = use_count + 1 WHERE id = ?1",
                        params![tc_id],
                    ).ok();
                    tc_role
                }
                Err(_) => return Err("Invalid team code".into()),
            }
        }
    } else {
        "user".to_string()
    };

    conn.execute(
        "INSERT INTO users (id, username, display_name, email, password_hash, role, created_at, updated_at, online_status, last_login)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'online',?9)",
        params![id, input.username, input.display_name, input.email, hash, role, ts, ts, ts],
    ).map_err(|err| {
        if err.to_string().contains("UNIQUE") {
            "Username or email already taken".to_string()
        } else {
            e(err)
        }
    })?;
    let token = B64.encode(format!("{}:{}", id, Uuid::new_v4()));
    Ok(AuthSession { user_id: id, username: input.username, token, role })
}

#[tauri::command]
pub fn social_login(db: State<'_, SocialDb>, input: LoginInput) -> CmdResult<AuthSession> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash FROM users WHERE username = ?1"
    ).map_err(e)?;
    let (user_id, username, hash): (String, String, String) = stmt.query_row(
        params![input.username],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| "Invalid username or password".to_string())?;

    if !pw_verify(&input.password, &hash) {
        return Err("Invalid username or password".into());
    }
    let ts = now();
    conn.execute("UPDATE users SET online_status='online', last_login=?1 WHERE id=?2", params![ts, user_id]).ok();
    let role: String = conn.query_row("SELECT role FROM users WHERE id=?1", params![user_id], |r| r.get(0)).unwrap_or_else(|_| "user".into());
    let token = B64.encode(format!("{}:{}", user_id, Uuid::new_v4()));
    Ok(AuthSession { user_id, username, token, role })
}

#[tauri::command]
pub fn social_logout(db: State<'_, SocialDb>, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("UPDATE users SET online_status='offline' WHERE id=?1", params![user_id]).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   PROFILE
   ══════════════════════════════════════════════════════ */

fn row_to_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        email: row.get(3)?,
        password_hash: String::new(),
        avatar_url: row.get(4)?,
        headline: row.get(5)?,
        about_me: row.get(6)?,
        who_id_like_to_meet: row.get(7)?,
        interests: row.get(8)?,
        music_interests: row.get(9)?,
        movie_interests: row.get(10)?,
        hero_song_path: row.get(11)?,
        hero_song_title: row.get(12)?,
        profile_css: row.get(13)?,
        profile_bg_url: row.get(14)?,
        mood: row.get(15)?,
        gender: row.get(16)?,
        age: row.get(17)?,
        location: row.get(18)?,
        orientation: row.get(19)?,
        status: row.get(20)?,
        body_type: row.get(21)?,
        ethnicity: row.get(22)?,
        zodiac_sign: row.get(23)?,
        smoke_drink: row.get(24)?,
        children: row.get(25)?,
        education: row.get(26)?,
        occupation: row.get(27)?,
        income: row.get(28)?,
        online_status: row.get(29)?,
        last_login: row.get(30)?,
        profile_views: row.get(31)?,
        role: row.get(32)?,
        created_at: row.get(33)?,
        updated_at: row.get(34)?,
    })
}

const USER_COLS: &str = "id,username,display_name,email,avatar_url,headline,about_me,who_id_like_to_meet,interests,music_interests,movie_interests,hero_song_path,hero_song_title,profile_css,profile_bg_url,mood,gender,age,location,orientation,status,body_type,ethnicity,zodiac_sign,smoke_drink,children,education,occupation,income,online_status,last_login,profile_views,role,created_at,updated_at";

#[tauri::command]
pub fn social_get_profile(db: State<'_, SocialDb>, user_id: String, viewer_id: Option<String>) -> CmdResult<User> {
    let conn = db.conn.lock().map_err(e)?;
    // bump views if someone else is viewing
    if let Some(ref vid) = viewer_id {
        if vid != &user_id {
            conn.execute("UPDATE users SET profile_views = profile_views + 1 WHERE id=?1", params![user_id]).ok();
        }
    }
    let sql = format!("SELECT {} FROM users WHERE id = ?1", USER_COLS);
    let user = conn.prepare(&sql).map_err(e)?
        .query_row(params![user_id], row_to_user).map_err(e)?;
    Ok(user)
}

#[tauri::command]
pub fn social_get_profile_by_username(db: State<'_, SocialDb>, username: String) -> CmdResult<User> {
    let conn = db.conn.lock().map_err(e)?;
    let sql = format!("SELECT {} FROM users WHERE username = ?1", USER_COLS);
    let user = conn.prepare(&sql).map_err(e)?
        .query_row(params![username], row_to_user).map_err(e)?;
    Ok(user)
}

#[tauri::command]
pub fn social_update_profile(db: State<'_, SocialDb>, user_id: String, input: UpdateProfileInput) -> CmdResult<User> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    // Build dynamic SET clauses
    let mut sets: Vec<String> = vec!["updated_at = ?1".into()];
    let mut idx = 2u32;
    macro_rules! maybe_set {
        ($field:ident, $col:expr) => {
            if input.$field.is_some() {
                sets.push(format!("{} = ?{}", $col, idx));
                idx += 1;
            }
        };
    }
    maybe_set!(display_name, "display_name");
    maybe_set!(avatar_url, "avatar_url");
    maybe_set!(headline, "headline");
    maybe_set!(about_me, "about_me");
    maybe_set!(who_id_like_to_meet, "who_id_like_to_meet");
    maybe_set!(interests, "interests");
    maybe_set!(music_interests, "music_interests");
    maybe_set!(movie_interests, "movie_interests");
    maybe_set!(hero_song_path, "hero_song_path");
    maybe_set!(hero_song_title, "hero_song_title");
    maybe_set!(profile_css, "profile_css");
    maybe_set!(profile_bg_url, "profile_bg_url");
    maybe_set!(mood, "mood");
    maybe_set!(gender, "gender");
    maybe_set!(age, "age");
    maybe_set!(location, "location");
    maybe_set!(orientation, "orientation");
    maybe_set!(status, "status");
    maybe_set!(body_type, "body_type");
    maybe_set!(ethnicity, "ethnicity");
    maybe_set!(zodiac_sign, "zodiac_sign");
    maybe_set!(smoke_drink, "smoke_drink");
    maybe_set!(children, "children");
    maybe_set!(education, "education");
    maybe_set!(occupation, "occupation");
    maybe_set!(income, "income");

    let sql = format!("UPDATE users SET {} WHERE id = ?{}", sets.join(", "), idx);
    let mut stmt = conn.prepare(&sql).map_err(e)?;

    // Bind values dynamically
    let mut bind_idx = 1;
    stmt.raw_bind_parameter(bind_idx, &ts).map_err(e)?; bind_idx += 1;
    macro_rules! maybe_bind {
        ($field:ident) => {
            if let Some(ref val) = input.$field {
                stmt.raw_bind_parameter(bind_idx, val).map_err(e)?;
                bind_idx += 1;
            }
        };
    }
    macro_rules! maybe_bind_i32 {
        ($field:ident) => {
            if let Some(val) = input.$field {
                stmt.raw_bind_parameter(bind_idx, val).map_err(e)?;
                bind_idx += 1;
            }
        };
    }
    maybe_bind!(display_name);
    maybe_bind!(avatar_url);
    maybe_bind!(headline);
    maybe_bind!(about_me);
    maybe_bind!(who_id_like_to_meet);
    maybe_bind!(interests);
    maybe_bind!(music_interests);
    maybe_bind!(movie_interests);
    maybe_bind!(hero_song_path);
    maybe_bind!(hero_song_title);
    maybe_bind!(profile_css);
    maybe_bind!(profile_bg_url);
    maybe_bind!(mood);
    maybe_bind!(gender);
    maybe_bind_i32!(age);
    maybe_bind!(location);
    maybe_bind!(orientation);
    maybe_bind!(status);
    maybe_bind!(body_type);
    maybe_bind!(ethnicity);
    maybe_bind!(zodiac_sign);
    maybe_bind!(smoke_drink);
    maybe_bind!(children);
    maybe_bind!(education);
    maybe_bind!(occupation);
    maybe_bind!(income);
    stmt.raw_bind_parameter(bind_idx, &user_id).map_err(e)?;
    stmt.raw_execute().map_err(e)?;
    drop(stmt);

    let sel = format!("SELECT {} FROM users WHERE id = ?1", USER_COLS);
    let user = conn.prepare(&sel).map_err(e)?
        .query_row(params![user_id], row_to_user).map_err(e)?;
    Ok(user)
}

/* ══════════════════════════════════════════════════════
   FRIENDS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_send_friend_request(db: State<'_, SocialDb>, from_user_id: String, to_user_id: String) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO friend_requests (id, from_user_id, to_user_id, status, created_at, updated_at) VALUES (?1,?2,?3,'pending',?4,?5)",
        params![id, from_user_id, to_user_id, ts, ts],
    ).map_err(|err| {
        if err.to_string().contains("UNIQUE") { "Friend request already sent".into() } else { e(err) }
    })?;
    Ok(id)
}

#[tauri::command]
pub fn social_respond_friend_request(db: State<'_, SocialDb>, request_id: String, accept: bool) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    let ts = now();
    let status = if accept { "accepted" } else { "rejected" };
    conn.execute(
        "UPDATE friend_requests SET status=?1, updated_at=?2 WHERE id=?3 AND status='pending'",
        params![status, ts, request_id],
    ).map_err(e)?;

    if accept {
        let (from_id, to_id): (String, String) = conn.prepare(
            "SELECT from_user_id, to_user_id FROM friend_requests WHERE id=?1"
        ).map_err(e)?.query_row(params![request_id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(e)?;
        let id1 = uid();
        let id2 = uid();
        conn.execute(
            "INSERT OR IGNORE INTO friendships (id, user_id, friend_id, created_at) VALUES (?1,?2,?3,?4)",
            params![id1, from_id, to_id, ts],
        ).map_err(e)?;
        conn.execute(
            "INSERT OR IGNORE INTO friendships (id, user_id, friend_id, created_at) VALUES (?1,?2,?3,?4)",
            params![id2, to_id, from_id, ts],
        ).map_err(e)?;
    }
    Ok(())
}

#[tauri::command]
pub fn social_get_friends(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Friend>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT f.id, u.id, u.username, u.display_name, u.avatar_url, u.headline, u.online_status,
                f.is_top_friend, f.top_friend_rank
         FROM friendships f JOIN users u ON u.id = f.friend_id
         WHERE f.user_id = ?1
         ORDER BY f.is_top_friend DESC, f.top_friend_rank ASC, f.created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Friend {
            id: row.get(0)?,
            user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            avatar_url: row.get(4)?,
            headline: row.get(5)?,
            online_status: row.get(6)?,
            is_top_friend: row.get::<_, i32>(7)? != 0,
            top_friend_rank: row.get(8)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_get_pending_requests(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<FriendRequest>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT fr.id, fr.from_user_id, u.username, u.display_name, u.avatar_url, fr.to_user_id, fr.status, fr.created_at
         FROM friend_requests fr JOIN users u ON u.id = fr.from_user_id
         WHERE fr.to_user_id = ?1 AND fr.status = 'pending'
         ORDER BY fr.created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(FriendRequest {
            id: row.get(0)?,
            from_user_id: row.get(1)?,
            from_username: row.get(2)?,
            from_display_name: row.get(3)?,
            from_avatar: row.get(4)?,
            to_user_id: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_set_top_friends(db: State<'_, SocialDb>, user_id: String, friend_ids: Vec<String>) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("UPDATE friendships SET is_top_friend=0, top_friend_rank=0 WHERE user_id=?1", params![user_id]).map_err(e)?;
    for (i, fid) in friend_ids.iter().enumerate() {
        conn.execute(
            "UPDATE friendships SET is_top_friend=1, top_friend_rank=?1 WHERE user_id=?2 AND friend_id=?3",
            params![i as i32 + 1, user_id, fid],
        ).map_err(e)?;
    }
    Ok(())
}

#[tauri::command]
pub fn social_remove_friend(db: State<'_, SocialDb>, user_id: String, friend_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM friendships WHERE user_id=?1 AND friend_id=?2", params![user_id, friend_id]).map_err(e)?;
    conn.execute("DELETE FROM friendships WHERE user_id=?1 AND friend_id=?2", params![friend_id, user_id]).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   MESSAGES
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_send_message(db: State<'_, SocialDb>, from_user_id: String, input: SendMessageInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO messages (id, from_user_id, to_user_id, subject, body, is_read, created_at)
         VALUES (?1,?2,?3,?4,?5,0,?6)",
        params![id, from_user_id, input.to_user_id, input.subject, input.body, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_inbox(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Message>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT m.id, m.from_user_id, u.username, u.display_name, u.avatar_url, m.to_user_id,
                m.subject, m.body, m.is_read, m.created_at
         FROM messages m JOIN users u ON u.id = m.from_user_id
         WHERE m.to_user_id = ?1
         ORDER BY m.created_at DESC
         LIMIT 200"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            from_user_id: row.get(1)?,
            from_username: row.get(2)?,
            from_display_name: row.get(3)?,
            from_avatar: row.get(4)?,
            to_user_id: row.get(5)?,
            subject: row.get(6)?,
            body: row.get(7)?,
            is_read: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_get_sent_messages(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Message>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT m.id, m.from_user_id, u.username, u.display_name, u.avatar_url, m.to_user_id,
                m.subject, m.body, m.is_read, m.created_at
         FROM messages m JOIN users u ON u.id = m.to_user_id
         WHERE m.from_user_id = ?1
         ORDER BY m.created_at DESC
         LIMIT 200"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            from_user_id: row.get(1)?,
            from_username: row.get(2)?,
            from_display_name: row.get(3)?,
            from_avatar: row.get(4)?,
            to_user_id: row.get(5)?,
            subject: row.get(6)?,
            body: row.get(7)?,
            is_read: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_mark_message_read(db: State<'_, SocialDb>, message_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("UPDATE messages SET is_read=1 WHERE id=?1", params![message_id]).map_err(e)?;
    Ok(())
}

#[tauri::command]
pub fn social_delete_message(db: State<'_, SocialDb>, message_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM messages WHERE id=?1", params![message_id]).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   BULLETINS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_post_bulletin(db: State<'_, SocialDb>, user_id: String, input: PostBulletinInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO bulletins (id, user_id, title, body, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![id, user_id, input.title, input.body, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_bulletins(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Bulletin>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT b.id, b.user_id, u.username, u.display_name, u.avatar_url, b.title, b.body, b.created_at
         FROM bulletins b JOIN users u ON u.id = b.user_id
         WHERE b.user_id IN (SELECT friend_id FROM friendships WHERE user_id=?1)
            OR b.user_id = ?1
         ORDER BY b.created_at DESC LIMIT 100"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Bulletin {
            id: row.get(0)?,
            user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            avatar_url: row.get(4)?,
            title: row.get(5)?,
            body: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

/* ══════════════════════════════════════════════════════
   PROFILE COMMENTS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_post_comment(db: State<'_, SocialDb>, author_user_id: String, input: PostCommentInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO profile_comments (id, profile_user_id, author_user_id, body, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![id, input.profile_user_id, author_user_id, input.body, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_comments(db: State<'_, SocialDb>, profile_user_id: String) -> CmdResult<Vec<ProfileComment>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.profile_user_id, c.author_user_id, u.username, u.display_name, u.avatar_url, c.body, c.created_at
         FROM profile_comments c JOIN users u ON u.id = c.author_user_id
         WHERE c.profile_user_id = ?1
         ORDER BY c.created_at DESC LIMIT 200"
    ).map_err(e)?;
    let rows = stmt.query_map(params![profile_user_id], |row| {
        Ok(ProfileComment {
            id: row.get(0)?,
            profile_user_id: row.get(1)?,
            author_user_id: row.get(2)?,
            author_username: row.get(3)?,
            author_display_name: row.get(4)?,
            author_avatar: row.get(5)?,
            body: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_delete_comment(db: State<'_, SocialDb>, comment_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute(
        "DELETE FROM profile_comments WHERE id=?1 AND (profile_user_id=?2 OR author_user_id=?2)",
        params![comment_id, user_id],
    ).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   BLOG
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_create_blog_post(db: State<'_, SocialDb>, user_id: String, input: CreateBlogPostInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO blog_posts (id, user_id, title, body, mood, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![id, user_id, input.title, input.body, input.mood.unwrap_or_default(), ts, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_blog_posts(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<BlogPost>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT bp.id, bp.user_id, u.username, u.display_name, u.avatar_url, bp.title, bp.body, bp.mood, bp.created_at, bp.updated_at
         FROM blog_posts bp JOIN users u ON u.id = bp.user_id
         WHERE bp.user_id = ?1
         ORDER BY bp.created_at DESC LIMIT 50"
    ).map_err(e)?;
    let mut posts: Vec<BlogPost> = stmt.query_map(params![user_id], |row| {
        Ok(BlogPost {
            id: row.get(0)?,
            user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            avatar_url: row.get(4)?,
            title: row.get(5)?,
            body: row.get(6)?,
            mood: row.get(7)?,
            comments: vec![],
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }).map_err(e)?.collect::<Result<Vec<_>, _>>().map_err(e)?;

    // Load comments for each blog post
    for post in &mut posts {
        let mut cstmt = conn.prepare(
            "SELECT bc.id, bc.author_user_id, u.username, u.display_name, u.avatar_url, bc.body, bc.created_at
             FROM blog_comments bc JOIN users u ON u.id = bc.author_user_id
             WHERE bc.blog_post_id = ?1 ORDER BY bc.created_at ASC"
        ).map_err(e)?;
        post.comments = cstmt.query_map(params![post.id], |row| {
            Ok(BlogComment {
                id: row.get(0)?,
                author_user_id: row.get(1)?,
                author_username: row.get(2)?,
                author_display_name: row.get(3)?,
                author_avatar: row.get(4)?,
                body: row.get(5)?,
                created_at: row.get(6)?,
            })
        }).map_err(e)?.collect::<Result<Vec<_>, _>>().map_err(e)?;
    }
    Ok(posts)
}

#[tauri::command]
pub fn social_post_blog_comment(db: State<'_, SocialDb>, author_user_id: String, input: PostBlogCommentInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO blog_comments (id, blog_post_id, author_user_id, body, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![id, input.blog_post_id, author_user_id, input.body, ts],
    ).map_err(e)?;
    Ok(id)
}

/* ══════════════════════════════════════════════════════
   PHOTOS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_add_photo(db: State<'_, SocialDb>, user_id: String, file_path: String, caption: String, album: Option<String>) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO photos (id, user_id, album_name, file_path, caption, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, user_id, album.unwrap_or_else(|| "Default".into()), file_path, caption, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_photos(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Photo>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, user_id, album_name, file_path, caption, is_default, created_at
         FROM photos WHERE user_id=?1 ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Photo {
            id: row.get(0)?,
            user_id: row.get(1)?,
            album_name: row.get(2)?,
            file_path: row.get(3)?,
            caption: row.get(4)?,
            is_default: row.get::<_,i32>(5)? != 0,
            created_at: row.get(6)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_delete_photo(db: State<'_, SocialDb>, photo_id: String, user_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("DELETE FROM photos WHERE id=?1 AND user_id=?2", params![photo_id, user_id]).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   MUSIC
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_add_music(db: State<'_, SocialDb>, user_id: String, title: String, artist: String, file_path: String) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO music_tracks (id, user_id, title, artist, file_path, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, user_id, title, artist, file_path, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_music(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<MusicTrack>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT id, user_id, title, artist, file_path, duration_secs, play_count, is_profile_song, created_at
         FROM music_tracks WHERE user_id=?1 ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(MusicTrack {
            id: row.get(0)?,
            user_id: row.get(1)?,
            title: row.get(2)?,
            artist: row.get(3)?,
            file_path: row.get(4)?,
            duration_secs: row.get(5)?,
            play_count: row.get(6)?,
            is_profile_song: row.get::<_,i32>(7)? != 0,
            created_at: row.get(8)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_set_profile_song(db: State<'_, SocialDb>, user_id: String, track_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    conn.execute("UPDATE music_tracks SET is_profile_song=0 WHERE user_id=?1", params![user_id]).map_err(e)?;
    conn.execute("UPDATE music_tracks SET is_profile_song=1 WHERE id=?1 AND user_id=?2", params![track_id, user_id]).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   STATUS UPDATES / FEED
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_post_status(db: State<'_, SocialDb>, user_id: String, input: PostStatusInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO status_updates (id, user_id, body, created_at) VALUES (?1,?2,?3,?4)",
        params![id, user_id, input.body, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_feed(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<StatusUpdate>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT s.id, s.user_id, u.username, u.display_name, u.avatar_url, s.body, s.created_at
         FROM status_updates s JOIN users u ON u.id = s.user_id
         WHERE s.user_id IN (SELECT friend_id FROM friendships WHERE user_id=?1) OR s.user_id=?1
         ORDER BY s.created_at DESC LIMIT 100"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(StatusUpdate {
            id: row.get(0)?,
            user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            avatar_url: row.get(4)?,
            body: row.get(5)?,
            created_at: row.get(6)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

/* ══════════════════════════════════════════════════════
   SEARCH
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_search_users(db: State<'_, SocialDb>, query: String) -> CmdResult<Vec<UserSearchResult>> {
    let conn = db.conn.lock().map_err(e)?;
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, avatar_url, headline, location, online_status
         FROM users
         WHERE username LIKE ?1 OR display_name LIKE ?1 OR location LIKE ?1 OR headline LIKE ?1
         LIMIT 50"
    ).map_err(e)?;
    let rows = stmt.query_map(params![pattern], |row| {
        Ok(UserSearchResult {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            avatar_url: row.get(3)?,
            headline: row.get(4)?,
            location: row.get(5)?,
            online_status: row.get(6)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

/* ══════════════════════════════════════════════════════
   KUDOS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_send_kudo(db: State<'_, SocialDb>, from_user_id: String, to_user_id: String, kind: String) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT OR IGNORE INTO kudos (id, from_user_id, to_user_id, kind, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![id, from_user_id, to_user_id, kind, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_kudos(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<Kudo>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT k.id, k.from_user_id, u.username, u.avatar_url, k.kind, k.created_at
         FROM kudos k JOIN users u ON u.id = k.from_user_id
         WHERE k.to_user_id = ?1
         ORDER BY k.created_at DESC LIMIT 100"
    ).map_err(e)?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(Kudo {
            id: row.get(0)?,
            from_user_id: row.get(1)?,
            from_username: row.get(2)?,
            from_avatar: row.get(3)?,
            kind: row.get(4)?,
            created_at: row.get(5)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

/* ══════════════════════════════════════════════════════
   GROUPS
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_create_group(db: State<'_, SocialDb>, user_id: String, input: CreateGroupInput) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT INTO groups (id, name, description, owner_user_id, category, created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, input.name, input.description, user_id, input.category.unwrap_or_else(|| "general".into()), ts],
    ).map_err(e)?;
    // Owner auto-joins as admin
    let mid = uid();
    conn.execute(
        "INSERT INTO group_members (id, group_id, user_id, role, joined_at) VALUES (?1,?2,?3,'admin',?4)",
        params![mid, id, user_id, ts],
    ).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub fn social_get_groups(db: State<'_, SocialDb>) -> CmdResult<Vec<Group>> {
    let conn = db.conn.lock().map_err(e)?;
    let mut stmt = conn.prepare(
        "SELECT g.id, g.name, g.description, g.owner_user_id, g.category, g.avatar_url,
                (SELECT COUNT(*) FROM group_members gm WHERE gm.group_id=g.id) as member_count,
                g.created_at
         FROM groups g ORDER BY member_count DESC LIMIT 100"
    ).map_err(e)?;
    let rows = stmt.query_map([], |row| {
        Ok(Group {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            owner_user_id: row.get(3)?,
            category: row.get(4)?,
            avatar_url: row.get(5)?,
            member_count: row.get(6)?,
            created_at: row.get(7)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_join_group(db: State<'_, SocialDb>, user_id: String, group_id: String) -> CmdResult<()> {
    let conn = db.conn.lock().map_err(e)?;
    let id = uid();
    let ts = now();
    conn.execute(
        "INSERT OR IGNORE INTO group_members (id, group_id, user_id, role, joined_at) VALUES (?1,?2,?3,'member',?4)",
        params![id, group_id, user_id, ts],
    ).map_err(e)?;
    Ok(())
}

/* ══════════════════════════════════════════════════════
   STATS / COUNTS
   ══════════════════════════════════════════════════════ */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SocialStats {
    pub friend_count: i32,
    pub pending_requests: i32,
    pub unread_messages: i32,
    pub photo_count: i32,
    pub music_count: i32,
    pub profile_views: i32,
    pub bulletin_count: i32,
    pub blog_count: i32,
    pub kudo_count: i32,
}

#[tauri::command]
pub fn social_get_stats(db: State<'_, SocialDb>, user_id: String) -> CmdResult<SocialStats> {
    let conn = db.conn.lock().map_err(e)?;
    let q = |sql: &str| -> i32 {
        conn.query_row(sql, params![user_id], |r| r.get(0)).unwrap_or(0)
    };
    Ok(SocialStats {
        friend_count: q("SELECT COUNT(*) FROM friendships WHERE user_id=?1"),
        pending_requests: q("SELECT COUNT(*) FROM friend_requests WHERE to_user_id=?1 AND status='pending'"),
        unread_messages: q("SELECT COUNT(*) FROM messages WHERE to_user_id=?1 AND is_read=0"),
        photo_count: q("SELECT COUNT(*) FROM photos WHERE user_id=?1"),
        music_count: q("SELECT COUNT(*) FROM music_tracks WHERE user_id=?1"),
        profile_views: q("SELECT profile_views FROM users WHERE id=?1"),
        bulletin_count: q("SELECT COUNT(*) FROM bulletins WHERE user_id=?1"),
        blog_count: q("SELECT COUNT(*) FROM blog_posts WHERE user_id=?1"),
        kudo_count: q("SELECT COUNT(*) FROM kudos WHERE to_user_id=?1"),
    })
}

/* ══════════════════════════════════════════════════════
   BROWSE — recently active users (like MySpace browse)
   ══════════════════════════════════════════════════════ */

#[tauri::command]
pub fn social_browse_users(db: State<'_, SocialDb>, offset: Option<i32>) -> CmdResult<Vec<UserSearchResult>> {
    let conn = db.conn.lock().map_err(e)?;
    let off = offset.unwrap_or(0);
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, avatar_url, headline, location, online_status
         FROM users
         ORDER BY last_login DESC
         LIMIT 40 OFFSET ?1"
    ).map_err(e)?;
    let rows = stmt.query_map(params![off], |row| {
        Ok(UserSearchResult {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            avatar_url: row.get(3)?,
            headline: row.get(4)?,
            location: row.get(5)?,
            online_status: row.get(6)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

/* ══════════════════════════════════════════════════════
   TEAM CODES — management
   ══════════════════════════════════════════════════════ */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TeamCode {
    pub id: String,
    pub code: String,
    pub label: String,
    pub role: String,
    pub max_uses: i32,
    pub use_count: i32,
    pub active: bool,
    pub created_at: String,
}

#[tauri::command]
pub fn social_get_team_codes(db: State<'_, SocialDb>, user_id: String) -> CmdResult<Vec<TeamCode>> {
    let conn = db.conn.lock().map_err(e)?;
    let role: String = conn.query_row("SELECT role FROM users WHERE id=?1", params![user_id], |r| r.get(0))
        .map_err(|_| "User not found".to_string())?;
    if role != "admin" && role != "team" {
        return Err("Unauthorized: team or admin role required".into());
    }
    let mut stmt = conn.prepare(
        "SELECT id, code, label, role, max_uses, use_count, active, created_at FROM team_codes ORDER BY created_at DESC"
    ).map_err(e)?;
    let rows = stmt.query_map([], |row| {
        Ok(TeamCode {
            id: row.get(0)?,
            code: row.get(1)?,
            label: row.get(2)?,
            role: row.get(3)?,
            max_uses: row.get(4)?,
            use_count: row.get(5)?,
            active: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
        })
    }).map_err(e)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(e)
}

#[tauri::command]
pub fn social_create_team_code(db: State<'_, SocialDb>, user_id: String, code: String, label: String, role: String, max_uses: Option<i32>) -> CmdResult<TeamCode> {
    let conn = db.conn.lock().map_err(e)?;
    let user_role: String = conn.query_row("SELECT role FROM users WHERE id=?1", params![user_id], |r| r.get(0))
        .map_err(|_| "User not found".to_string())?;
    if user_role != "admin" {
        return Err("Unauthorized: admin role required".into());
    }
    let id = uid();
    let ts = now();
    let mu = max_uses.unwrap_or(0);
    conn.execute(
        "INSERT INTO team_codes (id, code, label, role, max_uses, use_count, created_by, created_at, active) VALUES (?1,?2,?3,?4,?5,0,?6,?7,1)",
        params![id, code, label, role, mu, user_id, ts],
    ).map_err(|err| {
        if err.to_string().contains("UNIQUE") { "Code already exists".into() } else { e(err) }
    })?;
    Ok(TeamCode { id, code, label, role, max_uses: mu, use_count: 0, active: true, created_at: ts })
}

#[tauri::command]
pub fn social_validate_team_code(db: State<'_, SocialDb>, code: String) -> CmdResult<String> {
    let conn = db.conn.lock().map_err(e)?;
    let result: Result<(String, String, i32, i32), _> = conn.query_row(
        "SELECT label, role, max_uses, use_count FROM team_codes WHERE code = ?1 AND active = 1",
        params![code.trim()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    );
    match result {
        Ok((label, role, max_uses, use_count)) => {
            if max_uses > 0 && use_count >= max_uses {
                Err("This code has been fully redeemed".into())
            } else {
                Ok(format!("{} ({})", label, role))
            }
        }
        Err(_) => Err("Invalid team code".into()),
    }
}
