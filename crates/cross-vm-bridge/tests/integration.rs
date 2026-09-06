use x3_cross_vm_bridge::{CrossVmBridge, CrossVmOperation, NoOpDispatcher};

// Integration tests for the cross-VM bridge exercising the public API
// - queue_operation + execute_pending (happy path)
// - validation rejects malformed/zero-amount operations

#[test]
fn integration_execute_transfers_and_atomic_swap() {
    let mut bridge = CrossVmBridge::new();

    // Transfer SVM -> EVM
    let t1 = CrossVmOperation::TransferToEvm {
        source: vec![0x11; 32],
        destination: [0x22; 20],
        amount: 1_000u128,
    };

    // Transfer EVM -> SVM
    let t2 = CrossVmOperation::TransferToSvm {
        source: [0xAA; 20],
        destination: vec![0xBB; 32],
        amount: 2_000u128,
    };

    // Atomic swap between EVM and SVM parties
    let swap = CrossVmOperation::AtomicSwap {
        evm_party: [0xCC; 20],
        svm_party: vec![0xDD; 32],
        evm_asset: [0xEE; 20],
        svm_asset: vec![0xFF; 32],
        evm_amount: 500u128,
        svm_amount: 700u128,
    };

    bridge.queue_operation(t1).expect("queue t1");
    bridge.queue_operation(t2).expect("queue t2");
    bridge.queue_operation(swap).expect("queue swap");

    assert_eq!(bridge.pending_count(), 3);

    let dispatcher = NoOpDispatcher::testnet();
    let results = bridge
        .execute_pending_with_dispatcher(&dispatcher)
        .expect("execute pending");

    // all operations executed and succeeded
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));
    assert_eq!(bridge.completed_count(), 3);

    // outputs should contain expected markers from execute_operation()
    let combined: Vec<u8> = results.iter().flat_map(|r| r.output.clone()).collect();
    let s = String::from_utf8_lossy(&combined);
    assert!(s.contains("SVM:withdraw") || s.contains("EVM:withdraw"));
}

#[test]
fn integration_validation_rejects_invalid_operations() {
    let mut bridge = CrossVmBridge::new();

    // zero amount is invalid
    let invalid_zero = CrossVmOperation::TransferToEvm {
        source: vec![0x01; 32],
        destination: [0x02; 20],
        amount: 0u128,
    };
    assert!(bridge.queue_operation(invalid_zero).is_err());

    // malformed SVM destination (not 32 bytes)
    let invalid_addr = CrossVmOperation::TransferToSvm {
        source: [0x01; 20],
        destination: vec![0x02; 31], // invalid length
        amount: 1u128,
    };
    assert!(bridge.queue_operation(invalid_addr).is_err());
}
