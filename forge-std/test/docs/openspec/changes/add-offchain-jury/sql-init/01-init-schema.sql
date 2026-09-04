# SQL initialization script for jury audit database
# This script runs automatically when the PostgreSQL container starts

-- ============================================================
-- Create roles and permissions
-- ============================================================
CREATE ROLE jury_readonly WITH LOGIN PASSWORD 'readonly_password_change_me';

GRANT CONNECT ON DATABASE jury_audit TO jury_readonly;
GRANT USAGE ON SCHEMA public TO jury_readonly;

-- ============================================================
-- Audit Log Table
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    event_type VARCHAR(64) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    actor VARCHAR(255),
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    content_hash VARCHAR(64),
    
    -- Indexes for efficient querying
    CONSTRAINT valid_event_type CHECK (
        event_type IN (
            'SESSION_CREATED',
            'COMMIT_SUBMITTED',
            'REVEAL_PHASE_ADVANCED',
            'VOTE_REVEALED',
            'VOTES_AGGREGATED',
            'SESSION_COMPLETED',
            'SESSION_CANCELLED',
            'AUDIT_RETRIEVAL'
        )
    )
);

CREATE INDEX idx_audit_logs_session_id ON audit_logs(session_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_metadata ON audit_logs USING GIN(metadata);

-- ============================================================
-- Jury Session State Table
-- ============================================================
CREATE TABLE IF NOT EXISTS jury_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_ids TEXT[] NOT NULL,  -- Array of task IDs for jury decision
    state VARCHAR(32) NOT NULL DEFAULT 'CREATED',
    
    -- Members
    jury_members JSONB NOT NULL,  -- Serialized jury member list
    jury_size INT NOT NULL CHECK (jury_size >= 3 AND jury_size <= 21),
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    commit_deadline TIMESTAMP WITH TIME ZONE NOT NULL,
    reveal_deadline TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    
    -- Results (after aggregation)
    result_yes_votes INT DEFAULT 0,
    result_no_votes INT DEFAULT 0,
    result_total_votes INT DEFAULT 0,
    result_final BOOLEAN,
    
    -- Audit integrity
    audit_log_hash VARCHAR(64),
    on_chain_anchor TEXT,
    
    CONSTRAINT valid_session_state CHECK (
        state IN (
            'CREATED',
            'COMMIT_PHASE',
            'REVEAL_PHASE',
            'COMPLETED',
            'CANCELLED'
        )
    )
);

CREATE INDEX idx_jury_sessions_state ON jury_sessions(state);
CREATE INDEX idx_jury_sessions_created_at ON jury_sessions(created_at DESC);
CREATE INDEX idx_jury_sessions_completed_at ON jury_sessions(completed_at DESC);

-- ============================================================
-- Jury Votes Table (for audit trail)
-- ============================================================
CREATE TABLE IF NOT EXISTS jury_votes (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES jury_sessions(id) ON DELETE CASCADE,
    member_id VARCHAR(255) NOT NULL,
    
    -- Commit phase
    commitment_hash VARCHAR(64),
    commitment_timestamp TIMESTAMP WITH TIME ZONE,
    
    -- Reveal phase
    vote BOOLEAN,
    nonce VARCHAR(255),
    reveal_timestamp TIMESTAMP WITH TIME ZONE,
    reveal_verified BOOLEAN DEFAULT false,
    
    CONSTRAINT vote_phase_consistency CHECK (
        (commitment_hash IS NOT NULL AND commitment_timestamp IS NOT NULL) OR
        (vote IS NOT NULL AND nonce IS NOT NULL AND reveal_timestamp IS NOT NULL)
    )
);

CREATE INDEX idx_jury_votes_session_id ON jury_votes(session_id);
CREATE INDEX idx_jury_votes_member_id ON jury_votes(member_id);
CREATE INDEX idx_jury_votes_reveal_verified ON jury_votes(reveal_verified);
CREATE UNIQUE INDEX idx_jury_votes_session_member ON jury_votes(session_id, member_id);

-- ============================================================
-- Audit Log Sealing Records (for integrity verification)
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_log_seals (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    sealed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    on_chain_tx_hash VARCHAR(255),
    on_chain_tx_timestamp TIMESTAMP WITH TIME ZONE,
    verification_count INT DEFAULT 0,
    last_verified_at TIMESTAMP WITH TIME ZONE,
    
    UNIQUE(session_id)
);

CREATE INDEX idx_audit_log_seals_session_id ON audit_log_seals(session_id);
CREATE INDEX idx_audit_log_seals_on_chain_tx_hash ON audit_log_seals(on_chain_tx_hash);

-- ============================================================
-- Session Analytics View
-- ============================================================
CREATE VIEW session_analytics AS
SELECT 
    js.id as session_id,
    js.state,
    js.jury_size,
    COUNT(DISTINCT av.member_id) as votes_received,
    SUM(CASE WHEN av.vote = true THEN 1 ELSE 0 END) as yes_votes,
    SUM(CASE WHEN av.vote = false THEN 1 ELSE 0 END) as no_votes,
    js.result_final,
    AGE(js.completed_at, js.created_at) as duration,
    js.created_at,
    js.completed_at
FROM jury_sessions js
LEFT JOIN jury_votes av ON js.id = av.session_id AND av.reveal_verified = true
GROUP BY js.id, js.state, js.jury_size, js.result_final, js.created_at, js.completed_at;

-- ============================================================
-- Grant permissions for readonly role
-- ============================================================
GRANT SELECT ON audit_logs TO jury_readonly;
GRANT SELECT ON jury_sessions TO jury_readonly;
GRANT SELECT ON jury_votes TO jury_readonly;
GRANT SELECT ON audit_log_seals TO jury_readonly;
GRANT SELECT ON session_analytics TO jury_readonly;

-- ============================================================
-- Data retention policy (manual archival recommended)
-- ============================================================
-- Older audit logs can be archived to cold storage
-- Example: SELECT * INTO audit_logs_archive_2026_01 FROM audit_logs 
--          WHERE timestamp < '2026-02-01'::date AND session_id IN 
--          (SELECT session_id FROM jury_sessions WHERE completed_at < '2026-02-01'::date)
