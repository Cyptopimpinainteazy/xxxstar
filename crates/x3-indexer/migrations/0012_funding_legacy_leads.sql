-- Historical research leads are a PRIVATE staging feed, never public/official opportunities.
-- No foreign key to funding_swarm_grants: an archived program cannot bypass
-- source verification, eligibility review or approval stages by being imported.
CREATE TABLE funding_legacy_leads (
    source_file TEXT NOT NULL CHECK (length(source_file) BETWEEN 11 AND 256),
    legacy_id TEXT NOT NULL CHECK (length(legacy_id) BETWEEN 4 AND 48),
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 240),
    category TEXT NOT NULL CHECK (length(category) BETWEEN 1 AND 128),
    source_line INTEGER NOT NULL CHECK (source_line > 0),
    historical_status TEXT NOT NULL,
    historical_verification_date DATE NULL,
    content_sha256 TEXT NOT NULL CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    raw_record JSONB NOT NULL CHECK (jsonb_typeof(raw_record) = 'object'),
    current_availability TEXT NOT NULL DEFAULT 'UNKNOWN'
        CHECK (current_availability = 'UNKNOWN'),
    current_eligibility TEXT NOT NULL DEFAULT 'UNKNOWN'
        CHECK (current_eligibility = 'UNKNOWN'),
    review_state TEXT NOT NULL DEFAULT 'UNVERIFIED'
        CHECK (review_state = 'UNVERIFIED'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_file, legacy_id)
);

CREATE INDEX funding_legacy_leads_category_idx
    ON funding_legacy_leads (category, created_at DESC);
