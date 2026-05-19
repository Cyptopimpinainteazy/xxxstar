-- Chain connectors (onboarding)
CREATE TABLE IF NOT EXISTS chain_connectors (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id         TEXT NOT NULL,
    rpc_url          TEXT NOT NULL,
    auth_type        TEXT NOT NULL DEFAULT 'none',
    credential_mask  TEXT,
    credential_enc   TEXT,
    notes            TEXT,
    status           TEXT NOT NULL DEFAULT 'active',
    created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_connectors_chain ON chain_connectors(chain_id);
