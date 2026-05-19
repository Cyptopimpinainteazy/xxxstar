//! Fuzz target: VRF proof verification
//!
//! Feeds arbitrary proofs and verification data to VRF verify function
//! to find crashes or verification bypasses.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_vrf::{VrfProof, VrfProvider, VrfPublicKey};

fuzz_target!(|data: &[u8]| {
    if data.len() < 128 {
        return;
    }

    // Try to decode a VrfProof from the data
    let proof_data = &data[..96]; // 32 + 64 bytes
    if let Ok(proof) = VrfProof::decode(&mut &proof_data[..]) {
        // Extract seed (next 32 bytes)
        let mut seed = [0u8; 32];
        if data.len() >= 128 {
            seed.copy_from_slice(&data[96..128]);
        }

        // Extract public key (next 32 bytes, if available)
        let public_key = if data.len() >= 160 {
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&data[128..160]);
            VrfPublicKey(key_bytes)
        } else {
            // Use derived key for consistency
            let provider = x3_vrf::get_vrf_provider();
            let secret_key = x3_vrf::VrfSecretKey([0u8; 64]);
            provider.derive_public_key(&secret_key)
        };

        // Get the VRF provider
        let provider = x3_vrf::get_vrf_provider();

        // Test VRF verify - should not panic
        let is_valid = provider.verify(&proof, &seed, &public_key);

        // INVARIANT: Verification should be deterministic
        let is_valid2 = provider.verify(&proof, &seed, &public_key);
        assert_eq!(is_valid, is_valid2, "Verification must be deterministic");

        // INVARIANT: Verification result should be boolean (no panics)
        let _ = is_valid; // Just ensure it's accessible

        // Test with generated proof (if we can generate one)
        if let Ok(generated_proof) = provider.prove(&seed) {
            let should_be_valid = provider.verify(&generated_proof, &seed, &public_key);
            // Note: This might not be valid if public key doesn't match, but should not panic
            let _ = should_be_valid;
        }
    }
});