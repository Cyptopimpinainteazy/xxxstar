//! Fallback module — serial execution when confidence is too low.
//!
//! # Invariant: EXEC-PREDICT-004
//!
//! When the predictor's average confidence drops below the threshold,
//! all transactions are placed in a single shard for serial execution.

use crate::ShardGroup;

/// Create a single serial-execution shard containing all transactions.
///
/// This is the safe fallback when ML predictions aren't confident enough.
/// Serial execution is always correct (though slower).
pub fn serial_execution(tx_count: usize) -> Vec<ShardGroup> {
    vec![ShardGroup {
        shard_id: 0,
        tx_indices: (0..tx_count).collect(),
        color: 0,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_fallback_contains_all_txs() {
        let shards = serial_execution(10);
        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0].tx_indices.len(), 10);
        assert_eq!(shards[0].tx_indices, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn serial_fallback_empty() {
        let shards = serial_execution(0);
        assert_eq!(shards.len(), 1);
        assert!(shards[0].tx_indices.is_empty());
    }
}
