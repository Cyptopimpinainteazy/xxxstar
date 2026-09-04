//! Fuzz target: VRF proof generation
//!
//! Feeds arbitrary seeds to VRF prove function to find crashes,
//! panics, or unexpected behavior in randomness generation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use x3_vrf::VrfProvider;

fuzz_target!(|data: &[u8]| {
    // Need at least 32 bytes for seed
    if data.len() < 32 {
        return;
    }

    // Extract seed (first 32 bytes)
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&data[..32]);

    // Get the VRF provider
    let provider = x3_vrf::get_vrf_provider();

    // Test VRF prove - should not panic regardless of input
    let result = provider.prove(&seed);

    match result {
        Ok(proof) => {
            // INVARIANT: Proof output should be 32 bytes
            assert_eq!(proof.output.len(), 32, "VRF output must be 32 bytes");

            // INVARIANT: Proof data should be present
            assert_eq!(proof.proof.len(), 64, "VRF proof must be 64 bytes");

            // INVARIANT: Output should be deterministic for same seed
            let result2 = provider.prove(&seed).unwrap();
            assert_eq!(proof.output, result2.output, "VRF output must be deterministic");

            // INVARIANT: Proof should be verifiable (using same seed)
            let public_key = provider.derive_public_key(&x3_vrf::VrfSecretKey([0u8; 64])); // Mock key
            assert!(provider.verify(&proof, &seed, &public_key), "Generated proof should verify");
        }
        Err(_) => {
            // Errors are acceptable for invalid inputs
            // But ensure no panics occur
        }
    }
});