use rusqlite::{Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct SocialDb {
    pub conn: Mutex<Connection>,
}

impl SocialDb {
    pub fn new(app_dir: PathBuf) -> SqlResult<Self> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("x3_social.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Mutex::new(conn) };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS users (
                id              TEXT PRIMARY KEY,
                username        TEXT NOT NULL UNIQUE,
                display_name    TEXT NOT NULL,
                email           TEXT NOT NULL UNIQUE,
                password_hash   TEXT NOT NULL,
                avatar_url      TEXT DEFAULT '',
                headline        TEXT DEFAULT '',
                about_me        TEXT DEFAULT '',
                who_id_like_to_meet TEXT DEFAULT '',
                interests       TEXT DEFAULT '',
                music_interests TEXT DEFAULT '',
                movie_interests TEXT DEFAULT '',
                hero_song_path  TEXT DEFAULT '',
                hero_song_title TEXT DEFAULT '',
                profile_css     TEXT DEFAULT '',
                profile_bg_url  TEXT DEFAULT '',
                mood            TEXT DEFAULT 'happy',
                gender          TEXT DEFAULT '',
                age             INTEGER DEFAULT 0,
                location        TEXT DEFAULT '',
                orientation     TEXT DEFAULT '',
                status          TEXT DEFAULT 'single',
                body_type       TEXT DEFAULT '',
                ethnicity       TEXT DEFAULT '',
                zodiac_sign     TEXT DEFAULT '',
                smoke_drink     TEXT DEFAULT '',
                children        TEXT DEFAULT '',
                education       TEXT DEFAULT '',
                occupation      TEXT DEFAULT '',
                income          TEXT DEFAULT '',
                online_status   TEXT DEFAULT 'offline',
                last_login      TEXT DEFAULT '',
                profile_views   INTEGER DEFAULT 0,
                role            TEXT NOT NULL DEFAULT 'user',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS friend_requests (
                id              TEXT PRIMARY KEY,
                from_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                to_user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                status          TEXT NOT NULL DEFAULT 'pending',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL,
                UNIQUE(from_user_id, to_user_id)
            );

            CREATE TABLE IF NOT EXISTS friendships (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                friend_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                is_top_friend   INTEGER DEFAULT 0,
                top_friend_rank INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL,
                UNIQUE(user_id, friend_id)
            );

            CREATE TABLE IF NOT EXISTS profile_comments (
                id              TEXT PRIMARY KEY,
                profile_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                author_user_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                body            TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                from_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                to_user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                subject         TEXT NOT NULL DEFAULT '',
                body            TEXT NOT NULL,
                is_read         INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS bulletins (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title           TEXT NOT NULL,
                body            TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS blog_posts (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title           TEXT NOT NULL,
                body            TEXT NOT NULL,
                mood            TEXT DEFAULT '',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS blog_comments (
                id              TEXT PRIMARY KEY,
                blog_post_id    TEXT NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
                author_user_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                body            TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS photos (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                album_name      TEXT DEFAULT 'Default',
                file_path       TEXT NOT NULL,
                caption         TEXT DEFAULT '',
                is_default      INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS music_tracks (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title           TEXT NOT NULL,
                artist          TEXT DEFAULT '',
                file_path       TEXT NOT NULL,
                duration_secs   INTEGER DEFAULT 0,
                play_count      INTEGER DEFAULT 0,
                is_profile_song INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS status_updates (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                body            TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS kudos (
                id              TEXT PRIMARY KEY,
                from_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                to_user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                kind            TEXT NOT NULL DEFAULT 'cool',
                created_at      TEXT NOT NULL,
                UNIQUE(from_user_id, to_user_id, kind)
            );

            CREATE TABLE IF NOT EXISTS groups (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                description     TEXT DEFAULT '',
                owner_user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                category        TEXT DEFAULT 'general',
                avatar_url      TEXT DEFAULT '',
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS group_members (
                id              TEXT PRIMARY KEY,
                group_id        TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                role            TEXT DEFAULT 'member',
                joined_at       TEXT NOT NULL,
                UNIQUE(group_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS team_codes (
                id              TEXT PRIMARY KEY,
                code            TEXT NOT NULL UNIQUE,
                label           TEXT NOT NULL DEFAULT 'Team Member',
                role            TEXT NOT NULL DEFAULT 'team',
                max_uses        INTEGER DEFAULT 0,
                use_count       INTEGER DEFAULT 0,
                created_by      TEXT DEFAULT '',
                created_at      TEXT NOT NULL,
                active          INTEGER DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_team_codes_code ON team_codes(code);
            CREATE INDEX IF NOT EXISTS idx_friend_req_to ON friend_requests(to_user_id);
            CREATE INDEX IF NOT EXISTS idx_friend_req_from ON friend_requests(from_user_id);
            CREATE INDEX IF NOT EXISTS idx_friendships_user ON friendships(user_id);
            CREATE INDEX IF NOT EXISTS idx_friendships_friend ON friendships(friend_id);
            CREATE INDEX IF NOT EXISTS idx_comments_profile ON profile_comments(profile_user_id);
            CREATE INDEX IF NOT EXISTS idx_messages_to ON messages(to_user_id);
            CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_user_id);
            CREATE INDEX IF NOT EXISTS idx_bulletins_user ON bulletins(user_id);
            CREATE INDEX IF NOT EXISTS idx_status_user ON status_updates(user_id);
            CREATE INDEX IF NOT EXISTS idx_photos_user ON photos(user_id);
            CREATE INDEX IF NOT EXISTS idx_music_user ON music_tracks(user_id);
            CREATE INDEX IF NOT EXISTS idx_blog_user ON blog_posts(user_id);
        ")?;

        /* ── Seed default team codes if table is empty ── */
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM team_codes", [], |r| r.get(0)
        ).unwrap_or(0);
        if count == 0 {
            let ts = chrono::Utc::now().to_rfc3339();
            conn.execute_batch(&format!("
                INSERT OR IGNORE INTO team_codes (id, code, label, role, max_uses, use_count, created_by, created_at, active)
                VALUES
                    ('tc-001', 'X3-TEAM-2026',   'Core Team',     'team',  0, 0, 'system', '{ts}', 1),
                    ('tc-002', 'X3-ADMIN-KEY',    'Admin',         'admin', 0, 0, 'system', '{ts}', 1),
                    ('tc-003', 'X3-DEV-ACCESS',   'Developer',     'team',  0, 0, 'system', '{ts}', 1),
                    ('tc-004', 'X3-VIP-INVITE',   'VIP',           'vip',   50, 0, 'system', '{ts}', 1);
            ")).ok();
        }

        /* ── Migration: add role column if missing ── */
        let has_role: bool = conn.prepare("SELECT role FROM users LIMIT 0")
            .is_ok();
        if !has_role {
            conn.execute("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'", []).ok();
        }

        Ok(())
    }
}
