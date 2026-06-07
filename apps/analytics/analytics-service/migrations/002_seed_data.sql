-- Seed data for Analytics Service
-- Version: 1.0.0
-- Created: 2025-12-04
--
-- This file contains sample data for development and testing

-- =============================================================================
-- Sample Events
-- =============================================================================

INSERT INTO events (id, event_type, account, comit_hash, block_number, chain_type, metadata, timestamp, session_id)
VALUES
    -- Comit submissions
    ('11111111-1111-1111-1111-111111111111', 'comit_submitted', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', '0xabc123...', NULL, 'dual', '{"evm_gas": 21000, "svm_compute": 5000}', NOW() - INTERVAL '1 hour', 'session-001'),
    ('22222222-2222-2222-2222-222222222222', 'comit_confirmed', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', '0xabc123...', 100, 'dual', '{"block_hash": "0x..."}', NOW() - INTERVAL '55 minutes', 'session-001'),
    ('33333333-3333-3333-3333-333333333333', 'comit_submitted', '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', '0xdef456...', NULL, 'evm', '{"evm_gas": 50000}', NOW() - INTERVAL '30 minutes', 'session-002'),
    ('44444444-4444-4444-4444-444444444444', 'comit_confirmed', '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', '0xdef456...', 105, 'evm', NULL, NOW() - INTERVAL '25 minutes', 'session-002'),
    
    -- Wallet events
    ('55555555-5555-5555-5555-555555555555', 'wallet_connected', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', NULL, NULL, NULL, '{"wallet_type": "polkadot-js"}', NOW() - INTERVAL '2 hours', 'session-001'),
    ('66666666-6666-6666-6666-666666666666', 'wallet_connected', '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', NULL, NULL, NULL, '{"wallet_type": "metamask"}', NOW() - INTERVAL '45 minutes', 'session-002'),
    
    -- Transaction events
    ('77777777-7777-7777-7777-777777777777', 'transaction_sent', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', NULL, 101, 'evm', '{"to": "0x123...", "value": "1000000000000000000"}', NOW() - INTERVAL '50 minutes', 'session-001'),
    
    -- Failed comit
    ('88888888-8888-8888-8888-888888888888', 'comit_submitted', '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy', '0xfail789...', NULL, 'svm', NULL, NOW() - INTERVAL '15 minutes', 'session-003'),
    ('99999999-9999-9999-9999-999999999999', 'comit_failed', '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy', '0xfail789...', NULL, 'svm', '{"error": "insufficient_funds"}', NOW() - INTERVAL '10 minutes', 'session-003')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- Sample Comit Tracking
-- =============================================================================

INSERT INTO comit_tracking (comit_hash, account, chain_type, status, block_number, gas_used, submitted_at, confirmed_at)
VALUES
    ('0xabc123...', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 'dual', 'confirmed', 100, 26000, NOW() - INTERVAL '1 hour', NOW() - INTERVAL '55 minutes'),
    ('0xdef456...', '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', 'evm', 'confirmed', 105, 50000, NOW() - INTERVAL '30 minutes', NOW() - INTERVAL '25 minutes'),
    ('0xfail789...', '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy', 'svm', 'failed', NULL, NULL, NOW() - INTERVAL '15 minutes', NULL)
ON CONFLICT (comit_hash) DO NOTHING;

-- =============================================================================
-- Sample Hourly Metrics
-- =============================================================================

INSERT INTO metrics_hourly (hour, event_type, chain_type, count, unique_accounts)
VALUES
    (date_trunc('hour', NOW()), 'comit_submitted', 'dual', 2, 2),
    (date_trunc('hour', NOW()), 'comit_confirmed', 'dual', 1, 1),
    (date_trunc('hour', NOW()), 'comit_submitted', 'evm', 1, 1),
    (date_trunc('hour', NOW()), 'comit_confirmed', 'evm', 1, 1),
    (date_trunc('hour', NOW()), 'comit_submitted', 'svm', 1, 1),
    (date_trunc('hour', NOW()), 'comit_failed', 'svm', 1, 1),
    (date_trunc('hour', NOW()), 'wallet_connected', 'unknown', 2, 2),
    (date_trunc('hour', NOW() - INTERVAL '1 hour'), 'wallet_connected', 'unknown', 1, 1)
ON CONFLICT (hour, event_type, chain_type) DO UPDATE SET
    count = metrics_hourly.count + EXCLUDED.count;

-- =============================================================================
-- Sample Daily Metrics
-- =============================================================================

INSERT INTO metrics_daily (day, total_events, total_comits, successful_comits, failed_comits, unique_accounts, evm_transactions, svm_transactions, dual_transactions)
VALUES
    (CURRENT_DATE, 9, 6, 2, 1, 3, 2, 2, 2),
    (CURRENT_DATE - 1, 15, 10, 8, 2, 5, 4, 3, 3),
    (CURRENT_DATE - 2, 12, 8, 7, 1, 4, 3, 2, 3)
ON CONFLICT (day) DO NOTHING;

-- =============================================================================
-- Sample Account Stats
-- =============================================================================

INSERT INTO account_stats (account, total_comits, successful_comits, failed_comits, total_gas_used, first_seen, last_seen)
VALUES
    ('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', 5, 4, 1, 130000, NOW() - INTERVAL '7 days', NOW() - INTERVAL '55 minutes'),
    ('5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', 3, 3, 0, 150000, NOW() - INTERVAL '3 days', NOW() - INTERVAL '25 minutes'),
    ('5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy', 2, 0, 2, 0, NOW() - INTERVAL '1 day', NOW() - INTERVAL '10 minutes')
ON CONFLICT (account) DO NOTHING;

-- Refresh materialized view with sample data
REFRESH MATERIALIZED VIEW mv_recent_activity;

-- Verify seed data
DO $$
DECLARE
    event_count INT;
    comit_count INT;
    account_count INT;
BEGIN
    SELECT COUNT(*) INTO event_count FROM events;
    SELECT COUNT(*) INTO comit_count FROM comit_tracking;
    SELECT COUNT(*) INTO account_count FROM account_stats;
    
    RAISE NOTICE 'Seed data loaded: % events, % comits, % accounts', event_count, comit_count, account_count;
END $$;
