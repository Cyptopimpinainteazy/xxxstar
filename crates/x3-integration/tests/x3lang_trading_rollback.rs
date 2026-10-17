//! Transaction / rollback suite.
//!
//! A committed trade carries balances, reserves, debts, costs, events and staged external intents
//! out of the VM; a trade that fails at **any** step carries none of them. The mutation loop below
//! makes each step of the flagship program fail in turn and checks the whole journal comes back
//! empty. The nested-window tests use assembled bytecode on purpose: the compiler never emits
//! nested windows for a trade, and the engine's behaviour for them still has to be right.

#![cfg(all(feature = "std", feature = "compile"))]

mod trading_support;

use trading_support::*;
use x3_backend::opcode::Opcode;
use x3_vm::trading::TradingPreState;
use x3_x3_integration::trading::{execute_trade, TradeRun};
use x3bc_core::receipt::Outcome;
use x3bc_core::trading::{assemble_trading_module, TradingSection};
use x3bc_core::trap::TrapId;

const GAS_LIMIT: u64 = 1_000_000;

fn assert_nothing_survives(run: &TradeRun) {
    assert!(!run.committed);
    assert!(run.delta.is_none());
    assert!(matches!(run.receipt.outcome, Outcome::RolledBack { .. }));
    assert_eq!(
        run.receipt.post_state_commitment,
        run.receipt.pre_state_commitment
    );
    assert_eq!(run.receipt.post_balances, run.receipt.pre_balances);
    assert!(run.receipt.costs.is_empty());
    assert!(run.receipt.events.is_empty());
    assert!(run.receipt.intents.is_empty());
    assert!(run.receipt.debts.is_empty());
    assert!(run.storage_writes.is_empty());
}

#[test]
fn a_commit_carries_every_journaled_effect_out() {
    let artifact = compile(ARB_SOURCE);
    let run = execute_trade(&artifact.bytes, &pre_state(), &anchor(), GAS_LIMIT).expect("runs");
    assert!(run.committed);
    let delta = run.delta.as_ref().expect("delta");
    assert!(!run.receipt.events.is_empty(), "trade events are committed");
    assert!(
        !run.receipt.intents.is_empty(),
        "staged ledger intents are released on commit"
    );
    assert_eq!(
        delta.intents, run.receipt.intents,
        "the kernel applies exactly what was receipted"
    );
    assert_eq!(
        delta.trader_nonce_increment, 1,
        "a committed trade consumes the trader's nonce"
    );

    // Conservation, from the delta's final figures against the seeded pre-state: what the trader,
    // the two pools, the flash pool and the fee collector gained or lost sums to zero per asset.
    let (p1a, p1b) = delta.venue_reserve("x3dex_pool_1").expect("pool 1");
    let (p2a, p2b) = delta.venue_reserve("x3dex_pool_2").expect("pool 2");
    let usdx = (delta.trader_balance(USDX).expect("usdx") as i128 - TRADER_USDX as i128)
        + (p1a as i128 - POOL1.0 as i128)
        + (p2b as i128 - POOL2.1 as i128)
        + delta.flash_fee_paid("x3_flashloan").expect("flash") as i128
        + delta.fee_collector(USDX) as i128;
    assert_eq!(
        usdx, 0,
        "USDX is conserved across trader, pools, flash pool and fee collector"
    );
    let wetx = delta.trader_balance(WETX).expect("wetx") as i128
        + (p1b as i128 - POOL1.1 as i128)
        + (p2a as i128 - POOL2.0 as i128)
        + delta.fee_collector(WETX) as i128;
    assert_eq!(wetx, 0, "WETX is conserved");
}

/// The step each mutation makes fail, the trap it must raise, and the pre-state/source change.
struct Mutation {
    step: &'static str,
    trap: TrapId,
    source: fn(&str) -> String,
    pre: fn(&mut TradingPreState),
}

fn same(source: &str) -> String {
    source.to_string()
}

#[test]
fn a_failure_at_every_step_rolls_back_everything() {
    let mutations = [
        Mutation {
            step: "borrow",
            trap: TrapId::HostRefused,
            source: same,
            pre: |pre| pre.flash_pools[0].liquidity = 1,
        },
        Mutation {
            step: "swap 1",
            trap: TrapId::MinOutput,
            source: same,
            pre: |pre| pre.venues[0].reserve_b = 400 * UNIT,
        },
        Mutation {
            step: "swap 2",
            trap: TrapId::MinOutput,
            source: same,
            pre: |pre| pre.venues[1].reserve_b = 1_100 * UNIT,
        },
        Mutation {
            step: "repay",
            trap: TrapId::InsufficientBalance,
            // Let the second swap return less than the debt, with no float to cover it.
            source: |source| source.replace("min_out 1_005 USDX", "min_out 990 USDX"),
            pre: |pre| {
                pre.trader_balances.clear();
                pre.venues[1].reserve_a = 400_000 * UNIT;
                pre.venues[1].reserve_b = 800_000 * UNIT;
                pre.oracle.clear();
            },
        },
        Mutation {
            step: "charge costs",
            trap: TrapId::CostCeiling,
            source: same,
            pre: |pre| pre.costs.relay_fee = 30 * UNIT,
        },
        Mutation {
            step: "profit floor",
            trap: TrapId::ProfitBelowFloor,
            source: |source| source.replace("net_profit >= 1 USDX", "net_profit >= 1_000 USDX"),
            pre: |_| {},
        },
    ];

    for mutation in mutations {
        let source = (mutation.source)(ARB_SOURCE);
        let source = if mutation.step == "repay" {
            // The repay mutation has no oracle; drop the oracle clauses with it.
            source
                .replace("    max_oracle_deviation: 500 bps\n", "")
                .replace("    quote_freshness: 3\n", "")
        } else {
            source
        };
        let artifact = compile(&source);
        let mut pre = pre_state();
        (mutation.pre)(&mut pre);
        let run = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT)
            .unwrap_or_else(|error| panic!("{}: runs to a verdict: {error:?}", mutation.step));
        match &run.receipt.outcome {
            Outcome::RolledBack { trap, .. } => {
                assert_eq!(*trap, mutation.trap, "{}", mutation.step)
            }
            other => panic!("{}: expected rollback, got {other:?}", mutation.step),
        }
        assert_nothing_survives(&run);
    }
}

/// The flagship section with assembled code around it.
fn section() -> TradingSection {
    let artifact = compile(ARB_SOURCE);
    x3_backend::bc_format::BytecodeModule::from_bytes(&artifact.bytes)
        .expect("decodes")
        .trading
        .expect("section")
}

fn op(code: &mut Vec<u8>, opcode: Opcode, operands: &[u8]) {
    code.push(opcode as u8);
    code.extend_from_slice(operands);
}

#[test]
fn nested_atomic_rollback_discards_inner_and_outer_effects() {
    let mut code = Vec::new();
    op(&mut code, Opcode::AtomicBegin, &0u16.to_le_bytes());
    op(&mut code, Opcode::TradeBegin, &[]);
    op(&mut code, Opcode::TradeBorrow, &0u16.to_le_bytes());
    op(&mut code, Opcode::AtomicBegin, &1u16.to_le_bytes());
    op(&mut code, Opcode::TradeSwap, &1u16.to_le_bytes()); // state-changing, inside the inner window
    op(&mut code, Opcode::AtomicRollback, &1u16.to_le_bytes());
    op(&mut code, Opcode::RetVoid, &[]);
    let bytes = assemble_trading_module(section(), code);
    let run = execute_trade(&bytes, &pre_state(), &anchor(), GAS_LIMIT).expect("verdict");
    match &run.receipt.outcome {
        Outcome::RolledBack { trap, .. } => assert_eq!(*trap, TrapId::AtomicAborted),
        other => panic!("{other:?}"),
    }
    assert_nothing_survives(&run);
}

#[test]
fn an_inner_commit_does_not_survive_an_outer_failure() {
    let mut code = Vec::new();
    op(&mut code, Opcode::AtomicBegin, &0u16.to_le_bytes());
    op(&mut code, Opcode::TradeBegin, &[]);
    op(&mut code, Opcode::TradeBorrow, &0u16.to_le_bytes());
    op(&mut code, Opcode::AtomicBegin, &1u16.to_le_bytes());
    op(&mut code, Opcode::TradeSwap, &1u16.to_le_bytes());
    op(&mut code, Opcode::AtomicCommit, &1u16.to_le_bytes()); // inner window commits …
    op(&mut code, Opcode::TradeCommit, &[]); // … but the debt is still open: the trade traps
    op(&mut code, Opcode::AtomicCommit, &0u16.to_le_bytes());
    op(&mut code, Opcode::RetVoid, &[]);
    let bytes = assemble_trading_module(section(), code);
    let run = execute_trade(&bytes, &pre_state(), &anchor(), GAS_LIMIT).expect("verdict");
    match &run.receipt.outcome {
        Outcome::RolledBack { trap, .. } => assert_eq!(*trap, TrapId::DebtOutstanding),
        other => panic!("{other:?}"),
    }
    assert_nothing_survives(&run);
}

#[test]
fn a_trade_opcode_outside_an_atomic_window_is_refused_by_the_verifier() {
    let mut code = Vec::new();
    op(&mut code, Opcode::TradeBegin, &[]);
    op(&mut code, Opcode::TradeBorrow, &0u16.to_le_bytes());
    op(&mut code, Opcode::RetVoid, &[]);
    let bytes = assemble_trading_module(section(), code);
    assert!(x3_vm::Verifier::verify_module_bytes(
        &bytes,
        &x3_vm::VerifyOptions::on_chain_trading()
    )
    .is_err());
}
