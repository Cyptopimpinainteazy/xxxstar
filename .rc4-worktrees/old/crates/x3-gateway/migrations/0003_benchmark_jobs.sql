-- Create benchmark_jobs table to track job submissions, status, and lifecycle
CREATE TABLE IF NOT EXISTS benchmark_jobs (
    job_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    chain_name TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    rpc_endpoints TEXT[] NOT NULL,
    explorer_endpoint TEXT,
    workload_trace_uri TEXT,
    date_range_start_unix BIGINT NOT NULL,
    date_range_end_unix BIGINT NOT NULL,
    status TEXT NOT NULL,
    report_id TEXT,
    submitted_at_unix BIGINT NOT NULL,
    updated_at_unix BIGINT NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indices for efficient querying
CREATE INDEX IF NOT EXISTS benchmark_jobs_tenant_updated_idx
    ON benchmark_jobs (tenant_id, updated_at_unix DESC);

CREATE INDEX IF NOT EXISTS benchmark_jobs_status_idx
    ON benchmark_jobs (status);

CREATE INDEX IF NOT EXISTS benchmark_jobs_report_id_idx
    ON benchmark_jobs (report_id)
    WHERE report_id IS NOT NULL;
