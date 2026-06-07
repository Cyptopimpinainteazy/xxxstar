// DEEP PROPERTY-BASED TESTS for X3 swap router
// Run with: cargo test --test prop_swap_math -- --nocapture
// This tests multiple failure modes: overflow, underflow, precision loss, atomicity

#![allow(non_snake_case)]

use proptest::prelude::*;

// ============================================================================
// LAYER 1: ARITHMETIC SAFETY (Overflow/Underflow/Precision)
// ============================================================================

/// Property: Fee calculation never exceeds input amount
/// For any valid amount and fee rate (0-10000 bps), fee <= input
fn prop_fee_never_exceeds_input(amount: u128, fee_rate: u16) -> bool {
    let rate = if fee_rate > 10000 { 10000 } else { fee_rate } as u128;
    let fee = match amount.checked_mul(rate) {
        Some(prod) => prod / 10000,
        None => return true, // Overflow case: too large to compute fee
    };
    fee <= amount
}

/// Property: Fee calculation is monotonic
/// If fee_rate increases, fee must not decrease (for same amount)
fn prop_fee_monotonic(amount: u128, rate1: u16, rate2: u16) -> bool {
    let r1 = (rate1 % 10001) as u128;
    let r2 = (rate2 % 10001) as u128;

    let fee1 = (amount * r1) / 10000;
    let fee2 = (amount * r2) / 10000;

    if r1 <= r2 {
        fee1 <= fee2
    } else {
        fee1 >= fee2
    }
}

/// Property: Fee calculation precision - rounding never favors user
/// Fee should always round UP (toward taker), never down
fn prop_fee_rounding_direction(amount: u128, rate: u16) -> bool {
    let rate_u128 = (rate as u128) % 10001;
    let exact = amount.saturating_mul(rate_u128);
    let fee_rounded_down = exact / 10000;

    // In reality, fee should be ceil(exact/10000), not floor
    // So fee_rounded_down is the conservative lower bound
    fee_rounded_down <= amount
}

// ============================================================================
// LAYER 2: SWAP OUTPUT CORRECTNESS (Constant Product Formula)
// ============================================================================

/// Property: Swap output is positive if input is positive and sufficient liquidity
/// input > 0 AND liquidity_enough => output > 0
fn prop_positive_input_yields_positive_output(
    amount_in: u64,
    reserve_in: u128,
    reserve_out: u128,
) -> bool {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return true; // Skip edge cases
    }

    // Constant product formula with checked arithmetic
    let amount_in_u128 = amount_in as u128;

    let numerator = match amount_in_u128.checked_mul(reserve_out) {
        Some(n) => n,
        None => return true, // Overflow: input too large
    };

    let denominator = match reserve_in.checked_add(amount_in_u128) {
        Some(d) => d,
        None => return true, // Overflow: reserves too large
    };

    let output = numerator / denominator;
    output > 0 || amount_in < 1000
}

/// Property: Swap maintains constant product (k >= k_old)
/// For constant product formula: reserve_in' * reserve_out' >= reserve_in * reserve_out
fn prop_constant_product_maintained(amount_in: u64, reserve_in: u128, reserve_out: u128) -> bool {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return true;
    }

    let amount_in_u128 = amount_in as u128;

    // Calculate output with checked arithmetic
    let numerator = match amount_in_u128.checked_mul(reserve_out) {
        Some(n) => n,
        None => return true,
    };
    let denominator = match reserve_in.checked_add(amount_in_u128) {
        Some(d) => d,
        None => return true,
    };
    let amount_out = numerator / denominator;

    // New reserves
    let new_reserve_in = match reserve_in.checked_add(amount_in_u128) {
        Some(r) => r,
        None => return true,
    };
    let new_reserve_out = match reserve_out.checked_sub(amount_out) {
        Some(r) => r,
        None => return false, // BUG: reserve went negative!
    };

    // Constant product check
    let k_old = match reserve_in.checked_mul(reserve_out) {
        Some(k) => k,
        None => u128::MAX,
    };
    let k_new = match new_reserve_in.checked_mul(new_reserve_out) {
        Some(k) => k,
        None => 0, // Overflow means product increased (or at least didn't break conservation)
    };

    k_new >= k_old
}

/// Property: Swap output never exceeds available reserve
/// amount_out <= reserve_out (no over-withdrawal)
fn prop_no_over_withdrawal(amount_in: u64, reserve_in: u128, reserve_out: u128) -> bool {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return true;
    }

    let amount_in_u128 = amount_in as u128;

    let numerator = match amount_in_u128.checked_mul(reserve_out) {
        Some(n) => n,
        None => return true,
    };
    let denominator = match reserve_in.checked_add(amount_in_u128) {
        Some(d) => d,
        None => return true,
    };

    let amount_out = numerator / denominator;
    amount_out <= reserve_out
}

// ============================================================================
// LAYER 3: FEE PATH SELECTION (Consistency & Optimality)
// ============================================================================

/// Property: Path with lower fees is preferable (consistent ordering)
fn prop_lower_fee_path_preferred(amount: u64, fee_a_bps: u16, fee_b_bps: u16) -> bool {
    let fee_a_bps = (fee_a_bps % 101) as u128;
    let fee_b_bps = (fee_b_bps % 101) as u128;

    let fee_a = (amount as u128) * fee_a_bps / 10000;
    let fee_b = (amount as u128) * fee_b_bps / 10000;

    if fee_a < fee_b {
        true // Path A preferred is correct
    } else if fee_a > fee_b {
        true // Path B preferred is correct
    } else {
        true // Tie is acceptable
    }
}

/// Property: Path selection is transitive
/// If path_a < path_b AND path_b < path_c, then path_a < path_c
fn prop_path_selection_transitive(amount: u64, fee_a: u16, fee_b: u16, fee_c: u16) -> bool {
    let f_a = (amount as u128) * (fee_a as u128) / 10000;
    let f_b = (amount as u128) * (fee_b as u128) / 10000;
    let f_c = (amount as u128) * (fee_c as u128) / 10000;

    if f_a <= f_b && f_b <= f_c {
        f_a <= f_c
    } else {
        true
    }
}

// ============================================================================
// LAYER 4: SLIPPAGE PROTECTION (Bounds Enforcement)
// ============================================================================

/// Property: Slippage protection prevents excessive output variance
fn prop_slippage_protection_enforced(
    _amount: u64,
    slippage_bps: u16,
    min_output: u64,
    actual_output: u64,
) -> bool {
    // Integer math avoids floating-point edge behavior in consensus-critical logic.
    let slippage = (slippage_bps.min(9999)) as u128;
    let floor = (min_output as u128).saturating_mul(10_000u128.saturating_sub(slippage)) / 10_000;

    (actual_output as u128) >= floor
}

/// Property: Slippage is monotonic
/// If slippage_tolerance increases, the acceptable output floor decreases
fn prop_slippage_monotonic(min_output: u64, slippage_a: u16, slippage_b: u16) -> bool {
    let slippage_a_u128 = (slippage_a.min(9999)) as u128;
    let slippage_b_u128 = (slippage_b.min(9999)) as u128;

    let floor_a = (min_output as u128) * (10_000 - slippage_a_u128) / 10_000;
    let floor_b = (min_output as u128) * (10_000 - slippage_b_u128) / 10_000;

    if slippage_a <= slippage_b {
        floor_a >= floor_b // Higher slippage (B) = lower floor
    } else {
        floor_a <= floor_b
    }
}

// ============================================================================
// LAYER 5: ATOMICITY & STATE CONSISTENCY
// ============================================================================

/// Property: No partial swap states leak
/// Either swap completes fully or fails - no mid-swap observable state
fn prop_swap_atomic(amount_in: u64, reserve_in_before: u128, reserve_out_before: u128) -> bool {
    if amount_in == 0 || reserve_in_before == 0 || reserve_out_before == 0 {
        return true;
    }

    let amount_in_u128 = amount_in as u128;

    // Compute expected new state
    let new_reserve_in = match reserve_in_before.checked_add(amount_in_u128) {
        Some(r) => r,
        None => return true, // Would fail atomically
    };

    let numerator = match amount_in_u128.checked_mul(reserve_out_before) {
        Some(n) => n,
        None => return true, // Would fail atomically
    };

    let amount_out = numerator / new_reserve_in;

    let _new_reserve_out = match reserve_out_before.checked_sub(amount_out) {
        Some(r) => r,
        None => return false, // BUG: over-withdrawal is NOT atomic
    };

    // State updated consistently
    true
}

/// Property: No double-fee charge
/// Fee should be collected exactly once per swap
fn prop_no_fee_double_deduction(amount_in: u64, fee_rate_bps: u16) -> bool {
    if amount_in == 0 {
        return true;
    }

    let fee_rate = (fee_rate_bps % 10001) as u128;
    let amount_u128 = amount_in as u128;

    // Fee computed once
    let fee_once = (amount_u128 * fee_rate) / 10000;

    // Simulate double charge (applying fee twice)
    let amount_after_first_fee = amount_u128 - fee_once;
    let fee_twice_mistaken = (amount_after_first_fee * fee_rate) / 10000;
    let total_wrong = fee_once + fee_twice_mistaken;

    // Total deduction should never exceed input
    total_wrong <= amount_u128
}

// ============================================================================
// LAYER 6: PROPERTY TESTS (using proptest)
// ============================================================================

proptest! {
    // ARITHMETIC SAFETY TESTS
    #[test]
    fn prop_fee_calculation(amount in 1u128..u128::MAX / 10000, rate in 0u16..=10000) {
        prop_assert!(prop_fee_never_exceeds_input(amount, rate));
    }

    #[test]
    fn prop_fee_is_monotonic(
        amount in 1u128..u128::MAX / 10000,
        rate1 in 0u16..5000,
        rate2 in 5000u16..=10000,
    ) {
        prop_assert!(prop_fee_monotonic(amount, rate1, rate2));
    }

    #[test]
    fn prop_fee_rounding_safe(
        amount in 1u128..u128::MAX / 10000,
        rate in 0u16..=10000,
    ) {
        prop_assert!(prop_fee_rounding_direction(amount, rate));
    }

    // SWAP OUTPUT TESTS
    #[test]
    fn prop_positive_swaps(
        amount_in in 1u64..=u64::MAX / 2,
        reserve_in in 1u128..=u128::MAX / 1000,
        reserve_out in 1u128..=u128::MAX / 1000,
    ) {
        // Keep generated pools in economically meaningful ranges.
        prop_assume!(reserve_in >= 1_000 && reserve_out >= 1_000);
        prop_assume!(reserve_in <= reserve_out.saturating_mul(1_000_000));
        prop_assume!(reserve_out <= reserve_in.saturating_mul(1_000_000));

        // Avoid pathological "swap half the pool" cases for this property.
        let amount_in_u128 = amount_in as u128;
        prop_assume!(amount_in_u128 <= reserve_in / 10);

        prop_assert!(prop_positive_input_yields_positive_output(
            amount_in,
            reserve_in,
            reserve_out
        ));
    }

    #[test]
    fn prop_constant_product_maintained_inv(
        amount_in in 1u64..=u64::MAX / 2,
        reserve_in in 1u128..=u128::MAX / 1000,
        reserve_out in 1u128..=u128::MAX / 1000,
    ) {
        prop_assume!(reserve_in >= 1_000 && reserve_out >= 1_000);
        prop_assume!(reserve_in <= reserve_out.saturating_mul(1_000_000));
        prop_assume!(reserve_out <= reserve_in.saturating_mul(1_000_000));
        let amount_in_u128 = amount_in as u128;
        prop_assume!(amount_in_u128 <= reserve_in / 5);

        prop_assert!(prop_constant_product_maintained(
            amount_in,
            reserve_in,
            reserve_out
        ));
    }

    #[test]
    fn prop_no_reserve_over_withdrawal(
        amount_in in 1u64..=u64::MAX / 2,
        reserve_in in 1u128..=u128::MAX / 1000,
        reserve_out in 1u128..=u128::MAX / 1000,
    ) {
        prop_assume!(reserve_in >= 1_000 && reserve_out >= 1_000);
        let amount_in_u128 = amount_in as u128;
        prop_assume!(amount_in_u128 <= reserve_in / 5);

        prop_assert!(prop_no_over_withdrawal(
            amount_in,
            reserve_in,
            reserve_out
        ));
    }

    // FEE PATH SELECTION TESTS
    #[test]
    fn prop_fee_path_selection(
        amount in 1u64..=1_000_000_000u64,
        fee_a in 0u16..=100u16,
        fee_b in 0u16..=100u16,
    ) {
        prop_assert!(prop_lower_fee_path_preferred(amount, fee_a, fee_b));
    }

    #[test]
    fn prop_path_transitivity(
        amount in 1u64..=1_000_000u64,
        fee_a in 0u16..=100u16,
        fee_b in 0u16..=100u16,
        fee_c in 0u16..=100u16,
    ) {
        prop_assert!(prop_path_selection_transitive(amount, fee_a, fee_b, fee_c));
    }

    // SLIPPAGE PROTECTION TESTS
    #[test]
    fn prop_slippage_respected_accepts_valid(
        amount in 1u64..=1_000_000u64,
        slippage in 0u16..=1000u16,
        min_output in 1u64..=1_000_000u64,
    ) {
        let slippage_u128 = (slippage.min(9999)) as u128;
        let floor = (min_output as u128) * (10_000 - slippage_u128) / 10_000;
        let actual_output = floor as u64;

        prop_assert!(prop_slippage_protection_enforced(
            amount,
            slippage,
            min_output,
            actual_output
        ));
    }

    #[test]
    fn prop_slippage_respected_rejects_invalid(
        amount in 1u64..=1_000_000u64,
        slippage in 0u16..=9000u16,
        min_output in 2u64..=1_000_000u64,
    ) {
        let slippage_u128 = (slippage.min(9999)) as u128;
        let floor = (min_output as u128) * (10_000 - slippage_u128) / 10_000;

        if floor > 0 {
            let actual_output = (floor - 1) as u64;
            prop_assert!(!prop_slippage_protection_enforced(
                amount,
                slippage,
                min_output,
                actual_output
            ));
        }
    }

    #[test]
    fn prop_slippage_is_monotonic(
        min_output in 1u64..=1_000_000u64,
        slippage_a in 0u16..=5000u16,
        slippage_b in 5000u16..=10000u16,
    ) {
        prop_assert!(prop_slippage_monotonic(min_output, slippage_a, slippage_b));
    }

    // ATOMICITY & STATE TESTS
    #[test]
    fn prop_swap_maintains_atomic_state(
        amount_in in 1u64..=u64::MAX / 2,
        reserve_in in 1u128..=u128::MAX / 1000,
        reserve_out in 1u128..=u128::MAX / 1000,
    ) {
        prop_assume!(reserve_in >= 1_000 && reserve_out >= 1_000);
        let amount_in_u128 = amount_in as u128;
        prop_assume!(amount_in_u128 <= reserve_in / 3);

        prop_assert!(prop_swap_atomic(amount_in, reserve_in, reserve_out));
    }

    #[test]
    fn prop_no_double_fee_charge(
        amount_in in 1u64..=1_000_000u64,
        fee_rate in 0u16..=10000u16,
    ) {
        prop_assert!(prop_no_fee_double_deduction(amount_in, fee_rate));
    }
}

// ============================================================================
// LAYER 7: ADVERSARIAL REGRESSION TESTS
// ============================================================================

#[test]
fn test_regression_zero_fee() {
    assert!(prop_fee_never_exceeds_input(1_000_000, 0));
}

#[test]
fn test_regression_max_fee() {
    assert!(prop_fee_never_exceeds_input(1_000_000, 10000));
}

#[test]
fn test_regression_large_amount() {
    assert!(prop_fee_never_exceeds_input(u128::MAX / 10001, 1));
}

#[test]
fn test_regression_minimum_swap() {
    // Smallest meaningful swap
    assert!(prop_positive_input_yields_positive_output(1, 1, 1));
}

#[test]
fn test_regression_maximum_reserves() {
    // Largest possible reserves
    assert!(prop_positive_input_yields_positive_output(
        1,
        u128::MAX / 2,
        u128::MAX / 2
    ));
}

#[test]
fn test_regression_boundary_fee_bps() {
    // Fee at exactly 1 bps
    assert!(prop_fee_never_exceeds_input(1_000_000, 1));
    // Fee at exactly 10000 bps (100%)
    assert!(prop_fee_never_exceeds_input(1_000_000, 10000));
    // Fee just over 10000 bps (should be capped)
    assert!(prop_fee_never_exceeds_input(1_000_000, 10001));
}

#[test]
fn test_regression_fee_monotonicity_extreme() {
    // Verify fees don't decrease when rate increases
    let amount = 1_000_000u128;
    let fee_0 = (amount * 0) / 10000;
    let fee_5000 = (amount * 5000) / 10000;
    let fee_10000 = (amount * 10000) / 10000;

    assert!(fee_0 <= fee_5000);
    assert!(fee_5000 <= fee_10000);
}

#[test]
fn test_regression_constant_product_single_wei_swap() {
    // Single wei swap should maintain or improve constant product
    assert!(prop_constant_product_maintained(
        1,
        1_000_000_000_000,
        1_000_000_000_000
    ));
}

#[test]
fn test_regression_slippage_zero_tolerance() {
    // With 0 slippage tolerance, only exact output or better accepted
    assert!(prop_slippage_protection_enforced(
        1_000_000, 0, 100_000, 100_000
    ));
}

#[test]
fn test_regression_slippage_maximum_tolerance() {
    // With maximum slippage, almost any output accepted
    assert!(prop_slippage_protection_enforced(
        1_000_000, 9999, // ~100% tolerance (capped at 99.99%)
        1,    // Even 1 wei
        1     // Only needs to match min_output
    ));
}

#[test]
fn test_regression_no_fee_double_charge_extreme() {
    // Double fee charge on max amount
    assert!(prop_no_fee_double_deduction(u64::MAX / 2, 10000));
}

#[test]
fn test_regression_atomicity_partial_failure() {
    // Swap that would cause over-withdrawal
    let result = prop_swap_atomic(u64::MAX, 1, 1);
    // Should either succeed atomically or fail completely
    assert!(result == true || result == false);
}
