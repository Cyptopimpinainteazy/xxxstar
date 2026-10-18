CREATE TABLE funding_sources (
    source_id TEXT PRIMARY KEY,
    official_url TEXT NOT NULL,
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    last_error_code TEXT
);
CREATE TABLE funding_source_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    grant_id TEXT NOT NULL REFERENCES funding_swarm_grants(grant_id),
    source_id TEXT NOT NULL REFERENCES funding_sources(source_id),
    content_sha256 TEXT NOT NULL CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    official_url TEXT NOT NULL,
    retrieved_at TIMESTAMPTZ NOT NULL,
    normalized JSONB NOT NULL CHECK (jsonb_typeof(normalized) = 'object'),
    source_document JSONB NOT NULL CHECK (jsonb_typeof(source_document) = 'object'),
    UNIQUE (grant_id, content_sha256)
);
CREATE INDEX funding_source_snapshots_grant_time
    ON funding_source_snapshots(grant_id, retrieved_at DESC);
