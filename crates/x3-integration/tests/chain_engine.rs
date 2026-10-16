//! The chain executes X3BC with `x3-vm`.
//!
//! `X3Executor::execute_on_chain` is the entry the kernel's wasm adapter calls on every node. It ran
//! `mini_x3`, a second interpreter that existed because `x3-vm` needed `std`; it runs `x3-vm` now,
//! in every build. These tests pin the observable differences that switch makes, so a regression to
//! a second engine (or a drift between what tools simulate and what blocks execute) shows up here:
//!
//! - gas is `x3-vm`'s per-opcode schedule, the same figure a simulation reports;
//! - the opcodes `x3-vm` alone executed (`atomic_check`) run on chain;
//! - an opcode the engine cannot run is refused at intake, not on the block that reaches it;
//! - the receipt contract the kernel relies on (gas exhaustion, faults, private submission, slot
//!   seeds) is unchanged.

#![cfg(feature = "std")]

use x3_backend::bc_format::FeatureFlags;
use x3_backend::{BytecodeModule, FunctionEntry};
use x3_x3_integration::{X3Executor, X3IntegrationError};

const GAS: u64 = 1_000_000;

fn module(code: Vec<u8>) -> BytecodeModule {
    let mut module = BytecodeModule::new();
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 1,
    });
    module.code = code;
    module
}

fn bytes(code: Vec<u8>) -> Vec<u8> {
    module(code).to_bytes()
}

/// `LoadImm r1, 0; LoadImm r2, 5; EvmSstore r1 r2; Ret r2`.
fn store_five() -> Vec<u8> {
    bytes(vec![
        0x18, 0x01, 0x00, // LoadImm r1, 0
        0x18, 0x02, 0x05, // LoadImm r2, 5
        0xB4, 0x01, 0x02, // EvmSstore slot=r1 value=r2
        0x05, 0x02, // Ret r2
    ])
}

#[test]
fn chain_gas_is_the_engines_per_opcode_schedule_and_matches_a_simulation() {
    let program = store_five();
    let chain = X3Executor::execute_on_chain(&program, GAS, &[], false).expect("runs");
    assert!(chain.success);
    // A slot write costs 5_000 on x3-vm's schedule. The interpreter this replaced charged one per
    // instruction, so this receipt would have said 4.
    assert!(
        chain.gas_used >= 5_000,
        "the chain must charge the engine's schedule, got {}",
        chain.gas_used
    );
    assert_eq!(chain.instructions_executed, 4);

    // A tool simulating the same program on the same engine gets the same figure.
    let mut vm = x3_vm::VM::from_bytes(&program).expect("loads");
    vm.call_function(0, &[]).expect("runs");
    assert_eq!(
        chain.gas_used,
        vm.gas_used(),
        "simulation and chain are one engine"
    );
}

#[test]
fn atomic_check_runs_on_chain() {
    // AtomicBegin; AtomicCheck r1; AtomicCommit; Ret r1  -> true
    let program = bytes(vec![
        0x90, 0x00, 0x00, // AtomicBegin id 0
        0x93, 0x01, // AtomicCheck r1
        0x91, 0x00, 0x00, // AtomicCommit id 0
        0x05, 0x01, // Ret r1
    ]);
    let receipt = X3Executor::execute_on_chain(&program, GAS, &[], false).expect("admitted");
    assert!(
        receipt.success,
        "{:?}",
        String::from_utf8_lossy(&receipt.return_data)
    );
    assert_eq!(receipt.return_data, vec![1u8], "a window was open");
}

#[test]
fn an_opcode_the_engine_cannot_run_is_refused_before_anything_runs() {
    // `ctx_sender r1` — the engine has no block context yet.
    let program = bytes(vec![0x80, 0x01, 0x05, 0x01]);
    match X3Executor::execute_on_chain(&program, GAS, &[], false) {
        Err(X3IntegrationError::VerificationFailed(reason)) => {
            assert!(reason.contains("NotExecutable"), "{reason}")
        }
        other => panic!("must be refused at intake, got {other:?}"),
    }
}

#[test]
fn gas_exhaustion_is_an_unsuccessful_receipt_charged_the_whole_limit() {
    let program = store_five();
    let receipt = X3Executor::execute_on_chain(&program, 100, &[], false).expect("a receipt");
    assert!(!receipt.success);
    assert_eq!(receipt.gas_used, 100);
    assert!(
        receipt.storage_writes.is_empty(),
        "a failed run writes nothing"
    );
}

#[test]
fn a_fault_after_a_write_reports_no_write_and_the_gas_it_consumed() {
    // Store a slot, then divide by zero.
    let program = bytes(vec![
        0x18, 0x01, 0x00, // LoadImm r1, 0
        0x18, 0x02, 0x05, // LoadImm r2, 5
        0xB4, 0x01, 0x02, // EvmSstore slot=r1 value=r2
        0x18, 0x03, 0x00, // LoadImm r3, 0
        0x23, 0x04, 0x02, 0x03, // DivI r4 = r2 / r3
        0x05, 0x04, // Ret r4
    ]);
    let receipt = X3Executor::execute_on_chain(&program, GAS, &[], false).expect("a receipt");
    assert!(!receipt.success);
    assert!(
        String::from_utf8_lossy(&receipt.return_data).contains("DivisionByZero"),
        "the receipt names the fault"
    );
    assert!(
        receipt.storage_writes.is_empty(),
        "the write before the fault never reaches the chain"
    );
    assert!(
        receipt.gas_used >= 5_000 && receipt.gas_used < GAS,
        "charged what it consumed, not nothing and not the limit: {}",
        receipt.gas_used
    );
}

#[test]
fn private_submission_is_refused_without_a_channel_and_run_with_one() {
    let mut demanding = module(vec![0x18, 0x01, 0x07, 0x05, 0x01]);
    demanding
        .features
        .set(FeatureFlags::PRIVATE_SUBMISSION_REQUIRED);
    let program = demanding.to_bytes();
    assert!(matches!(
        X3Executor::execute_on_chain(&program, GAS, &[], false),
        Err(X3IntegrationError::ExecutionFailed(_))
    ));
    let receipt = X3Executor::execute_on_chain(&program, GAS, &[], true).expect("a channel exists");
    assert!(receipt.success);
}

#[test]
fn a_seeded_slot_is_read_and_reported_as_the_old_value() {
    // Load slot 0, then store 9 over it; return what was loaded.
    let program = bytes(vec![
        0x18, 0x01, 0x00, // LoadImm r1, 0
        0xB3, 0x03, 0x01, // EvmSload r3 <- slot r1
        0x18, 0x02, 0x09, // LoadImm r2, 9
        0xB4, 0x01, 0x02, // EvmSstore slot r1 <- r2
        0x05, 0x03, // Ret r3
    ]);
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&0x5833_4556_4D5F_534Cu64.to_le_bytes());
    let mut seven = [0u8; 32];
    seven[0] = 1;
    seven[1] = 8;
    seven[2..10].copy_from_slice(&7i64.to_le_bytes());

    let receipt =
        X3Executor::execute_on_chain(&program, GAS, &[(key, seven)], false).expect("runs");
    assert!(receipt.success);
    assert_eq!(receipt.return_data, 7i64.to_le_bytes().to_vec());
    assert_eq!(receipt.storage_writes.len(), 1);
    assert_eq!(receipt.storage_writes[0].old_value, Some(seven));
}
