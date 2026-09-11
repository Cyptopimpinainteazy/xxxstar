#![cfg(feature = "valkey")]

use x3_cross_vm_coordinator::{
    CoordinatorOperation, OperationAttemptLedger, OperationAttemptStatus, DistributedLeaseStore,
    ValkeyAttemptStore, ValkeyCasStore, ValkeyLeaseAuthority, ValkeySecretRegistry,
};

#[cfg(feature = "canonical-proofs")]
use std::sync::{Arc, Barrier};
#[cfg(feature = "canonical-proofs")]
use std::thread;
#[cfg(feature = "canonical-proofs")]
use x3_cross_vm_coordinator::{
    ConcurrentSwapCoordinator, CoordinatorConfig, HtlcHash, InMemoryPersistence,
    SessionPersistence, SwapPhase, SwapSession, ValkeyDistributedCoordinator,
};
#[cfg(feature = "canonical-proofs")]
use x3_atomic_swap::{
    adapter::{FinalityProof, VmType},
    intent::{AtomicIntentBuilder, ChainKind, RefundPath},
    CrossDomainOperation, CrossDomainProofBundle,
};

fn test_url() -> Option<String> {
    match std::env::var("X3_TEST_VALKEY_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("X3_REQUIRE_VALKEY_TESTS").ok().as_deref() == Some("1") => {
            panic!("X3_TEST_VALKEY_URL is required")
        }
        Err(_) => None,
    }
}

fn namespace(suffix: &str) -> String {
    format!("x3-ci-{}-{suffix}", std::process::id())
}

#[test]
fn live_valkey_fencing_takeover_and_release_are_monotonic() {
    let Some(url) = test_url() else { return };
    let ns = namespace("lease");
    let a = ValkeyLeaseAuthority::with_namespace(&url, &ns).unwrap();
    let b = ValkeyLeaseAuthority::with_namespace(&url, &ns).unwrap();

    let first = a.acquire("swap-1", "proc-a", 100, 10).unwrap();
    assert!(b.acquire("swap-1", "proc-b", 105, 10).is_err());

    let second = b.acquire("swap-1", "proc-b", 110, 10).unwrap();
    assert_eq!(second.fence, first.fence + 1);
    assert!(a.validate(&first, 111).is_err());
    b.validate(&second, 111).unwrap();

    b.release(&second).unwrap();
    let third = a.acquire("swap-1", "proc-a", 112, 10).unwrap();
    assert_eq!(third.fence, second.fence + 1);
}

#[test]
fn live_valkey_secret_registry_is_globally_exclusive() {
    let Some(url) = test_url() else { return };
    let ns = namespace("secret");
    let a = ValkeySecretRegistry::with_namespace(&url, &ns).unwrap();
    let b = ValkeySecretRegistry::with_namespace(&url, &ns).unwrap();
    let hash = [0x55u8; 32];

    a.claim(hash, "swap-a").unwrap();
    b.claim(hash, "swap-a").expect("same-session retry");
    assert!(b.claim(hash, "swap-b").is_err());
    assert_eq!(a.owner(hash).unwrap().as_deref(), Some("swap-a"));
}

#[test]
fn live_valkey_binary_cas_matches_generic_contract() {
    let Some(url) = test_url() else { return };
    let store = ValkeyCasStore::with_namespace(&url, namespace("cas")).unwrap();
    let key = b"binary-key";
    let first = b"\0old\xff";
    let second = b"\0new\xfe";

    assert!(store.compare_and_set(key, None, first));
    assert_eq!(store.load(key).as_deref(), Some(first.as_slice()));
    assert!(!store.compare_and_set(key, None, second));
    assert!(!store.compare_and_set(key, Some(b"wrong"), second));
    assert!(store.compare_and_set(key, Some(first), second));
    assert_eq!(store.load(key).as_deref(), Some(second.as_slice()));
}


#[test]
fn live_valkey_attempt_history_survives_store_restart() {
    let Some(url) = test_url() else { return };
    let ns = namespace("attempt-history");

    let started = {
        let ledger = OperationAttemptLedger::new(
            ValkeyAttemptStore::with_namespace(&url, &ns).unwrap(),
        );
        let started = ledger
            .record_started(
                "swap-ledger",
                CoordinatorOperation::FastClaim,
                "attempt-1",
                "relayer-a",
                7,
                "ethereum",
                100,
            )
            .unwrap();
        ledger.record_broadcast(&started, "0xabc", 101).unwrap()
    };

    // Fresh client/store instance simulates process restart.
    let restarted = OperationAttemptLedger::new(
        ValkeyAttemptStore::with_namespace(&url, &ns).unwrap(),
    );
    let history = restarted.history("swap-ledger").unwrap();
    assert_eq!(history.len(), 2);
    assert!(history.iter().any(|entry| {
        entry.status == OperationAttemptStatus::Broadcast
            && entry.tx_id.as_deref() == Some("0xabc")
    }));

    let canonical = restarted
        .record_finalized(&started, [0x71; 32], 102)
        .unwrap();
    assert_eq!(canonical.proof_hash, [0x71; 32]);
}

#[test]
fn live_valkey_canonical_proof_hash_cannot_be_replaced() {
    let Some(url) = test_url() else { return };
    let ns = namespace("attempt-canonical");
    let ledger = OperationAttemptLedger::new(
        ValkeyAttemptStore::with_namespace(&url, &ns).unwrap(),
    );

    let first = ledger
        .record_started(
            "swap-ledger",
            CoordinatorOperation::SlowClaim,
            "attempt-a",
            "relayer-a",
            8,
            "solana",
            100,
        )
        .unwrap();
    let first = ledger.record_broadcast(&first, "sig-a", 101).unwrap();
    ledger.record_finalized(&first, [0x72; 32], 102).unwrap();

    let second = ledger
        .record_started(
            "swap-ledger",
            CoordinatorOperation::SlowClaim,
            "attempt-b",
            "relayer-b",
            9,
            "solana",
            103,
        )
        .unwrap();
    let second = ledger.record_broadcast(&second, "sig-b", 104).unwrap();
    assert!(ledger.record_finalized(&second, [0x73; 32], 105).is_err());

    let canonical = ledger
        .canonical("swap-ledger", CoordinatorOperation::SlowClaim)
        .unwrap()
        .unwrap();
    assert_eq!(canonical.proof_hash, [0x72; 32]);
    assert_eq!(canonical.tx_id, "sig-a");
}


#[cfg(feature = "canonical-proofs")]
#[test]
fn live_valkey_verified_bundle_becomes_settlement_ready_proof_set() {
    let Some(url) = test_url() else { return };
    let ns = namespace("proof-vault");
    let runtime_intent_id = [0xabu8; 32];

    let intent = AtomicIntentBuilder::new()
        .source_chain(ChainKind::X3)
        .destination_chain(ChainKind::Ethereum)
        .source_asset("X3")
        .destination_asset("ETH")
        .amount_in(1_000)
        .min_amount_out(900)
        .receiver("receiver")
        .hashlock([7u8; 32])
        .source_timeout(2_000)
        .destination_timeout(1_000)
        .refund_path(RefundPath {
            chain: ChainKind::X3,
            address: "refund".into(),
            asset: None,
        })
        .build(9001)
        .unwrap();

    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&SwapSession {
        session_id: "swap-proof-vault".into(),
        hash_lock: HtlcHash([7u8; 32]),
        htlc_fast: None,
        htlc_slow: None,
        flash_legs: vec![],
        leg_outcomes: vec![],
        phase: SwapPhase::ClaimingFast,
        timelock_fast: 2_000,
        timelock_slow: 1_000,
        created_at: 100,
        updated_at: 100,
        operation_journal: vec![],
        requires_merkle_verification: false,
    });

    let coordinator = ValkeyDistributedCoordinator::with_namespace(
        ConcurrentSwapCoordinator::with_persistence(
            CoordinatorConfig::default(),
            persistence,
        ),
        &url,
        &ns,
    )
    .unwrap();

    let lease = coordinator
        .acquire_session_lease("swap-proof-vault", "relayer-a", 100, 30)
        .unwrap();
    let started = coordinator
        .record_attempt_started(
            &lease,
            CoordinatorOperation::FastClaim,
            "claim-attempt-1",
            "eth-mainnet",
            101,
        )
        .unwrap();
    let broadcast = coordinator
        .record_attempt_broadcast(&lease, &started, "0xclaim", 102)
        .unwrap();

    let block_hash = "0xblock55".to_string();
    let bundle = CrossDomainProofBundle::new(
        &intent,
        runtime_intent_id,
        "eth-mainnet".into(),
        VmType::Evm,
        CrossDomainOperation::Claim,
        "0xclaim".into(),
        55,
        block_hash.clone(),
        vec![1, 2, 3],
        FinalityProof {
            chain_id: "eth-mainnet".into(),
            vm_type: VmType::Evm,
            tx_id: "0xclaim".into(),
            block_number: 55,
            block_hash,
            confirmations: 12,
            finalized: true,
            finality_source: "live-valkey-test".into(),
            safe_to_reveal_secret: true,
        },
    )
    .unwrap();

    let canonical = coordinator
        .record_attempt_finalized_with_bundle(
            &lease,
            &broadcast,
            &intent,
            runtime_intent_id,
            &bundle,
            103,
        )
        .unwrap();
    assert_eq!(canonical.proof_hash, bundle.proof_hash);

    // Exact rebind is idempotent.
    coordinator
        .bind_session_intent(&lease, &intent, runtime_intent_id, 104)
        .unwrap();

    // The same coordinator session can never be rebound to another runtime H256.
    assert!(coordinator
        .bind_session_intent(&lease, &intent, [0xcdu8; 32], 105)
        .is_err());

    let restored = coordinator
        .canonical_proof_bundle(bundle.proof_hash)
        .unwrap()
        .expect("bundle persisted by proof hash");
    assert_eq!(restored.proof_hash, bundle.proof_hash);
    restored.verify_runtime_binding(runtime_intent_id).unwrap();

    let set = coordinator
        .assemble_verified_claim_proof_set(
            "swap-proof-vault",
            &intent,
            runtime_intent_id,
            &[("eth-mainnet".into(), VmType::Evm)],
        )
        .unwrap();
    assert_eq!(set.runtime_intent_id, runtime_intent_id);
    assert_eq!(set.bundles.len(), 1);
    assert_eq!(set.bundles[0].proof_hash, bundle.proof_hash);

    let envelope = coordinator
        .claim_submission_envelope(
            "swap-proof-vault",
            &intent,
            runtime_intent_id,
            &[("eth-mainnet".into(), VmType::Evm)],
        )
        .unwrap();
    assert_eq!(envelope.call_index(), 33);
    assert!(envelope.scale_call_args().starts_with(&runtime_intent_id));
    assert_eq!(envelope.proof_hashes(), vec![bundle.proof_hash]);

    let prepared = coordinator
        .prepare_claim_submission(
            "swap-proof-vault",
            &intent,
            runtime_intent_id,
            &[("eth-mainnet".into(), VmType::Evm)],
            106,
        )
        .unwrap();

    // Simulate two processes broadcasting the exact same runtime call with
    // different transaction ids. Atomic compare-and-append permits one only.
    let barrier = Arc::new(Barrier::new(3));
    let a = {
        let coordinator = coordinator.clone();
        let lease = lease.clone();
        let prepared = prepared.clone();
        let barrier = barrier.clone();
        thread::spawn(move || {
            barrier.wait();
            coordinator.record_settlement_submission_broadcast(
                &lease,
                &prepared,
                "0xsubmit-a",
                107,
            )
        })
    };
    let b = {
        let coordinator = coordinator.clone();
        let lease = lease.clone();
        let prepared = prepared.clone();
        let barrier = barrier.clone();
        thread::spawn(move || {
            barrier.wait();
            coordinator.record_settlement_submission_broadcast(
                &lease,
                &prepared,
                "0xsubmit-b",
                107,
            )
        })
    };

    barrier.wait();
    let results = [a.join().unwrap(), b.join().unwrap()];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);

    let broadcast = results
        .into_iter()
        .find_map(Result::ok)
        .expect("one broadcast winner");
    assert_eq!(
        coordinator
            .settlement_submission_recovery(prepared.submission_id)
            .unwrap(),
        Some(x3_cross_vm_coordinator::SettlementOutboxRecovery::QueryBroadcastStatus)
    );

    let included = coordinator
        .record_settlement_submission_included(&broadcast, 777, 108)
        .unwrap();
    assert_eq!(
        coordinator
            .settlement_submission_recovery(prepared.submission_id)
            .unwrap(),
        Some(x3_cross_vm_coordinator::SettlementOutboxRecovery::ObserveSettlementState)
    );

    coordinator
        .record_settlement_terminal_observed(&included, 109)
        .unwrap();
    assert_eq!(
        coordinator
            .settlement_submission_recovery(prepared.submission_id)
            .unwrap(),
        Some(x3_cross_vm_coordinator::SettlementOutboxRecovery::Done)
    );
}
