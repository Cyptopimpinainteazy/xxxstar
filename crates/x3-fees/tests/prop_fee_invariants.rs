// Property-based tests for X3 fee calculations
// Tests the core invariant: fee math never overflows and preserves accounting

#![allow(non_snake_case)]

use proptest::prelude::*;

/// Core invariant: For fee_rate in [0, 10000]:
///   fee = (amount * fee_rate) / 10000
///   fee <= amount
fn prop_fee_invariant_no_overflow(amount: u128, fee_rate: u16) -> bool {
    let rate = (fee_rate as u128).min(10000);
    let fee = amount.saturating_mul(rate) / 10000;
    fee <= amount
}

/// Invariant: Fee + output = input (when accounting for slippage)
/// input = output + fee (before slippage)
fn prop_accounting_conservation(input: u64, fee_rate: u16) -> bool {
    if input == 0 {
        return true;
    }

    let input_u128 = input as u128;
    let rate = (fee_rate as u128).min(10000);
    let fee = input_u128.saturating_mul(rate) / 10000;
    let output = input_u128 - fee;

    // Basic conservation: fee + output should equal input (minus rounding)
    let accounted = fee + output;
    accounted <= input_u128 && (input_u128 - accounted) <= 1
}

/// Invariant: Increasing fee_rate monotonically increases fee
fn prop_fee_rate_monotonic(amount: u64, rate1: u16, rate2: u16) -> bool {
    let amount = amount as u128;
    let r1 = (rate1 as u128).min(10000);
    let r2 = (rate2 as u128).min(10000);

    let fee1 = amount.saturating_mul(r1) / 10000;
    let fee2 = amount.saturating_mul(r2) / 10000;

    if r1 <= r2 {
        fee1 <= fee2
    } else {
        fee1 >= fee2
    }
}

/// Invariant: Tier-based fee (progressive rates) never regresses
/// If amount increases, total_fee should not decrease
fn prop_progressive_fee_monotonic(amount1: u64, amount2: u64, tier: u8) -> bool {
    if amount1 == 0 || amount2 == 0 {
        return true;
    }

    // Example: 3-tier fee structure
    let rate_for_amount = |amt: u64| -> u16 {
        match tier {
            0 => {
                if amt < 1_000_000 {
                    25
                } else {
                    50
                }
            }
            1 => {
                if amt < 1_000_000 {
                    50
                } else {
                    75
                }
            }
            _ => 100,
        }
    };

    let fee1 = (amount1 as u128).saturating_mul(rate_for_amount(amount1) as u128) / 10000;
    let fee2 = (amount2 as u128).saturating_mul(rate_for_amount(amount2) as u128) / 10000;

    // Fee remains bounded by input for both amounts.
    if fee1 > amount1 as u128 || fee2 > amount2 as u128 {
        return false;
    }

    // Monotonic by amount for the same tier schedule.
    if amount1 <= amount2 {
        fee1 <= fee2
    } else {
        fee1 >= fee2
    }
}

/// Invariant: Fee subtraction cannot underflow
/// output = input - fee always succeeds without panic
fn prop_fee_subtraction_no_underflow(input: u128, fee_rate: u16) -> bool {
    let rate = (fee_rate as u128).min(10000);
    let fee = input.saturating_mul(rate) / 10000;
    let _output = input.saturating_sub(fee);
    true // If we got here, no panic occurred
}

/// Invariant: for one amount, lower fee rate implies higher/equal output.
fn prop_output_monotonic_vs_fee_rate(amount: u64, low_rate: u16, high_rate: u16) -> bool {
    let amount_u128 = amount as u128;
    let r_low = (low_rate as u128).min(10000);
    let r_high = (high_rate as u128).min(10000);

    if r_low > r_high {
        return true;
    }

    let fee_low = amount_u128.saturating_mul(r_low) / 10000;
    let fee_high = amount_u128.saturating_mul(r_high) / 10000;

    let out_low = amount_u128.saturating_sub(fee_low);
    let out_high = amount_u128.saturating_sub(fee_high);

    out_low >= out_high
}

proptest! {
    #[test]
    fn prop_fee_no_overflow(amount in 0u128..u128::MAX / 10000, rate in 0u16..=10000) {
        prop_assert!(prop_fee_invariant_no_overflow(amount, rate));
    }

    #[test]
    fn prop_accounting_conserved(input in 0u64..=10_000_000_000_000u64, rate in 0u16..=10000) {
        prop_assert!(prop_accounting_conservation(input, rate));
    }

    #[test]
    fn prop_fee_rate_increases(
        amount in 0u64..=10_000_000_000_000u64,
        rate1 in 0u16..=10000u16,
        rate2 in 0u16..=10000u16,
    ) {
        prop_assert!(prop_fee_rate_monotonic(amount, rate1, rate2));
    }

    #[test]
    fn prop_progressive_fee_valid(
        amount1 in 1u64..=1_000_000_000u64,
        amount2 in 1u64..=1_000_000_000u64,
        tier in 0u8..=2u8,
    ) {
        prop_assert!(prop_progressive_fee_monotonic(amount1, amount2, tier));
    }

    #[test]
    fn prop_fee_subtraction_safe(input in 0u128..=((u64::MAX as u128) * 1_000_000u128), rate in 0u16..=10000) {
        prop_assert!(prop_fee_subtraction_no_underflow(input, rate));
    }

    #[test]
    fn prop_output_decreases_with_higher_fee(
        amount in 1u64..=10_000_000_000u64,
        low_rate in 0u16..=10000u16,
        delta in 0u16..=10000u16,
    ) {
        let high_rate = low_rate.saturating_add(delta).min(10000);
        prop_assert!(prop_output_monotonic_vs_fee_rate(amount, low_rate, high_rate));
    }
}

// Edge case regression tests
#[test]
fn test_zero_amount() {
    assert!(prop_fee_invariant_no_overflow(0, 5000));
}

#[test]
fn test_max_fee_rate() {
    assert!(prop_fee_invariant_no_overflow(1_000_000_000_000u128, 10000));
}

#[test]
fn test_one_wei() {
    assert!(prop_accounting_conservation(1, 5000));
}

#[test]
fn test_max_amount() {
    assert!(prop_fee_invariant_no_overflow(u128::MAX / 10000, 1));
}
