CREATE TABLE IF NOT EXISTS orchestra_intents (
    intent_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    risk_class TEXT NOT NULL,
    submitter TEXT NOT NULL,
    requires_approval BOOLEAN NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS orchestra_intents_tenant_created_idx
    ON orchestra_intents (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS orchestra_intents_status_idx
    ON orchestra_intents (status);

CREATE TABLE IF NOT EXISTS approval_cases (
    case_id TEXT PRIMARY KEY,
    intent_id TEXT NOT NULL REFERENCES orchestra_intents(intent_id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    review_kind TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    summary TEXT NOT NULL,
    metadata JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS approval_cases_intent_created_idx
    ON approval_cases (intent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS approval_cases_status_idx
    ON approval_cases (status);

CREATE TABLE IF NOT EXISTS vote_windows (
    window_id TEXT PRIMARY KEY,
    approval_case_id TEXT NOT NULL REFERENCES approval_cases(case_id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    opens_at_unix BIGINT NOT NULL,
    closes_at_unix BIGINT NOT NULL,
    electorate JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS vote_windows_case_created_idx
    ON vote_windows (approval_case_id, created_at DESC);

CREATE INDEX IF NOT EXISTS vote_windows_status_idx
    ON vote_windows (status);

CREATE TABLE IF NOT EXISTS vote_receipts (
    receipt_id TEXT PRIMARY KEY,
    window_id TEXT NOT NULL REFERENCES vote_windows(window_id) ON DELETE CASCADE,
    voter_id TEXT NOT NULL,
    vote_choice TEXT NOT NULL,
    rationale TEXT,
    cast_at_unix BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (window_id, voter_id)
);

CREATE INDEX IF NOT EXISTS vote_receipts_window_cast_idx
    ON vote_receipts (window_id, cast_at_unix DESC);

CREATE TABLE IF NOT EXISTS evidence_bundles (
    bundle_id TEXT PRIMARY KEY,
    intent_id TEXT REFERENCES orchestra_intents(intent_id) ON DELETE SET NULL,
    approval_case_id TEXT REFERENCES approval_cases(case_id) ON DELETE SET NULL,
    vote_window_id TEXT REFERENCES vote_windows(window_id) ON DELETE SET NULL,
    artifact_uri TEXT NOT NULL,
    digest TEXT NOT NULL,
    summary JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS evidence_bundles_intent_created_idx
    ON evidence_bundles (intent_id, created_at DESC);

CREATE INDEX IF NOT EXISTS evidence_bundles_case_created_idx
    ON evidence_bundles (approval_case_id, created_at DESC);

CREATE INDEX IF NOT EXISTS evidence_bundles_window_created_idx
    ON evidence_bundles (vote_window_id, created_at DESC);