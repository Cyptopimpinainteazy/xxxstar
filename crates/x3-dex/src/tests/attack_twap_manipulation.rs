use crate::{TWAPExecutor, TWAPSliceExecution};

#[test]
fn twap_resists_single_block_price_spike() {
    let order = TWAPExecutor::create_twap_order([3u8; 32], 1, 2, 20_000, 200, 20, 10)
        .expect("twap order creation must succeed");

    // 19 normal observations and one manipulated outlier.
    let mut executions = Vec::<TWAPSliceExecution>::new();
    for i in 0..19u32 {
        executions.push(TWAPSliceExecution {
            slice_index: i,
            slice_amount: 1_000,
            execution_price: 10_000,
            execution_fee: 2,
            executed_block: 11 + i as u64,
            timestamp: 11 + i as u64,
        });
    }
    executions.push(TWAPSliceExecution {
        slice_index: 19,
        slice_amount: 1_000,
        execution_price: 11_000,
        execution_fee: 2,
        executed_block: 30,
        timestamp: 30,
    });

    let stats = TWAPExecutor::calculate_statistics(&order, &executions, 10_000)
        .expect("statistics must be computable");

    let deviation_bps = if stats.average_price >= 10_000 {
        ((stats.average_price - 10_000) as u128 * 10_000 / 10_000u128) as u32
    } else {
        ((10_000 - stats.average_price) as u128 * 10_000 / 10_000u128) as u32
    };

    // Observation-window mitigation target: single-block spike should not move TWAP by >=5%.
    assert!(deviation_bps < 500);
}
