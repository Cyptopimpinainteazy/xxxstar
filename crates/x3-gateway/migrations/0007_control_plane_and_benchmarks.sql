-- Gateway control-plane and benchmark tables. Chain tables use the indexer migration.

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

CREATE TABLE IF NOT EXISTS approval_cases (
    case_id TEXT PRIMARY KEY,
    intent_id TEXT NOT NULL REFERENCES orchestra_intents(intent_id),
    status TEXT NOT NULL,
    review_kind TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    summary TEXT NOT NULL,
    metadata JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS vote_windows (
    window_id TEXT PRIMARY KEY,
    approval_case_id TEXT NOT NULL REFERENCES approval_cases(case_id),
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    opens_at_unix BIGINT NOT NULL,
    closes_at_unix BIGINT NOT NULL,
    electorate JSONB NOT NULL,
    tally JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (opens_at_unix < closes_at_unix)
);

CREATE TABLE IF NOT EXISTS vote_receipts (
    receipt_id TEXT PRIMARY KEY,
    window_id TEXT NOT NULL REFERENCES vote_windows(window_id),
    voter_id TEXT NOT NULL,
    vote_choice TEXT NOT NULL,
    rationale TEXT,
    cast_at_unix BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT vote_receipts_window_id_voter_id_key UNIQUE (window_id, voter_id)
);

CREATE TABLE IF NOT EXISTS evidence_bundles (
    bundle_id TEXT PRIMARY KEY,
    intent_id TEXT REFERENCES orchestra_intents(intent_id),
    approval_case_id TEXT REFERENCES approval_cases(case_id),
    vote_window_id TEXT REFERENCES vote_windows(window_id),
    artifact_uri TEXT NOT NULL,
    digest TEXT NOT NULL,
    summary JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS benchmark_reports (
    report_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    chain_name TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    signer TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    baseline_avg_tps DOUBLE PRECISION NOT NULL,
    baseline_p50_latency_ms BIGINT NOT NULL,
    baseline_p95_latency_ms BIGINT NOT NULL,
    baseline_p99_latency_ms BIGINT NOT NULL,
    baseline_failure_rate DOUBLE PRECISION NOT NULL,
    x3_avg_tps DOUBLE PRECISION NOT NULL,
    x3_p50_latency_ms BIGINT NOT NULL,
    x3_p95_latency_ms BIGINT NOT NULL,
    x3_p99_latency_ms BIGINT NOT NULL,
    x3_failure_rate DOUBLE PRECISION NOT NULL,
    projected_soft_confirmation_improvement TEXT NOT NULL,
    projected_app_throughput_improvement TEXT NOT NULL,
    projected_route_latency_delta TEXT NOT NULL,
    projected_bridge_latency_delta TEXT NOT NULL,
    workload_profile JSONB NOT NULL,
    artifacts JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_orchestra_intents_lookup_0 ON orchestra_intents (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_orchestra_intents_lookup_1 ON orchestra_intents (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_approval_cases_lookup_0 ON approval_cases (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_approval_cases_lookup_1 ON approval_cases (intent_id);

CREATE INDEX IF NOT EXISTS idx_vote_windows_lookup_0 ON vote_windows (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_vote_windows_lookup_1 ON vote_windows (approval_case_id);

CREATE INDEX IF NOT EXISTS idx_evidence_bundles_lookup_0 ON evidence_bundles (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_evidence_bundles_lookup_1 ON evidence_bundles (intent_id);

CREATE INDEX IF NOT EXISTS idx_evidence_bundles_lookup_2 ON evidence_bundles (approval_case_id);

CREATE INDEX IF NOT EXISTS idx_evidence_bundles_lookup_3 ON evidence_bundles (vote_window_id);

CREATE INDEX IF NOT EXISTS idx_benchmark_reports_lookup_0 ON benchmark_reports (generated_at DESC);

CREATE INDEX IF NOT EXISTS idx_benchmark_reports_lookup_1 ON benchmark_reports (tenant_id, generated_at DESC);
