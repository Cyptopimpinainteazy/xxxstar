-- Migration 003: Airdrops, Faucets, Wallets, Chain Discoveries
-- Apply with: sqlite3 infra-structure/db/chains.db < infra-structure/db/migrations/003_airdrops_faucets_wallets.sql

CREATE TABLE IF NOT EXISTS wallets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,
    address         TEXT NOT NULL,
    label           TEXT DEFAULT 'auto',
    private_key_enc TEXT,
    ecosystem       TEXT NOT NULL DEFAULT 'evm',
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    balance         TEXT DEFAULT '0',
    balance_usd     REAL DEFAULT 0,
    last_balance_check DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, address)
);

CREATE TABLE IF NOT EXISTS airdrops (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    project         TEXT,
    token_symbol    TEXT,
    token_address   TEXT,
    airdrop_type    TEXT NOT NULL DEFAULT 'unknown',
    status          TEXT NOT NULL DEFAULT 'discovered',
    source          TEXT,
    source_url      TEXT,
    claim_url       TEXT,
    claim_start     DATETIME,
    claim_deadline  DATETIME,
    snapshot_date   DATETIME,
    estimated_value REAL DEFAULT 0,
    actual_value    REAL DEFAULT 0,
    eligibility_criteria TEXT,
    notes           TEXT,
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS airdrop_claims (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    airdrop_id      INTEGER NOT NULL REFERENCES airdrops(id) ON DELETE CASCADE,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    tx_hash         TEXT,
    amount          TEXT,
    amount_usd      REAL DEFAULT 0,
    gas_cost        TEXT,
    claimed_at      DATETIME,
    confirmed_at    DATETIME,
    error_message   TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS faucets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    provider        TEXT,
    url             TEXT NOT NULL,
    faucet_type     TEXT NOT NULL DEFAULT 'web',
    token_symbol    TEXT,
    amount_per_claim TEXT,
    cooldown_hours  REAL DEFAULT 24,
    requires_auth   BOOLEAN NOT NULL DEFAULT 0,
    auth_type       TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    last_checked    DATETIME,
    last_claimed    DATETIME,
    total_claims    INTEGER NOT NULL DEFAULT 0,
    total_received  TEXT DEFAULT '0',
    source          TEXT,
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, url)
);

CREATE TABLE IF NOT EXISTS faucet_claims (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    faucet_id       INTEGER NOT NULL REFERENCES faucets(id) ON DELETE CASCADE,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    tx_hash         TEXT,
    amount          TEXT,
    claimed_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    received_at     DATETIME,
    next_claim_at   DATETIME,
    error_message   TEXT
);

CREATE TABLE IF NOT EXISTS chain_discoveries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT,
    chain_name      TEXT NOT NULL,
    chain_numeric_id INTEGER,
    ecosystem       TEXT DEFAULT 'unknown',
    chain_type      TEXT DEFAULT 'unknown',
    is_testnet      BOOLEAN NOT NULL DEFAULT 0,
    source          TEXT NOT NULL,
    source_url      TEXT,
    rpc_url         TEXT,
    native_token    TEXT,
    status          TEXT NOT NULL DEFAULT 'new',
    added_to_chains BOOLEAN NOT NULL DEFAULT 0,
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at    DATETIME
);

CREATE INDEX IF NOT EXISTS idx_wallets_chain ON wallets(chain_id);
CREATE INDEX IF NOT EXISTS idx_wallets_active ON wallets(is_active);
CREATE INDEX IF NOT EXISTS idx_airdrops_chain ON airdrops(chain_id);
CREATE INDEX IF NOT EXISTS idx_airdrops_status ON airdrops(status);
CREATE INDEX IF NOT EXISTS idx_airdrops_deadline ON airdrops(claim_deadline);
CREATE INDEX IF NOT EXISTS idx_airdrops_type ON airdrops(airdrop_type);
CREATE INDEX IF NOT EXISTS idx_airdrop_claims_airdrop ON airdrop_claims(airdrop_id);
CREATE INDEX IF NOT EXISTS idx_airdrop_claims_wallet ON airdrop_claims(wallet_id);
CREATE INDEX IF NOT EXISTS idx_faucets_chain ON faucets(chain_id);
CREATE INDEX IF NOT EXISTS idx_faucets_status ON faucets(status);
CREATE INDEX IF NOT EXISTS idx_faucet_claims_faucet ON faucet_claims(faucet_id);
CREATE INDEX IF NOT EXISTS idx_faucet_claims_wallet ON faucet_claims(wallet_id);
CREATE INDEX IF NOT EXISTS idx_discoveries_status ON chain_discoveries(status);
CREATE INDEX IF NOT EXISTS idx_discoveries_chain ON chain_discoveries(chain_id);
