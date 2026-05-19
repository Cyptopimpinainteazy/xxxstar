ALTER TABLE benchmark_reports
    ADD COLUMN IF NOT EXISTS workload_profile JSONB NOT NULL DEFAULT '{}'::jsonb;
