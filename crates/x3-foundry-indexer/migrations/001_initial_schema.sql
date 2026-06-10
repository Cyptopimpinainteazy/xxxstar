-- X3 Foundry Indexer — Initial Database Schema
-- ==============================================
-- This schema supports indexing of FoundryRegistry, FoundryRevenueRouter,
-- and FoundryAppFactory events from the X3 Chain EVM.

-- ── dApp Registrations ──────────────────────────────────────────────────────
-- Tracks every dApp registered via the FoundryRegistry contract.

CREATE TABLE IF NOT EXISTS dapp_registrations (
    id              UUID PRIMARY KEY,
    app_id          VARCHAR(128) NOT NULL UNIQUE,
    name            VARCHAR(256) NOT NULL,
    dapp_type       VARCHAR(64)  NOT NULL,
    creator_wallet  VARCHAR(64)  NOT NULL,
    contract_address VARCHAR(64) NOT NULL DEFAULT '',
    chain           VARCHAR(32)  NOT NULL DEFAULT 'x3-mainnet',
    metadata_uri    TEXT,
    version         VARCHAR(32)  NOT NULL DEFAULT '0.1.0',
    license         VARCHAR(64)  NOT NULL DEFAULT 'Apache-2.0',
    registered_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    block_number    BIGINT       NOT NULL,
    tx_hash         VARCHAR(128) NOT NULL,
    active          BOOLEAN      NOT NULL DEFAULT TRUE,

    -- Index for fast lookups
    CONSTRAINT idx_dapp_registrations_app_id UNIQUE (app_id)
);

CREATE INDEX IF NOT EXISTS idx_dapp_registrations_creator
    ON dapp_registrations (creator_wallet);

CREATE INDEX IF NOT EXISTS idx_dapp_registrations_type
    ON dapp_registrations (dapp_type);

CREATE INDEX IF NOT EXISTS idx_dapp_registrations_block
    ON dapp_registrations (block_number DESC);

-- ── Revenue Events ──────────────────────────────────────────────────────────
-- Every revenue event emitted by the FoundryRevenueRouter.

CREATE TABLE IF NOT EXISTS revenue_events (
    id              UUID PRIMARY KEY,
    app_id          VARCHAR(128) NOT NULL,
    amount          DECIMAL(78, 0) NOT NULL DEFAULT 0,
    fee_token       VARCHAR(32)  NOT NULL DEFAULT 'X3',
    platform_fee    DECIMAL(78, 0) NOT NULL DEFAULT 0,
    creator_revenue DECIMAL(78, 0) NOT NULL DEFAULT 0,
    ai_agent_fee    DECIMAL(78, 0) NOT NULL DEFAULT 0,
    maintenance_fee DECIMAL(78, 0) NOT NULL DEFAULT 0,
    referral_fee    DECIMAL(78, 0) NOT NULL DEFAULT 0,
    recorded_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    block_number    BIGINT       NOT NULL,
    tx_hash         VARCHAR(128) NOT NULL,
    chain           VARCHAR(32)  NOT NULL DEFAULT 'x3-mainnet',

    -- Indexes for common query patterns
    CONSTRAINT idx_revenue_events_app_id UNIQUE (id)
);

CREATE INDEX IF NOT EXISTS idx_revenue_events_app
    ON revenue_events (app_id);

CREATE INDEX IF NOT EXISTS idx_revenue_events_time
    ON revenue_events (recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_revenue_events_app_time
    ON revenue_events (app_id, recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_revenue_events_block
    ON revenue_events (block_number DESC);

-- ── App Created Events ──────────────────────────────────────────────────────
-- Every app created via the FoundryAppFactory.

CREATE TABLE IF NOT EXISTS app_created_events (
    id                   UUID PRIMARY KEY,
    app_id               VARCHAR(128) NOT NULL UNIQUE,
    name                 VARCHAR(256) NOT NULL,
    factory_address      VARCHAR(64)  NOT NULL,
    implementation_address VARCHAR(64) NOT NULL DEFAULT '',
    proxy_address        VARCHAR(64)  NOT NULL DEFAULT '',
    creator_wallet       VARCHAR(64)  NOT NULL,
    created_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    block_number         BIGINT       NOT NULL,
    tx_hash              VARCHAR(128) NOT NULL,
    chain                VARCHAR(32)  NOT NULL DEFAULT 'x3-mainnet'
);

CREATE INDEX IF NOT EXISTS idx_app_created_events_creator
    ON app_created_events (creator_wallet);

CREATE INDEX IF NOT EXISTS idx_app_created_events_block
    ON app_created_events (block_number DESC);

-- ── Indexer State ───────────────────────────────────────────────────────────
-- Tracks the last indexed block for crash recovery.

CREATE TABLE IF NOT EXISTS indexer_state (
    id              VARCHAR(64) PRIMARY KEY DEFAULT 'default',
    chain           VARCHAR(32)  NOT NULL DEFAULT 'x3-mainnet',
    last_indexed_block BIGINT   NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

INSERT INTO indexer_state (id, chain, last_indexed_block)
VALUES ('default', 'x3-mainnet', 0)
ON CONFLICT (id) DO NOTHING;
