//! Invariants of the production EVM execution boundary.
//!
//! This file previously began with `#![cfg(any())]` and so never compiled or ran;
//! its one test drove `FrontierEvmExecutor` with a `0x01`-prefixed CREATE wrapper
//! payload against an `EvmExecutor::execute` signature that no longer exists.
//! The EVM path the runtime actually wires is `WasmEvmAdapter` ->
//! `mini_evm::execute_evm`, and the properties worth pinning down at this
//! boundary are the ones a caller can observe: what the contract sees as call
//! data, and what one execution can and cannot see of another.

use sp_core::{H160, U256};
use x3_evm_integration::{mini_evm::execute_evm, EvmConfig};

fn caller() -> H160 {
    H160::repeat_byte(0xAA)
}

fn run(code: &[u8]) -> x3_evm_integration::EvmExecutionResult {
    execute_evm(code, caller(), U256::zero(), &EvmConfig::default())
        .expect("execution should succeed")
}

/// `PUSH1 0x42; PUSH1 0x00; SSTORE` then return nothing.
const WRITE_SLOT_ZERO: &[u8] = &[
    0x60, 0x42, // PUSH1 0x42 (value)
    0x60, 0x00, // PUSH1 0x00 (key)
    0x55, // SSTORE
    0x60, 0x00, // PUSH1 0x00 (size)
    0x60, 0x00, // PUSH1 0x00 (offset)
    0xf3, // RETURN
];

/// `PUSH1 0x00; SLOAD; PUSH1 0x00; MSTORE; PUSH1 0x20; PUSH1 0x00; RETURN` —
/// returns storage slot 0 without writing it.
const READ_SLOT_ZERO: &[u8] = &[
    0x60, 0x00, // PUSH1 0x00 (key)
    0x54, // SLOAD
    0x60, 0x00, // PUSH1 0x00 (offset)
    0x52, // MSTORE
    0x60, 0x20, // PUSH1 0x20 (size)
    0x60, 0x00, // PUSH1 0x00 (offset)
    0xf3, // RETURN
];

#[test]
fn the_contract_sees_the_payload_as_call_data() {
    // CALLDATASIZE; PUSH1 0x00; MSTORE; PUSH1 0x20; PUSH1 0x00; RETURN
    let code = [0x36, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];
    let result = run(&code);

    // The adapter passes the payload as *both* the code and the call data. That is
    // load-bearing: a contract sized against CALLDATASIZE depends on it, and a
    // change to pass empty call data would silently alter every such contract.
    assert_eq!(
        result.output[31] as usize,
        code.len(),
        "CALLDATASIZE must be the payload length"
    );
}

#[test]
fn state_written_by_one_execution_is_invisible_to_the_next() {
    // A real write, in its own execution.
    let written = run(WRITE_SLOT_ZERO);
    assert!(written.success);

    // A *different* contract that only reads slot 0 must see a fresh backend, not
    // the value the previous execution stored. The runtime re-executes from
    // canonical ledger state each time; if this backend ever became persistent
    // between calls, slot 0 would come back as 0x42 here.
    let read_back = run(READ_SLOT_ZERO);
    assert_eq!(read_back.output.len(), 32);
    assert_eq!(
        read_back.output[31], 0x00,
        "executions must not share in-memory EVM state"
    );
}
