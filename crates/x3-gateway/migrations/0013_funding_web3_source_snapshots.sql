-- Private official Web3 funding-source observations, not a public grant/award ledger.
-- A verified page can document program guidance without proving current funding
-- availability, X3 eligibility, an authorized application, or an actual award.
CREATE TABLE funding_web3_source_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES funding_sources(source_id),
    official_url TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    source_document TEXT NOT NULL CHECK (length(source_document) BETWEEN 1 AND 1048576),
    normalized JSONB NOT NULL CHECK (jsonb_typeof(normalized) = 'object'),
    retrieved_at TIMESTAMPTZ NOT NULL,
    last_observed_at TIMESTAMPTZ NOT NULL,
    UNIQUE (source_id, content_sha256)
);
CREATE INDEX funding_web3_source_snapshots_latest_idx
    ON funding_web3_source_snapshots (source_id, last_observed_at DESC);
