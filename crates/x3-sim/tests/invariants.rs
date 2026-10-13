//! Proves the invariant checker is not vacuous.
//!
//! A checker that never fires is worse than no checker: it turns "we ran the
//! simulator" into false confidence. Each test below constructs the exact state
//! a bug would produce and requires the checker to name it.

use x3_cross_vm_coordinator::{
    CoordinatorOperation, CoordinatorOperationReceipt, HtlcCreateParams, HtlcHash, HtlcId,
    HtlcRecord, HtlcStatus, SwapPhase, SwapSession, VmTarget,
};
use x3_sim::{check_session, check_sessions, Violation};

const NOW: u64 = 1_700_000_000;

fn htlc(id: u8, status: HtlcStatus, lock: HtlcHash, timelock: u64) -> HtlcRecord {
    HtlcRecord {
        id: HtlcId(vec![id]),
        params: HtlcCreateParams {
            vm: VmTarget::X3Vm,
            recipient: vec![0x11; 32],
            hash_lock: lock,
            timelock,
            asset: vec![0xAA; 32],
            amount: 1_000,
        },
        status,
        created_at_block: 100,
        confirmations_required: 1,
        confirmations: 1,
        params_hash: [id; 32],
    }
}

fn receipt(operation: CoordinatorOperation, fingerprint_byte: u8) -> CoordinatorOperationReceipt {
    CoordinatorOperationReceipt {
        operation,
        evidence_fingerprint: [fingerprint_byte; 32],
        completed_at: NOW,
    }
}

fn base_session(id: &str) -> SwapSession {
    SwapSession {
        session_id: id.to_string(),
        hash_lock: HtlcHash([0x55; 32]),
        htlc_fast: None,
        htlc_slow: None,
        flash_legs: Vec::new(),
        leg_outcomes: Vec::new(),
        phase: SwapPhase::Setup,
        timelock_fast: NOW + 3_600,
        timelock_slow: NOW + 7_200,
        created_at: NOW,
        updated_at: NOW,
        operation_journal: Vec::new(),
        requires_merkle_verification: false,
    }
}

fn codes(violations: &[Violation]) -> Vec<&'static str> {
    violations.iter().map(|v| v.code).collect()
}

#[test]
fn a_healthy_completed_swap_has_no_violations() {
    let lock = HtlcHash([0x55; 32]);
    let mut session = base_session("healthy-complete");
    session.phase = SwapPhase::Complete;
    session.htlc_fast = Some(htlc(1, HtlcStatus::Claimed, lock, NOW + 3_600));
    session.htlc_slow = Some(htlc(2, HtlcStatus::Claimed, lock, NOW + 7_200));
    session.operation_journal = vec![
        receipt(CoordinatorOperation::FastHtlcLock, 1),
        receipt(CoordinatorOperation::SlowHtlcLock, 2),
        receipt(CoordinatorOperation::FastClaim, 3),
        receipt(CoordinatorOperation::SlowClaim, 4),
    ];

    let found = check_session(&session);
    assert!(
        found.is_empty(),
        "a correct finished swap must not be flagged: {:?}",
        codes(&found)
    );
}

/// The regression this simulator was built to catch: a swap that was claimed
/// and later refunded. `record_refunds` overwrites both leg statuses, so the
/// status fields alone look like a clean refund — the journal is the evidence.
#[test]
fn refund_after_claim_is_detected_from_the_journal() {
    let lock = HtlcHash([0x55; 32]);
    let mut session = base_session("refund-after-claim");
    session.phase = SwapPhase::Refunded;
    session.htlc_fast = Some(htlc(1, HtlcStatus::Refunded, lock, NOW + 3_600));
    session.htlc_slow = Some(htlc(2, HtlcStatus::Refunded, lock, NOW + 7_200));
    session.operation_journal = vec![
        receipt(CoordinatorOperation::FastHtlcLock, 1),
        receipt(CoordinatorOperation::SlowHtlcLock, 2),
        receipt(CoordinatorOperation::FastClaim, 3),
        receipt(CoordinatorOperation::RefundBoth, 4),
    ];

    let found = check_session(&session);
    assert!(
        codes(&found).contains(&"REFUND_AFTER_CLAIM"),
        "the journal records a claim followed by a refund; got {:?}",
        codes(&found)
    );
}

#[test]
fn one_leg_claimed_and_the_other_refunded_is_detected() {
    let lock = HtlcHash([0x55; 32]);
    let mut session = base_session("claim-refund-mix");
    session.phase = SwapPhase::ClaimingSlow;
    session.htlc_fast = Some(htlc(1, HtlcStatus::Claimed, lock, NOW + 3_600));
    session.htlc_slow = Some(htlc(2, HtlcStatus::Refunded, lock, NOW + 7_200));

    assert!(codes(&check_session(&session)).contains(&"CLAIM_REFUND_MIX"));
}

#[test]
fn a_phase_that_outran_its_htlc_record_is_detected() {
    let mut session = base_session("lost-write");
    session.phase = SwapPhase::LegsComplete;
    // htlc_fast is None: the record for the leg that got us here is gone.

    assert!(codes(&check_session(&session)).contains(&"PHASE_WITHOUT_FAST_HTLC"));
}

#[test]
fn an_inverted_timelock_pair_is_detected() {
    let mut session = base_session("timelock-inverted");
    session.timelock_fast = NOW + 7_200;
    session.timelock_slow = NOW + 3_600;

    assert!(codes(&check_session(&session)).contains(&"TIMELOCK_ORDER_INVERTED"));
}

#[test]
fn a_duplicated_journal_entry_is_detected() {
    let mut session = base_session("journal-duplicate");
    session.operation_journal = vec![
        receipt(CoordinatorOperation::FastHtlcLock, 1),
        receipt(CoordinatorOperation::FastHtlcLock, 1),
    ];

    assert!(codes(&check_session(&session)).contains(&"DUPLICATE_JOURNAL_ENTRY"));
}

#[test]
fn two_sessions_settling_one_hash_lock_is_detected() {
    let lock = HtlcHash([0x77; 32]);
    let mut first = base_session("double-settle-a");
    first.hash_lock = lock;
    first.phase = SwapPhase::Complete;
    first.htlc_fast = Some(htlc(1, HtlcStatus::Claimed, lock, NOW + 3_600));
    first.htlc_slow = Some(htlc(2, HtlcStatus::Claimed, lock, NOW + 7_200));

    let mut second = first.clone();
    second.session_id = "double-settle-b".to_string();

    let found = check_sessions(&[first, second]);
    assert!(
        codes(&found).contains(&"DOUBLE_SETTLE"),
        "one hash lock completed two swaps; got {:?}",
        codes(&found)
    );
    // The violation must blame a session that actually exists: the simulator
    // keys its before/after state by session id, so a synthetic lock id would
    // make that lookup miss and leave the packet unattributed.
    let double = found
        .iter()
        .find(|violation| violation.code == "DOUBLE_SETTLE")
        .expect("DOUBLE_SETTLE is reported");
    assert!(
        ["double-settle-a", "double-settle-b"].contains(&double.session_id.as_str()),
        "DOUBLE_SETTLE must blame a real session, got `{}`",
        double.session_id
    );
    assert!(
        double.detail.contains("2 sessions completed"),
        "the detail must keep the group size: {}",
        double.detail
    );
}
