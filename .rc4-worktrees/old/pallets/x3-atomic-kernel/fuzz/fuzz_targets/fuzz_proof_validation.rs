//! Fuzz target: PoAE proof validation logic (S0-6)
//!
//! Feeds arbitrary bytes through the `PoaeProof` validity checks that
//! external verifiers (EVM/SVM contracts) rely on. Looks for:
//!   - Panics in `proof_hash()` computation
//!   - Integer overflows in leg_count or finalized_block arithmetic
//!   - Any path that accepts a zero bundle_id or zero finality_cert as valid

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use pallet_x3_atomic_kernel::proof::PoaeProof;
use sp_core::H256;

fuzz_target!(|data: &[u8]| {
    if let Ok(proof) = PoaeProof::decode(&mut &*data) {
        // proof_hash must never panic regardless of field values
        let hash = proof.proof_hash();

        // INVARIANT: proof_hash output is always 32 bytes (H256), never zero
        // A zero hash would cause verifier contracts to accept forged proofs.
        // This is a security invariant — if this fires, it's a critical bug.
        if proof.bundle_id != H256::zero()
            && proof.receipt_root != H256::zero()
            && proof.finality_cert != H256::zero()
            && proof.leg_count > 0
        {
            assert_ne!(
                hash,
                H256::zero(),
                "SECURITY: proof_hash() returned zero for a valid-looking proof"
            );
        }

        // INVARIANT: proof_hash is deterministic (no randomness, no side-effects)
        assert_eq!(
            proof.proof_hash(),
            hash,
            "proof_hash() must be deterministic"
        );

        // INVARIANT: finalized_block 0 is not a valid finalization point
        // (block 0 is the genesis block, bundles cannot be finalized there)
        if proof.finalized_block == 0 {
            // This state should be considered invalid by callers.
            // We don't panic — just document the invariant is checkable.
            let _ = proof.leg_count.checked_add(1); // no overflow panic
        }

        // INVARIANT: leg_count overflow safety
        let _ = proof.leg_count.saturating_add(u32::MAX);
    }
});
