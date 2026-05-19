//! E2E test: state root replay verification
//!
//! Verifies CHAIN-CONSENSUS-001: Replay sequence of extrinsics from genesis
//! and assert the resulting state-root matches the original node's state-root.

use invariant_macros::invariant;
use crate::utils::{init_test_logging, TestConfig, TestEnvironment};

#[tokio::test]
#[invariant("CHAIN-CONSENSUS-001")]
async fn chain_replay_matches_state_root() -> crate::TestResult {
    init_test_logging();

    // Config for first environment (node A)
    let mut cfg_a = TestConfig::default();
    cfg_a.rpc_url = "http://localhost:9933".to_string();

    // Config for replay environment (node B) - different ports
    let mut cfg_b = TestConfig::default();
    cfg_b.rpc_url = "http://localhost:9943".to_string();
    cfg_b.websocket_url = "ws://localhost:9944".to_string();

    // Start first env and wait for readiness
    let env_a = TestEnvironment::new(cfg_a.clone()).await?;
    assert!(env_a.is_ready());

    // Wait for baseline blocks to be produced
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Determine block range to capture (small number for CI)
    let from = 1u64;
    let to = 20u64;

    // Export extrinsics from node A
    let extrinsics = env_a.export_extrinsics(from, to).await?;
    assert!(!extrinsics.is_empty(), "No extrinsics exported");

    // Get state root on node A at block `to`
    // Use chain_getHeader at specific block hash
    let block_hash_resp = env_a.get_state_root(None).await?; // latest header
    let state_root_a = env_a.get_state_root(None).await?;

    // Start second env (fresh node B)
    let env_b = TestEnvironment::new(cfg_b.clone()).await?;
    assert!(env_b.is_ready());

    // Submit extrinsics to node B in same order
    env_a.submit_extrinsics_to_rpc(&cfg_b.rpc_url, &extrinsics).await?;

    // Wait for node B to import and finalize blocks to cover the same range (timeout)
    let timeout = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        if timeout < std::time::Instant::now() {
            return Err("Timeout waiting for replay node to process extrinsics".into());
        }
        // Compare latest state root repeatedly; break when equal or after timeout
        let state_root_b = env_b.get_state_root(None).await?;
        if state_root_a == state_root_b {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
