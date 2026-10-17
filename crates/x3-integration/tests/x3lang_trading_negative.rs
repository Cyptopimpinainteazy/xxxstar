//! Every refusal the production trading path owes, each proven on the production path.
//!
//! Compile-time refusals go through `x3-lang/compiler`'s X3BC lowering; artifact refusals through
//! the no_std decoder and verifier; runtime refusals through `crates/x3-vm` and must leave a
//! rolled-back receipt with **no** delta. Receipt refusals are in `x3lang_trading_receipt_replay.rs`.

#![cfg(all(feature = "std", feature = "compile"))]

mod trading_support;

use trading_support::*;
use x3_vm::trading::TradingPreState;
use x3_x3_integration::trading::{execute_trade, TradeRun};
use x3_x3_integration::X3IntegrationError;
use x3bc_core::receipt::Outcome;
use x3bc_core::trap::TrapId;

const GAS_LIMIT: u64 = 1_000_000;

fn run_with(pre: &TradingPreState) -> TradeRun {
    let artifact = compile(ARB_SOURCE);
    execute_trade(&artifact.bytes, pre, &anchor(), GAS_LIMIT).expect("runs to a verdict")
}

/// A runtime refusal: rolled back, the named trap, nothing to apply, nothing charged.
fn assert_rolled_back(run: &TradeRun, expected: TrapId) {
    assert!(!run.committed, "must not commit");
    assert!(run.delta.is_none(), "a rolled-back trade applies nothing");
    match &run.receipt.outcome {
        Outcome::RolledBack { trap, .. } => assert_eq!(*trap, expected),
        other => panic!("expected rollback with {expected:?}, got {other:?}"),
    }
    assert_eq!(run.receipt.post_balances, run.receipt.pre_balances);
    assert_eq!(
        run.receipt.post_state_commitment,
        run.receipt.pre_state_commitment
    );
    assert!(run.receipt.costs.is_empty());
}

// ---------------------------------------------------------------- compile time

#[test]
fn unresolved_binding_is_refused_at_compile_time() {
    let source = ARB_SOURCE.replace("swap wetx WETX", "swap ghost WETX");
    assert!(compile_err(&source).contains("unknown binding 'ghost'"));
}

#[test]
fn unresolved_debt_is_refused_at_compile_time() {
    let source = ARB_SOURCE.replace("    repay debt\n", "");
    assert!(compile_err(&source).contains("unrepaid debt"));
}

#[test]
fn amount_overflow_is_refused_at_compile_time() {
    let source = ARB_SOURCE.replace(
        "borrow 1_000 USDX",
        "borrow 400000000000000000000000000000000 USDX",
    );
    assert!(compile_err(&source).contains("overflows u128"));
}

#[test]
fn an_asset_that_is_not_on_the_x3_chain_cannot_lower() {
    let source = ARB_SOURCE.replace("x3.x3chain.asset_2", "evm.ethereum.0xC02a");
    let error = compile_err(&source);
    assert!(
        error.contains("X3BC-L"),
        "explicit lowering diagnostic: {error}"
    );
}

#[test]
fn an_asset_id_that_does_not_fit_the_chain_asset_type_cannot_lower() {
    // `AssetId` is `u32` on the X3 chain; a checked conversion refuses 2^32.
    let source = ARB_SOURCE.replace("x3.x3chain.asset_2", "x3.x3chain.asset_4294967296");
    let error = compile_err(&source);
    assert!(
        error.contains("X3BC-L"),
        "explicit lowering diagnostic: {error}"
    );
}

// ---------------------------------------------------------------- artifact

#[test]
fn truncated_bytecode_is_refused_before_execution() {
    let artifact = compile(ARB_SOURCE);
    for cut in [4, 24, artifact.bytes.len() / 2, artifact.bytes.len() - 1] {
        let truncated = &artifact.bytes[..cut];
        let error = execute_trade(truncated, &pre_state(), &anchor(), GAS_LIMIT)
            .expect_err("truncated bytecode must not run");
        assert!(
            matches!(
                error,
                X3IntegrationError::InvalidBytecode(_) | X3IntegrationError::VerificationFailed(_)
            ),
            "cut at {cut}: {error:?}"
        );
    }
}

#[test]
fn malformed_trading_section_is_refused_before_execution() {
    let artifact = compile(ARB_SOURCE);
    let mut bytes = artifact.bytes.clone();
    // Flip a byte in the last third (inside the trading section) and repair the checksum so the
    // section decoder — not the corruption check — is what has to refuse it.
    let at = bytes.len() - bytes.len() / 3;
    bytes[at] ^= 0xFF;
    x3bc_core::bytecode::reseal(&mut bytes);
    assert!(execute_trade(&bytes, &pre_state(), &anchor(), GAS_LIMIT).is_err());
}

#[test]
fn unsupported_bytecode_version_is_refused() {
    let artifact = compile(ARB_SOURCE);
    let mut bytes = artifact.bytes.clone();
    // Format 1.9: a minor this reader does not know.
    bytes[4..8].copy_from_slice(&((1u32 << 16) | (9 << 8)).to_le_bytes());
    x3bc_core::bytecode::reseal(&mut bytes);
    let error = execute_trade(&bytes, &pre_state(), &anchor(), GAS_LIMIT).expect_err("refused");
    assert!(
        matches!(error, X3IntegrationError::InvalidBytecode(_)),
        "{error:?}"
    );
}

#[test]
fn a_reader_without_the_trading_extension_refuses_the_artifact() {
    // The base (1.0) loader contract: a trading module demands 1.1, so a 1.0 reader refuses it
    // rather than running it without its trading semantics.
    let artifact = compile(ARB_SOURCE);
    assert!(!x3bc_core::bytecode::loader_satisfies_base(
        x3bc_core::bytecode::min_version(&artifact.bytes).expect("header")
    ));
}

// ---------------------------------------------------------------- runtime

#[test]
fn wrong_chain_id_rolls_back() {
    let mut pre = pre_state();
    pre.chain_id = 1;
    assert_rolled_back(&run_with(&pre), TrapId::ChainMismatch);
}

#[test]
fn wrong_provider_version_rolls_back() {
    let mut pre = pre_state();
    pre.flash_pools[0].version = 2;
    assert_rolled_back(&run_with(&pre), TrapId::CapabilityVersion);
}

#[test]
fn wrong_venue_version_rolls_back() {
    let mut pre = pre_state();
    pre.venues[0].version = 2;
    assert_rolled_back(&run_with(&pre), TrapId::CapabilityVersion);
}

#[test]
fn missing_required_capability_rolls_back() {
    let mut pre = pre_state();
    pre.venues.retain(|venue| venue.name != "x3dex_pool_2");
    assert_rolled_back(&run_with(&pre), TrapId::MissingCapability);
}

#[test]
fn gas_exhaustion_rolls_back_everything() {
    let artifact = compile(ARB_SOURCE);
    let run = execute_trade(&artifact.bytes, &pre_state(), &anchor(), 50).expect("verdict");
    assert_rolled_back(&run, TrapId::GasExhausted);
}

#[test]
fn stale_oracle_rolls_back() {
    let mut pre = pre_state();
    // quote_freshness: 3 — an observation four blocks old is stale.
    pre.oracle[0].observed_at_block = EXECUTED_AT - 4;
    assert_rolled_back(&run_with(&pre), TrapId::OracleStale);
}

#[test]
fn missing_oracle_rolls_back() {
    let mut pre = pre_state();
    pre.oracle.clear();
    assert_rolled_back(&run_with(&pre), TrapId::OracleMissing);
}

#[test]
fn divergent_oracle_sources_roll_back() {
    let mut pre = pre_state();
    let mut second = pre.oracle[0].clone();
    second.source = "x3_oracle_b".into();
    second.price_num = 5;
    second.price_den = 2; // 2.5 vs 2.0: 2_500 bps apart, the policy allows 500
    pre.oracle.push(second);
    assert_rolled_back(&run_with(&pre), TrapId::OracleDivergent);
}

#[test]
fn venue_price_far_from_the_oracle_rolls_back() {
    let mut pre = pre_state();
    pre.oracle[0].price_num = 3; // pools price WETX near 2 USDX; the oracle says 3
    assert_rolled_back(&run_with(&pre), TrapId::OracleDeviation);
}

#[test]
fn slippage_violation_rolls_back() {
    let mut pre = pre_state();
    // A shallow pool 1 at the same spot price: 1_000 USDX moves it ~1%, over `max_slippage: 50
    // bps`, while the output (~493 WETX) still clears `min_out 490`.
    pre.venues[0].reserve_a = 100_000 * UNIT;
    pre.venues[0].reserve_b = 50_000 * UNIT;
    assert_rolled_back(&run_with(&pre), TrapId::Slippage);
}

#[test]
fn flash_fee_over_the_policy_rolls_back() {
    let mut pre = pre_state();
    pre.flash_pools[0].fee_bps = 20; // policy: max_flash_fee 10 bps
    assert_rolled_back(&run_with(&pre), TrapId::FlashFeeExceeded);
}

#[test]
fn profit_below_the_compiled_floor_rolls_back() {
    let source = ARB_SOURCE.replace("net_profit >= 1 USDX", "net_profit >= 100 USDX");
    let artifact = compile(&source);
    let run = execute_trade(&artifact.bytes, &pre_state(), &anchor(), GAS_LIMIT).expect("verdict");
    assert_rolled_back(&run, TrapId::ProfitBelowFloor);
}

#[test]
fn costs_over_the_policy_ceiling_roll_back() {
    let mut pre = pre_state();
    pre.costs.settlement_fee = 25 * UNIT; // policy: max_gas 20 USDX total cost
    assert_rolled_back(&run_with(&pre), TrapId::CostCeiling);
}

#[test]
fn host_refusal_rolls_back() {
    let mut pre = pre_state();
    pre.flash_pools[0].liquidity = BORROW - 1;
    assert_rolled_back(&run_with(&pre), TrapId::HostRefused);
}

#[test]
fn failure_after_swap_but_before_repay_rolls_back() {
    let mut pre = pre_state();
    pre.venues[1].reserve_b = 1_100 * UNIT; // second swap cannot meet min_out
    let run = run_with(&pre);
    assert_rolled_back(&run, TrapId::MinOutput);
    assert!(
        run.receipt.debts.is_empty(),
        "the opened debt does not survive the rollback"
    );
}

#[test]
fn balance_overflow_is_a_checked_trap() {
    let mut pre = pre_state();
    pre.trader_balances.insert(USDX, u128::MAX - 10);
    assert_rolled_back(&run_with(&pre), TrapId::ArithmeticOverflow);
}

#[test]
fn deadline_is_enforced_from_the_submission_block() {
    let mut pre = pre_state();
    pre.block_number = SUBMITTED_AT + 6; // deadline: 5 blocks
    assert_rolled_back(&run_with(&pre), TrapId::DeadlineExceeded);
}

#[test]
fn private_submission_policy_refuses_a_public_route() {
    // The artifact demands private submission (`require_private_submission: true`); a context
    // without a private channel refuses it before a single instruction runs.
    let artifact = compile(ARB_SOURCE);
    assert!(x3bc_core::bytecode::requires_private_submission(
        &artifact.bytes
    ));
    let mut pre = pre_state();
    pre.private_channel = false;
    let error = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT)
        .expect_err("a public route must not run a private-only trade");
    assert!(
        matches!(error, X3IntegrationError::PrivateSubmissionUnavailable),
        "{error:?}"
    );
}

#[test]
fn invalid_checked_numeric_conversion_is_a_trap() {
    // The program returns its realized profit as the call's `i64` result. A profit beyond
    // `i64::MAX` base units must trap on the checked conversion, not wrap or saturate — and the
    // trap rolls the whole trade back.
    let source = ARB_SOURCE
        .replace("    max_oracle_deviation: 500 bps\n", "")
        .replace("    quote_freshness: 3\n", "");
    let artifact = compile(&source);
    let mut pre = pre_state();
    pre.oracle.clear();
    pre.venues[1].reserve_b = 10u128.pow(31); // pool 2 pays ~2.5e28 USDX base units for ~498 WETX
    let run = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT).expect("verdict");
    assert_rolled_back(&run, TrapId::InvalidConversion);
}
