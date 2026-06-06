CREATE TABLE IF NOT EXISTS funding_swarm_grants (
    grant_id TEXT PRIMARY KEY,
    external_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    sponsor TEXT NOT NULL,
    status TEXT NOT NULL,
    stage TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL DEFAULT 0,
    amount_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS funding_swarm_grants_status_score_idx
    ON funding_swarm_grants (status, score DESC, created_at DESC);

CREATE TABLE IF NOT EXISTS funding_swarm_events (
    event_id TEXT PRIMARY KEY,
    grant_id TEXT REFERENCES funding_swarm_grants(grant_id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'public',
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS funding_swarm_events_visibility_created_idx
    ON funding_swarm_events (visibility, created_at DESC);

CREATE TABLE IF NOT EXISTS funding_swarm_publications (
    publication_id TEXT PRIMARY KEY,
    grant_id TEXT REFERENCES funding_swarm_grants(grant_id) ON DELETE SET NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'published',
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS funding_swarm_publications_status_published_idx
    ON funding_swarm_publications (status, published_at DESC, created_at DESC);

INSERT INTO funding_swarm_grants (
    grant_id,
    external_id,
    title,
    sponsor,
    status,
    stage,
    score,
    amount_usd,
    metadata
) VALUES
    (
        'grant-demo-opensource-security-tooling',
        'DEMO-001',
        'Open-source security tooling for cross-chain dapps',
        'Foundations Collective',
        'open',
        'discovery',
        92.4,
        75000,
        '{"focus":["security","developer-tooling"],"region":"global"}'::jsonb
    ),
    (
        'grant-demo-interop-observability',
        'DEMO-002',
        'Interoperability observability and replay diagnostics',
        'Ecosystem Labs',
        'submitted',
        'review',
        86.1,
        50000,
        '{"focus":["observability","reliability"],"region":"americas"}'::jsonb
    ),
    (
        'grant-demo-zk-composer',
        'DEMO-003',
        'Zero-knowledge proof composer templates for app teams',
        'Protocol Growth DAO',
        'funded',
        'awarded',
        95.3,
        120000,
        '{"focus":["zk","education"],"region":"emea"}'::jsonb
    )
ON CONFLICT (grant_id) DO NOTHING;

INSERT INTO funding_swarm_events (
    event_id,
    grant_id,
    event_type,
    visibility,
    detail
) VALUES
    (
        'event-demo-001',
        'grant-demo-opensource-security-tooling',
        'grant_discovered',
        'public',
        '{"source":"manual-intake","note":"Opportunity triaged and scorecard initialized"}'::jsonb
    ),
    (
        'event-demo-002',
        'grant-demo-interop-observability',
        'proposal_submitted',
        'public',
        '{"round":"R2-2026","note":"Proposal sent for sponsor review"}'::jsonb
    ),
    (
        'event-demo-003',
        'grant-demo-zk-composer',
        'award_paid',
        'public',
        '{"tx_ref":"DEMO-TX-9482","note":"Milestone 1 payout confirmed"}'::jsonb
    )
ON CONFLICT (event_id) DO NOTHING;

INSERT INTO funding_swarm_publications (
    publication_id,
    grant_id,
    kind,
    title,
    body,
    status,
    published_at
) VALUES
    (
        'pub-demo-001',
        'grant-demo-zk-composer',
        'weekly_update',
        'Funding Swarm Weekly: first award confirmed',
        'The ZK composer track reached funded status and milestone payout was recorded.',
        'published',
        NOW() - INTERVAL '1 day'
    ),
    (
        'pub-demo-002',
        NULL,
        'scoreboard_snapshot',
        'Funding Swarm snapshot published',
        'Current scoreboard includes three active demo grants across discovery, review, and award stages.',
        'published',
        NOW()
    )
ON CONFLICT (publication_id) DO NOTHING;
