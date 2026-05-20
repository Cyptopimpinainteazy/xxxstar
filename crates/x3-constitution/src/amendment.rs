//! Amendment proof type — required before any constitutional change may execute.

use crate::types::{ConstitutionHash, InvariantBounds};
use serde::{Deserialize, Serialize};

/// A proof that a proposed constitutional amendment is valid.
///
/// Properties proven (Article V):
/// 1. The new spec is a **refinement** of the prior spec (bounds only tighten).
/// 2. All **meta-invariants** (determinism, termination, proof-requirement) are preserved.
/// 3. The amendment **terminates** — no infinite governance loops introduced.
/// 4. The amendment is **safe** — it cannot produce an unreachable state.
///
/// In vΩ-1.0 this struct captures the commitment to a formal proof; the actual
/// Coq/Lean4 proof object is referenced by `proof_root` and verified off-chain
/// prior to submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmendmentProof {
    /// Hash of the prior (current) constitution being amended.
    pub prior_constitution_hash: ConstitutionHash,
    /// Hash of the proposed new constitution.
    pub proposed_constitution_hash: ConstitutionHash,
    /// The proposed new invariant bounds (must be a refinement of prior).
    pub proposed_bounds: InvariantBounds,
    /// Merkle root of the formal proof tree (Coq/Lean4 proof object).
    pub proof_root: [u8; 32],
    /// Commitment that meta-invariants are preserved (determinism, termination,
    /// proof-requirement for governance).
    pub meta_invariants_preserved: bool,
    /// Termination certificate — the amendment introduces no unbounded loops.
    pub termination_certified: bool,
    /// Safety certificate — no unreachable / deadlock states introduced.
    pub safety_certified: bool,
}

impl AmendmentProof {
    /// Returns `true` if all constitutional requirements for a valid amendment are met.
    ///
    /// This is the minimum set of conditions that must hold before `ConstitutionEngine`
    /// will permit applying the amendment on-chain.
    pub fn is_valid(
        &self,
        current_hash: &ConstitutionHash,
        prior_bounds: &InvariantBounds,
    ) -> bool {
        // 1. Must reference the actual current constitution
        &self.prior_constitution_hash == current_hash
        // 2. Proposed bounds must be a refinement (only tightening allowed)
        && self.proposed_bounds.is_refinement_of(prior_bounds)
        // 3. Meta-invariants must be preserved
        && self.meta_invariants_preserved
        // 4. Termination must be certified
        && self.termination_certified
        // 5. Safety must be certified
        && self.safety_certified
        // 6. Proof root must be non-zero (proof must exist)
        && self.proof_root != [0u8; 32]
    }
}

/// Verifies amendment proofs against the current constitution state.
#[derive(Debug, Default)]
pub struct AmendmentVerifier;

impl AmendmentVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify an amendment proof against the current constitution hash and bounds.
    ///
    /// Returns `Ok(())` if the amendment is valid and may be applied.
    /// Returns `Err(reason)` with a human-readable rejection reason otherwise.
    pub fn verify(
        &self,
        proof: &AmendmentProof,
        current_hash: &ConstitutionHash,
        prior_bounds: &InvariantBounds,
    ) -> Result<(), String> {
        if &proof.prior_constitution_hash != current_hash {
            return Err(format!(
                "amendment references prior hash {} but current constitution is {}",
                proof.prior_constitution_hash, current_hash
            ));
        }

        if !proof.proposed_bounds.is_refinement_of(prior_bounds) {
            return Err(
                "proposed bounds are not a valid refinement: bounds may only be narrowed, \
                 never widened (Article V)"
                    .to_string(),
            );
        }

        if !proof.meta_invariants_preserved {
            return Err(
                "meta-invariants not certified as preserved (Article V: determinism, \
                 termination, proof-requirement must be preserved)"
                    .to_string(),
            );
        }

        if !proof.termination_certified {
            return Err(
                "termination not certified (Article V: amendment must not introduce \
                 unbounded loops)"
                    .to_string(),
            );
        }

        if !proof.safety_certified {
            return Err(
                "safety not certified (Article V: amendment must not introduce \
                 unreachable or deadlock states)"
                    .to_string(),
            );
        }

        if proof.proof_root == [0u8; 32] {
            return Err(
                "proof root is zero — no formal proof object attached (Article V: \
                 unprovable amendments are invalid regardless of vote outcome)"
                    .to_string(),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::articles::ConstitutionManifest;

    fn valid_proof(hash: ConstitutionHash, bounds: &InvariantBounds) -> AmendmentProof {
        let tighter = InvariantBounds {
            max_supply: bounds.max_supply / 2,
            max_treasury_pct: 25,
            max_agent_count: bounds.max_agent_count / 2,
            max_proposal_depth: 1,
            max_agent_epoch_budget: bounds.max_agent_epoch_budget / 2,
        };
        AmendmentProof {
            prior_constitution_hash: hash,
            proposed_constitution_hash: ConstitutionHash([0xab; 32]),
            proposed_bounds: tighter,
            proof_root: [0xcc; 32],
            meta_invariants_preserved: true,
            termination_certified: true,
            safety_certified: true,
        }
    }

    #[test]
    fn valid_amendment_passes() {
        let manifest = ConstitutionManifest::default();
        let hash = manifest.constitution_hash();
        let bounds = InvariantBounds::default();
        let proof = valid_proof(hash, &bounds);
        let verifier = AmendmentVerifier::new();
        assert!(verifier.verify(&proof, &hash, &bounds).is_ok());
    }

    #[test]
    fn wrong_prior_hash_rejected() {
        let hash = ConstitutionHash([0x01; 32]);
        let wrong_hash = ConstitutionHash([0x02; 32]);
        let bounds = InvariantBounds::default();
        let mut proof = valid_proof(hash, &bounds);
        proof.prior_constitution_hash = wrong_hash;
        let verifier = AmendmentVerifier::new();
        assert!(verifier.verify(&proof, &hash, &bounds).is_err());
    }

    #[test]
    fn widening_bounds_rejected() {
        let hash = ConstitutionHash([0x01; 32]);
        let bounds = InvariantBounds::default();
        let mut proof = valid_proof(hash, &bounds);
        proof.proposed_bounds.max_supply = bounds.max_supply * 2; // widen — invalid
        let verifier = AmendmentVerifier::new();
        assert!(verifier.verify(&proof, &hash, &bounds).is_err());
    }

    #[test]
    fn missing_proof_root_rejected() {
        let hash = ConstitutionHash([0x01; 32]);
        let bounds = InvariantBounds::default();
        let mut proof = valid_proof(hash, &bounds);
        proof.proof_root = [0u8; 32]; // zero = no proof
        let verifier = AmendmentVerifier::new();
        assert!(verifier.verify(&proof, &hash, &bounds).is_err());
    }

    #[test]
    fn vote_alone_insufficient_without_proof_root() {
        // Simulates a scenario where governance voted but no formal proof was attached.
        let hash = ConstitutionHash([0x01; 32]);
        let bounds = InvariantBounds::default();
        let mut proof = valid_proof(hash, &bounds);
        proof.proof_root = [0u8; 32]; // governance voted but proof is zero
        let verifier = AmendmentVerifier::new();
        let result = verifier.verify(&proof, &hash, &bounds);
        assert!(
            result.is_err(),
            "voting alone must be insufficient per Article V"
        );
    }
}
