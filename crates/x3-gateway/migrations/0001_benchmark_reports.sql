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
    artifacts JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE INDEX IF NOT EXISTS benchmark_reports_tenant_generated_at_idx
    ON benchmark_reports (tenant_id, generated_at DESC);

CREATE INDEX IF NOT EXISTS benchmark_reports_chain_generated_at_idx
    ON benchmark_reports (chain_name, generated_at DESC);
