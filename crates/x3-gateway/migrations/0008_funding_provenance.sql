-- Forward-only classification: retain historical migration checksums and rows.
ALTER TABLE funding_swarm_grants
    ADD COLUMN is_example BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN award_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE funding_swarm_events
    ADD COLUMN is_example BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE funding_swarm_publications
    ADD COLUMN is_example BOOLEAN NOT NULL DEFAULT FALSE;

-- Exact identities from migration 0006; do not classify user records by title
-- or a broad prefix match. Metadata makes classification visible to admins.
UPDATE funding_swarm_grants
SET is_example = TRUE,
    metadata = metadata || '{"record_kind":"historical_example"}'::jsonb
WHERE grant_id IN ('grant-demo-opensource-security-tooling',
                  'grant-demo-interop-observability',
                  'grant-demo-zk-composer');
UPDATE funding_swarm_events SET is_example = TRUE
WHERE event_id IN ('event-demo-001', 'event-demo-002', 'event-demo-003');
UPDATE funding_swarm_publications SET is_example = TRUE
WHERE publication_id IN ('pub-demo-001', 'pub-demo-002');

-- A status string or opportunity amount is not verified award documentation.
-- No existing row is promoted to a verified award. There is deliberately no
-- public/API setter for award_verified pending an evidence-controlled workflow.
