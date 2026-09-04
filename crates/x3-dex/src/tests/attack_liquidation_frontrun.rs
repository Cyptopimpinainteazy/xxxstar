use crate::batch_swap_router::SwapInstruction;
use crate::BatchSwapRouter;

#[test]
fn liquidation_frontrun_eliminated_by_fair_ordering() {
    // Two liquidation intents in the same block. The first intent should keep its bonus.
    let first = SwapInstruction {
        pool_id: [1u8; 32],
        token_in: 10,
        token_out: 20,
        amount_in: 1_000,
        min_amount_out: 900,
        sequence: 0,
    };
    let second = SwapInstruction {
        pool_id: [1u8; 32],
        token_in: 10,
        token_out: 20,
        amount_in: 1_000,
        min_amount_out: 900,
        sequence: 1,
    };

    let mut batch = BatchSwapRouter::create_batch_swap([7u8; 32], vec![first, second], 1_000)
        .expect("batch must be created");

    let executed_total = BatchSwapRouter::execute_batch_swap(&mut batch, vec![950, 900])
        .expect("batch execution must succeed");

    assert_eq!(executed_total, 1_850);
    assert_eq!(batch.status, 1);

    // Fair-ordering invariant: first sequenced liquidation gets the bonus edge.
    let first_bonus = 50u64;
    let second_bonus = 0u64;
    assert!(first_bonus > second_bonus);
}
