// runtime/tests/fraud_proofs_proptest.rs
//
// Property-style tests for the fraud-proof backbone.
// Uses manual loop-based fuzzing (no external proptest crate) to avoid
// toolchain-level linker incompatibilities with regex-automata on rustc 1.93.1.
//
// ## Invariants covered
//  COMMITTEE-SELECT-001 — deterministic same seed
//  COMMITTEE-SELECT-002 — no duplicate validators in selection
//  COMMITTEE-SELECT-003 — output length is min(k, |eligible|)
//  FRAUD-PROOF-PALLET-001 — proof_id is stable/deterministic
//  FRAUD-PROOF-PALLET-002 — different block hashes yield different proof_ids
//  WITNESS-CANON-001      — commitment is deterministic across calls
//  FRAUD-PROOF-002        — verifier fn is referentially transparent
//  FRAUD-PROOF-003        — non-fraudulent proofs are always rejected
use sp_core::H256;
use x3_chain_runtime::fraud_proofs::{
    committee::select_committee,
    scheduler_v1::scheduler_commitment_from_bytes,
    types::{DisputedBlockMeta, FraudProofV1, HeaderRef, PROOF_TYPE_SCHED_MISMATCH_V1},
    verifier::{compute_proof_id, verify_scheduler_mismatch_v1},
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn seed_from_byte(b: u8) -> H256 {
    H256::from([b; 32])
}

fn zero_hash() -> H256 {
    H256::from([0u8; 32])
}

/// Minimal 1-tx no-deps witness that the reference scheduler accepts.
fn minimal_witness() -> Vec<u8> {
    vec![
        0x01, // version = 1
        0x04, // rules_version = 1 (compact)
        0x04, // tx_count = 1 (compact)
        // tx_ids[0]: 32 zero bytes
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, // access_list[0].access_count = 0
        0x00, // dep_edges = 0
        0x00, // reserved = 0
    ]
}

fn make_disputed(commitment: H256) -> DisputedBlockMeta<u64> {
    DisputedBlockMeta {
        block_hash: zero_hash(),
        block_number: 5,
        rules_version: 1,
        scheduler_commitment: commitment,
        proposer: 99u64,
    }
}

fn make_proof(witness: Vec<u8>, observed: H256, expected: H256) -> FraudProofV1<u64> {
    FraudProofV1 {
        proof_type: PROOF_TYPE_SCHED_MISMATCH_V1,
        header_ref: HeaderRef {
            block_number: 5,
            block_hash: zero_hash(),
        },
        reexec_witness: witness,
        tx_set_commitment: zero_hash(),
        claimed_scheduler_commitment: observed,
        expected_hash: expected,
        observed_hash: observed,
        reporter: 1u64,
        nonce: 0,
    }
}

// ── Committee selection properties ───────────────────────────────────────────

/// COMMITTEE-SELECT-001: select_committee is fully deterministic — same seed always
/// yields the same ordered result.
#[test]
fn prop_committee_deterministic() {
    for &n in &[1usize, 5, 15, 32, 49] {
        for &k in &[1usize, 5, 12, 20] {
            for seed_byte in (0u8..=255).step_by(17) {
                let validators: Vec<u64> = (0..n as u64).collect();
                let seed = seed_from_byte(seed_byte);
                let a = select_committee(&validators, seed, k);
                let b = select_committee(&validators, seed, k);
                assert_eq!(
                    a, b,
                    "COMMITTEE-SELECT-001: n={n} k={k} seed={seed_byte} — not deterministic"
                );
            }
        }
    }
}

/// COMMITTEE-SELECT-002: no validator appears more than once in the selection.
#[test]
fn prop_committee_no_duplicates() {
    for &n in &[2usize, 8, 20, 50, 60] {
        for &k in &[1usize, 7, 20, 25] {
            for seed_byte in (0u8..=255).step_by(31) {
                let validators: Vec<u64> = (0..n as u64).collect();
                let seed = seed_from_byte(seed_byte);
                let result = select_committee(&validators, seed, k);
                let mut sorted = result.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(
                    sorted.len(),
                    result.len(),
                    "COMMITTEE-SELECT-002: n={n} k={k} seed={seed_byte} — duplicates found"
                );
            }
        }
    }
}

/// COMMITTEE-SELECT-003: returned slice length == min(k, n).
#[test]
fn prop_committee_size() {
    for &n in &[1usize, 5, 20, 49] {
        for &k in &[1usize, 5, 15, 30] {
            for seed_byte in (0u8..=255).step_by(37) {
                let validators: Vec<u64> = (0..n as u64).collect();
                let seed = seed_from_byte(seed_byte);
                let result = select_committee(&validators, seed, k);
                assert_eq!(
                    result.len(),
                    k.min(n),
                    "COMMITTEE-SELECT-003: n={n} k={k} seed={seed_byte} — wrong length"
                );
            }
        }
    }
}

/// All returned validators come from the eligible set.
#[test]
fn prop_committee_members_are_eligible() {
    for &n in &[1usize, 10, 40] {
        for &k in &[1usize, 8, 20] {
            for seed_byte in (0u8..=255).step_by(43) {
                let validators: Vec<u64> = (0..n as u64).collect();
                let seed = seed_from_byte(seed_byte);
                let result = select_committee(&validators, seed, k);
                for &v in &result {
                    assert!(
                        validators.contains(&v),
                        "COMMITTEE: n={n} k={k} seed={seed_byte} — {v} not in eligible set"
                    );
                }
            }
        }
    }
}

// ── Commitment determinism properties ────────────────────────────────────────

/// WITNESS-CANON-001 / FRAUD-PROOF-002: commitment is referentially transparent —
/// calling scheduler_commitment_from_bytes twice on the same input yields equal results.
#[test]
fn prop_commitment_deterministic() {
    let witness = minimal_witness();
    for _ in 0..20 {
        let c1 = scheduler_commitment_from_bytes(&witness, 1, 256).expect("valid witness");
        let c2 = scheduler_commitment_from_bytes(&witness, 1, 256).expect("valid witness");
        assert_eq!(
            c1, c2,
            "WITNESS-CANON-001: commitment not deterministic across calls"
        );
    }
}

/// FRAUD-PROOF-PALLET-001: compute_proof_id is stable — same proof + same block_hash
/// always produces the same id, across 50 repeated calls.
#[test]
fn prop_proof_id_stable() {
    let witness = minimal_witness();
    let commitment = scheduler_commitment_from_bytes(&witness, 1, 256).expect("valid witness");
    let forged = H256::from([0xFEu8; 32]);
    let proof = make_proof(witness, forged, commitment);

    let reference_id = compute_proof_id(&proof, zero_hash());
    for _ in 0..50 {
        let id = compute_proof_id(&proof, zero_hash());
        assert_eq!(
            id, reference_id,
            "FRAUD-PROOF-PALLET-001: proof_id unstable across repeated calls"
        );
    }
}

/// FRAUD-PROOF-PALLET-002: different block hashes must yield different proof_ids.
#[test]
fn prop_proof_id_depends_on_block_hash() {
    let witness = minimal_witness();
    let commitment = scheduler_commitment_from_bytes(&witness, 1, 256).expect("valid witness");
    let forged = H256::from([0xFEu8; 32]);
    let proof = make_proof(witness, forged, commitment);

    for (a_byte, b_byte) in (0u8..=127).step_by(8).zip((128u8..=255).step_by(8)) {
        let hash_a = H256::from([a_byte; 32]);
        let hash_b = H256::from([b_byte; 32]);
        let id_a = compute_proof_id(&proof, hash_a);
        let id_b = compute_proof_id(&proof, hash_b);
        assert_ne!(
            id_a, id_b,
            "FRAUD-PROOF-PALLET-002: a={a_byte} b={b_byte} — same proof_id for different block hashes"
        );
    }
}

// ── Verifier properties ───────────────────────────────────────────────────────

/// FRAUD-PROOF-003: when recomputed commitment == disputed commitment, the verifier
/// must return an error (non-fraudulent situation).
#[test]
fn prop_non_fraudulent_always_rejected() {
    let witness = minimal_witness();
    let real_commitment = scheduler_commitment_from_bytes(&witness, 1, 256).expect("valid witness");

    // Disputed block has the CORRECT commitment → not fraudulent.
    let disputed = make_disputed(real_commitment);
    let proof = make_proof(witness, real_commitment, real_commitment);

    for _ in 0..10 {
        let result = verify_scheduler_mismatch_v1(&proof, &disputed, 256);
        assert!(
            result.is_err(),
            "FRAUD-PROOF-003: non-fraudulent proof must be rejected by verifier"
        );
    }
}
