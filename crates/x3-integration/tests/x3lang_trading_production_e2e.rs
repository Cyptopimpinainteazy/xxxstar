//! **The** production proof for X3 Lang trading (docs/roadmap/X3LANG_PRODUCTION_UNIFICATION.md §G).
//!
//! ```text
//! .x3 source -> canonical compiler -> X3BC encode -> no_std decode -> production verification
//!   -> production host execution (crates/x3-vm) -> transaction commit -> signed receipt
//!   -> independent economic replay
//! ```
//!
//! No hand-assembled bytecode: the artifact is whatever `x3-lang/compiler` emits for
//! `x3-lang/examples/x3chain_flash_arb.x3`. Every expected figure is re-derived in
//! `trading_support` from the constant-product formula and the cost schedule.
//!
//! The paired failure test traps after a state-changing swap and proves nothing at all survives.

#![cfg(all(feature = "std", feature = "compile"))]

mod trading_support;

use trading_support::*;
use x3_backend::bc_format::BytecodeModule;
use x3_vm::{Verifier, VerifyOptions};
use x3_x3_integration::trading::{execute_trade, replay_receipt, sign_receipt};
use x3bc_core::receipt::{CostKind, HostCallKind, Outcome};
use x3bc_core::trading::{TradeOp, TRADING_SECTION_VERSION};

const GAS_LIMIT: u64 = 1_000_000;

#[test]
fn x3lang_trading_source_to_signed_replayed_receipt_on_production_path() {
    // 1. Canonical compiler.
    let artifact = compile(ARB_SOURCE);
    assert!(artifact.bytes.starts_with(b"X3BC"), "canonical X3BC magic");
    assert_eq!(
        artifact.source_hash,
        x3bc_core::hash::sha256(ARB_SOURCE.as_bytes())
    );
    assert_eq!(
        artifact.mode,
        x3bc_core::trading::CompilationModeId::Testnet
    );

    // 2-3. The production no_std decoder reads the module and its trading section.
    let module = BytecodeModule::from_bytes(&artifact.bytes).expect("no_std decoder accepts it");
    let section = module
        .trading
        .as_ref()
        .expect("the trading program is carried in the canonical trading section");
    assert_eq!(section.version, TRADING_SECTION_VERSION);
    assert_eq!(section.chain_id, X3_CHAIN_ID);
    let kinds: Vec<&str> = section.ops.iter().map(TradeOp::name).collect();
    assert_eq!(
        kinds,
        [
            "borrow",
            "swap",
            "swap",
            "repay",
            "charge_costs",
            "assert_min_profit",
            "assert_all_debts_closed"
        ],
        "every source statement lowered, costs charged before the profit floor"
    );

    // 4. Production verification rules (the chain's options, trading enabled).
    Verifier::verify_module_bytes(&artifact.bytes, &VerifyOptions::on_chain_trading())
        .expect("the production verifier accepts the compiled trade");

    // 5-10. Production host execution through crates/x3-vm, committed.
    let pre = pre_state();
    let run = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT).expect("executes");
    assert!(
        run.committed,
        "the profitable trade commits: {:?}",
        run.receipt.outcome
    );
    assert_eq!(run.receipt.outcome, Outcome::Committed);

    let expected = expected_arb();
    let delta = run
        .delta
        .as_ref()
        .expect("a committed trade has a delta to apply");

    // Borrow -> route -> swap -> repay, against the seeded pools.
    assert_eq!(
        delta.venue_reserve("x3dex_pool_1"),
        Some((POOL1.0 + BORROW, POOL1.1 - expected.swap1_out))
    );
    assert_eq!(
        delta.venue_reserve("x3dex_pool_2"),
        Some((POOL2.0 + expected.swap1_out, POOL2.1 - expected.swap2_out))
    );
    assert_eq!(
        delta.flash_fee_paid("x3_flashloan"),
        Some(expected.flash_fee)
    );

    // Every cost, by kind, in the receipt.
    let cost = |kind: CostKind| -> u128 {
        run.receipt
            .costs
            .iter()
            .filter(|c| c.kind == kind)
            .map(|c| c.amount)
            .sum()
    };
    let gas_cost = u128::from(run.receipt.gas_charged) * GAS_PRICE;
    assert!(run.receipt.gas_charged > 0 && run.receipt.gas_charged >= run.receipt.gas_used);
    assert_eq!(cost(CostKind::Gas), gas_cost);
    assert_eq!(cost(CostKind::FlashFee), expected.flash_fee);
    assert_eq!(
        cost(CostKind::VenueFee),
        expected.swap1_fee + expected.swap2_fee
    );
    assert_eq!(cost(CostKind::RelayFee), RELAY_FEE);
    assert_eq!(cost(CostKind::SettlementFee), SETTLEMENT_FEE);
    assert_eq!(cost(CostKind::ProtocolFee), PROTOCOL_FEE);
    assert_eq!(cost(CostKind::OracleFee), 2 * ORACLE_FEE);

    // Final profit only after every cost.
    let net = expected.swap2_out as i128
        - BORROW as i128
        - expected.flash_fee as i128
        - expected.fixed_costs as i128
        - gas_cost as i128;
    assert!(
        net > UNIT as i128,
        "the fixture is profitable after costs: {net}"
    );
    assert_eq!(run.receipt.profit.asset, USDX);
    assert_eq!(run.receipt.profit.net, net);
    assert_eq!(
        delta.trader_balance(USDX),
        Some((TRADER_USDX as i128 + net) as u128)
    );
    assert_eq!(delta.trader_balance(WETX), Some(0));
    let charge_at = run
        .receipt
        .transcript
        .iter()
        .position(|call| call.kind == HostCallKind::ChargeCosts)
        .expect("costs are committed");
    let profit_at = run
        .receipt
        .transcript
        .iter()
        .position(|call| call.kind == HostCallKind::AssertProfit)
        .expect("profit is evaluated");
    assert!(
        charge_at < profit_at,
        "costs are committed before the final profit evaluation"
    );

    // Debt fully repaid.
    assert_eq!(run.receipt.debts.len(), 1);
    assert_eq!(run.receipt.debts[0].principal, BORROW);
    assert_eq!(run.receipt.debts[0].fee, expected.flash_fee);
    assert_eq!(run.receipt.debts[0].repaid, BORROW + expected.flash_fee);

    // 11. The receipt binds the artifact, the chain and the execution.
    assert_eq!(run.receipt.source_hash, artifact.source_hash);
    assert_eq!(
        run.receipt.bytecode_hash,
        x3bc_core::hash::sha256(&artifact.bytes)
    );
    assert_eq!(run.receipt.chain_id, X3_CHAIN_ID);
    assert_eq!(run.receipt.anchor, anchor());
    assert_eq!(run.receipt.pre_state, pre);
    assert_ne!(
        run.receipt.pre_state_commitment,
        run.receipt.post_state_commitment
    );

    // Signed by a registered attestor.
    let signed = sign_receipt(&run.receipt, &attestor());
    assert_eq!(signed.attestation.attestor_id, "x3-attestor-1");

    // 12. Independent replay from the artifact and the transcript recomputes every figure.
    let report =
        replay_receipt(&artifact.bytes, &signed, &registry()).expect("an honest receipt replays");
    assert_eq!(
        report.recomputed, run.receipt,
        "replay re-derives the identical receipt"
    );
    assert_eq!(report.recomputed.profit.net, net);
}

#[test]
fn x3lang_trading_trap_after_swap_rolls_back_everything() {
    // Pool 2 is too shallow to return `min_out 1_005 USDX`: the second swap traps after the borrow
    // and the first swap have already moved balances, reserves and debt inside the VM.
    let artifact = compile(ARB_SOURCE);
    let mut pre = pre_state();
    pre.venues[1].reserve_b = 1_100 * UNIT;

    let run =
        execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT).expect("runs to a verdict");
    assert!(!run.committed);
    assert!(
        run.delta.is_none(),
        "a rolled-back trade has nothing to apply"
    );
    match &run.receipt.outcome {
        Outcome::RolledBack { trap, op_index } => {
            assert_eq!(*trap, x3bc_core::trap::TrapId::MinOutput);
            assert_eq!(*op_index, 2, "the second swap is the one that trapped");
        }
        other => panic!("expected a rollback, got {other:?}"),
    }

    // The trap came after real state-changing steps …
    let kinds: Vec<HostCallKind> = run
        .receipt
        .transcript
        .iter()
        .map(|call| call.kind)
        .collect();
    assert_eq!(kinds[..2], [HostCallKind::Borrow, HostCallKind::Swap]);

    // … and none of them survive: balances, reserves, debts, costs, events and intents are the
    // pre-state exactly.
    assert_eq!(
        run.receipt.post_state_commitment,
        run.receipt.pre_state_commitment
    );
    assert_eq!(run.receipt.post_balances, run.receipt.pre_balances);
    assert!(
        run.receipt.costs.is_empty(),
        "no cost is charged by a rolled-back trade"
    );
    assert!(run.receipt.events.is_empty());
    assert!(run.receipt.intents.is_empty());
    assert_eq!(run.receipt.profit.net, 0);

    // The rollback receipt is signed and replays too: the failure is reproducible.
    let signed = sign_receipt(&run.receipt, &attestor());
    let report = replay_receipt(&artifact.bytes, &signed, &registry()).expect("replays");
    assert_eq!(report.recomputed, run.receipt);
}
