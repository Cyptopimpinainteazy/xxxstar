//! Fuzz target: Codec parsing for x3-supply-ledger structures
//!
//! Feeds arbitrary bytes through pallet structure decoders
//! to ensure no panics in serialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::{Decode, Encode};
use x3_asset_kernel_types::{CrossVmAssetMessage, UniversalAssetKernel};

fuzz_target!(|data: &[u8]| {
    // Ensure SCALE roundtrip stability for universal kernel accounting state.
    if let Ok(kernel) = UniversalAssetKernel::decode(&mut &*data) {
        let re_encoded = kernel.encode();
        let re_decoded = UniversalAssetKernel::decode(&mut &re_encoded[..]).unwrap();
        assert_eq!(kernel, re_decoded, "Codec must be deterministic");
        let _ = kernel.check_invariant();
    }

    // Ensure SCALE roundtrip stability for cross-VM lifecycle messages.
    if let Ok(message) = CrossVmAssetMessage::decode(&mut &*data) {
        let re_encoded = message.encode();
        let re_decoded = CrossVmAssetMessage::decode(&mut &re_encoded[..]).unwrap();
        assert_eq!(message, re_decoded, "Codec must be deterministic");
        let _ = message.validate_at(0);
    }
});
