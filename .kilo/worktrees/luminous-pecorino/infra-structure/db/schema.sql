-- X3 Chain Infrastructure — Chain Database Schema
-- Supports 60,000+ blockchains with full metadata, RPC endpoints, and status tracking

CREATE TABLE IF NOT EXISTS chains (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT UNIQUE NOT NULL,              -- short slug: "eth", "sol", "arb-one"
    chain_name      TEXT NOT NULL,                     -- human name: "Ethereum Mainnet"
    chain_numeric_id INTEGER,                          -- EIP-155 chain ID (1 for ETH, 137 for Polygon, etc.)
    ecosystem       TEXT NOT NULL DEFAULT 'evm',       -- evm | svm | cosmos | substrate | move | other
    chain_type      TEXT NOT NULL DEFAULT 'L1',        -- L1 | L2 | L3 | sidechain | appchain | testnet | devnet
    consensus       TEXT DEFAULT 'unknown',            -- pow | pos | poa | dpos | pbft | tendermint | nakamoto | unknown
    native_token    TEXT,                              -- symbol: ETH, SOL, ATOM
    is_evm          BOOLEAN NOT NULL DEFAULT 0,
    is_svm          BOOLEAN NOT NULL DEFAULT 0,
    is_testnet      BOOLEAN NOT NULL DEFAULT 0,
    supports_gpu    BOOLEAN NOT NULL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'active',    -- active | deprecated | inactive | unknown
    logo_url        TEXT,
    website_url     TEXT,
    explorer_url    TEXT,
    docs_url        TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS rpc_endpoints (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL REFERENCES chains(chain_id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    protocol        TEXT NOT NULL DEFAULT 'https',     -- https | wss | http | ws
    provider        TEXT,                              -- infura | alchemy | drpc | public | custom
    tier            TEXT NOT NULL DEFAULT 'public',    -- public | authenticated | premium
    is_primary      BOOLEAN NOT NULL DEFAULT 0,
    is_healthy      BOOLEAN NOT NULL DEFAULT 1,
    latency_ms      INTEGER,
    last_checked    DATETIME,
    max_batch_size  INTEGER DEFAULT 100,
    rate_limit_rps  INTEGER,
    requests_minute INTEGER NOT NULL DEFAULT 0,   -- rolling count this minute
    requests_total  BIGINT NOT NULL DEFAULT 0,    -- lifetime request count
    minute_reset_at INTEGER NOT NULL DEFAULT 0,   -- epoch second when minute counter resets
    avg_latency_ms  REAL,                         -- exponential moving average latency
    p99_latency_ms  REAL,                         -- 99th percentile latency
    error_count     INTEGER NOT NULL DEFAULT 0,   -- consecutive errors (resets on success)
    last_error      TEXT,                         -- last error message
    last_success_at DATETIME,                     -- timestamp of last successful request
    weight          REAL NOT NULL DEFAULT 1.0,    -- rotation weight (higher = preferred)
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Chain connectors registered via onboarding (credentials are masked + encrypted)
CREATE TABLE IF NOT EXISTS chain_connectors (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id         TEXT NOT NULL,
    rpc_url          TEXT NOT NULL,
    auth_type        TEXT NOT NULL DEFAULT 'none', -- none | bearer | basic | api_key | custom
    credential_mask  TEXT,                         -- masked credential for UI display
    credential_enc   TEXT,                         -- encrypted credential payload (AES-256-GCM JSON)
    notes            TEXT,
    status           TEXT NOT NULL DEFAULT 'active',
    created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS chain_metrics (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL REFERENCES chains(chain_id) ON DELETE CASCADE,
    block_height    BIGINT,
    tps_current     REAL DEFAULT 0,
    tps_peak        REAL DEFAULT 0,
    tps_theoretical REAL DEFAULT 0,
    gas_price_gwei  REAL,
    active_validators INTEGER,
    total_txns_24h  BIGINT DEFAULT 0,
    finality_seconds REAL,
    measured_at     DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS gpu_validation_stats (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL REFERENCES chains(chain_id) ON DELETE CASCADE,
    sig_algorithm   TEXT NOT NULL DEFAULT 'secp256k1', -- secp256k1 | ed25519 | sr25519 | bls12-381
    hash_algorithm  TEXT NOT NULL DEFAULT 'keccak256', -- keccak256 | sha256 | blake2b | poseidon
    sig_pubkey_size INTEGER DEFAULT 64,
    sig_size        INTEGER DEFAULT 65,
    hash_output_size INTEGER DEFAULT 32,
    gpu_verifications_total BIGINT DEFAULT 0,
    gpu_verifications_failed BIGINT DEFAULT 0,
    avg_verify_time_us REAL,                          -- microseconds per verification on GPU
    last_verified   DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_chains_ecosystem ON chains(ecosystem);
CREATE INDEX IF NOT EXISTS idx_chains_type ON chains(chain_type);
CREATE INDEX IF NOT EXISTS idx_chains_status ON chains(status);
CREATE INDEX IF NOT EXISTS idx_chains_is_evm ON chains(is_evm);
CREATE INDEX IF NOT EXISTS idx_chains_is_svm ON chains(is_svm);
CREATE INDEX IF NOT EXISTS idx_chains_numeric_id ON chains(chain_numeric_id);
CREATE INDEX IF NOT EXISTS idx_chains_testnet ON chains(is_testnet);
CREATE INDEX IF NOT EXISTS idx_rpc_chain ON rpc_endpoints(chain_id);
CREATE INDEX IF NOT EXISTS idx_rpc_healthy ON rpc_endpoints(is_healthy);
CREATE INDEX IF NOT EXISTS idx_rpc_rotation ON rpc_endpoints(chain_id, is_healthy, weight DESC, avg_latency_ms ASC);
CREATE INDEX IF NOT EXISTS idx_metrics_chain ON chain_metrics(chain_id);
CREATE INDEX IF NOT EXISTS idx_gpu_stats_chain ON gpu_validation_stats(chain_id);
CREATE INDEX IF NOT EXISTS idx_connectors_chain ON chain_connectors(chain_id);

-- Full-text search on chain names
CREATE VIRTUAL TABLE IF NOT EXISTS chains_fts USING fts5(chain_id, chain_name, ecosystem, native_token, content=chains, content_rowid=id);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS chains_ai AFTER INSERT ON chains BEGIN
    INSERT INTO chains_fts(rowid, chain_id, chain_name, ecosystem, native_token)
    VALUES (new.id, new.chain_id, new.chain_name, new.ecosystem, new.native_token);
END;

CREATE TRIGGER IF NOT EXISTS chains_ad AFTER DELETE ON chains BEGIN
    INSERT INTO chains_fts(chains_fts, rowid, chain_id, chain_name, ecosystem, native_token)
    VALUES ('delete', old.id, old.chain_id, old.chain_name, old.ecosystem, old.native_token);
END;

CREATE TRIGGER IF NOT EXISTS chains_au AFTER UPDATE ON chains BEGIN
    INSERT INTO chains_fts(chains_fts, rowid, chain_id, chain_name, ecosystem, native_token)
    VALUES ('delete', old.id, old.chain_id, old.chain_name, old.ecosystem, old.native_token);
    INSERT INTO chains_fts(rowid, chain_id, chain_name, ecosystem, native_token)
    VALUES (new.id, new.chain_id, new.chain_name, new.ecosystem, new.native_token);
END;

-- ──────────────────────────────────────────────────────────────────────────────
-- Airdrops, Faucets, Wallets (auto-discovery & claim tracking)
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS wallets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,                     -- FK to chains.chain_id (soft ref, chain may not exist yet)
    address         TEXT NOT NULL,
    label           TEXT DEFAULT 'auto',               -- auto | manual | imported
    private_key_enc TEXT,                              -- encrypted private key (AES-256-GCM)
    ecosystem       TEXT NOT NULL DEFAULT 'evm',       -- evm | svm | cosmos | substrate
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    balance         TEXT DEFAULT '0',                  -- native token balance (string for big numbers)
    balance_usd     REAL DEFAULT 0,
    last_balance_check DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, address)
);

CREATE TABLE IF NOT EXISTS airdrops (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,                     -- chain where airdrop is distributed
    name            TEXT NOT NULL,                     -- e.g. "LayerZero Season 2"
    project         TEXT,                              -- project name
    token_symbol    TEXT,                              -- airdrop token symbol
    token_address   TEXT,                              -- contract address of airdrop token
    airdrop_type    TEXT NOT NULL DEFAULT 'unknown',   -- retroactive | testnet | quest | holder | unknown
    status          TEXT NOT NULL DEFAULT 'discovered',-- discovered | eligible | claimed | expired | ineligible
    source          TEXT,                              -- where we found it: crawler | manual | aggregator
    source_url      TEXT,                              -- original link
    claim_url       TEXT,                              -- where to claim
    claim_start     DATETIME,                         -- when claiming opens
    claim_deadline  DATETIME,                         -- last day to claim
    snapshot_date   DATETIME,                         -- when snapshot was taken
    estimated_value REAL DEFAULT 0,                   -- estimated USD value
    actual_value    REAL DEFAULT 0,                   -- actual USD value (after claim)
    eligibility_criteria TEXT,                         -- JSON or text description
    notes           TEXT,
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS airdrop_claims (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    airdrop_id      INTEGER NOT NULL REFERENCES airdrops(id) ON DELETE CASCADE,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',   -- pending | submitted | confirmed | failed
    tx_hash         TEXT,
    amount          TEXT,                              -- token amount claimed
    amount_usd      REAL DEFAULT 0,
    gas_cost        TEXT,                              -- gas cost for claim tx
    claimed_at      DATETIME,
    confirmed_at    DATETIME,
    error_message   TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS faucets (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT NOT NULL,                     -- testnet chain
    name            TEXT NOT NULL,                     -- e.g. "Sepolia PoW Faucet"
    provider        TEXT,                              -- alchemy | infura | chainlink | quicknode | community
    url             TEXT NOT NULL,                     -- faucet URL
    faucet_type     TEXT NOT NULL DEFAULT 'web',       -- web | api | pow | social
    token_symbol    TEXT,                              -- what token it dispenses
    amount_per_claim TEXT,                             -- amount per drip
    cooldown_hours  REAL DEFAULT 24,                  -- hours between claims
    requires_auth   BOOLEAN NOT NULL DEFAULT 0,        -- needs login/social
    auth_type       TEXT,                              -- github | twitter | captcha | none
    status          TEXT NOT NULL DEFAULT 'active',    -- active | dead | rate_limited | maintenance
    last_checked    DATETIME,
    last_claimed    DATETIME,
    total_claims    INTEGER NOT NULL DEFAULT 0,
    total_received  TEXT DEFAULT '0',                  -- total tokens received
    source          TEXT,                              -- crawler | manual
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, url)
);

CREATE TABLE IF NOT EXISTS faucet_claims (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    faucet_id       INTEGER NOT NULL REFERENCES faucets(id) ON DELETE CASCADE,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',   -- pending | submitted | received | failed | cooldown
    tx_hash         TEXT,
    amount          TEXT,
    claimed_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    received_at     DATETIME,
    next_claim_at   DATETIME,                         -- when cooldown expires
    error_message   TEXT
);

CREATE TABLE IF NOT EXISTS chain_discoveries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id        TEXT,                              -- may be null for brand-new chains
    chain_name      TEXT NOT NULL,
    chain_numeric_id INTEGER,
    ecosystem       TEXT DEFAULT 'unknown',
    chain_type      TEXT DEFAULT 'unknown',            -- L1 | L2 | testnet | devnet
    is_testnet      BOOLEAN NOT NULL DEFAULT 0,
    source          TEXT NOT NULL,                     -- crawler | chainlist | manual
    source_url      TEXT,
    rpc_url         TEXT,                              -- first discovered RPC
    native_token    TEXT,
    status          TEXT NOT NULL DEFAULT 'new',       -- new | added | ignored | invalid
    added_to_chains BOOLEAN NOT NULL DEFAULT 0,       -- whether we inserted into chains table
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    processed_at    DATETIME
);

-- Indexes for new tables
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

-- ──────────────────────────────────────────────────────────────────────────────
-- Additional Tables for Crypto Bot Features
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS proxies (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL,                     -- proxy URL (e.g., http://proxy.example.com:8080)
    type            TEXT NOT NULL DEFAULT 'http',      -- http | socks5 | vpn
    country         TEXT,                              -- geolocation
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    last_used       DATETIME,
    success_rate    REAL DEFAULT 1.0,                  -- success rate (0-1)
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(url)
);

CREATE TABLE IF NOT EXISTS llm_configs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    provider        TEXT NOT NULL DEFAULT 'openrouter',-- openrouter | other
    api_key_enc     TEXT,                              -- encrypted API key
    model           TEXT DEFAULT 'gpt-3.5-turbo',      -- default model
    prompt_template TEXT,                              -- template for form filling/tasks
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS grants (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    provider        TEXT,                              -- e.g., Gitcoin, foundation name
    url             TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'discovered',-- discovered | pre-filled | staged | submitted | awarded | rejected
    pre_filled_form TEXT,                              -- JSON or text of pre-filled form
    staged_at       DATETIME,                          -- when staged for approval
    submitted_at    DATETIME,
    amount          TEXT,                              -- potential grant amount
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS auto_trading_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    coin_symbol     TEXT NOT NULL,
    exchange        TEXT NOT NULL,                     -- e.g., uniswap, binance
    action          TEXT NOT NULL,                     -- buy | sell | stake | lend
    amount          TEXT,
    tx_hash         TEXT,
    status          TEXT NOT NULL DEFAULT 'pending',
    executed_at     DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_social_accounts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    platform        TEXT NOT NULL,                     -- google | twitter | discord | etc.
    username        TEXT,
    credentials_enc TEXT,                              -- encrypted login credentials
    last_login      DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS treasury_transfers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id       INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    amount          TEXT,
    token_symbol    TEXT,
    tx_hash         TEXT,
    reason          TEXT DEFAULT 'inactive_reclamation', -- inactive_reclamation | other
    transferred_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Additional Indexes
CREATE INDEX IF NOT EXISTS idx_proxies_active ON proxies(is_active);
CREATE INDEX IF NOT EXISTS idx_grants_status ON grants(status);
CREATE INDEX IF NOT EXISTS idx_auto_trading_wallet ON auto_trading_logs(wallet_id);
CREATE INDEX IF NOT EXISTS idx_social_accounts_wallet ON user_social_accounts(wallet_id);
CREATE INDEX IF NOT EXISTS idx_treasury_wallet ON treasury_transfers(wallet_id);

CREATE TABLE IF NOT EXISTS referrals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    provider        TEXT,
    url             TEXT NOT NULL,
    referral_number TEXT,                              -- generated referral code
    plan            TEXT,                              -- description of referral plan
    rules           TEXT,                              -- simple rules
    status          TEXT DEFAULT 'discovered',
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_referrals_status ON referrals(status);

CREATE TABLE IF NOT EXISTS llm_endpoints (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL UNIQUE,
    provider        TEXT DEFAULT 'ollama',
    is_healthy      BOOLEAN DEFAULT 1,
    latency_ms      INTEGER,
    last_checked    DATETIME,
    models          TEXT,  -- JSON array of models
    version         TEXT,
    source          TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_llm_healthy ON llm_endpoints(is_healthy);
