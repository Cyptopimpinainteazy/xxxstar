//! Fuzz target: Codec parsing for VRF structures
//!
//! Feeds arbitrary bytes through RandomnessRequest, RandomnessResult,
//! and VrfProof decoders to ensure no panics in serialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_vrf::{RandomnessRequest, RandomnessResult, VrfProof};

fuzz_target!(|data: &[u8]| {
    // Test RandomnessRequest decoding
    if let Ok(request) = RandomnessRequest::decode(&mut &*data) {
        // INVARIANT: Request ID should not be zero
        assert_ne!(request.request_id, sp_core::H256::zero(), "Request ID cannot be zero");

        // INVARIANT: Max fee should be reasonable (not excessively large)
        let max_fee_u128 = request.max_fee.low_u128();
        assert!(max_fee_u128 < 1_000_000_000_000_000_000_000, "Max fee unreasonably large");

        // INVARIANT: Block number should be reasonable
        assert!(request.block_number < 1_000_000_000, "Block number unreasonably large");

        // INVARIANT: Deterministic re-encoding
        let re_encoded = request.encode();
        let re_decoded = RandomnessRequest::decode(&mut &re_encoded[..]).unwrap();
        assert_eq!(request, re_decoded, "Codec must be deterministic");
    }

    // Test RandomnessResult decoding (using different offset in data)
    if data.len() > 100 {
        let result_data = &data[50..];
        if let Ok(result) = RandomnessResult::decode(&mut &result_data[..]) {
            // INVARIANT: Request ID should not be zero
            assert_ne!(result.request_id, sp_core::H256::zero(), "Result request ID cannot be zero");

            // INVARIANT: Fulfilled block should be >= request block (in valid cases)
            // We don't assert this as it could be violated in fuzzing

            // INVARIANT: Randomness should not be all zeros (though technically possible)
            // Allow it for fuzzing but note it

            // INVARIANT: Deterministic re-encoding
            let re_encoded = result.encode();
            let re_decoded = RandomnessResult::decode(&mut &re_encoded[..]).unwrap();
            assert_eq!(result, re_decoded, "Codec must be deterministic");
        }
    }

    // Test VrfProof decoding
    if data.len() > 200 {
        let proof_data = &data[150..];
        if let Ok(proof) = VrfProof::decode(&mut &proof_data[..]) {
            // INVARIANT: Proof arrays should not cause panics when accessed
            let _ = proof.output;
            let _ = proof.proof;

            // INVARIANT: Deterministic re-encoding
            let re_encoded = proof.encode();
            let re_decoded = VrfProof::decode(&mut &re_encoded[..]).unwrap();
            assert_eq!(proof, re_decoded, "Codec must be deterministic");
        }
    }
});