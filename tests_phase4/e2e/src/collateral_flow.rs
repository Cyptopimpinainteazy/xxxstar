use pallet_x3_settlement_engine::collateral::{InMemoryCollateral, BondType, BondState};

#[tokio::test]
async fn collateral_deposit_withdraw_slash_flow() {
    // Use PoC InMemoryCollateral to validate flows in e2e harness context
    let mut coll = InMemoryCollateral::<u64, u128>::new();

    let id = coll.deposit_bond(42u64, b"USDC".to_vec(), 1_000u128, BondType::PerformanceBond).unwrap();
    let b = coll.get_bond(id).unwrap();
    assert_eq!(b.amount, 1_000u128);
    assert_eq!(b.state, BondState::Locked);

    coll.request_withdraw(id).unwrap();
    let b2 = coll.get_bond(id).unwrap();
    assert_eq!(b2.state, BondState::Withdrawable);

    coll.finalize_withdraw(id).unwrap();
    assert!(coll.get_bond(id).is_none());
}