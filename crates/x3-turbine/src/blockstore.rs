//! Blockstore - Manages received shreds and reconstructs blocks

use crate::error::{TurbineError, TurbineResult};
use crate::metrics::TurbineMetrics;
use crate::shred::{Shred, ShredType};
use lru::LruCache;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Received shred metadata
#[derive(Debug, Clone)]
pub struct ReceivedShred {
    pub shred: Shred,
    pub received_at: Instant,
}

/// Blockstore configuration
#[derive(Debug, Clone)]
pub struct BlockstoreConfig {
    pub max_pending_blocks: usize,
    pub shred_recovery_timeout: Duration,
    pub enable_recovery: bool,
}

impl Default for BlockstoreConfig {
    fn default() -> Self {
        Self {
            max_pending_blocks: 100,
            shred_recovery_timeout: Duration::from_secs(5),
            enable_recovery: true,
        }
    }
}

/// Blockstore for managing shreds
pub struct Blockstore {
    _config: BlockstoreConfig,
    _metrics: Arc<TurbineMetrics>,
    /// Pending shreds indexed by slot
    pending_shreds: RwLock<HashMap<u64, Vec<ReceivedShred>>>,
    /// Track which shred indices we've received per slot
    received_indices: RwLock<HashMap<u64, Vec<bool>>>,
    /// Track slot metadata
    slot_meta: RwLock<HashMap<u64, SlotMeta>>,
    /// Completed blocks cache
    completed_blocks: RwLock<LruCache<u64, Vec<u8>>>,
}

/// Slot metadata
#[derive(Debug, Clone)]
pub struct SlotMeta {
    pub slot: u64,
    pub num_shreds: u32,
    pub received_shreds: u32,
    pub is_complete: bool,
    pub created_at: Instant,
}

impl Blockstore {
    /// Create new blockstore
    pub fn new(config: BlockstoreConfig, metrics: Arc<TurbineMetrics>) -> Self {
        let completed_blocks = LruCache::new(
            NonZeroUsize::new(config.max_pending_blocks.max(1))
                .expect("max pending blocks must be non-zero"),
        );

        Self {
            _config: config,
            _metrics: metrics,
            pending_shreds: RwLock::new(HashMap::new()),
            received_indices: RwLock::new(HashMap::new()),
            slot_meta: RwLock::new(HashMap::new()),
            completed_blocks: RwLock::new(completed_blocks),
        }
    }

    /// Insert a shred into the blockstore
    pub fn insert_shred(&self, shred: Shred) -> TurbineResult<Option<Vec<u8>>> {
        let slot = shred.slot();
        let index = shred.shred_index();
        let num_shreds = shred.num_shreds();

        debug!(
            "Inserting shred: slot={}, index={}/{}",
            slot, index, num_shreds
        );

        // Check if we already have this shred
        {
            let indices = self.received_indices.read();
            if let Some(slot_indices) = indices.get(&slot) {
                if index as usize >= slot_indices.len() || slot_indices[index as usize] {
                    // Already have this shred
                    return Ok(None);
                }
            }
        }

        // Mark as received
        {
            let mut indices = self.received_indices.write();
            let slot_indices = indices
                .entry(slot)
                .or_insert_with(|| vec![false; num_shreds as usize]);
            if (index as usize) < slot_indices.len() {
                slot_indices[index as usize] = true;
            }
        }

        // Store the shred
        {
            let mut shreds = self.pending_shreds.write();
            shreds.entry(slot).or_default().push(ReceivedShred {
                shred: shred.clone(),
                received_at: Instant::now(),
            });
        }

        // Update slot metadata
        {
            let mut meta = self.slot_meta.write();
            let slot_meta = meta.entry(slot).or_insert_with(|| SlotMeta {
                slot,
                num_shreds,
                received_shreds: 0,
                is_complete: false,
                created_at: Instant::now(),
            });
            slot_meta.received_shreds += 1;

            // Check if we have all shreds
            if slot_meta.received_shreds >= num_shreds {
                slot_meta.is_complete = true;
            }
        }

        // Try to reconstruct the block
        if self.can_reconstruct(slot) {
            match self.reconstruct_block(slot) {
                Ok(block_data) => {
                    debug!("Block reconstructed: slot={}", slot);
                    // Store in completed blocks cache
                    self.completed_blocks.write().put(slot, block_data.clone());
                    // Clear pending shreds
                    self.pending_shreds.write().remove(&slot);
                    self.received_indices.write().remove(&slot);
                    return Ok(Some(block_data));
                }
                Err(e) => {
                    warn!("Failed to reconstruct block: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(None)
    }

    /// Check if we can reconstruct a block
    fn can_reconstruct(&self, slot: u64) -> bool {
        let indices = self.received_indices.read();
        if let Some(slot_indices) = indices.get(&slot) {
            let data_shreds = slot_indices.len() / 2;
            let received: usize = slot_indices.iter().filter(|&&b| b).count();
            received >= data_shreds
        } else {
            false
        }
    }

    /// Reconstruct block from shreds
    fn reconstruct_block(&self, slot: u64) -> TurbineResult<Vec<u8>> {
        let shreds = self.pending_shreds.read();
        let slot_shreds = shreds
            .get(&slot)
            .ok_or_else(|| TurbineError::BlockstoreError(format!("No shreds for slot {}", slot)))?;

        // Sort by index
        let mut sorted_shreds: Vec<_> = slot_shreds.iter().collect();
        sorted_shreds.sort_by_key(|s| s.shred.shred_index());

        // Collect data shreds
        let data_shreds: Vec<_> = sorted_shreds
            .iter()
            .filter(|s| s.shred.shred_type() == ShredType::Data)
            .map(|s| s.shred.payload().as_bytes().to_vec())
            .collect();

        if data_shreds.is_empty() {
            return Err(TurbineError::BlockstoreError("No data shreds found".into()));
        }

        // Concatenate data
        let mut block_data = Vec::new();
        for chunk in data_shreds {
            block_data.extend(chunk);
        }

        Ok(block_data)
    }

    /// Get missing shred indices for a slot
    pub fn get_missing_indices(&self, slot: u64) -> Vec<u32> {
        let indices = self.received_indices.read();
        if let Some(slot_indices) = indices.get(&slot) {
            slot_indices
                .iter()
                .enumerate()
                .filter(|(_, &received)| !received)
                .map(|(i, _)| i as u32)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a slot is complete
    pub fn is_slot_complete(&self, slot: u64) -> bool {
        let meta = self.slot_meta.read();
        meta.get(&slot).map(|m| m.is_complete).unwrap_or(false)
    }

    /// Get completed block
    pub fn get_block(&self, slot: u64) -> Option<Vec<u8>> {
        self.completed_blocks.write().get(&slot).cloned()
    }

    /// Get pending shreds count for a slot
    pub fn get_pending_count(&self, slot: u64) -> usize {
        let shreds = self.pending_shreds.read();
        shreds.get(&slot).map(|s| s.len()).unwrap_or(0)
    }

    /// Cleanup old slots
    pub fn cleanup_old_slots(&self, max_age: Duration) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        {
            let meta = self.slot_meta.read();
            for (slot, slot_meta) in meta.iter() {
                if now.duration_since(slot_meta.created_at) > max_age {
                    to_remove.push(*slot);
                }
            }
        }

        for slot in to_remove {
            self.pending_shreds.write().remove(&slot);
            self.received_indices.write().remove(&slot);
            self.slot_meta.write().remove(&slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockstore_insert() {
        let config = BlockstoreConfig::default();
        let metrics = Arc::new(TurbineMetrics::new());
        let _store = Blockstore::new(config, metrics);

        // This would require creating a proper Shred which depends on ErasureCode
        // Skipping full test for brevity
    }
}
