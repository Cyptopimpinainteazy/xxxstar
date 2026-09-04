use rusqlite::{Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct CrmDb {
    pub conn: Mutex<Connection>,
}

impl CrmDb {
    pub fn new(app_dir: PathBuf) -> SqlResult<Self> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("x3_crm.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Mutex::new(conn) };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            /* ── Contacts ── */
            CREATE TABLE IF NOT EXISTS crm_contacts (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                first_name      TEXT NOT NULL,
                last_name       TEXT DEFAULT '',
                email           TEXT DEFAULT '',
                phone           TEXT DEFAULT '',
                company         TEXT DEFAULT '',
                job_title       TEXT DEFAULT '',
                avatar_url      TEXT DEFAULT '',
                address         TEXT DEFAULT '',
                city            TEXT DEFAULT '',
                state           TEXT DEFAULT '',
                zip             TEXT DEFAULT '',
                country         TEXT DEFAULT '',
                website         TEXT DEFAULT '',
                notes           TEXT DEFAULT '',
                tags            TEXT DEFAULT '',
                source          TEXT DEFAULT 'manual',
                stage           TEXT DEFAULT 'lead',
                priority        TEXT DEFAULT 'medium',
                last_contacted  TEXT DEFAULT '',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── Calendar Events ── */
            CREATE TABLE IF NOT EXISTS crm_events (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                title           TEXT NOT NULL,
                description     TEXT DEFAULT '',
                location        TEXT DEFAULT '',
                event_type      TEXT DEFAULT 'meeting',
                start_at        TEXT NOT NULL,
                end_at          TEXT NOT NULL,
                all_day         INTEGER DEFAULT 0,
                color           TEXT DEFAULT '#ff6b35',
                recurrence      TEXT DEFAULT '',
                reminder_mins   INTEGER DEFAULT 15,
                contact_id      TEXT DEFAULT '',
                deal_id         TEXT DEFAULT '',
                completed       INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── Deals / Pipeline ── */
            CREATE TABLE IF NOT EXISTS crm_deals (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                contact_id      TEXT DEFAULT '',
                title           TEXT NOT NULL,
                value           REAL DEFAULT 0.0,
                currency        TEXT DEFAULT 'USD',
                stage           TEXT DEFAULT 'prospect',
                probability     INTEGER DEFAULT 10,
                expected_close  TEXT DEFAULT '',
                notes           TEXT DEFAULT '',
                won             INTEGER DEFAULT 0,
                lost            INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── Activity Log ── */
            CREATE TABLE IF NOT EXISTS crm_activities (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                contact_id      TEXT DEFAULT '',
                deal_id         TEXT DEFAULT '',
                event_id        TEXT DEFAULT '',
                activity_type   TEXT NOT NULL DEFAULT 'note',
                subject         TEXT DEFAULT '',
                body            TEXT DEFAULT '',
                created_at      TEXT NOT NULL
            );

            /* ── Email Templates ── */
            CREATE TABLE IF NOT EXISTS crm_email_templates (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                name            TEXT NOT NULL,
                subject         TEXT NOT NULL DEFAULT '',
                body            TEXT NOT NULL DEFAULT '',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── SMTP Config ── */
            CREATE TABLE IF NOT EXISTS crm_smtp_config (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL UNIQUE,
                host            TEXT NOT NULL DEFAULT '',
                port            INTEGER DEFAULT 587,
                username        TEXT DEFAULT '',
                password        TEXT DEFAULT '',
                from_name       TEXT DEFAULT '',
                from_email      TEXT DEFAULT '',
                use_tls         INTEGER DEFAULT 1,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── Sent Emails Log ── */
            CREATE TABLE IF NOT EXISTS crm_sent_emails (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                contact_id      TEXT DEFAULT '',
                to_email        TEXT NOT NULL,
                subject         TEXT NOT NULL,
                body            TEXT NOT NULL,
                status          TEXT DEFAULT 'sent',
                error_message   TEXT DEFAULT '',
                template_id     TEXT DEFAULT '',
                created_at      TEXT NOT NULL
            );

            /* ── Campaigns ── */
            CREATE TABLE IF NOT EXISTS crm_campaigns (
                id              TEXT PRIMARY KEY,
                owner_user_id   TEXT NOT NULL,
                name            TEXT NOT NULL,
                description     TEXT DEFAULT '',
                campaign_type   TEXT DEFAULT 'email',
                status          TEXT DEFAULT 'draft',
                target_contacts INTEGER DEFAULT 0,
                sent_count      INTEGER DEFAULT 0,
                opened_count    INTEGER DEFAULT 0,
                clicked_count   INTEGER DEFAULT 0,
                conversion_count INTEGER DEFAULT 0,
                scheduled_at    TEXT DEFAULT '',
                started_at      TEXT DEFAULT '',
                completed_at    TEXT DEFAULT '',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_crm_campaigns_owner ON crm_campaigns(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_campaigns_status ON crm_campaigns(status);

            CREATE INDEX IF NOT EXISTS idx_crm_contacts_owner ON crm_contacts(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_events_owner ON crm_events(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_events_start ON crm_events(start_at);
            CREATE INDEX IF NOT EXISTS idx_crm_deals_owner ON crm_deals(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_activities_owner ON crm_activities(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_activities_contact ON crm_activities(contact_id);
            CREATE INDEX IF NOT EXISTS idx_crm_sent_emails_owner ON crm_sent_emails(owner_user_id);

            /* ── Agent Tasks ── */
            CREATE TABLE IF NOT EXISTS crm_agent_tasks (
                id                  TEXT PRIMARY KEY,
                agent_id            TEXT NOT NULL,
                owner_user_id       TEXT NOT NULL,
                assigned_to_user_id TEXT NOT NULL,
                task_type           TEXT NOT NULL DEFAULT '',
                prompt              TEXT NOT NULL DEFAULT '',
                result              TEXT DEFAULT '',
                status              TEXT DEFAULT 'pending',
                leads_generated     INTEGER DEFAULT 0,
                created_at          TEXT NOT NULL,
                completed_at        TEXT DEFAULT ''
            );

            /* ── Agent Conversations ── */
            CREATE TABLE IF NOT EXISTS crm_agent_conversations (
                id          TEXT PRIMARY KEY,
                agent_id    TEXT NOT NULL,
                user_id     TEXT NOT NULL,
                role        TEXT NOT NULL DEFAULT 'user',
                content     TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL
            );

            /* ── Lead Funnel (shared pipeline — King sees all) ── */
            CREATE TABLE IF NOT EXISTS crm_lead_funnel (
                id              TEXT PRIMARY KEY,
                contact_id      TEXT NOT NULL,
                owner_user_id   TEXT NOT NULL,
                funnel_stage    TEXT DEFAULT 'discovered',
                agent_id        TEXT DEFAULT '',
                score           INTEGER DEFAULT 50,
                notes           TEXT DEFAULT '',
                shared_with_king INTEGER DEFAULT 1,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL
            );

            /* ── User Email Assignments (x3star.net) ── */
            CREATE TABLE IF NOT EXISTS crm_user_emails (
                id              TEXT PRIMARY KEY,
                user_id         TEXT NOT NULL,
                email_address   TEXT NOT NULL UNIQUE,
                smtp_username   TEXT DEFAULT '',
                created_at      TEXT NOT NULL,
                active          INTEGER DEFAULT 1
            );

            /* ── Users ── */
            CREATE TABLE IF NOT EXISTS crm_users (
                id          TEXT PRIMARY KEY,
                username    TEXT NOT NULL UNIQUE,
                role        TEXT NOT NULL DEFAULT 'user',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            /* ── User Proxy Assignments ── */
            CREATE TABLE IF NOT EXISTS crm_user_proxies (
                id          TEXT PRIMARY KEY,
                user_id     TEXT NOT NULL,
                proxy_host  TEXT NOT NULL,
                proxy_port  INTEGER DEFAULT 0,
                proxy_type  TEXT DEFAULT 'socks5',
                username    TEXT DEFAULT '',
                password    TEXT DEFAULT '',
                active      INTEGER DEFAULT 1,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_crm_agent_tasks_owner ON crm_agent_tasks(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_agent_convos ON crm_agent_conversations(agent_id, user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_lead_funnel_owner ON crm_lead_funnel(owner_user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_lead_funnel_stage ON crm_lead_funnel(funnel_stage);
            CREATE INDEX IF NOT EXISTS idx_crm_user_emails_user ON crm_user_emails(user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_user_proxies_user ON crm_user_proxies(user_id);
            CREATE INDEX IF NOT EXISTS idx_crm_users_username ON crm_users(username);

            /* ── v2: Contact sorting fields ── */
            /* SQLite ALTER TABLE ADD COLUMN is idempotent-safe with IF NOT EXISTS absent,
               so we use a helper table approach instead */
        ")?;

        // Add network, ranking columns to crm_contacts (safe: ignore if already exists)
        let _ = conn.execute_batch("ALTER TABLE crm_contacts ADD COLUMN network TEXT DEFAULT '';");
        let _ = conn.execute_batch("ALTER TABLE crm_contacts ADD COLUMN ranking INTEGER DEFAULT 0;");

        // Index for sorting
        let _ = conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_crm_contacts_network ON crm_contacts(network);");
        let _ = conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_crm_contacts_ranking ON crm_contacts(ranking);");
        let _ = conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_crm_contacts_country ON crm_contacts(country);");

        /* ── v3: RAG document cache ── */
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS crm_rag_docs (
                id          TEXT PRIMARY KEY,
                file_path   TEXT NOT NULL UNIQUE,
                content     TEXT NOT NULL,
                token_count INTEGER DEFAULT 0,
                indexed_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_crm_rag_docs_path ON crm_rag_docs(file_path);

            /* ── Media library ── */
            CREATE TABLE IF NOT EXISTS crm_media (
                id          TEXT PRIMARY KEY,
                file_name   TEXT NOT NULL,
                file_path   TEXT NOT NULL,
                file_type   TEXT DEFAULT '',
                file_size   INTEGER DEFAULT 0,
                tags        TEXT DEFAULT '',
                created_at  TEXT NOT NULL
            );

            /* ── v4: 90-Day Rollout Phases ── */
            CREATE TABLE IF NOT EXISTS crm_rollout_phases (
                id          TEXT PRIMARY KEY,
                phase_num   INTEGER NOT NULL,
                title       TEXT NOT NULL,
                description TEXT DEFAULT '',
                start_day   INTEGER NOT NULL,
                end_day     INTEGER NOT NULL,
                status      TEXT DEFAULT 'pending',
                milestones  TEXT DEFAULT '[]',
                progress    INTEGER DEFAULT 0,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_crm_rollout_status ON crm_rollout_phases(status);

            /* ── v4: Generated Pages (homepage builder) ── */
            CREATE TABLE IF NOT EXISTS crm_generated_pages (
                id          TEXT PRIMARY KEY,
                slug        TEXT NOT NULL UNIQUE,
                title       TEXT NOT NULL,
                page_type   TEXT DEFAULT 'landing',
                html_content TEXT DEFAULT '',
                meta_title  TEXT DEFAULT '',
                meta_desc   TEXT DEFAULT '',
                og_image    TEXT DEFAULT '',
                seo_keywords TEXT DEFAULT '',
                status      TEXT DEFAULT 'draft',
                agent_id    TEXT DEFAULT '',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_crm_pages_slug ON crm_generated_pages(slug);
            CREATE INDEX IF NOT EXISTS idx_crm_pages_status ON crm_generated_pages(status);
        ")?;

        Ok(())
    }
}
