use x3_lang_execution::profit::{LedgerCost, Profit};

#[test]
fn signed_net_underflow_is_rejected() {
    let costs = [LedgerCost {
        amount: i128::MAX as u128,
        kind: "gas".into(),
    }];
    assert!(Profit::from_ledger(0, i128::MAX as u128, 0, &costs).is_err());
}

#[test]
fn margin_scaling_overflow_is_rejected() {
    assert!(Profit::from_ledger(i128::MAX as u128, 0, 0, &[]).is_err());
}

#[test]
fn margin_must_fit_its_signed_representation() {
    assert!(Profit::from_ledger(1, i64::MAX as u128, 0, &[]).is_err());
}
