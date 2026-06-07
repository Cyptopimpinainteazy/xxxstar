//! Fraud-proof verifier — pure deterministic logic, no storage.
//!
//! Invariants:
//!   FRAUD-PROOF-001 — valid proposer-lie must be slashable
//!   FRAUD-PROOF-002 — fraud proof verification is deterministic
//!   FRAUD-PROOF-003 — accepted proof must be single-use (replay protection at call-site)

#![allow(dead_code)]

use super::scheduler_v1::scheduler_commitment_from_bytes;
use super::types::{DisputedBlockMeta, FraudProofV1, PROOF_TYPE_SCHED_MISMATCH_V1};
use super::witness_v1::WitnessError;
use sp_core::H256;
use sp_io::hashing::blake2_256;

// ── Verifier error ────────────────────────────────────────────────────────────

/// All reasons a fraud proof is rejected.
///
/// The runtime extrinsic maps each variant to a `DispatchError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// `proof_type` is not a recognised value.
    InvalidProofType,
    /// Witness bytes could not be decoded / canonically validated.
    InvalidWitnessEncoding(WitnessError),
    /// `observed_hash` does not match `disputed.scheduler_commitment`.
    CommitmentMismatch,
    /// The recomputed commitment equals the observed one — not fraudulent.
    NotFraudulent,
}

impl From<WitnessError> for VerifyError {
    fn from(e: WitnessError) -> Self {
        VerifyError::InvalidWitnessEncoding(e)
    }
}

// ── Proof-id derivation ───────────────────────────────────────────────────────

/// `proof_id = blake2_256(proof_type || block_hash || observed_hash || expected_hash || blake2_256(reexec_witness))`
///
/// Canonical, deterministic, used for replay protection storage key.
pub fn compute_proof_id<AccountId>(
    proof: &FraudProofV1<AccountId>,
    disputed_block_hash: H256,
) -> H256 {
    let witness_hash = H256(blake2_256(&proof.reexec_witness));
    let mut input = [0u8; 1 + 32 * 4];
    input[0] = proof.proof_type;
    input[1..33].copy_from_slice(disputed_block_hash.as_bytes());
    input[33..65].copy_from_slice(proof.observed_hash.as_bytes());
    input[65..97].copy_from_slice(proof.expected_hash.as_bytes());
    input[97..129].copy_from_slice(witness_hash.as_bytes());
    H256(blake2_256(&input))
}

// ── Scheduler mismatch verifier ───────────────────────────────────────────────

/// Verify a `SchedulerMismatchV1` fraud proof.
///
/// Returns `(proof_id, proposer_account)` on success.
/// All logic is pure (no storage reads); the caller handles replay protection and slashing.
///
/// # Parameters
/// - `proof` — submitted by reporter
/// - `disputed` — the block metadata the runtime must supply
/// - `max_tx_count` — runtime constant `MAX_TXS_PER_BLOCK`
pub fn verify_scheduler_mismatch_v1<AccountId: Clone>(
    proof: &FraudProofV1<AccountId>,
    disputed: &DisputedBlockMeta<AccountId>,
    max_tx_count: u32,
) -> Result<(H256, AccountId), VerifyError> {
    // ── 1. Proof type ─────────────────────────────────────────────────────────
    if proof.proof_type != PROOF_TYPE_SCHED_MISMATCH_V1 {
        return Err(VerifyError::InvalidProofType);
    }

    // ── 2. observed_hash must match what is in the block ─────────────────────
    if proof.observed_hash != disputed.scheduler_commitment {
        return Err(VerifyError::CommitmentMismatch);
    }

    // ── 3. Decode witness + validate canonicality + recompute ─────────────────
    let recomputed = scheduler_commitment_from_bytes(
        &proof.reexec_witness,
        disputed.rules_version,
        max_tx_count,
    )?;

    // ── 4. expected_hash must match what the reporter claims we recompute to ──
    if recomputed != proof.expected_hash {
        return Err(VerifyError::CommitmentMismatch);
    }

    // ── 5. Must actually be fraudulent ────────────────────────────────────────
    if recomputed == disputed.scheduler_commitment {
        return Err(VerifyError::NotFraudulent);
    }

    // ── 6. Compute proof_id for caller to use as replay-protection key ────────
    let proof_id = compute_proof_id(proof, disputed.block_hash);

    Ok((proof_id, disputed.proposer.clone()))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fraud_proofs::types::{
        DisputedBlockMeta, FraudProofV1, HeaderRef, PROOF_TYPE_SCHED_MISMATCH_V1,
    };
    use crate::fraud_proofs::witness_v1::{
        AccessKeyV1, AccessListV1, SchedulerWitnessV1, WITNESS_VERSION,
    };
    use codec::{Compact, Encode};

    type AccountId = u64;

    fn make_tx_id(i: u64) -> H256 {
        let mut b = [0u8; 32];
        b[..8].copy_from_slice(&i.to_be_bytes());
        H256(b)
    }

    fn build_witness(n: usize) -> SchedulerWitnessV1 {
        let tx_ids: Vec<H256> = (0..n as u64).map(make_tx_id).collect();
        let access_lists: Vec<AccessListV1> = (0..n)
            .map(|i| {
                let mut k = [0u8; 32];
                k[..8].copy_from_slice(&(i as u64).to_be_bytes());
                AccessListV1 {
                    access_count: Compact(1),
                    accesses: sp_std::vec![AccessKeyV1 {
                        domain: 0,
                        key: H256(k)
                    }],
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
            reserved: sp_std::vec![],
        }
    }

    fn make_disputed(scheduler_commitment: H256) -> DisputedBlockMeta<AccountId> {
        DisputedBlockMeta {
            block_hash: H256([0xAA; 32]),
            block_number: 42,
            rules_version: 1,
            scheduler_commitment,
            proposer: 99u64,
        }
    }

    fn make_proof(
        witness: &SchedulerWitnessV1,
        observed: H256,
        expected: H256,
    ) -> FraudProofV1<AccountId> {
        FraudProofV1 {
            proof_type: PROOF_TYPE_SCHED_MISMATCH_V1,
            header_ref: HeaderRef {
                block_number: 42,
                block_hash: H256([0xAA; 32]),
            },
            tx_set_commitment: H256::zero(),
            claimed_scheduler_commitment: observed,
            reexec_witness: witness.encode(),
            expected_hash: expected,
            observed_hash: observed,
            reporter: 1u64,
            nonce: 0,
        }
    }

    // ── FRAUD-PROOF-001: valid proof detected ─────────────────────────────────

    #[test]
    fn valid_proof_detects_mismatch() {
        let witness = build_witness(3);
        let real_commitment = witness.compute_commitments().unwrap().scheduler_commitment;

        // Forge a wrong commitment that will be "in the block"
        let forged = H256([0xFF; 32]);

        let proof = make_proof(&witness, forged, real_commitment);
        let disputed = make_disputed(forged);

        let result = verify_scheduler_mismatch_v1(&proof, &disputed, 1000);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let (_, proposer) = result.unwrap();
        assert_eq!(proposer, 99u64);
    }

    // ── FRAUD-PROOF-001 negative: not fraudulent ──────────────────────────────

    #[test]
    fn non_fraudulent_proof_rejected() {
        let witness = build_witness(3);
        let commitment = witness.compute_commitments().unwrap().scheduler_commitment;

        // Block has the correct commitment — not a lie
        let proof = make_proof(&witness, commitment, commitment);
        let disputed = make_disputed(commitment);

        let err = verify_scheduler_mismatch_v1(&proof, &disputed, 1000).unwrap_err();
        assert_eq!(err, VerifyError::NotFraudulent);
    }

    // ── FRAUD-PROOF-001 negative: wrong expected_hash ─────────────────────────

    #[test]
    fn wrong_expected_hash_rejected() {
        let witness = build_witness(3);
        let real = witness.compute_commitments().unwrap().scheduler_commitment;
        let forged_block = H256([0xFF; 32]);

        // Reporter claims wrong expected_hash
        let mut proof = make_proof(&witness, forged_block, real);
        proof.expected_hash = H256([0x11; 32]); // wrong

        let disputed = make_disputed(forged_block);
        let err = verify_scheduler_mismatch_v1(&proof, &disputed, 1000).unwrap_err();
        assert_eq!(err, VerifyError::CommitmentMismatch);
    }

    // ── FRAUD-PROOF-002: determinism ─────────────────────────────────────────

    #[test]
    fn verification_is_deterministic() {
        let witness = build_witness(4);
        let real = witness.compute_commitments().unwrap().scheduler_commitment;
        let forged = H256([0xBB; 32]);

        let proof = make_proof(&witness, forged, real);
        let disputed = make_disputed(forged);

        let r1 = verify_scheduler_mismatch_v1(&proof, &disputed, 1000);
        let r2 = verify_scheduler_mismatch_v1(&proof, &disputed, 1000);

        // Both calls must agree
        assert_eq!(r1.is_ok(), r2.is_ok());
        if let (Ok((id1, _)), Ok((id2, _))) = (r1, r2) {
            assert_eq!(id1, id2, "proof_id must be identical across calls");
        }
    }

    // ── Invalid proof type ────────────────────────────────────────────────────

    #[test]
    fn invalid_proof_type_rejected() {
        let witness = build_witness(2);
        let commitment = witness.compute_commitments().unwrap().scheduler_commitment;
        let mut proof = make_proof(&witness, H256([0xCC; 32]), commitment);
        proof.proof_type = 0x02; // unknown type

        let disputed = make_disputed(H256([0xCC; 32]));
        let err = verify_scheduler_mismatch_v1(&proof, &disputed, 1000).unwrap_err();
        assert_eq!(err, VerifyError::InvalidProofType);
    }

    // ── observed_hash mismatch ────────────────────────────────────────────────

    #[test]
    fn observed_hash_does_not_match_block_commitment() {
        let witness = build_witness(2);
        let real = witness.compute_commitments().unwrap().scheduler_commitment;

        let forged_block = H256([0xDD; 32]);
        let proof = make_proof(&witness, forged_block, real);

        // Block actually has a different scheduler_commitment
        let disputed = make_disputed(H256([0xEE; 32]));

        let err = verify_scheduler_mismatch_v1(&proof, &disputed, 1000).unwrap_err();
        assert_eq!(err, VerifyError::CommitmentMismatch);
    }

    // ── Proof-id stability ────────────────────────────────────────────────────

    #[test]
    fn proof_id_is_stable() {
        let witness = build_witness(3);
        let real = witness.compute_commitments().unwrap().scheduler_commitment;
        let forged = H256([0xFF; 32]);
        let proof = make_proof(&witness, forged, real);

        let id1 = compute_proof_id(&proof, H256([0xAA; 32]));
        let id2 = compute_proof_id(&proof, H256([0xAA; 32]));
        assert_eq!(id1, id2);

        // Different witness bytes → different id
        let witness2 = build_witness(4);
        let real2 = witness2.compute_commitments().unwrap().scheduler_commitment;
        let proof2 = make_proof(&witness2, forged, real2);
        let id3 = compute_proof_id(&proof2, H256([0xAA; 32]));
        assert_ne!(id1, id3);
    }
}
