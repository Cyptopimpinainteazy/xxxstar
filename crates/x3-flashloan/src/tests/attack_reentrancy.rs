use crate::{AssetId, BorrowPurpose, BorrowRequest, ChainKind, FlashloanId, FlashloanPool};

#[test]
fn flash_loan_reentrancy_rejected() {
    let chain = ChainKind::Evm(1);
    let asset = AssetId::new("USDC");

    let mut pool = FlashloanPool::new();
    pool.deposit(chain, asset.clone(), 1_000_000_000);

    let request_1 = BorrowRequest {
        id: FlashloanId::new(),
        chain,
        asset: asset.clone(),
        amount: 200_000_000,
        purpose: BorrowPurpose::ArbExecution,
    };

    let request_2 = BorrowRequest {
        id: FlashloanId::new(),
        chain,
        asset,
        amount: 100_000_000,
        purpose: BorrowPurpose::ArbExecution,
    };

    let receipt_1 = pool
        .borrow(&request_1)
        .expect("first borrow should succeed");
    let second = pool.borrow(&request_2);

    assert!(
        second.is_err(),
        "pool must reject concurrent same-asset borrow while first loan is outstanding"
    );
    assert!(pool.is_outstanding(&receipt_1.id));
}
