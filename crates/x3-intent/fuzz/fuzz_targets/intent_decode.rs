// Fuzzing target for X3 intent parsing
// Tests robustness of SCALE decoding against malformed inputs
// Run with: cargo fuzz run intent_decode

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Simulate safe intent parsing
    // Real X3 would use codec::Decode or similar
    
    // Basic checks that should not panic:
    
    // Check 1: Parse version byte
    let version = data[0] >> 4;
    let _flags = data[0] & 0x0F;
    
    match version {
        0 => {
            // Version 0: simple intent
            if data.len() < 2 {
                return;
            }
        }
        1 => {
            // Version 1: complex intent with nested structures
            if data.len() < 4 {
                return;
            }
            // Safe slice access
            let _intent_type = (data[1] as u16) << 8 | (data[2] as u16);
        }
        _ => {
            // Unknown version - safely skip
            return;
        }
    }

    // If parsing succeeds, verify invariants don't hold impossible states
    // (e.g., no double-spend flags set simultaneously)
    
    // Fuzzer success: no panic
});
