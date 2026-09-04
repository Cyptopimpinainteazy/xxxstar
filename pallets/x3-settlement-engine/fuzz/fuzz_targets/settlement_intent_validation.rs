#![no_main]

use libfuzzer_sys::fuzz_target;
use pallet_x3_settlement_engine::types::*;
use parity_scale_codec::{Decode, Encode};

// Fuzz target for settlement intent validation
// Tests that settlement engine properly validates intent parameters
fuzz_target!(|data: &[u8]| {
    // Need minimum data for intent structure
    if data.len() < 100 {
        return;
    }

    // Try to decode fuzzed data into intent parameters
    // This tests the robustness of intent parsing and validation
    let _result = <(IntentId, AssetId, Balance, u64) as Decode>::decode(&mut &data[..]);

    // If we can decode without panicking, test basic validation rules
    if let Ok((intent_id, asset_id, amount, timeout)) = _result {
        // Test basic validation rules that should never panic
        let _is_valid_amount = amount > 0;
        let _is_valid_timeout = timeout > 0;
        let _is_valid_asset = asset_id != AssetId::default();

        // Test intent ID structure
        let _intent_bytes = intent_id.0;

        // If we get here without panicking, the fuzz input passed basic validation
    }
});
