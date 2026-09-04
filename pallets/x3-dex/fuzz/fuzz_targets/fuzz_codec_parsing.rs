//! Fuzz target: Codec parsing for DEX structures
//!
//! Feeds arbitrary bytes through LiquidityPool and LPPosition decoders
//! to ensure no panics in serialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use x3_dex::amm_pools::{LiquidityPool, LPPosition};

fuzz_target!(|data: &[u8]| {
    // Test LiquidityPool decoding
    if let Ok(pool) = LiquidityPool::decode(&mut &*data) {
        // INVARIANT: Pool ID should not be zero for valid pools
        if pool.pool_id == 0 {
            // This is acceptable - pool ID 0 might be used for special cases
            // But ensure reserves are consistent
            if pool.reserve_a != 0 || pool.reserve_b != 0 {
                // If reserves exist, LP supply should be non-zero
                assert!(pool.total_lp_supply > 0, "Pool with reserves must have LP supply");
            }
        }

        // INVARIANT: Fee should not exceed 100%
        assert!(pool.fee_basis_points <= 10000, "Fee cannot exceed 100%");

        // INVARIANT: Deterministic re-encoding
        let re_encoded = pool.encode();
        let re_decoded = LiquidityPool::decode(&mut &re_encoded[..]).unwrap();
        assert_eq!(pool, re_decoded, "Codec must be deterministic");
    }

    // Test LPPosition decoding (using different offset in data)
    if data.len() > 50 {
        let pos_data = &data[50..];
        if let Ok(position) = LPPosition::decode(&mut &pos_data[..]) {
            // INVARIANT: Position ID should be non-zero for valid positions
            if position.position_id != 0 {
                // Valid position should reference a valid pool
                assert!(position.pool_id > 0, "Position must reference valid pool");
                // LP balance should be positive
                assert!(position.lp_balance > 0, "Position must have positive LP balance");
            }

            // INVARIANT: Deterministic re-encoding
            let re_encoded = position.encode();
            let re_decoded = LPPosition::decode(&mut &re_encoded[..]).unwrap();
            assert_eq!(position, re_decoded, "Codec must be deterministic");
        }
    }
});