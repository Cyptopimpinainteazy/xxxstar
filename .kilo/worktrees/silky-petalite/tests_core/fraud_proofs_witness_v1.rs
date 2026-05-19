//! Fraud Proof Witness v1 — Unit & Property Tests
//!
//! Invariants tested:
//!   WITNESS-CANON-001 — two different byte orderings of the same logical witness:
//!                       exactly one must pass decode_and_validate
//!   WITNESS-CANON-002 — identical witness on different calls produces identical commitments
//!   WITNESS-BOUNDS-001 — MAX_WITNESS_BYTES and MAX_ACCESSES_PER_TX caps enforced
//!   WITNESS-GRAPH-001 — canonical graph encoding is deterministic for identical input
//!   WITNESS-ORDER-001 — Kahn sort with tx_id tie-break is deterministic

// Bring the runtime crate in as a dependency. When run via `cargo test -p runtime`
// these tests compile as part of the runtime crate's test harness.

use codec::{Compact, Encode};
use sp_core::H256;

use runtime::fraud_proofs::witness_v1::{
    AccessKeyV1, AccessListV1, SchedulerWitnessV1, WitnessError, MAX_ACCESSES_PER_TX,
    MAX_WITNESS_BYTES, WITNESS_VERSION,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Build a minimal valid witness with `n` transactions, each accessing `accesses_per_tx`
/// storage keys.  All tx_ids and keys are laid out to respect sort order.
fn build_valid_witness(n: usize, accesses_per_tx: u32) -> SchedulerWitnessV1 {
    let tx_ids: Vec<H256> = (0..n)
        .map(|i| {
            let mut b = [0u8; 32];
            // first bytes encode index so they're ascending
            let idx_be = (i as u64).to_be_bytes();
            b[..8].copy_from_slice(&idx_be);
            H256(b)
        })
        .collect();

    let access_lists: Vec<AccessListV1> = (0..n)
        .map(|tx_i| {
            let accesses: Vec<AccessKeyV1> = (0..accesses_per_tx)
                .map(|ak_j| {
                    let mut b = [0u8; 32];
                    // make keys unique and ascending: encode (tx_i, ak_j)
                    let combined = ((tx_i as u64) << 32) | (ak_j as u64);
                    b[..8].copy_from_slice(&combined.to_be_bytes());
                    AccessKeyV1 { domain: 0u8, key: H256(b) }
                })
                .collect();
            AccessListV1 {
                access_count: Compact(accesses.len() as u32),
                accesses,
            }
        })
        .collect();

    SchedulerWitnessV1 {
        version: WITNESS_VERSION,
        rules_version: 1,
        tx_count: Compact(n as u32),
        tx_ids,
        access_lists,
        seed: None,
        reserved: Vec::new(),
    }
}

// ── Validity tests ─────────────────────────────────────────────────────────────

#[test]
fn valid_witness_roundtrip() {
    let w = build_valid_witness(4, 2);
    let encoded = w.encode();
    let result = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000);
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[test]
fn valid_witness_empty_tx_set() {
    let w = build_valid_witness(0, 0);
    let encoded = w.encode();
    let result = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000);
    assert!(result.is_ok());
}

// ── WITNESS-CANON-001: canonicality enforcement ────────────────────────────────

#[test]
fn canon_001_unsorted_tx_ids_rejected() {
    // Swap first two tx_ids so they're no longer ascending
    let mut w = build_valid_witness(4, 1);
    w.tx_ids.swap(0, 1);
    let encoded = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::TxIdsNotSorted);
}

#[test]
fn canon_001_duplicate_tx_ids_rejected() {
    let mut w = build_valid_witness(4, 1);
    w.tx_ids[1] = w.tx_ids[0]; // duplicate
    let encoded = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::TxIdsNotSorted);
}

#[test]
fn canon_001_unsorted_access_list_rejected() {
    let mut w = build_valid_witness(3, 3);
    // Swap first two accesses in the first tx's list → no longer sorted
    w.access_lists[0].accesses.swap(0, 1);
    let encoded = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::AccessListNotSorted);
}

#[test]
fn canon_001_duplicate_access_key_rejected() {
    let mut w = build_valid_witness(3, 3);
    // Make first two accesses identical
    w.access_lists[0].accesses[1] = w.access_lists[0].accesses[0].clone();
    w.access_lists[0].access_count = Compact(3);
    let encoded = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::AccessListNotSorted);
}

// ── WITNESS-CANON-002: commitment determinism ──────────────────────────────────

#[test]
fn canon_002_commitments_are_deterministic() {
    let w = build_valid_witness(5, 3);
    let enc = w.encode();

    let w1 = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap();
    let w2 = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap();

    let c1 = w1.compute_commitments().unwrap();
    let c2 = w2.compute_commitments().unwrap();

    assert_eq!(c1.scheduler_commitment, c2.scheduler_commitment);
    assert_eq!(c1.graph_commitment, c2.graph_commitment);
    assert_eq!(c1.order_commitment, c2.order_commitment);
    assert_eq!(c1.tx_set_commitment, c2.tx_set_commitment);
}

#[test]
fn canon_002_different_tx_sets_produce_different_commitments() {
    let w1 = build_valid_witness(3, 2);
    let w2 = build_valid_witness(4, 2);

    let c1 = w1.compute_commitments().unwrap();
    let c2 = w2.compute_commitments().unwrap();

    assert_ne!(c1.scheduler_commitment, c2.scheduler_commitment);
    assert_ne!(c1.tx_set_commitment, c2.tx_set_commitment);
}

// ── WITNESS-BOUNDS-001: size caps ─────────────────────────────────────────────

#[test]
fn bounds_001_witness_too_large_rejected() {
    let oversized = vec![0u8; MAX_WITNESS_BYTES + 1];
    let err = SchedulerWitnessV1::decode_and_validate(&oversized, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::WitnessTooLarge);
}

#[test]
fn bounds_001_access_count_exceeded_rejected() {
    let mut w = build_valid_witness(3, 1);
    // Fake the access_count field to exceed MAX_ACCESSES_PER_TX
    w.access_lists[0].access_count = Compact(MAX_ACCESSES_PER_TX + 1);
    let enc = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::AccessCountExceeded);
}

#[test]
fn bounds_001_tx_count_overflow_rejected() {
    let w = build_valid_witness(5, 0);
    let enc = w.encode();
    // max_tx_count = 3 — reject because 5 > 3
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 1, 3).unwrap_err();
    assert_eq!(err, WitnessError::TxCountMismatch);
}

// ── Version and rules_version checks ─────────────────────────────────────────

#[test]
fn bad_version_rejected() {
    let mut w = build_valid_witness(2, 1);
    w.version = 0;
    let enc = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::BadVersion);
}

#[test]
fn rules_version_mismatch_rejected() {
    let w = build_valid_witness(2, 1);
    let enc = w.encode();
    // witness has rules_version=1, block expects 2
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 2, 1000).unwrap_err();
    assert_eq!(err, WitnessError::RulesVersionMismatch);
}

#[test]
fn reserved_non_empty_rejected() {
    let mut w = build_valid_witness(2, 1);
    w.reserved = vec![0x00]; // non-empty
    let enc = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::ReservedNonEmpty);
}

#[test]
fn tx_count_mismatch_rejected() {
    let mut w = build_valid_witness(4, 1);
    // Claim tx_count=5 but only 4 tx_ids
    w.tx_count = Compact(5);
    let enc = w.encode();
    let err = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000).unwrap_err();
    assert_eq!(err, WitnessError::TxCountMismatch);
}

// ── WITNESS-GRAPH-001: graph encoding stability ────────────────────────────────

#[test]
fn graph_001_same_access_pattern_produces_same_graph_commitment() {
    // Two witnesses with identical tx_ids and access_lists but re-encoded
    let w = build_valid_witness(4, 2);

    let enc1 = w.encode();
    let enc2 = w.encode();

    let w1 = SchedulerWitnessV1::decode_and_validate(&enc1, 1, 1000).unwrap();
    let w2 = SchedulerWitnessV1::decode_and_validate(&enc2, 1, 1000).unwrap();

    assert_eq!(
        w1.compute_commitments().unwrap().graph_commitment,
        w2.compute_commitments().unwrap().graph_commitment,
    );
}

#[test]
fn graph_001_no_conflicts_produces_empty_graph() {
    // All access lists are disjoint: no edges expected.
    // With our build_valid_witness each tx gets its own unique keys so no conflicts.
    let w = build_valid_witness(5, 3);
    let commitments = w.compute_commitments().unwrap();

    // Non-conflicting graph should differ from conflicting graph
    let w_conflict = build_conflicting_witness(3);
    let comms_conflict = w_conflict.compute_commitments().unwrap();

    // graph commitments must differ
    assert_ne!(commitments.graph_commitment, comms_conflict.graph_commitment);
}

/// Build a witness where all transactions access the same key (maximum conflict).
fn build_conflicting_witness(n: usize) -> SchedulerWitnessV1 {
    let shared_key = H256([0xABu8; 32]);
    let tx_ids: Vec<H256> = (0..n)
        .map(|i| {
            let mut b = [0u8; 32];
            b[..8].copy_from_slice(&(i as u64).to_be_bytes());
            H256(b)
        })
        .collect();

    let access_lists: Vec<AccessListV1> = (0..n)
        .map(|_| AccessListV1 {
            access_count: Compact(1),
            accesses: vec![AccessKeyV1 { domain: 0, key: shared_key }],
        })
        .collect();

    SchedulerWitnessV1 {
        version: WITNESS_VERSION,
        rules_version: 1,
        tx_count: Compact(n as u32),
        tx_ids,
        access_lists,
        seed: None,
        reserved: Vec::new(),
    }
}

// ── WITNESS-ORDER-001: topological sort determinism ───────────────────────────

#[test]
fn order_001_fully_conflicting_witness_order_is_deterministic() {
    let w1 = build_conflicting_witness(5);
    let w2 = build_conflicting_witness(5);

    let c1 = w1.compute_commitments().unwrap();
    let c2 = w2.compute_commitments().unwrap();

    assert_eq!(c1.order_commitment, c2.order_commitment);
}

#[test]
fn order_001_no_conflict_witness_order_is_deterministic() {
    let w1 = build_valid_witness(5, 2);
    let w2 = build_valid_witness(5, 2);

    let c1 = w1.compute_commitments().unwrap();
    let c2 = w2.compute_commitments().unwrap();

    assert_eq!(c1.order_commitment, c2.order_commitment);
}

#[test]
fn order_001_different_conflict_pattern_different_order() {
    let w_no_conflict   = build_valid_witness(3, 2);
    let w_full_conflict = build_conflicting_witness(3);

    let c1 = w_no_conflict.compute_commitments().unwrap();
    let c2 = w_full_conflict.compute_commitments().unwrap();

    // Different conflict structure → different order_commitment
    assert_ne!(c1.order_commitment, c2.order_commitment);
}

// ── Seed field ────────────────────────────────────────────────────────────────

#[test]
fn seed_some_roundtrip() {
    let mut w = build_valid_witness(3, 1);
    w.seed = Some(H256([0x55u8; 32]));
    let enc = w.encode();
    let decoded = SchedulerWitnessV1::decode_and_validate(&enc, 1, 1000);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap().seed, Some(H256([0x55u8; 32])));
}
