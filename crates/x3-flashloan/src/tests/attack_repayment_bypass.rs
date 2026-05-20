use crate::{
    AssetId, BorrowPurpose, BorrowRequest, ChainKind, FlashloanError, FlashloanId, FlashloanPool,
};

#[test]
fn flash_loan_repayment_bypass_reverts() {
    let chain = ChainKind::Evm(1);
    let asset = AssetId::new("USDC");

    let mut pool = FlashloanPool::new();
    pool.deposit(chain, asset.clone(), 1_000_000_000);

    let request = BorrowRequest {
        id: FlashloanId::new(),
        chain,
        asset: asset.clone(),
        amount: 200_000_000,
        purpose: BorrowPurpose::ArbExecution,
    };

    let receipt = pool.borrow(&request).expect("borrow should succeed");
    let available_after_borrow = pool.available(chain, &asset);

    let underpayment = receipt.total_owed() - 1;
    let repay_result = pool.repay(&receipt.id, underpayment);

    assert!(matches!(
        repay_result,
        Err(FlashloanError::InsufficientRepayment { .. })
    ));

    // Capital is still not restored; the outstanding loan cannot be bypassed.
    assert_eq!(pool.available(chain, &asset), available_after_borrow);
    assert!(pool.is_outstanding(&receipt.id));
}
