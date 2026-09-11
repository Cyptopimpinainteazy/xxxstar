#![cfg(feature = "valkey")]

use std::sync::{Arc, Barrier};
use std::thread;

use x3_cross_vm_coordinator::{
    ConcurrentSwapCoordinator, CoordinatorConfig, CoordinatorOperation, HtlcCreateParams, HtlcHash,
    HtlcId, HtlcRecord, HtlcSecret, HtlcStatus, InMemoryPersistence, SessionPersistence,
    SwapPhase, SwapSession, ValkeyDistributedCoordinator, VmTarget,
};

fn valkey_url() -> Option<String> {
    match std::env::var("X3_TEST_VALKEY_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("X3_REQUIRE_VALKEY_TESTS").ok().as_deref() == Some("1") => {
            panic!("X3_TEST_VALKEY_URL is required for distributed chaos tests")
        }
        Err(_) => None,
    }
}

fn namespace(test: &str) -> String {
    format!("x3-chaos-{}-{test}", std::process::id())
}

fn record(id: u8, hash_lock: HtlcHash, now: u64) -> HtlcRecord {
    HtlcRecord {
        id: HtlcId(vec![id; 32]),
        params: HtlcCreateParams {
            vm: VmTarget::Svm,
            recipient: vec![1; 32],
            hash_lock,
            timelock: now + 3_600,
            asset: vec![2; 32],
            amount: 1_000,
        },
        status: HtlcStatus::Funded,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [id; 32],
    }
}

fn session(
    id: &str,
    phase: SwapPhase,
    secret: &HtlcSecret,
    now: u64,
    fast: bool,
    slow: bool,
) -> SwapSession {
    SwapSession {
        session_id: id.to_string(),
        hash_lock: secret.hash(),
        htlc_fast: fast.then(|| record(1, secret.hash(), now)),
        htlc_slow: slow.then(|| record(2, secret.hash(), now)),
        flash_legs: vec![],
        leg_outcomes: vec![],
        phase,
        timelock_fast: now + 3_600,
        timelock_slow: now + 7_200,
        created_at: now,
        updated_at: now,
        operation_journal: vec![],
        requires_merkle_verification: false,
    }
}

fn coordinator(
    persistence: Arc<InMemoryPersistence>,
    url: &str,
    ns: &str,
) -> ValkeyDistributedCoordinator<InMemoryPersistence> {
    ValkeyDistributedCoordinator::with_namespace(
        ConcurrentSwapCoordinator::with_persistence(
            CoordinatorConfig::default(),
            persistence,
        ),
        url,
        ns,
    )
    .expect("Valkey coordinator")
}

#[test]
fn crash_after_fast_lock_then_restart_replay_is_idempotent() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x61; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&session("crash-lock", SwapPhase::Setup, &secret, now, false, false));

    let ns = namespace("crash-lock");
    {
        let proc_a = coordinator(persistence.clone(), &url, &ns);
        let lease = proc_a
            .acquire_session_lease("crash-lock", "proc-a", now, 5)
            .unwrap();
        proc_a
            .record_htlc_fast(&lease, record(3, secret.hash(), now), now + 1)
            .unwrap();
        // Simulated process death: no release, no graceful shutdown.
    }

    let proc_b = coordinator(persistence.clone(), &url, &ns);
    let lease_b = proc_b
        .acquire_session_lease("crash-lock", "proc-b", now + 5, 30)
        .unwrap();

    // Duplicate delivery of the exact already-observed lock must collapse.
    proc_b
        .record_htlc_fast(&lease_b, record(3, secret.hash(), now), now + 6)
        .unwrap();

    let state = persistence.load("crash-lock").unwrap();
    assert_eq!(
        state
            .operation_journal
            .iter()
            .filter(|r| r.operation == CoordinatorOperation::FastHtlcLock)
            .count(),
        1
    );
}

#[test]
fn delayed_stale_result_cannot_commit_after_takeover() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x62; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&session("stale-result", SwapPhase::Setup, &secret, now, false, false));

    let ns = namespace("stale-result");
    let proc_a = coordinator(persistence.clone(), &url, &ns);
    let proc_b = coordinator(persistence.clone(), &url, &ns);

    let stale = proc_a
        .acquire_session_lease("stale-result", "proc-a", now, 5)
        .unwrap();
    let current = proc_b
        .acquire_session_lease("stale-result", "proc-b", now + 5, 30)
        .unwrap();

    assert!(proc_a
        .record_htlc_fast(&stale, record(4, secret.hash(), now), now + 6)
        .is_err());

    proc_b
        .record_htlc_fast(&current, record(4, secret.hash(), now), now + 6)
        .unwrap();

    let state = persistence.load("stale-result").unwrap();
    assert_eq!(
        state
            .operation_journal
            .iter()
            .filter(|r| r.operation == CoordinatorOperation::FastHtlcLock)
            .count(),
        1
    );
}

#[test]
fn simultaneous_workers_have_exactly_one_active_lease_owner() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x63; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&session("lease-race", SwapPhase::Setup, &secret, now, false, false));

    let ns = namespace("lease-race");
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();

    for owner in ["proc-a", "proc-b"] {
        let url = url.clone();
        let ns = ns.clone();
        let barrier = barrier.clone();
        let persistence = persistence.clone();
        handles.push(thread::spawn(move || {
            let proc = coordinator(persistence, &url, &ns);
            barrier.wait();
            proc.acquire_session_lease("lease-race", owner, now, 30)
        }));
    }

    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
}

#[test]
fn same_secret_racing_across_sessions_has_one_global_owner() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x64; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());

    for id in ["secret-a", "secret-b"] {
        persistence.save(&session(
            id,
            SwapPhase::ClaimingFast,
            &secret,
            now,
            true,
            false,
        ));
    }

    let ns = namespace("secret-race");
    let proc_a = coordinator(persistence.clone(), &url, &ns);
    let proc_b = coordinator(persistence.clone(), &url, &ns);
    let lease_a = proc_a
        .acquire_session_lease("secret-a", "proc-a", now, 30)
        .unwrap();
    let lease_b = proc_b
        .acquire_session_lease("secret-b", "proc-b", now, 30)
        .unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let a = {
        let proc = proc_a.clone();
        let barrier = barrier.clone();
        let secret = secret.clone();
        thread::spawn(move || {
            barrier.wait();
            proc.record_fast_claim(&lease_a, secret, now + 1)
        })
    };
    let b = {
        let proc = proc_b.clone();
        let barrier = barrier.clone();
        let secret = secret.clone();
        thread::spawn(move || {
            barrier.wait();
            proc.record_fast_claim(&lease_b, secret, now + 1)
        })
    };

    barrier.wait();
    let results = [a.join().unwrap(), b.join().unwrap()];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
}

#[test]
fn completed_swap_cannot_later_become_refunded_after_takeover() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x65; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&session(
        "terminal-xor-complete",
        SwapPhase::ClaimingSlow,
        &secret,
        now,
        true,
        true,
    ));

    let ns = namespace("terminal-xor-complete");
    let proc_a = coordinator(persistence.clone(), &url, &ns);
    let lease_a = proc_a
        .acquire_session_lease("terminal-xor-complete", "proc-a", now, 5)
        .unwrap();
    proc_a.record_slow_claim(&lease_a, now + 1).unwrap();

    let proc_b = coordinator(persistence.clone(), &url, &ns);
    let lease_b = proc_b
        .acquire_session_lease("terminal-xor-complete", "proc-b", now + 5, 30)
        .unwrap();
    assert!(proc_b.record_refunds(&lease_b, now + 6).is_err());

    let final_state = persistence.load("terminal-xor-complete").unwrap();
    assert_eq!(final_state.phase, SwapPhase::Complete);
    assert_ne!(final_state.phase, SwapPhase::Refunded);
}

#[test]
fn refunded_swap_cannot_later_claim_after_takeover() {
    let Some(url) = valkey_url() else { return };
    let now = 1_700_000_000;
    let secret = HtlcSecret([0x66; 32]);
    let persistence = Arc::new(InMemoryPersistence::new());
    persistence.save(&session(
        "terminal-xor-refund",
        SwapPhase::Aborting,
        &secret,
        now,
        true,
        true,
    ));

    let ns = namespace("terminal-xor-refund");
    let proc_a = coordinator(persistence.clone(), &url, &ns);
    let lease_a = proc_a
        .acquire_session_lease("terminal-xor-refund", "proc-a", now, 5)
        .unwrap();
    proc_a.record_refunds(&lease_a, now + 1).unwrap();

    let proc_b = coordinator(persistence.clone(), &url, &ns);
    let lease_b = proc_b
        .acquire_session_lease("terminal-xor-refund", "proc-b", now + 5, 30)
        .unwrap();
    assert!(proc_b
        .record_fast_claim(&lease_b, secret, now + 6)
        .is_err());

    let final_state = persistence.load("terminal-xor-refund").unwrap();
    assert_eq!(final_state.phase, SwapPhase::Refunded);
    assert_ne!(final_state.phase, SwapPhase::Complete);
}
