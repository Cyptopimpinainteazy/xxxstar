//! Shard planner — greedy graph coloring for conflict-free TX groups.
//!
//! Builds a conflict graph where edges represent write-write or write-read
//! conflicts between transactions, then colors the graph so that same-color
//! transactions can execute in parallel.

use crate::{AccessPrediction, PredictionError, ShardGroup};
use std::collections::{HashMap, HashSet};

/// Plans parallel shards from access-pattern predictions.
pub struct ShardPlanner {
    max_shards: u32,
}

impl ShardPlanner {
    /// Create a new shard planner.
    pub fn new(max_shards: u32) -> Self {
        Self { max_shards }
    }

    /// Plan shards from predicted access patterns.
    ///
    /// Uses greedy graph coloring: two TXs conflict if they access the same
    /// storage key and at least one is a write.
    ///
    /// # Invariant: EXEC-PREDICT-001
    pub fn plan(
        &self,
        predictions: &[Vec<AccessPrediction>],
    ) -> Result<Vec<ShardGroup>, PredictionError> {
        let n = predictions.len();
        if n == 0 {
            return Ok(vec![]);
        }

        // Build adjacency list (conflict graph)
        let adjacency = self.build_conflict_graph(predictions);

        // Greedy graph coloring
        let colors = self.greedy_color(n, &adjacency);

        // Group TXs by color into shards
        let shards = self.colors_to_shards(&colors);

        Ok(shards)
    }

    /// Build a conflict graph from predicted accesses.
    ///
    /// Two transactions conflict if:
    /// - They access the same storage key, AND
    /// - At least one access is a write
    fn build_conflict_graph(&self, predictions: &[Vec<AccessPrediction>]) -> Vec<HashSet<usize>> {
        let n = predictions.len();
        let mut adjacency: Vec<HashSet<usize>> = vec![HashSet::new(); n];

        // Index: storage_key → list of (tx_index, is_write)
        let mut key_to_txs: HashMap<Vec<u8>, Vec<(usize, bool)>> = HashMap::new();

        for (tx_idx, preds) in predictions.iter().enumerate() {
            for pred in preds {
                key_to_txs
                    .entry(pred.storage_key.clone())
                    .or_default()
                    .push((tx_idx, pred.is_write));
            }
        }

        // For each key, find conflicting pairs
        for (_key, accesses) in &key_to_txs {
            for i in 0..accesses.len() {
                for j in (i + 1)..accesses.len() {
                    let (tx_i, write_i) = accesses[i];
                    let (tx_j, write_j) = accesses[j];

                    // Conflict if at least one write
                    if write_i || write_j {
                        adjacency[tx_i].insert(tx_j);
                        adjacency[tx_j].insert(tx_i);
                    }
                }
            }
        }

        adjacency
    }

    /// Greedy graph coloring.
    ///
    /// Assigns colors 0..max_shards. If more colors are needed than max_shards,
    /// excess TXs are placed in the last shard (serialized together).
    fn greedy_color(&self, n: usize, adjacency: &[HashSet<usize>]) -> Vec<u32> {
        let mut colors = vec![u32::MAX; n];

        // Order by degree (descending) for better coloring
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| adjacency[b].len().cmp(&adjacency[a].len()));

        for &node in &order {
            // Find the smallest color not used by neighbors
            let used_colors: HashSet<u32> = adjacency[node]
                .iter()
                .filter_map(|&neighbor| {
                    if colors[neighbor] != u32::MAX {
                        Some(colors[neighbor])
                    } else {
                        None
                    }
                })
                .collect();

            let mut color = 0u32;
            while used_colors.contains(&color) {
                color += 1;
            }

            // Cap at max_shards - 1
            if color >= self.max_shards {
                color = self.max_shards - 1;
            }

            colors[node] = color;
        }

        colors
    }

    /// Convert color assignments to shard groups.
    fn colors_to_shards(&self, colors: &[u32]) -> Vec<ShardGroup> {
        let mut shard_map: HashMap<u32, Vec<usize>> = HashMap::new();

        for (tx_idx, &color) in colors.iter().enumerate() {
            shard_map.entry(color).or_default().push(tx_idx);
        }

        let mut shards: Vec<ShardGroup> = shard_map
            .into_iter()
            .enumerate()
            .map(|(shard_id, (color, tx_indices))| ShardGroup {
                shard_id: shard_id as u32,
                tx_indices,
                color,
            })
            .collect();

        // Sort by shard_id for determinism
        shards.sort_by_key(|s| s.shard_id);

        shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccessPrediction;

    #[test]
    fn no_conflicts_single_shard() {
        let planner = ShardPlanner::new(16);

        // 3 TXs accessing different keys — no conflicts
        let predictions = vec![
            vec![AccessPrediction {
                storage_key: vec![0x01],
                is_write: true,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: vec![0x02],
                is_write: true,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: vec![0x03],
                is_write: true,
                confidence: 9000,
            }],
        ];

        let shards = planner.plan(&predictions).unwrap();

        // All should get color 0 (no conflicts)
        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0].tx_indices.len(), 3);
    }

    #[test]
    fn write_conflicts_separate_shards() {
        let planner = ShardPlanner::new(16);

        // 2 TXs writing to same key — must be in separate shards
        let predictions = vec![
            vec![AccessPrediction {
                storage_key: vec![0x01],
                is_write: true,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: vec![0x01],
                is_write: true,
                confidence: 9000,
            }],
        ];

        let shards = planner.plan(&predictions).unwrap();

        assert_eq!(shards.len(), 2);
        assert_eq!(shards[0].tx_indices.len(), 1);
        assert_eq!(shards[1].tx_indices.len(), 1);
    }

    #[test]
    fn read_only_no_conflict() {
        let planner = ShardPlanner::new(16);

        // 2 TXs reading same key — no conflict
        let predictions = vec![
            vec![AccessPrediction {
                storage_key: vec![0x01],
                is_write: false,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: vec![0x01],
                is_write: false,
                confidence: 9000,
            }],
        ];

        let shards = planner.plan(&predictions).unwrap();

        // Same color — single shard
        assert_eq!(shards.len(), 1);
    }

    #[test]
    fn respects_max_shards() {
        let planner = ShardPlanner::new(2);

        // 3 mutually conflicting TXs — need 3 colors but max is 2
        let key = vec![0x01];
        let predictions = vec![
            vec![AccessPrediction {
                storage_key: key.clone(),
                is_write: true,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: key.clone(),
                is_write: true,
                confidence: 9000,
            }],
            vec![AccessPrediction {
                storage_key: key.clone(),
                is_write: true,
                confidence: 9000,
            }],
        ];

        let shards = planner.plan(&predictions).unwrap();

        // At most 2 shards
        assert!(shards.len() <= 2);
    }
}
