use crate::TWAPExecutor;

#[test]
fn oracle_update_is_atomic_no_stale_window() {
    let mut order =
        TWAPExecutor::create_twap_order([8u8; 32], 1, 2, 10_000, 100, 10, 1).expect("valid order");

    let mut next = TWAPExecutor::get_next_slice(&order, 2)
        .expect("next slice query must succeed")
        .expect("a slice should be due");

    let before_executed = order.total_executed;
    assert_eq!(before_executed, 0);

    let execution = TWAPExecutor::execute_slice(&mut order, &mut next, 10_050, 2)
        .expect("slice execution must succeed");

    // Atomicity expectation: once execute_slice returns, all dependent state reflects new value.
    assert!(next.is_executed);
    assert_eq!(order.total_executed, next.amount);
    assert_eq!(execution.slice_amount, next.amount);
    assert!(order.total_executed > before_executed);
}
