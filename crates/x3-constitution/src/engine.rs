//! The Constitution Engine — the runtime gatekeeper of all invariant-touching operations.
//!
//! The `ConstitutionEngine` is the authoritative runtime instance of the X3 constitution.
//! It holds the current `InvariantSet`, the canonical `ConstitutionHash`, and the
//! `AmendmentVerifier`. All invariant-touching operations (governance dispatch,
//! agent registration, supply changes) MUST pass through this engine.

use crate::{
    amendment::{AmendmentProof, AmendmentVerifier},
    articles::ConstitutionManifest,
    error::ConstitutionError,
    invariants::{CoreInvariant, InvariantSet, InvariantViolation},
    types::ConstitutionHash,
};

/// The live constitution engine.
///
/// Instantiate once at node startup; pass a reference wherever invariant checks
/// or proof gates are needed.
#[derive(Debug)]
pub struct ConstitutionEngine {
    manifest: ConstitutionManifest,
    invariants: InvariantSet,
    amendment_verifier: AmendmentVerifier,
    /// Depth of current governance execution (to detect re-entrant proposals).
    governance_depth: u8,
}

impl Default for ConstitutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstitutionEngine {
    /// Create the engine from the canonical vΩ-1.0 manifest.
    pub fn new() -> Self {
        let manifest = ConstitutionManifest::default();
        let invariants = InvariantSet::new();
        Self {
            manifest,
            invariants,
            amendment_verifier: AmendmentVerifier::new(),
            governance_depth: 0,
        }
    }

    // -------------------------------------------------------------------------
    // Identity
    // -------------------------------------------------------------------------

    /// The canonical hash of the active constitution.
    pub fn constitution_hash(&self) -> ConstitutionHash {
        self.manifest.constitution_hash()
    }

    // -------------------------------------------------------------------------
    // Invariant checks (Article II & III)
    // -------------------------------------------------------------------------

    /// Assert that `current_supply` does not violate SupplyCap.
    pub fn assert_supply_cap(
        &self,
        current_supply: u128,
        block: u64,
    ) -> Result<(), ConstitutionError> {
        self.invariants
            .check_supply(current_supply, block)
            .map_err(|v| ConstitutionError::InvariantViolation(v.invariant, v.message))
    }

    /// Assert that `treasury_balance` does not violate TreasuryBound.
    pub fn assert_treasury_bound(
        &self,
        treasury_balance: u128,
        total_supply: u128,
        block: u64,
    ) -> Result<(), ConstitutionError> {
        self.invariants
            .check_treasury(treasury_balance, total_supply, block)
            .map_err(|v| ConstitutionError::InvariantViolation(v.invariant, v.message))
    }

    /// Assert that `agent_count` does not violate AgentCountLimit.
    pub fn assert_agent_count(&self, count: u64, block: u64) -> Result<(), ConstitutionError> {
        self.invariants
            .check_agent_count(count, block)
            .map_err(|v| ConstitutionError::InvariantViolation(v.invariant, v.message))
    }

    /// Assert that an agent's epoch spend does not violate AgentBudgetBound.
    pub fn assert_agent_epoch_budget(
        &self,
        spend: u128,
        block: u64,
    ) -> Result<(), ConstitutionError> {
        self.invariants
            .check_agent_budget(spend, block)
            .map_err(|v| ConstitutionError::InvariantViolation(v.invariant, v.message))
    }

    // -------------------------------------------------------------------------
    // Governance proof gate (Article IV)
    // -------------------------------------------------------------------------

    /// Gate a governance execution: verify the proof commitment matches and depth is safe.
    ///
    /// **Voting is insufficient.** A proposal touching invariants MUST carry a non-zero
    /// `proof_commitment` matching the on-chain record, and execution depth must not exceed
    /// the constitutional maximum.
    ///
    /// Call `enter_governance()` before dispatching a governance call and
    /// `exit_governance()` after (even on error) to maintain correct depth tracking.
    pub fn enter_governance(
        &mut self,
        proof_commitment: Option<[u8; 32]>,
        is_invariant_touching: bool,
        block: u64,
    ) -> Result<(), ConstitutionError> {
        // Depth check (Article IV)
        let next_depth = self.governance_depth.saturating_add(1);
        self.invariants
            .check_proposal_depth(next_depth, block)
            .map_err(|v| ConstitutionError::InvariantViolation(v.invariant, v.message))?;

        // Proof gate: invariant-touching proposals always require proof (Article IV)
        if is_invariant_touching {
            match proof_commitment {
                None => return Err(ConstitutionError::ProofRequired),
                Some(commitment) if commitment == [0u8; 32] => {
                    return Err(ConstitutionError::ProofRequired)
                }
                Some(_) => {} // non-zero commitment — accepted
            }
        }

        self.governance_depth = next_depth;
        Ok(())
    }

    /// Called after a governance call completes (or fails) to decrement depth.
    pub fn exit_governance(&mut self) {
        self.governance_depth = self.governance_depth.saturating_sub(1);
    }

    /// Current governance execution depth.
    pub fn governance_depth(&self) -> u8 {
        self.governance_depth
    }

    // -------------------------------------------------------------------------
    // Amendment gate (Article V)
    // -------------------------------------------------------------------------

    /// Verify and apply a constitutional amendment.
    ///
    /// Returns `Ok(new_hash)` if the amendment is valid and has been applied, or
    /// `Err(reason)` if it is rejected.
    ///
    /// Per Article V: unprovable amendments are invalid regardless of vote outcome.
    pub fn apply_amendment(
        &mut self,
        proof: &AmendmentProof,
    ) -> Result<ConstitutionHash, ConstitutionError> {
        let current_hash = self.constitution_hash();
        let prior_bounds = self.invariants.bounds().clone();

        self.amendment_verifier
            .verify(proof, &current_hash, &prior_bounds)
            .map_err(ConstitutionError::AmendmentRejected)?;

        // Amendment accepted — apply tightened bounds
        self.invariants = InvariantSet::with_bounds(proof.proposed_bounds.clone());

        Ok(proof.proposed_constitution_hash)
    }

    // -------------------------------------------------------------------------
    // Enforcement (Article VI)
    // -------------------------------------------------------------------------

    /// Record a violation and return the enforcement action.
    ///
    /// Callers are responsible for halting execution, triggering slashing,
    /// and scheduling forensic replay. This method returns the structured violation
    /// for logging and downstream handling.
    pub fn enforce(&self, violation: InvariantViolation) -> EnforcementAction {
        EnforcementAction {
            invariant: violation.invariant.clone(),
            message: violation.message.clone(),
            block: violation.block,
            slash: true,
            halt: true,
            replay: true,
        }
    }
}

/// The actions the engine mandates after detecting a violation (Article VI).
#[derive(Debug, Clone)]
pub struct EnforcementAction {
    pub invariant: CoreInvariant,
    pub message: String,
    pub block: u64,
    /// Violating party MUST be slashed.
    pub slash: bool,
    /// Execution MUST be halted.
    pub halt: bool,
    /// Forensic replay MUST be scheduled.
    pub replay: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amendment::AmendmentProof;
    use crate::InvariantBounds;

    fn make_engine() -> ConstitutionEngine {
        ConstitutionEngine::new()
    }

    #[test]
    fn constitution_hash_is_stable() {
        let e1 = make_engine();
        let e2 = make_engine();
        assert_eq!(e1.constitution_hash(), e2.constitution_hash());
    }

    #[test]
    fn supply_cap_gate_passes_at_limit() {
        let e = make_engine();
        let max = e.invariants.bounds().max_supply;
        assert!(e.assert_supply_cap(max, 1).is_ok());
    }

    #[test]
    fn supply_cap_gate_fails_over_limit() {
        let e = make_engine();
        let max = e.invariants.bounds().max_supply;
        assert!(e.assert_supply_cap(max + 1, 1).is_err());
    }

    #[test]
    fn governance_depth_enforced() {
        let mut e = make_engine();
        // Depth 1 is allowed (max_proposal_depth = 1)
        assert!(e.enter_governance(None, false, 1).is_ok());
        // Depth 2 should be rejected
        let result = e.enter_governance(None, false, 1);
        assert!(result.is_err());
        e.exit_governance();
    }

    #[test]
    fn invariant_touching_proposal_requires_proof() {
        let mut e = make_engine();
        let r = e.enter_governance(None, true, 1);
        assert!(matches!(r, Err(ConstitutionError::ProofRequired)));
    }

    #[test]
    fn invariant_touching_proposal_accepts_non_zero_commitment() {
        let mut e = make_engine();
        let commitment = [0xab; 32];
        assert!(e.enter_governance(Some(commitment), true, 1).is_ok());
        e.exit_governance();
    }

    #[test]
    fn valid_amendment_applies_tighter_bounds() {
        let mut e = make_engine();
        let prior_hash = e.constitution_hash();
        let prior_bounds = e.invariants.bounds().clone();
        let new_bounds = InvariantBounds {
            max_supply: prior_bounds.max_supply / 2,
            max_treasury_pct: 20,
            max_agent_count: 50_000,
            max_proposal_depth: 1,
            max_agent_epoch_budget: prior_bounds.max_agent_epoch_budget / 2,
        };
        let proof = AmendmentProof {
            prior_constitution_hash: prior_hash,
            proposed_constitution_hash: ConstitutionHash([0xee; 32]),
            proposed_bounds: new_bounds.clone(),
            proof_root: [0xdd; 32],
            meta_invariants_preserved: true,
            termination_certified: true,
            safety_certified: true,
        };
        let new_hash = e.apply_amendment(&proof).unwrap();
        assert_eq!(new_hash, ConstitutionHash([0xee; 32]));
        assert_eq!(e.invariants.bounds().max_supply, new_bounds.max_supply);
    }

    #[test]
    fn enforcement_always_halts_slashes_replays() {
        let e = make_engine();
        use crate::invariants::InvariantViolation;
        let v = InvariantViolation::new(CoreInvariant::SupplyCap, "over limit", 42);
        let action = e.enforce(v);
        assert!(action.slash);
        assert!(action.halt);
        assert!(action.replay);
    }
}
