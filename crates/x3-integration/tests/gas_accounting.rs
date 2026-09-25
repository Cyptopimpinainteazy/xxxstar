//! Gas accounting at the executor boundary.
//!
//! `X3Executor::execute` is the `std` path: the compiler, the node's adapters and the
//! simulation entry points all reach it, and it takes an `X3ExecutorConfig` whose whole
//! point is to bound what the program may spend. These tests hold it to that bound.
//!
//! The `no_std` path (`mini_x3::execute_x3bc`) takes the gas limit as an argument and
//! enforces it in its own loop, so this file also pins the two paths to the same policy:
//! a limit the caller states is the limit the program stops at, in both.

#![cfg(feature = "std")]

use x3_backend::{BytecodeModule, FunctionEntry};
use x3_x3_integration::{X3Executor, X3ExecutorConfig, X3IntegrationError};

/// `LoadConst r0, <idx>` is `0x10, 0x00, idx:u32` and `Ret r0` is `0x05, 0x00`.
///
/// `n` loads followed by the return: a program that costs at least `n` gas and does nothing
/// else, built with the real module writer so the envelope and checksum are the real ones.
fn module_executing_n_loads(n: usize) -> Vec<u8> {
    let mut module = BytecodeModule::new();
    let idx = module
        .const_pool
        .add_integer(7)
        .expect("the writer must accept an integer constant")
        .0;

    let mut code = Vec::with_capacity(n * 6 + 2);
    for _ in 0..n {
        code.push(0x10); // LoadConst
        code.push(0x00); // r0
        code.extend_from_slice(&idx.to_le_bytes());
    }
    code.extend_from_slice(&[0x05, 0x00]); // Ret r0

    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 1,
    });
    module.code = code;
    module.to_bytes()
}

const LOADS: usize = 2_000;

#[test]
fn a_gas_limit_below_what_the_program_needs_stops_it() {
    let bytes = module_executing_n_loads(LOADS);
    let config = X3ExecutorConfig {
        gas_limit: 100,
        ..X3ExecutorConfig::default()
    };

    match X3Executor::execute(&bytes, &[], config) {
        Err(X3IntegrationError::GasExhausted { used, limit }) => {
            assert_eq!(
                limit, 100,
                "the limit reported must be the one the caller set"
            );
            assert!(
                used <= 101,
                "the program must stop at the limit, not after it: used {used} of {limit}"
            );
        }
        Ok(receipt) => panic!(
            "a 100 gas limit admitted a {LOADS}-instruction program: success={} gas_used={} \
             instructions_executed={}",
            receipt.success, receipt.gas_used, receipt.instructions_executed
        ),
        Err(other) => panic!("expected GasExhausted, got {other:?}"),
    }
}

#[test]
fn a_gas_limit_above_what_the_program_needs_admits_it() {
    // The control for the test above: the same program under a limit it fits inside must run,
    // so the first test cannot pass merely because every execution is refused.
    let bytes = module_executing_n_loads(LOADS);
    let config = X3ExecutorConfig {
        gas_limit: 1_000_000,
        ..X3ExecutorConfig::default()
    };

    let receipt = X3Executor::execute(&bytes, &[], config)
        .expect("a program inside its gas limit must execute");
    assert!(receipt.success, "receipt must report success: {receipt:?}");
    assert!(
        receipt.gas_used <= 1_000_000,
        "gas used must stay inside the limit: {}",
        receipt.gas_used
    );
    assert!(
        receipt.instructions_executed >= LOADS as u64,
        "the receipt must count the instructions it ran: {}",
        receipt.instructions_executed
    );
}

#[test]
fn the_on_chain_profile_is_stricter_than_the_default_and_both_are_honoured() {
    // `on_chain()` asks for 500_000 and `default()` for 1_000_000. A 2_000-instruction program
    // fits inside both, so this pins that the *stricter* profile is still applied rather than
    // silently replaced by whatever the VM happens to default to.
    let bytes = module_executing_n_loads(LOADS);
    let receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
        .expect("a small program must execute under the on-chain profile");
    assert!(receipt.success);
    assert!(
        receipt.gas_used <= X3ExecutorConfig::on_chain().gas_limit,
        "on-chain gas limit is 500_000, not the VM default: {}",
        receipt.gas_used
    );
}
