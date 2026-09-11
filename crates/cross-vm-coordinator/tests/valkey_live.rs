#![cfg(feature = "valkey")]

use x3_cross_vm_coordinator::{
    CoordinatorOperation, OperationAttemptLedger, OperationAttemptStatus, DistributedLeaseStore,
    ValkeyAttemptStore, ValkeyCasStore,
    ValkeyLeaseAuthority, ValkeySecretRegistry,
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
