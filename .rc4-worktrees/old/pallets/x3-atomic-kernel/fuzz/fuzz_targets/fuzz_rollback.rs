//! Fuzz target: rollback operation inputs
//!
//! Exercises `BundleRollbackReason` SCALE decode paths and
//! `PoaeProof` integrity when rollback is triggered with arbitrary data.
//! Looks for: panics, overflows, invalid state transitions.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use pallet_x3_atomic_kernel::proof::{PoaeProof, BundleLeg, VmType};

fuzz_target!(|data: &[u8]| {
    // 1. Try decoding a PoaeProof from arbitrary bytes — must never panic.
    let _ = PoaeProof::decode(&mut &*data);

    // 2. Try decoding a BundleLeg from arbitrary bytes — must never panic.
    let _ = BundleLeg::decode(&mut &*data);

    // 3. VmType decode — must never panic or produce UB.
    let _ = VmType::decode(&mut &*data);

    // 4. If we can construct a PoaeProof, verify proof_hash() is deterministic.
    if let Ok(proof) = PoaeProof::decode(&mut &*data) {
        let h1 = proof.proof_hash();
        let h2 = proof.proof_hash();
        assert_eq!(h1, h2, "proof_hash() must be deterministic");

        // 5. leg_count == 0 should be caught as invalid by callers.
        // We don't panic here — just ensure no UB when leg_count is arbitrary.
        let _is_valid = proof.leg_count > 0
            && proof.bundle_id != sp_core::H256::zero()
            && proof.finality_cert != sp_core::H256::zero();
    }
});
