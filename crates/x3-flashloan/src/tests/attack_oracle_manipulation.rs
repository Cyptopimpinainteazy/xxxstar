use crate::{
    AssetId, AtomicExecutor, BorrowPurpose, BorrowRequest, ChainKind, FlashloanError, FlashloanId,
    FlashloanPool, LegOutcome,
};

#[test]
fn flash_loan_oracle_manipulation_fails() {
    let chain = ChainKind::Evm(1);
    let asset = AssetId::new("USDC");

    let mut pool = FlashloanPool::new();
    pool.deposit(chain, asset.clone(), 10_000_000_000_000);
    let available_before = pool.available(chain, &asset);

    let borrow = BorrowRequest {
        id: FlashloanId::new(),
        chain,
        asset: asset.clone(),
        amount: available_before,
        purpose: BorrowPurpose::ArbExecution,
    };

    let receipt = pool.borrow(&borrow).expect("borrow should be issued");

    let mut executor = AtomicExecutor::new();
    let ctx_id = executor.begin();
    executor.record_leg(
        ctx_id,
        LegOutcome::Failure {
            chain,
            reason: "oracle_spot_price_manipulation".to_string(),
        },
    );

    let result = executor.finalize(ctx_id);
    assert!(matches!(result, Err(FlashloanError::AtomicRevert { .. })));

    pool.revert(&receipt.id)
        .expect("failed atomic path must restore principal");

    assert_eq!(pool.available(chain, &asset), available_before);

    // Mitigation expectation: TWAP oracle + minimum observation window.
}
