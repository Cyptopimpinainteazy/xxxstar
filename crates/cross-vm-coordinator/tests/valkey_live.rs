#![cfg(feature = "valkey")]

use x3_cross_vm_coordinator::{
    DistributedLeaseStore, ValkeyCasStore, ValkeyLeaseAuthority, ValkeySecretRegistry,
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
