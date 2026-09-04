-- Analytics Service Database Schema
-- Version: 1.0.0
-- Created: 2025-12-04

-- =============================================================================
-- Events Table - Core event tracking
-- =============================================================================

CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    account VARCHAR(100),
    comit_hash VARCHAR(100),
    block_number BIGINT,
    chain_type VARCHAR(20), -- 'evm', 'svm', 'dual'
    metadata JSONB,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    session_id VARCHAR(100),
    user_agent TEXT,
    ip_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Events indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_account ON events(account);
CREATE INDEX IF NOT EXISTS idx_events_chain_type ON events(chain_type);
CREATE INDEX IF NOT EXISTS idx_events_comit_hash ON events(comit_hash);
CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);

-- =============================================================================
-- Comit Tracking Table - Track comit transaction lifecycle
-- =============================================================================

CREATE TABLE IF NOT EXISTS comit_tracking (
    comit_hash VARCHAR(100) PRIMARY KEY,
    account VARCHAR(100) NOT NULL,
    chain_type VARCHAR(20) NOT NULL, -- 'evm', 'svm', 'dual'
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'confirmed', 'failed'
    block_number BIGINT,
    gas_used BIGINT,
    evm_gas_used BIGINT,
    svm_compute_units BIGINT,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Comit tracking indexes
CREATE INDEX IF NOT EXISTS idx_comit_tracking_account ON comit_tracking(account);
CREATE INDEX IF NOT EXISTS idx_comit_tracking_status ON comit_tracking(status);
CREATE INDEX IF NOT EXISTS idx_comit_tracking_submitted_at ON comit_tracking(submitted_at DESC);
CREATE INDEX IF NOT EXISTS idx_comit_tracking_chain_type ON comit_tracking(chain_type);

-- =============================================================================
-- Metrics Hourly Table - Pre-aggregated hourly metrics
-- =============================================================================

CREATE TABLE IF NOT EXISTS metrics_hourly (
    id SERIAL PRIMARY KEY,
    hour TIMESTAMPTZ NOT NULL,
    event_type VARCHAR(50),
    chain_type VARCHAR(20),
    count BIGINT NOT NULL DEFAULT 0,
    unique_accounts BIGINT NOT NULL DEFAULT 0,
    total_gas BIGINT DEFAULT 0,
    avg_confirmation_time_ms DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(hour, event_type, chain_type)
);

-- Metrics indexes
CREATE INDEX IF NOT EXISTS idx_metrics_hourly_hour ON metrics_hourly(hour DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_hourly_event_type ON metrics_hourly(event_type);

-- =============================================================================
-- Daily Aggregates Table - Daily rollups for dashboards
-- =============================================================================

CREATE TABLE IF NOT EXISTS metrics_daily (
    id SERIAL PRIMARY KEY,
    day DATE NOT NULL,
    total_events BIGINT NOT NULL DEFAULT 0,
    total_comits BIGINT NOT NULL DEFAULT 0,
    successful_comits BIGINT NOT NULL DEFAULT 0,
    failed_comits BIGINT NOT NULL DEFAULT 0,
    unique_accounts BIGINT NOT NULL DEFAULT 0,
    evm_transactions BIGINT NOT NULL DEFAULT 0,
    svm_transactions BIGINT NOT NULL DEFAULT 0,
    dual_transactions BIGINT NOT NULL DEFAULT 0,
    total_gas_used BIGINT DEFAULT 0,
    avg_confirmation_time_ms DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(day)
);

CREATE INDEX IF NOT EXISTS idx_metrics_daily_day ON metrics_daily(day DESC);

-- =============================================================================
-- Account Stats Table - Per-account statistics
-- =============================================================================

CREATE TABLE IF NOT EXISTS account_stats (
    account VARCHAR(100) PRIMARY KEY,
    total_comits BIGINT NOT NULL DEFAULT 0,
    successful_comits BIGINT NOT NULL DEFAULT 0,
    failed_comits BIGINT NOT NULL DEFAULT 0,
    total_gas_used BIGINT DEFAULT 0,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_account_stats_total_comits ON account_stats(total_comits DESC);
CREATE INDEX IF NOT EXISTS idx_account_stats_last_seen ON account_stats(last_seen DESC);

-- =============================================================================
-- Functions and Triggers
-- =============================================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for comit_tracking
DROP TRIGGER IF EXISTS update_comit_tracking_updated_at ON comit_tracking;
CREATE TRIGGER update_comit_tracking_updated_at
    BEFORE UPDATE ON comit_tracking
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Trigger for account_stats
DROP TRIGGER IF EXISTS update_account_stats_updated_at ON account_stats;
CREATE TRIGGER update_account_stats_updated_at
    BEFORE UPDATE ON account_stats
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to update account stats on comit status change
CREATE OR REPLACE FUNCTION update_account_stats_on_comit()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO account_stats (account, total_comits, first_seen, last_seen)
        VALUES (NEW.account, 1, NOW(), NOW())
        ON CONFLICT (account) DO UPDATE SET
            total_comits = account_stats.total_comits + 1,
            last_seen = NOW();
    ELSIF TG_OP = 'UPDATE' AND OLD.status != NEW.status THEN
        IF NEW.status = 'confirmed' THEN
            UPDATE account_stats SET
                successful_comits = successful_comits + 1,
                total_gas_used = total_gas_used + COALESCE(NEW.gas_used, 0),
                last_seen = NOW()
            WHERE account = NEW.account;
        ELSIF NEW.status = 'failed' THEN
            UPDATE account_stats SET
                failed_comits = failed_comits + 1,
                last_seen = NOW()
            WHERE account = NEW.account;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for account stats
DROP TRIGGER IF EXISTS comit_tracking_account_stats ON comit_tracking;
CREATE TRIGGER comit_tracking_account_stats
    AFTER INSERT OR UPDATE ON comit_tracking
    FOR EACH ROW
    EXECUTE FUNCTION update_account_stats_on_comit();

-- =============================================================================
-- Materialized Views for Dashboard
-- =============================================================================

-- Recent activity summary (last 24 hours)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_recent_activity AS
SELECT
    date_trunc('hour', timestamp) as hour,
    event_type,
    chain_type,
    COUNT(*) as event_count,
    COUNT(DISTINCT account) as unique_accounts
FROM events
WHERE timestamp > NOW() - INTERVAL '24 hours'
GROUP BY date_trunc('hour', timestamp), event_type, chain_type
ORDER BY hour DESC;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_recent_activity 
    ON mv_recent_activity(hour, event_type, chain_type);

-- Function to refresh materialized views
CREATE OR REPLACE FUNCTION refresh_analytics_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_recent_activity;
END;
$$ language 'plpgsql';

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON TABLE events IS 'Core event tracking table for all analytics events';
COMMENT ON TABLE comit_tracking IS 'Tracks lifecycle of Comit transactions across EVM and SVM';
COMMENT ON TABLE metrics_hourly IS 'Pre-aggregated hourly metrics for fast dashboard queries';
COMMENT ON TABLE metrics_daily IS 'Daily rollup metrics for historical analysis';
COMMENT ON TABLE account_stats IS 'Per-account statistics for user dashboards';
COMMENT ON COLUMN events.chain_type IS 'Transaction chain type: evm, svm, or dual for cross-VM';
COMMENT ON COLUMN comit_tracking.status IS 'Comit status: pending, confirmed, or failed';
