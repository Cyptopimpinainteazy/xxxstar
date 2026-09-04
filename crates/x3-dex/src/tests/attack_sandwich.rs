use crate::{BatchSwapRouter, PoolReserves, RealSlippageCalculator};

#[test]
fn sandwich_attack_profit_bounded_by_slippage() {
    let pool_id = [9u8; 32];
    let fee_bps = 30;

    let mut reserve_a: u64 = 1_000_000;
    let mut reserve_b: u64 = 1_000_000;

    let front_run_in: u64 = 10_000;
    let victim_in: u64 = 50_000;

    // Baseline quote for the attacker's entry trade.
    let baseline_pool = PoolReserves {
        pool_id,
        token_a: 1,
        token_b: 2,
        reserve_a,
        reserve_b,
        total_liquidity: 1_000_000,
        fee_bps,
        last_update_block: 100,
    };
    let _baseline_quote = RealSlippageCalculator::generate_quote(&baseline_pool, front_run_in, 100)
        .expect("baseline quote must be computable");

    // Front-run: attacker buys token B with token A.
    let (attacker_bought_b, front_fee) = RealSlippageCalculator::calculate_output_amount(
        front_run_in,
        reserve_a,
        reserve_b,
        fee_bps,
    )
    .expect("front-run quote must succeed");
    let front_in_after_fee = front_run_in.saturating_sub(front_fee);
    reserve_a = reserve_a.saturating_add(front_in_after_fee);
    reserve_b = reserve_b.saturating_sub(attacker_bought_b);

    // Victim large swap executes at worse price.
    let (victim_out, victim_fee) =
        RealSlippageCalculator::calculate_output_amount(victim_in, reserve_a, reserve_b, fee_bps)
            .expect("victim quote must succeed");
    let victim_in_after_fee = victim_in.saturating_sub(victim_fee);
    reserve_a = reserve_a.saturating_add(victim_in_after_fee);
    reserve_b = reserve_b.saturating_sub(victim_out);

    // Back-run: attacker sells token B back into token A pool.
    let (attacker_exit_a, _back_fee) = RealSlippageCalculator::calculate_output_amount(
        attacker_bought_b,
        reserve_b,
        reserve_a,
        fee_bps,
    )
    .expect("back-run quote must succeed");

    let attacker_profit = attacker_exit_a.saturating_sub(front_run_in);
    let attacker_profit_bps = if front_run_in == 0 {
        0
    } else {
        ((attacker_profit as u128) * 10_000 / (front_run_in as u128)) as u32
    };

    let mev_protection = BatchSwapRouter::apply_mev_protection(0, 10, 3_000, 100);

    // Mitigation expectation: dynamic slippage + MEV protections cap extractable profit.
    assert!(attacker_profit_bps <= mev_protection.max_slippage_bps);
}
