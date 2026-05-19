//! Fuzz target: Codec parsing for Oracle structures
//!
//! Feeds arbitrary bytes through PriceSubmission and PriceData decoders
//! to ensure no panics in serialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;
use pallet_x3_oracle::{PriceSubmission, PriceData};

// Mock block number type for fuzzing
type MockBlockNumber = u32;

fuzz_target!(|data: &[u8]| {
    // Test PriceSubmission decoding
    if let Ok(submission) = PriceSubmission::<MockBlockNumber>::decode(&mut &*data) {
        // INVARIANT: Price should be reasonable (not excessively large)
        assert!(submission.price < 1_000_000_000_000, "Price unreasonably large");

        // INVARIANT: Timestamp should be reasonable
        assert!(submission.timestamp < 2_000_000_000, "Timestamp unreasonably large");

        // INVARIANT: Block number should be reasonable
        assert!(submission.block < 10_000_000, "Block number unreasonably large");

        // INVARIANT: Deterministic re-encoding
        let re_encoded = submission.encode();
        let re_decoded = PriceSubmission::<MockBlockNumber>::decode(&mut &re_encoded[..]).unwrap();
        assert_eq!(submission, re_decoded, "Codec must be deterministic");
    }

    // Test PriceData decoding (using different offset in data)
    if data.len() > 50 {
        let price_data = &data[50..];
        if let Ok(price_data) = PriceData::<MockBlockNumber>::decode(&mut &price_data[..]) {
            // INVARIANT: Price should be reasonable
            assert!(price_data.price < 1_000_000_000_000, "Price unreasonably large");

            // INVARIANT: Submission count should be reasonable
            assert!(price_data.submission_count < 1000, "Submission count unreasonably large");

            // INVARIANT: If there are submissions, price should be non-zero
            if price_data.submission_count > 0 {
                assert!(price_data.price > 0, "Price should be positive when submissions exist");
            }

            // INVARIANT: Timestamp should be reasonable
            assert!(price_data.timestamp < 2_000_000_000, "Timestamp unreasonably large");

            // INVARIANT: Last updated block should be reasonable
            assert!(price_data.last_updated < 10_000_000, "Block number unreasonably large");

            // INVARIANT: Deterministic re-encoding
            let re_encoded = price_data.encode();
            let re_decoded = PriceData::<MockBlockNumber>::decode(&mut &re_encoded[..]).unwrap();
            assert_eq!(price_data, re_decoded, "Codec must be deterministic");
        }
    }
});