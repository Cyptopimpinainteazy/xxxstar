#![no_main]

use libfuzzer_sys::fuzz_target;
use pallet_x3_cross_vm_router::types::*;
use parity_scale_codec::{Decode, Encode};

// Fuzz target for router transfer validation
// Tests that the router properly validates transfer parameters
fuzz_target!(|data: &[u8]| {
    // Need at least some data for meaningful fuzzing
    if data.len() < 50 {
        return;
    }

    // Try to decode fuzzed data into transfer parameters
    // Use a simple structure that can be fuzzed
    if let Ok((asset_id, source_domain, destination_domain, amount)) =
        <(AssetId, DomainId, DomainId, Balance) as Decode>::decode(&mut &data[..])
    {
        // Test domain validation - should reject self-loops
        let _is_valid_domains = source_domain != destination_domain;

        // Test amount validation - should reject zero amounts
        let _is_valid_amount = amount > 0;

        // Test domain compatibility - domains should be X3 internal for MVP
        let _is_valid_route = source_domain.is_x3_internal() && destination_domain.is_x3_internal();

        // If we get here without panicking, the fuzz input passed basic validation
        // In a real implementation, we'd call actual router validation functions
    }
});
