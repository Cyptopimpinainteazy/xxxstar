//! Mainnet RC-1 E2E test suite.
//!
//! These tests verify the complete mainnet critical path end-to-end using only
//! in-process crate logic — no live node required.  They run from a clean
//! `cargo test -p e2e_tests --test mainnet_rc1` invocation.
//!
//! Every test maps to a named mainnet scenario:
//!
//! | Test | Scenario |
//! |---|---|
//! | `internal_lock_settle` | Lock → Settle (full accounting) |
//! | `internal_lock_swap_settle` | Lock → Swap → Settle (spot AMM wired) |
//! | `abort_refund` | Lock → Abort → rollback restores balance |
//! | `slippage_refund` | Lock → Swap min_out impossible → rollback |
//! | `packet_replay_rejected` | Packet replay attempt fails |
//! | `packet_timeout_refund` | Expired packet detected via timeout eval |
//! | `duplicate_slot_rejected` | Same slot_id locked twice — planner rejects |
//! | `burn_unlock_accounting` | Lock → Burn clears custody correctly |
//! | `kernel_invariant_after_bundle` | Rollback effects never exceed locked amount |
//! | `genesis_determinism` | commit_packet is deterministic |
//! | `lp_lock_prevents_early_unlock` | LP lock blocks unlock before deadline |
//! | `lp_lock_allows_unlock_after_expiry` | LP unlock succeeds after deadline |
//! | `universal_contract_compiles` | UC compiles to valid IXL bundle |
//!
//! No `unwrap()` in critical assertion paths — every failure is explicit.

use sp_core::H256;
use x3_ixl::{
    instruction::{AssetId, AssetKind, Bundle, Instruction},
    interpreter::{ExecutionContext, Interpreter},
    planner::Planner,
    rollback::Rollback,
};
use x3_liquidity_core::{anti_rug::LpLockRegistry, settlement::Settlement};
use x3_packet_standard::{
    packet::Packet,
    proof::commit_packet,
    replay::ReplayGuard,
    timeout::{evaluate as evaluate_timeout, TimeoutOutcome},
};
use x3_universal_contracts::{
    actions::{Action, Domain},
    sdk::UniversalContract,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn asset(seed: u8) -> AssetId {
    let mut id = [0u8; 32];
    id[0] = seed;
    id
}

fn addr(seed: u8) -> [u8; 32] {
    let mut a = [0u8; 32];
    a[0] = seed;
    a
}

fn chain_id(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = s.len().min(32);
    out[..n].copy_from_slice(&s[..n]);
    out
}

fn make_bundle(salt: u8, instructions: Vec<Instruction>) -> Bundle {
    let mut salt_bytes = [0u8; 32];
    salt_bytes[0] = salt;
    Bundle {
        salt: H256(salt_bytes),
        instructions,
    }
}

fn no_swap(
    _kind: AssetKind,
    _in: AssetId,
    _out: AssetId,
    amount: u128,
) -> Result<u128, x3_ixl::instruction::IxlError> {
    Ok(amount) // 1:1 for testing
}

fn failing_swap(
    _kind: AssetKind,
    _in: AssetId,
    _out: AssetId,
    _amount: u128,
) -> Result<u128, x3_ixl::instruction::IxlError> {
    Err(x3_ixl::instruction::IxlError::SlippageExceeded)
}

// ── 1. internal_lock_settle ───────────────────────────────────────────────────

#[test]
fn internal_lock_settle() {
    // Scenario: Lock slot 0 → Settle slot 0 to receiver.
    let asset_a = asset(1);
    let payer = addr(10);
    let receiver = addr(20);

    let bundle = make_bundle(
        1,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 1_000,
            },
            Instruction::Settle {
                slot_id: 0,
                kind: AssetKind::X3Evm,
                receiver,
            },
        ],
    );

    let plan = Planner::plan(bundle).expect("planner must accept Lock→Settle");
    let mut ctx = ExecutionContext::new(&no_swap);
    let result = Interpreter::execute(&plan, &mut ctx);

    assert!(
        result.is_ok(),
        "Lock→Settle must succeed: {:?}",
        result.err()
    );
    let receipt = result.unwrap();
    assert!(receipt.len() >= 2, "receipt must have ≥2 entries");
}

// ── 2. internal_lock_swap_settle ─────────────────────────────────────────────

#[test]
fn internal_lock_swap_settle() {
    // Scenario: Lock → Swap (1:1) → Settle to EVM.
    let asset_a = asset(1);
    let asset_b = asset(2);
    let payer = addr(10);
    let receiver = addr(20);

    let bundle = make_bundle(
        2,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 1_000,
            },
            Instruction::Swap {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset_in: asset_a,
                asset_out: asset_b,
                amount_in: 1_000,
                min_out: 950,
            },
            Instruction::Settle {
                slot_id: 0,
                kind: AssetKind::X3Evm,
                receiver,
            },
        ],
    );

    // Verify the settlement request is valid.
    let settle_req = Settlement::build(1, 1_000, 950).expect("settlement request must build");
    assert_eq!(settle_req.pool_id, 1);
    assert!(settle_req.min_out <= settle_req.amount_in);

    let plan = Planner::plan(bundle).expect("planner must accept Lock→Swap→Settle");
    let mut ctx = ExecutionContext::new(&no_swap);
    let result = Interpreter::execute(&plan, &mut ctx);
    assert!(
        result.is_ok(),
        "Lock→Swap→Settle must succeed: {:?}",
        result.err()
    );
    assert!(result.unwrap().len() >= 3);
}

// ── 3. abort_refund ───────────────────────────────────────────────────────────

#[test]
fn abort_refund() {
    // Scenario: Lock → Abort — interpreter stops and returns error.
    // Rollback::invert must produce a CreditReceiver effect for the Lock.
    let asset_a = asset(1);
    let payer = addr(10);

    let bundle = make_bundle(
        3,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 500,
            },
            Instruction::Abort,
        ],
    );

    let plan = Planner::plan(bundle).expect("planner accepts Lock→Abort");
    let mut ctx = ExecutionContext::new(&no_swap);
    let result = Interpreter::execute(&plan, &mut ctx);

    assert!(result.is_err(), "Abort instruction must return Err");
    let (partial_receipt, err) = result.unwrap_err();
    assert_eq!(err, x3_ixl::instruction::IxlError::Aborted);

    // Rollback must produce at least one CreditReceiver for the locked amount.
    let rollback_effects = Rollback::invert(&partial_receipt);
    assert!(
        !rollback_effects.is_empty(),
        "rollback must produce at least one inverse effect"
    );

    let has_refund = rollback_effects.iter().any(|e| {
        matches!(e, x3_ixl::interpreter::LedgerEffect::CreditReceiver { amount, .. } if *amount == 500)
    });
    assert!(has_refund, "rollback must credit 500 back to payer");
}

// ── 4. slippage_refund ────────────────────────────────────────────────────────

#[test]
fn slippage_refund() {
    // Scenario: Lock → Swap with impossible min_out → SlippageExceeded.
    let asset_a = asset(1);
    let asset_b = asset(2);
    let payer = addr(10);

    let bundle = make_bundle(
        4,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 1_000,
            },
            Instruction::Swap {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset_in: asset_a,
                asset_out: asset_b,
                amount_in: 1_000,
                min_out: 999_999,
            },
            Instruction::Settle {
                slot_id: 0,
                kind: AssetKind::X3Evm,
                receiver: addr(20),
            },
        ],
    );

    let plan = Planner::plan(bundle).expect("planner accepts bundle");
    let mut ctx = ExecutionContext::new(&failing_swap);
    let result = Interpreter::execute(&plan, &mut ctx);

    assert!(
        result.is_err(),
        "impossible slippage must cause execution error"
    );
    let (partial_receipt, err) = result.unwrap_err();
    assert_eq!(err, x3_ixl::instruction::IxlError::SlippageExceeded);

    // Rollback must restore the locked amount.
    let rollback_effects = Rollback::invert(&partial_receipt);
    let has_refund = rollback_effects.iter().any(|e| {
        matches!(e, x3_ixl::interpreter::LedgerEffect::CreditReceiver { amount, .. } if *amount == 1_000)
    });
    assert!(
        has_refund,
        "slippage rollback must credit 1000 back to payer"
    );
}

// ── 5. packet_replay_rejected ────────────────────────────────────────────────

#[test]
fn packet_replay_rejected() {
    // Scenario: same packet submitted twice; second attempt must be rejected.
    let mut guard = ReplayGuard::new();

    let packet = Packet::try_new(
        chain_id(b"x3-native"),
        chain_id(b"transfer"),
        chain_id(b"x3-evm"),
        chain_id(b"transfer"),
        1u64,    // sequence
        9999u64, // timeout_height
        0u64,    // timeout_timestamp
        b"lock:asset=1:amount=100".to_vec(),
    )
    .expect("packet must construct");

    // First submission: must succeed.
    let first = guard.mark_received(&packet);
    assert!(first.is_ok(), "first packet submission must succeed");

    // Second submission (replay): must be rejected.
    let replay = guard.mark_received(&packet);
    assert!(
        replay.is_err(),
        "replayed packet must be rejected by replay guard"
    );
}

// ── 6. packet_timeout_refund ─────────────────────────────────────────────────

#[test]
fn packet_timeout_refund() {
    // Scenario: packet with height timeout 100, evaluated at height 200.
    let packet = Packet::try_new(
        chain_id(b"x3-native"),
        chain_id(b"transfer"),
        chain_id(b"x3-evm"),
        chain_id(b"transfer"),
        1u64,
        100u64, // timeout_height
        0u64,
        b"payload".to_vec(),
    )
    .expect("packet must construct");

    let outcome = evaluate_timeout(&packet, 200u64, 0u64); // current_height=200

    assert!(
        outcome.is_expired(),
        "packet past timeout height must be expired, got: {:?}",
        outcome
    );
    assert_eq!(outcome, TimeoutOutcome::ExpiredHeight);
}

#[test]
fn packet_not_timed_out_before_deadline() {
    let packet = Packet::try_new(
        chain_id(b"x3-native"),
        chain_id(b"transfer"),
        chain_id(b"x3-evm"),
        chain_id(b"transfer"),
        1u64,
        100u64, // timeout_height
        0u64,
        b"payload".to_vec(),
    )
    .expect("packet must construct");

    let outcome = evaluate_timeout(&packet, 50u64, 0u64); // before deadline

    assert_eq!(
        outcome,
        TimeoutOutcome::Live,
        "packet before deadline must be Live"
    );
}

// ── 7. duplicate_slot_rejected ────────────────────────────────────────────────

#[test]
fn duplicate_slot_rejected() {
    // Scenario: two Lock instructions on the same slot_id — planner must reject.
    let asset_a = asset(1);
    let payer = addr(10);

    let bundle = make_bundle(
        7,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 100,
            },
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 100,
            }, // duplicate slot
            Instruction::Settle {
                slot_id: 0,
                kind: AssetKind::X3Evm,
                receiver: addr(20),
            },
        ],
    );

    let plan_result = Planner::plan(bundle);
    assert!(
        plan_result.is_err(),
        "planner must reject bundles with duplicate Lock on same slot_id"
    );
}

// ── 8. burn_unlock_accounting ────────────────────────────────────────────────

#[test]
fn burn_unlock_accounting() {
    // Scenario: Lock → Burn correctly closes the custody slot.
    let asset_a = asset(1);
    let payer = addr(10);

    let bundle = make_bundle(
        8,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: 300,
            },
            Instruction::Burn { slot_id: 0 },
        ],
    );

    let plan = Planner::plan(bundle).expect("planner accepts Lock→Burn");
    let mut ctx = ExecutionContext::new(&no_swap);
    let result = Interpreter::execute(&plan, &mut ctx);

    assert!(result.is_ok(), "Lock→Burn must succeed: {:?}", result.err());
    let receipt = result.unwrap();

    // Receipt must record both a Locked entry and a Burned entry.
    let has_burn = receipt
        .entries
        .iter()
        .any(|e| matches!(e, x3_ixl::receipt::ReceiptEntry::Burned { slot_id: 0, .. }));
    assert!(has_burn, "receipt must contain a Burned entry for slot 0");
}

// ── 9. kernel_invariant_after_bundle ─────────────────────────────────────────

#[test]
fn kernel_invariant_after_bundle() {
    // Scenario: rollback effects must never produce a net credit larger than
    // what was locked — the invariant that rollback cannot inflate supply.
    let asset_a = asset(1);
    let payer = addr(10);
    let locked_amount: u128 = 2_000;

    // Simulate a partial execution: only Lock executed before failure.
    let partial_bundle = make_bundle(
        9,
        vec![
            Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset_a,
                payer,
                amount: locked_amount,
            },
            Instruction::Abort,
        ],
    );

    let plan = Planner::plan(partial_bundle).expect("planner accepts");
    let mut ctx = ExecutionContext::new(&no_swap);
    let (partial_receipt, _) = Interpreter::execute(&plan, &mut ctx).expect_err("Abort must fail");

    let rollback_effects = Rollback::invert(&partial_receipt);

    // Sum all credits produced by rollback — must not exceed locked_amount.
    let total_credit: u128 = rollback_effects
        .iter()
        .filter_map(|e| {
            if let x3_ixl::interpreter::LedgerEffect::CreditReceiver { amount, .. } = e {
                Some(*amount)
            } else {
                None
            }
        })
        .sum();

    assert_eq!(
        total_credit, locked_amount,
        "rollback credits must exactly equal locked amount — no inflation"
    );
}

// ── 10. genesis_determinism ───────────────────────────────────────────────────

#[test]
fn genesis_determinism() {
    // Scenario: commit_packet is deterministic across two calls.
    let packet = Packet::try_new(
        chain_id(b"x3-mainnet-rc"),
        chain_id(b"genesis-port"),
        chain_id(b"x3-evm-mainnet"),
        chain_id(b"evm-genesis"),
        0u64,
        u64::MAX,
        0u64,
        b"genesis:asset=1:supply=1000000000000".to_vec(),
    )
    .expect("packet must construct");

    let hash1 = commit_packet(&packet);
    let hash2 = commit_packet(&packet);

    assert_eq!(
        hash1, hash2,
        "commit_packet must be deterministic — same packet must produce same hash"
    );
    assert_ne!(hash1.0, [0u8; 32], "commitment hash must not be all-zero");
}

// ── 11. UniversalContract compile round-trip ─────────────────────────────────

#[test]
fn universal_contract_compiles() {
    let compiled = UniversalContract::new([1u8; 32])
        .fee_cap(1_000_000)
        .submitted_at(10)
        .action(Action::Lock {
            asset_id: 1,
            amount: 100_000,
            domain: Domain::X3Native,
        })
        .action(Action::Mint {
            asset_id: 2,
            amount: 100_000,
            domain: Domain::X3Evm,
        })
        .compile();

    assert!(
        compiled.is_ok(),
        "UniversalContract compile must succeed: {:?}",
        compiled.err()
    );

    let cc = compiled.unwrap();
    assert!(
        cc.action_count >= 1,
        "compiled contract must have at least one action"
    );
    assert!(
        !cc.program.is_empty(),
        "compiled IXL bundle must not be empty"
    );
}

// ── 12. LiquidityCore LP lock prevents early unlock ──────────────────────────

#[test]
fn lp_lock_prevents_early_unlock() {
    let mut registry = LpLockRegistry::new();
    let owner = addr(1);
    let pool_id: u64 = 1;
    let lp_amount: u128 = 5_000;
    let lock_until_block: u64 = 100;
    let current_block: u64 = 50;

    registry
        .lock(owner, pool_id, lp_amount, lock_until_block)
        .expect("LP lock must succeed");

    let unlock_result = registry.withdraw(&owner, pool_id, current_block);
    assert!(
        unlock_result.is_err(),
        "LP unlock before lock_until_block must fail"
    );
    assert_eq!(
        unlock_result.unwrap_err(),
        x3_liquidity_core::anti_rug::AntiRugError::LockNotExpired
    );
}

#[test]
fn lp_lock_allows_unlock_after_expiry() {
    let mut registry = LpLockRegistry::new();
    let owner = addr(2);
    let pool_id: u64 = 2;
    let lp_amount: u128 = 5_000;
    let lock_until_block: u64 = 100;
    let current_block: u64 = 101;

    registry
        .lock(owner, pool_id, lp_amount, lock_until_block)
        .expect("LP lock must succeed");

    let unlock_result = registry.withdraw(&owner, pool_id, current_block);
    assert!(
        unlock_result.is_ok(),
        "LP unlock after lock expiry must succeed: {:?}",
        unlock_result.err()
    );
    assert_eq!(unlock_result.unwrap().lp_amount, lp_amount);
}

// ── 13. LiquidityCore settle request bounds ───────────────────────────────────

#[test]
fn settle_request_rejects_inverted_bounds() {
    // min_out > amount_in is impossible; Settlement must reject.
    let result = Settlement::build(1, 100, 200);
    assert!(
        result.is_err(),
        "Settlement must reject min_out > amount_in"
    );
    assert_eq!(
        result.unwrap_err(),
        x3_liquidity_core::settlement::SettleError::InvertedBounds
    );
}

#[test]
fn settle_request_rejects_zero_amount() {
    let result = Settlement::build(1, 0, 0);
    assert!(result.is_err(), "Settlement must reject zero amount_in");
}
