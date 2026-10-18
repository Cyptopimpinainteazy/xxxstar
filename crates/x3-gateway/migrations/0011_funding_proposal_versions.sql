-- Private editable application content is separate from public funding records.
CREATE TABLE funding_proposal_versions (
    grant_id TEXT NOT NULL REFERENCES funding_swarm_grants(grant_id),
    version INTEGER NOT NULL CHECK (version > 0),
    source_snapshot_id TEXT NOT NULL REFERENCES funding_source_snapshots(snapshot_id),
    package JSONB NOT NULL CHECK (jsonb_typeof(package) = 'object'),
    package_sha256 TEXT NOT NULL CHECK (package_sha256 ~ '^[0-9a-f]{64}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (grant_id, version)
);
