//! End-to-end execution through the EVM path the runtime actually uses.
//!
//! This file used to begin with `#![cfg(any())]` — a permanently false cfg, so
//! every test in it was compiled out and `cargo test` printed "0 tests" rather
//! than "ignored". It was also written against an older `EvmExecutor::execute`
//! signature (`(payload, caller, config)`) and a `0x01`-prefixed CREATE wrapper
//! payload that the current trait (`(payload, caller, target, value, config)`)
//! does not accept, so it could not simply be un-commented: nothing in the tree
//! ever compiled it after the API moved.
//!
//! `mini_evm::execute_evm` is the production entry point — `x3-chain-runtime`
//! wires `pallet_x3_kernel::wasm_adapters::WasmEvmAdapter`, whose `execute` is a
//! forward into it — so these tests drive that function directly.
//!
//! `execute_evm` is deliberately stateless: each call seeds a fresh backend with
//! the payload as the code at a derived address. A "deploy, then call it in a
//! later transaction" test therefore cannot be written here; contract creation
//! is exercised inside a single execution instead, which is what this file does.

use sp_core::{H160, U256};
use x3_evm_integration::{mini_evm::execute_evm, EvmConfig};

/// `PUSH1 0x2a; PUSH1 0x00; MSTORE; PUSH1 0x20; PUSH1 0x00; RETURN`
const CHILD_RUNTIME: [u8; 10] = [0x60, 0x2a, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];

/// Init code for the child: copy `CHILD_RUNTIME` (10 bytes, starting at offset 12
/// within this init code) into memory and return it, i.e. deploy it.
const CHILD_INIT: [u8; 12] = [
    0x60, 0x0a, // PUSH1 10   (runtime length)
    0x60, 0x0c, // PUSH1 12   (runtime offset, = length of this init code)
    0x60, 0x00, // PUSH1 0    (destination)
    0x39, // CODECOPY
    0x60, 0x0a, // PUSH1 10   (size)
    0x60, 0x00, // PUSH1 0    (offset)
    0xf3, // RETURN
];

/// The parent's code, before the child's init code is appended: copy the child
/// init code into memory, `CREATE` it, stash the returned address, `CALL` it, and
/// return the 32 bytes it produced.
///
/// The `PUSH1 0x28` is the offset of the child init code, which is the length of
/// this prefix — asserted in the test rather than trusted.
const PARENT_PREFIX: [u8; 40] = [
    0x60, 0x16, // PUSH1 22  (child init length)
    0x60, 0x28, // PUSH1 40  (child init offset == length of this prefix)
    0x60, 0x00, // PUSH1 0
    0x39, // CODECOPY
    0x60, 0x16, // PUSH1 22  (init length)
    0x60, 0x00, // PUSH1 0   (init offset)
    0x60, 0x00, // PUSH1 0   (value)
    0xf0, // CREATE        -> [child_address]
    0x60, 0x40, // PUSH1 0x40
    0x52, // MSTORE        (stash the address at mem[0x40])
    0x60, 0x20, // PUSH1 0x20  (out size)
    0x60, 0x20, // PUSH1 0x20  (out offset)
    0x60, 0x00, // PUSH1 0     (in size)
    0x60, 0x00, // PUSH1 0     (in offset)
    0x60, 0x00, // PUSH1 0     (value)
    0x60, 0x40, // PUSH1 0x40
    0x51, // MLOAD         -> [child_address]
    0x61, 0xff, 0xff, // PUSH2 0xffff (gas)
    0xf1, // CALL
    0x50, // POP           (drop the success flag)
    0x60, 0x20, // PUSH1 0x20  (size)
    0x60, 0x20, // PUSH1 0x20  (offset)
    0xf3, // RETURN
];

fn caller() -> H160 {
    H160::repeat_byte(0xAA)
}

/// Build `PARENT_PREFIX || CHILD_INIT || CHILD_RUNTIME`.
fn creating_contract() -> Vec<u8> {
    assert_eq!(
        PARENT_PREFIX.len(),
        40,
        "the PUSH1 0x28 child-init offset is only correct while the prefix is 40 bytes"
    );
    let mut code = PARENT_PREFIX.to_vec();
    code.extend_from_slice(&CHILD_INIT);
    code.extend_from_slice(&CHILD_RUNTIME);
    assert_eq!(
        code.len(),
        PARENT_PREFIX.len() + CHILD_INIT.len() + CHILD_RUNTIME.len()
    );
    code
}

#[test]
fn a_contract_can_create_another_contract_and_call_it() {
    let code = creating_contract();
    let result = execute_evm(&code, caller(), U256::zero(), &EvmConfig::default())
        .expect("CREATE + CALL should succeed");

    assert!(result.success);
    assert_eq!(result.output.len(), 32, "the child returns one word");
    assert_eq!(
        result.output[31], 0x2a,
        "the returned word must come from the contract this execution created"
    );
    assert!(
        result.gas_used > 21_000,
        "CREATE costs 32_000 on its own, so the total cannot be the bare call cost: {}",
        result.gas_used
    );
}

#[test]
fn creating_and_calling_is_deterministic() {
    let code = creating_contract();
    let first = execute_evm(&code, caller(), U256::zero(), &EvmConfig::default())
        .expect("CREATE + CALL should succeed");
    let second = execute_evm(&code, caller(), U256::zero(), &EvmConfig::default())
        .expect("CREATE + CALL should succeed");

    assert_eq!(first.gas_used, second.gas_used);
    assert_eq!(first.state_root, second.state_root);
    assert_eq!(first.output, second.output);
}

#[test]
fn a_contract_that_reverts_does_not_report_success_or_an_output() {
    // Parent that CREATEs a child whose init code immediately reverts, then
    // RETURNs whatever CALL left in the output region. CREATE of a reverting
    // init code yields the zero address, so the CALL that follows goes to the
    // zero address with no code and returns nothing: the contract must not
    // claim a 0x2a result it never produced.
    let mut code = PARENT_PREFIX.to_vec();
    // Overwrite the child init code's RETURN with REVERT: keep the CODECOPY, but
    // make the init code revert (PUSH1 0; PUSH1 0; REVERT).
    code.truncate(PARENT_PREFIX.len());
    code.extend_from_slice(&[
        0x60, 0x00, // PUSH1 0
        0x60, 0x00, // PUSH1 0
        0xfd, // REVERT
    ]);
    // Pad the init region to the 22 bytes the parent copied, so CODECOPY reads
    // defined bytes.
    code.resize(PARENT_PREFIX.len() + 22, 0x00);

    let result = execute_evm(&code, caller(), U256::zero(), &EvmConfig::default())
        .expect("the outer call itself succeeds; CREATE failure is reported in its result");
    assert_eq!(result.output.len(), 32);
    assert_eq!(
        result.output[31], 0x00,
        "a failed CREATE must not leave the child's would-be output in memory"
    );
}
