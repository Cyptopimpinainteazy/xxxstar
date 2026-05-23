#![cfg(any())]

use sp_core::{H160, U256};
use x3_evm_integration::{EvmConfig, EvmExecutor, FrontierEvmExecutor};

/// Integration test: deploy a minimal storage contract (constructor stores 0x42)
/// whose runtime returns storage slot 0 on CALL. Verifies CREATE then CALL returns 0x42.
#[test]
fn integration_deploy_and_call_storage_contract() {
    let executor = FrontierEvmExecutor;

    // Construction bytecode:
    // constructor: SSTORE(0x42, 0x00) + CODECOPY(runtime) + RETURN(runtime)
    // runtime: SLOAD(0x00) + RETURN(32)
    let creation = vec![
        // constructor: store 0x42 at slot 0
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x55, // SSTORE
        // copy runtime (8 bytes) into memory and return it
        0x60, 0x08, // PUSH1 0x08 (size)
        0x60, 0x11, // PUSH1 0x11 (code offset - runtime starts after constructor)
        0x60, 0x00, // PUSH1 0x00 (mem offset)
        0x39, // CODECOPY
        0x60, 0x00, // PUSH1 0x00 (mem offset)
        0x60, 0x08, // PUSH1 0x08 (size)
        0xF3, // RETURN
        // ---- runtime (8 bytes) ----
        0x60, 0x00, // PUSH1 0x00
        0x54, // SLOAD
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0xF3, // RETURN
    ];

    // Use same wrapper format as other integration tests: leading 0x01 = CREATE,
    // then 8-byte value (0), then the initcode
    let mut payload = vec![0x01u8];
    payload.extend_from_slice(&0u64.to_be_bytes());
    payload.extend_from_slice(&creation);

    // Deploy contract
    let r = executor
        .execute(&payload, &[0xAAu8; 20], &EvmConfig::default())
        .unwrap();
    assert!(r.success, "contract CREATE should succeed");

    // Expect returned output to contain the deployed address (20 bytes)
    assert!(
        r.output.len() >= 20,
        "CREATE should return deployed address bytes"
    );
    let addr_bytes = &r.output[r.output.len().saturating_sub(20)..];
    let contract_addr = H160::from_slice(addr_bytes);

    // Now CALL the deployed contract (empty calldata) and expect 32-byte return with 0x42
    let caller = H160::from_slice(&[0xAAu8; 20]);
    let call_result = EvmExecutor::call(
        &executor,
        &[], // empty calldata -> runtime will SLOAD(slot 0) and RETURN
        caller,
        contract_addr,
        U256::zero(),
        &EvmConfig::default(),
    )
    .expect("call should succeed");

    assert!(call_result.success, "call execution should succeed");
    assert!(
        call_result.output.len() >= 32,
        "call should return 32-byte word"
    );
    // storage[0] was set to 0x42; check last byte
    assert_eq!(
        call_result.output[31], 0x42u8,
        "returned storage value must be 0x42"
    );
}
