//! Invariant checker.
//!
//! These are properties of the *coordinator's own state*, checked after every
//! simulated step. They reason about the resulting state rather than about
//! whether a call returned `Ok`, so a transition table that admitted a bad
//! transition would still be caught here.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use x3_cross_vm_coordinator::{CoordinatorOperation, HtlcStatus, SwapPhase, SwapSession};

/// A broken invariant, with enough detail to be a regression test name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub code: &'static str,
    pub session_id: String,
    pub detail: String,
}

fn is_claimed(status: HtlcStatus) -> bool {
    status == HtlcStatus::Claimed
}

fn is_refunded(status: HtlcStatus) -> bool {
    status == HtlcStatus::Refunded
}

/// Phases that can only be reached once the fast-chain HTLC exists.
fn phase_requires_fast_htlc(phase: SwapPhase) -> bool {
    matches!(
        phase,
        SwapPhase::HtlcsLocked
            | SwapPhase::ExecutingFlashLegs
            | SwapPhase::LegsComplete
            | SwapPhase::ClaimingFast
            | SwapPhase::ClaimingSlow
            | SwapPhase::Complete
    )
}

fn violation(code: &'static str, session: &SwapSession, detail: String) -> Violation {
    Violation {
        code,
        session_id: session.session_id.clone(),
        detail,
    }
}

/// Check one session in isolation.
pub fn check_session(session: &SwapSession) -> Vec<Violation> {
    let mut out = Vec::new();

    let fast = session.htlc_fast.as_ref().map(|h| h.status);
    let slow = session.htlc_slow.as_ref().map(|h| h.status);

    // I1: one leg claimed while the other was refunded is not an atomic swap.
    // Exactly one side moved value and the other gave up.
    if let (Some(f), Some(s)) = (fast, slow) {
        if (is_claimed(f) && is_refunded(s)) || (is_refunded(f) && is_claimed(s)) {
            out.push(violation(
                "CLAIM_REFUND_MIX",
                session,
                format!("fast={f:?} slow={s:?} — one leg claimed, the other refunded"),
            ));
        }
    }

    // I2: a completed swap means both legs were claimed.
    if session.phase == SwapPhase::Complete
        && !(fast == Some(HtlcStatus::Claimed) && slow == Some(HtlcStatus::Claimed))
    {
        out.push(violation(
            "COMPLETE_WITHOUT_BOTH_CLAIMS",
            session,
            format!("phase=Complete but fast={fast:?} slow={slow:?}"),
        ));
    }

    // I3: a refunded swap released nothing to a claimer.
    if session.phase == SwapPhase::Refunded
        && (fast.map(is_claimed).unwrap_or(false) || slow.map(is_claimed).unwrap_or(false))
    {
        out.push(violation(
            "REFUNDED_WITH_A_CLAIM",
            session,
            format!("phase=Refunded but fast={fast:?} slow={slow:?}"),
        ));
    }

    // I4: phases past locking presuppose the fast HTLC exists. Losing that
    // record while keeping the phase is the shape of a lost-write bug.
    if phase_requires_fast_htlc(session.phase) && session.htlc_fast.is_none() {
        out.push(violation(
            "PHASE_WITHOUT_FAST_HTLC",
            session,
            format!("phase={:?} but htlc_fast is None", session.phase),
        ));
    }

    // I5: the slow-chain timelock must sit strictly after the fast one, or the
    // refund ordering the protocol depends on is inverted.
    if session.timelock_slow <= session.timelock_fast {
        out.push(violation(
            "TIMELOCK_ORDER_INVERTED",
            session,
            format!(
                "fast={} slow={}",
                session.timelock_fast, session.timelock_slow
            ),
        ));
    }

    // I6: the idempotency journal must not record the same operation twice with
    // the same evidence — that would mean a "semantic operation" ran twice.
    let mut seen: HashSet<(String, [u8; 32])> = HashSet::new();
    for receipt in &session.operation_journal {
        let key = (
            format!("{:?}", receipt.operation),
            receipt.evidence_fingerprint,
        );
        if !seen.insert(key) {
            out.push(violation(
                "DUPLICATE_JOURNAL_ENTRY",
                session,
                format!("operation {:?} recorded twice", receipt.operation),
            ));
        }
    }

    // I8: the journal is durable evidence that survives a status overwrite.
    // `record_refunds` sets both legs to `Refunded`, so a refund that happened
    // *after* a claim leaves no trace in `status` — only in the journal. This
    // is the check that catches a swap paying out and then refunding.
    let journal_has_claim = session.operation_journal.iter().any(|r| {
        matches!(
            r.operation,
            CoordinatorOperation::FastClaim | CoordinatorOperation::SlowClaim
        )
    });
    let journal_has_refund = session
        .operation_journal
        .iter()
        .any(|r| r.operation == CoordinatorOperation::RefundBoth);
    if journal_has_claim && journal_has_refund {
        out.push(violation(
            "REFUND_AFTER_CLAIM",
            session,
            "journal records a claim and then a refund — both sides were paid".to_string(),
        ));
    }

    out
}

/// Check a whole ledger, including a property that spans sessions.
pub fn check_sessions(sessions: &[SwapSession]) -> Vec<Violation> {
    let mut out = Vec::new();
    for session in sessions {
        out.extend(check_session(session));
    }

    // I7: one hash lock must not settle twice. Two sessions sharing a lock can
    // only both complete if a secret was used against two different swaps.
    let mut by_lock: BTreeMap<[u8; 32], Vec<&SwapSession>> = BTreeMap::new();
    for session in sessions {
        by_lock
            .entry(session.hash_lock.0)
            .or_default()
            .push(session);
    }
    for (_lock, group) in by_lock {
        if group.len() < 2 {
            continue;
        }
        let mut completed: Vec<&SwapSession> = group
            .iter()
            .copied()
            .filter(|s| s.phase == SwapPhase::Complete)
            .collect();
        if completed.len() > 1 {
            // Blame a real session, never a synthetic lock id: consumers key
            // per-session state by session id (the simulator's before/after
            // snapshots), and an id no session ever had makes that lookup
            // miss. Sorting makes the choice deterministic; the detail stays
            // seed-independent (no session names, no lock bytes), so two
            // seeds that expose the same defect still dedupe to one
            // signature.
            completed.sort_by(|a, b| a.session_id.cmp(&b.session_id));
            if let Some(blamed) = completed.last() {
                out.push(Violation {
                    code: "DOUBLE_SETTLE",
                    session_id: blamed.session_id.clone(),
                    detail: format!(
                        "{} sessions completed against one hash lock",
                        completed.len()
                    ),
                });
            }
        }
    }

    out
}
