use crate::{CrossVmBridge, CrossVmOperation, NoOpDispatcher};

#[test]
fn cross_vm_arbitrage_blocked_by_atomic_state() {
    let mut bridge = CrossVmBridge::new();

    // Simulated price mismatch context (EVM vs SVM) as attack precondition.
    let _evm_price = 1_050u128;
    let _svm_price = 950u128;

    bridge
        .queue_operation(CrossVmOperation::TransferToEvm {
            source: vec![1u8; 32],
            destination: [2u8; 20],
            amount: 1_000,
        })
        .expect("operation should queue");

    let dispatcher = NoOpDispatcher::testnet();

    // Settlement without a prior lock/prepare phase must be rejected.
    let commit_result = bridge.commit(&dispatcher);
    assert!(
        commit_result.is_err(),
        "bridge must reject mint/settlement without corresponding atomic prepare lock"
    );
}
