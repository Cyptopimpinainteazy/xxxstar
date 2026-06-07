//! Tests for the Cross-VM Atomic Trade Coordinator.

use crate::config::CoordinatorConfig;
use crate::flashloan_adapter::FlashloanRouter;
use crate::state_machine::SwapCoordinator;
use crate::types::*;

#[test]
fn test_secret_hash_roundtrip() {
    let secret = HtlcSecret::generate();
    let hash = secret.hash();

    // Hash should be 32 bytes
    assert_eq!(hash.0.len(), 32);

    // Same secret → same hash (deterministic)
    let hash2 = secret.hash();
    assert_eq!(hash.0, hash2.0);
}

#[test]
fn test_secret_uniqueness() {
    let s1 = HtlcSecret::generate();
    // Introduce microsecond delay for entropy
    std::thread::sleep(std::time::Duration::from_millis(1));
    let s2 = HtlcSecret::generate();

    // Secrets should be different
    assert_ne!(s1.0, s2.0);
}

#[test]
fn test_setup_swap_creates_session() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, secret, hash) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    assert!(!session_id.is_empty());
    assert_eq!(secret.hash().0, hash.0);

    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Setup);
    assert_eq!(session.timelock_fast, now + 3600);
    assert_eq!(session.timelock_slow, now + 7200);
}

#[test]
fn test_phase_transitions_happy_path() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, secret, hash) = coordinator
        .setup_swap(
            VmTarget::Svm,
            VmTarget::Evm { chain_id: 1 },
            vec![FlashLeg {
                vm: VmTarget::Svm,
                provider: FlashloanProvider::MarginFi,
                borrow_asset: vec![0u8; 32],
                borrow_amount: 100_000_000_000,
                swap_target: vec![],
                swap_data: vec![],
                min_output: 99_500_000_000,
                gas_limit: 200_000,
            }],
            now,
        )
        .unwrap();

    // Phase 1 → 2: Lock HTLCs
    let fast_htlc = HtlcRecord {
        id: HtlcId::from_bytes(vec![1u8; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![2u8; 32],
            hash_lock: hash,
            timelock: now + 3600,
            asset: vec![0u8; 32],
            amount: 100_000_000_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 50,
        confirmations: 0,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_fast(&session_id, fast_htlc, now)
        .unwrap();

    let slow_htlc = HtlcRecord {
        id: HtlcId::from_bytes(vec![2u8; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![3u8; 20],
            hash_lock: hash,
            timelock: now + 7200,
            asset: vec![0u8; 20],
            amount: 1_000_000_000_000_000_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 18000000,
        confirmations_required: 12,
        confirmations: 0,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, slow_htlc, now)
        .unwrap();

    // Both locked → phase should be HtlcsLocked
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::HtlcsLocked);

    // Phase 2 → 3: Execute flash legs
    coordinator.begin_flash_execution(&session_id, now).unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::ExecutingFlashLegs);

    // Record successful leg
    coordinator
        .record_leg_outcome(
            &session_id,
            FlashLegOutcome::Success {
                tx_hash: vec![0xAA; 64],
                gas_used: 150_000,
                output_amount: 100_500_000_000,
                premium_paid: 0,
            },
            now,
        )
        .unwrap();

    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::LegsComplete);

    // Phase 3 → 4: Settlement
    coordinator.begin_settlement(&session_id, now).unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::ClaimingFast);

    coordinator
        .record_fast_claim(&session_id, secret, now)
        .unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::ClaimingSlow);

    coordinator.record_slow_claim(&session_id, now).unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Complete);
}

#[test]
fn test_flash_leg_revert_aborts_swap() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, _, hash) = coordinator
        .setup_swap(
            VmTarget::Svm,
            VmTarget::Evm { chain_id: 1 },
            vec![FlashLeg {
                vm: VmTarget::Svm,
                provider: FlashloanProvider::Solend,
                borrow_asset: vec![],
                borrow_amount: 100,
                swap_target: vec![],
                swap_data: vec![],
                min_output: 99,
                gas_limit: 200_000,
            }],
            now,
        )
        .unwrap();

    // Lock HTLCs
    let fast = HtlcRecord {
        id: HtlcId::from_bytes(vec![1; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![],
            hash_lock: hash,
            timelock: now + 3600,
            asset: vec![],
            amount: 100,
        },
        status: HtlcStatus::Funded,
        created_at_block: 0,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_fast(&session_id, fast, now)
        .unwrap();

    let slow = HtlcRecord {
        id: HtlcId::from_bytes(vec![2; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock: hash,
            timelock: now + 7200,
            asset: vec![],
            amount: 100,
        },
        status: HtlcStatus::Funded,
        created_at_block: 0,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, slow, now)
        .unwrap();

    // Begin flash execution
    coordinator.begin_flash_execution(&session_id, now).unwrap();

    // Record REVERTED leg
    let result = coordinator.record_leg_outcome(
        &session_id,
        FlashLegOutcome::Reverted {
            reason: "insufficient liquidity".to_string(),
        },
        now,
    );

    assert!(result.is_err());
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Aborting);
}

#[test]
fn test_timelock_near_expiry_prevents_execution() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, _, hash) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    let fast = HtlcRecord {
        id: HtlcId::from_bytes(vec![1; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![],
            hash_lock: hash,
            timelock: now + 3600,
            asset: vec![],
            amount: 100,
        },
        status: HtlcStatus::Funded,
        created_at_block: 0,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_fast(&session_id, fast, now)
        .unwrap();

    let slow = HtlcRecord {
        id: HtlcId::from_bytes(vec![2; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock: hash,
            timelock: now + 7200,
            asset: vec![],
            amount: 100,
        },
        status: HtlcStatus::Funded,
        created_at_block: 0,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, slow, now)
        .unwrap();

    // Try to begin flash execution NEAR timelock expiry (within safety margin)
    let near_expiry = now + 3600 - 200; // 200s before expiry, safety = 300s
    let result = coordinator.begin_flash_execution(&session_id, near_expiry);
    assert!(result.is_err());

    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Aborting);
}

#[test]
fn test_provider_selection_prefers_zero_fee() {
    let router = FlashloanRouter::new();

    // EVM: should select Balancer (0%) over Aave (0.05%)
    let provider = router.select_provider(&VmTarget::Evm { chain_id: 1 }, 1_000_000);
    assert!(matches!(provider, Some(FlashloanProvider::BalancerV2)));

    // SVM: should select MarginFi (0%) over Solend (0.3%)
    let provider = router.select_provider(&VmTarget::Svm, 1_000_000);
    assert!(matches!(provider, Some(FlashloanProvider::MarginFi)));

    // X3: should select X3Native
    let provider = router.select_provider(&VmTarget::X3Vm, 1_000_000);
    assert!(matches!(provider, Some(FlashloanProvider::X3Native)));
}

#[test]
fn test_provider_supports_vm() {
    assert!(FlashloanProvider::AaveV3.supports_vm(&VmTarget::Evm { chain_id: 1 }));
    assert!(!FlashloanProvider::AaveV3.supports_vm(&VmTarget::Svm));
    assert!(FlashloanProvider::Solend.supports_vm(&VmTarget::Svm));
    assert!(!FlashloanProvider::Solend.supports_vm(&VmTarget::Evm { chain_id: 1 }));
    assert!(FlashloanProvider::X3Native.supports_vm(&VmTarget::X3Vm));
}

#[test]
fn test_premium_calculation() {
    let router = FlashloanRouter::new();

    let legs = vec![
        FlashLeg {
            vm: VmTarget::Evm { chain_id: 1 },
            provider: FlashloanProvider::AaveV3, // 0.05% = 5 bps
            borrow_asset: vec![],
            borrow_amount: 1_000_000_000_000, // 1M (6 decimals)
            swap_target: vec![],
            swap_data: vec![],
            min_output: 0,
            gas_limit: 0,
        },
        FlashLeg {
            vm: VmTarget::Svm,
            provider: FlashloanProvider::Solend, // 0.3% = 30 bps
            borrow_asset: vec![],
            borrow_amount: 500_000_000_000, // 500K
            swap_target: vec![],
            swap_data: vec![],
            min_output: 0,
            gas_limit: 0,
        },
    ];

    let total = router.total_premium(&legs);
    let expected_aave = 1_000_000_000_000 * 5 / 10_000; // 500M = 0.05%
    let expected_solend = 500_000_000_000 * 30 / 10_000; // 1.5B = 0.3%
    assert_eq!(total, expected_aave + expected_solend);
}

#[test]
fn test_invalid_provider_for_vm_rejected() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let result = coordinator.setup_swap(
        VmTarget::Svm,
        VmTarget::Evm { chain_id: 1 },
        vec![FlashLeg {
            vm: VmTarget::Svm,
            provider: FlashloanProvider::AaveV3, // Aave doesn't work on SVM!
            borrow_asset: vec![],
            borrow_amount: 100,
            swap_target: vec![],
            swap_data: vec![],
            min_output: 0,
            gas_limit: 0,
        }],
        now,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoordinatorError::ProviderUnavailable { .. }
    ));
}

#[test]
fn test_abort_and_refund() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    coordinator.abort(&session_id, "manual abort", now).unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Aborting);

    coordinator.record_refunds(&session_id, now).unwrap();
    let session = coordinator.get_session(&session_id).unwrap();
    assert_eq!(session.phase, SwapPhase::Refunded);
}

#[test]
fn test_active_sessions_counter() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    assert_eq!(coordinator.active_sessions(), 0);

    let (id1, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();
    assert_eq!(coordinator.active_sessions(), 1);

    let (_id2, _, _) = coordinator
        .setup_swap(VmTarget::X3Vm, VmTarget::Evm { chain_id: 137 }, vec![], now)
        .unwrap();
    assert_eq!(coordinator.active_sessions(), 2);

    coordinator.abort(&id1, "test", now).unwrap();
    coordinator.record_refunds(&id1, now).unwrap();
    assert_eq!(coordinator.active_sessions(), 1);
}

#[test]
fn test_config_timelock_computation() {
    let config = CoordinatorConfig::default();
    let now = 1700000000u64;

    let (t_fast, t_slow) = config.compute_timelocks(now, &VmTarget::Svm);
    assert_eq!(t_fast, now + 3600);
    assert_eq!(t_slow, now + 7200);
    assert!(t_slow > t_fast);
}

#[test]
fn test_config_near_expiry() {
    let config = CoordinatorConfig::default();
    let timelock = 1700003600u64;

    // 5 minutes (300s) before: should be near
    assert!(config.is_near_expiry(timelock, timelock - 200));
    // Well before: should NOT be near
    assert!(!config.is_near_expiry(timelock, timelock - 500));
}

// ── Production-readiness tests added for 100% production push ────────────────

#[test]
fn test_session_count_tracks_all_sessions() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    assert_eq!(coordinator.session_count(), 0);
    assert_eq!(coordinator.active_sessions(), 0);

    let (id1, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();
    let (_id2, _, _) = coordinator
        .setup_swap(VmTarget::X3Vm, VmTarget::Evm { chain_id: 137 }, vec![], now)
        .unwrap();

    assert_eq!(coordinator.session_count(), 2);
    assert_eq!(coordinator.active_sessions(), 2);

    // Refund id1 — session_count stays 2 (not purged yet), active drops to 1
    coordinator.abort(&id1, "test", now).unwrap();
    coordinator.record_refunds(&id1, now).unwrap();
    assert_eq!(coordinator.session_count(), 2);
    assert_eq!(coordinator.active_sessions(), 1);
}

#[test]
fn test_purge_terminated_sessions_removes_stale_terminals() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (id1, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();
    let (_id2, _, _) = coordinator
        .setup_swap(VmTarget::X3Vm, VmTarget::Evm { chain_id: 137 }, vec![], now)
        .unwrap();

    // Terminate id1 at `now`
    coordinator.abort(&id1, "done", now).unwrap();
    coordinator.record_refunds(&id1, now).unwrap();

    // Purge with max_age = 3600s, advance time by 7200s
    let future = now + 7200;
    let purged = coordinator.purge_terminated_sessions(future, 3600);
    assert_eq!(
        purged, 1,
        "only id1 should be purged (it's terminal and stale)"
    );
    assert_eq!(
        coordinator.session_count(),
        1,
        "id2 still active — must survive purge"
    );
}

#[test]
fn test_purge_does_not_remove_active_sessions() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (_id, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    // Advance time far past any reasonable TTL — but session is still active (Setup phase)
    let purged = coordinator.purge_terminated_sessions(now + 86400, 0);
    assert_eq!(purged, 0, "active sessions must never be purged");
    assert_eq!(coordinator.session_count(), 1);
}

#[test]
fn test_purge_with_zero_age_removes_all_terminals() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (id1, _, _) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    coordinator.abort(&id1, "test", now).unwrap();
    coordinator.record_refunds(&id1, now).unwrap();

    // max_age = 0: any terminal session updated at or before `now` is stale
    let purged = coordinator.purge_terminated_sessions(now + 1, 0);
    assert_eq!(purged, 1);
    assert_eq!(coordinator.session_count(), 0);
}

#[test]
fn test_secret_generated_by_osrng_is_nonzero() {
    // Basic sanity check for the OsRng entropy source.
    // The chance of generating all-zero bytes with a real CSPRNG is 2^-256.
    let secret = HtlcSecret::generate();
    assert_ne!(
        secret.0, [0u8; 32],
        "OsRng must never produce all-zero bytes"
    );
    // Hash must also be non-zero
    let hash = secret.hash();
    assert_ne!(hash.0, [0u8; 32]);
}

#[test]
fn test_secret_generated_by_osrng_is_unique() {
    // Generate 10 secrets without sleeping — OsRng is not time-based,
    // so uniqueness depends only on OS entropy.
    let secrets: Vec<HtlcSecret> = (0..10).map(|_| HtlcSecret::generate()).collect();
    for i in 0..secrets.len() {
        for j in (i + 1)..secrets.len() {
            assert_ne!(
                secrets[i].0, secrets[j].0,
                "OsRng-generated secrets at indices {i} and {j} are identical — entropy failure"
            );
        }
    }
}

// ── HTLC Replay Protection (cross-session) ───────────────────────────────────

#[test]
fn test_htlc_wrong_secret_is_rejected() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, _correct_secret, _hash) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    let session_hash = coordinator.get_session(&session_id).unwrap().hash_lock;
    let fast_htlc = HtlcRecord {
        id: HtlcId(b"f1".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![],
            hash_lock: session_hash,
            timelock: now + 3600,
            asset: vec![0u8; 32],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_fast(&session_id, fast_htlc, now)
        .unwrap();
    let slow_htlc = HtlcRecord {
        id: HtlcId(b"s1".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock: session_hash,
            timelock: now + 7200,
            asset: vec![0u8; 20],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, slow_htlc, now)
        .unwrap();
    coordinator.begin_flash_execution(&session_id, now).unwrap();
    if coordinator.get_session(&session_id).unwrap().phase == SwapPhase::ExecutingFlashLegs {
        coordinator
            .record_leg_outcome(
                &session_id,
                FlashLegOutcome::Success {
                    tx_hash: vec![0xAB; 32],
                    gas_used: 1_000,
                    output_amount: 1_000,
                    premium_paid: 0,
                },
                now,
            )
            .ok();
    }
    coordinator.begin_settlement(&session_id, now).unwrap();

    // Present a completely different (wrong) secret
    let wrong_secret = HtlcSecret::generate();
    let result = coordinator.record_fast_claim(&session_id, wrong_secret, now);
    assert!(result.is_err(), "Wrong secret must be rejected");
}

#[test]
fn test_htlc_secret_replay_same_session_is_rejected() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1700000000u64;

    let (session_id, secret, _hash) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    let session_hash = coordinator.get_session(&session_id).unwrap().hash_lock;
    let fast_htlc = HtlcRecord {
        id: HtlcId(b"f2".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![],
            hash_lock: session_hash,
            timelock: now + 3600,
            asset: vec![0u8; 32],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_fast(&session_id, fast_htlc, now)
        .unwrap();
    let slow_htlc = HtlcRecord {
        id: HtlcId(b"s2".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock: session_hash,
            timelock: now + 7200,
            asset: vec![0u8; 20],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, slow_htlc, now)
        .unwrap();
    coordinator.begin_flash_execution(&session_id, now).unwrap();
    if coordinator.get_session(&session_id).unwrap().phase == SwapPhase::ExecutingFlashLegs {
        coordinator
            .record_leg_outcome(
                &session_id,
                FlashLegOutcome::Success {
                    tx_hash: vec![0xBC; 32],
                    gas_used: 1_000,
                    output_amount: 1_000,
                    premium_paid: 0,
                },
                now,
            )
            .ok();
    }
    coordinator.begin_settlement(&session_id, now).unwrap();

    // First claim succeeds
    coordinator
        .record_fast_claim(&session_id, secret.clone(), now)
        .expect("First claim with correct secret must succeed");

    // Replay of the same secret must be rejected
    let replay = coordinator.record_fast_claim(&session_id, secret, now);
    assert!(replay.is_err(), "Replay of own secret must be rejected");
}

fn make_fast_htlc(hash_lock: HtlcHash, now: u64) -> HtlcRecord {
    HtlcRecord {
        id: HtlcId(b"fast-phase".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![],
            hash_lock,
            timelock: now + 3600,
            asset: vec![0u8; 32],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    }
}

#[test]
fn test_session_id_uses_full_blake2b_hex_not_hash_lock() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1_700_000_000u64;

    let (session_id, _secret, hash_lock) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    assert!(session_id.starts_with("swap-"));
    assert_eq!(session_id.len(), 69, "expected swap- + 64 hex chars");
    assert!(session_id[5..].chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_ne!(session_id, format!("swap-{}", hash_lock.to_hex()));
}

#[test]
fn test_phase_guards_block_out_of_order_mutators() {
    let mut coordinator = SwapCoordinator::with_default_config();
    let now = 1_700_000_000u64;

    let (session_id, secret, hash_lock) = coordinator
        .setup_swap(VmTarget::Svm, VmTarget::Evm { chain_id: 1 }, vec![], now)
        .unwrap();

    coordinator
        .record_htlc_fast(&session_id, make_fast_htlc(hash_lock, now), now)
        .unwrap();

    let first_slow_lock = HtlcRecord {
        id: HtlcId(b"slow-phase-1".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock,
            timelock: now + 7200,
            asset: vec![0u8; 20],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [0u8; 32],
    };
    coordinator
        .record_htlc_slow(&session_id, first_slow_lock, now)
        .unwrap();

    let duplicate_slow_lock = HtlcRecord {
        id: HtlcId(b"slow-phase-2".to_vec()),
        params: HtlcCreateParams {
            vm: VmTarget::Evm { chain_id: 1 },
            recipient: vec![],
            hash_lock,
            timelock: now + 7200,
            asset: vec![0u8; 20],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 101,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [1u8; 32],
    };
    let slow_after_lock_phase = coordinator.record_htlc_slow(&session_id, duplicate_slow_lock, now);
    assert!(matches!(
        slow_after_lock_phase,
        Err(CoordinatorError::InvalidPhaseTransition { .. })
    ));

    let fast_claim_too_early = coordinator.record_fast_claim(&session_id, secret.clone(), now);
    assert!(matches!(
        fast_claim_too_early,
        Err(CoordinatorError::InvalidPhaseTransition { .. })
    ));

    let slow_claim_too_early = coordinator.record_slow_claim(&session_id, now);
    assert!(matches!(
        slow_claim_too_early,
        Err(CoordinatorError::InvalidPhaseTransition { .. })
    ));

    let refunds_too_early = coordinator.record_refunds(&session_id, now);
    assert!(matches!(
        refunds_too_early,
        Err(CoordinatorError::InvalidPhaseTransition { .. })
    ));

    assert_eq!(
        coordinator.get_session(&session_id).unwrap().phase,
        SwapPhase::HtlcsLocked,
        "out-of-order mutators must not advance the state machine"
    );
}
