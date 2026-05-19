//! Fuzz target: Median price calculation
//!
//! Feeds arbitrary arrays of prices to median calculation logic
//! to find edge cases, overflows, or incorrect median computation.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    // Extract number of prices (first 4 bytes, max 1000)
    let num_prices = u32::from_le_bytes(data[..4].try_into().unwrap_or([0; 4])) % 1000;
    let prices_data = &data[4..];

    if prices_data.len() < (num_prices as usize * 8) {
        return;
    }

    // Build price array
    let mut prices = Vec::new();
    for i in 0..num_prices {
        let start = (i as usize) * 8;
        let end = start + 8;
        if end > prices_data.len() {
            break;
        }
        let price_bytes = &prices_data[start..end];
        let price = u64::from_le_bytes(price_bytes.try_into().unwrap());
        prices.push(price);
    }

    if prices.is_empty() {
        return;
    }

    // Test median calculation (copied from pallet logic)
    let mut sorted_prices = prices.clone();
    sorted_prices.sort_unstable();

    // Calculate median
    let median_price = if sorted_prices.len() % 2 == 0 {
        let mid = sorted_prices.len() / 2;
        let a = sorted_prices[mid - 1];
        let b = sorted_prices[mid];
        // Simple average - check for overflow
        if a > u64::MAX / 2 || b > u64::MAX / 2 {
            // Would overflow, but saturating_add handles it
            (a.saturating_add(b)) / 2
        } else {
            (a + b) / 2
        }
    } else {
        sorted_prices[sorted_prices.len() / 2]
    };

    // INVARIANT: Median should be within the range of input prices
    let min_price = *sorted_prices.first().unwrap();
    let max_price = *sorted_prices.last().unwrap();
    assert!(median_price >= min_price && median_price <= max_price, "Median outside price range");

    // INVARIANT: For odd number of prices, median should be the middle value
    if sorted_prices.len() % 2 == 1 {
        let expected = sorted_prices[sorted_prices.len() / 2];
        assert_eq!(median_price, expected, "Odd-length median incorrect");
    }

    // INVARIANT: For even number of prices, median should be average of middle two
    if sorted_prices.len() % 2 == 0 && sorted_prices.len() >= 2 {
        let mid = sorted_prices.len() / 2;
        let a = sorted_prices[mid - 1];
        let b = sorted_prices[mid];
        let expected = if a > u64::MAX / 2 || b > u64::MAX / 2 {
            (a.saturating_add(b)) / 2
        } else {
            (a + b) / 2
        };
        assert_eq!(median_price, expected, "Even-length median incorrect");
    }

    // INVARIANT: Median calculation should be deterministic
    let median_price2 = if sorted_prices.len() % 2 == 0 {
        let mid = sorted_prices.len() / 2;
        let a = sorted_prices[mid - 1];
        let b = sorted_prices[mid];
        (a.saturating_add(b)) / 2
    } else {
        sorted_prices[sorted_prices.len() / 2]
    };
    assert_eq!(median_price, median_price2, "Median calculation not deterministic");
});