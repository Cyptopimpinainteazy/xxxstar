// crates/gpu-swarm/src/performance/memory_pooling.rs
// GPU Memory Pooling for efficient VRAM management

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use thiserror::Error;
use tracing::{debug, warn, span, Level};

#[derive(Error, Debug)]
pub enum MemoryPoolError {
    #[error("Insufficient GPU memory: requested {requested}, available {available}")]
    InsufficientMemory { requested: u64, available: u64 },
    
    #[error("Memory pool exhausted")]
    PoolExhausted,
    
    #[error("Invalid memory block")]
    InvalidBlock,
    
    #[error("Allocation failed: {0}")]
    AllocationFailed(String),
}

/// Represents a block of GPU memory
#[derive(Clone, Debug)]
pub struct MemoryBlock {
    pub id: u64,
    pub device_id: u32,
    pub offset: u64,
    pub size: u64,
    pub allocated: bool,
}

/// GPU Memory Pool - pre-allocates large chunks and manages sub-allocations
pub struct GPUMemoryPool {
    device_id: u32,
    total_size: u64,
    blocks: Arc<Mutex<Vec<MemoryBlock>>>,
    allocation_counter: Arc<Mutex<u64>>,
    metrics: MemoryMetrics,
}

#[derive(Clone, Debug)]
pub struct MemoryMetrics {
    pub allocated_bytes: Arc<Mutex<u64>>,
    pub fragmentation_percent: Arc<Mutex<f32>>,
    pub allocation_count: Arc<Mutex<u64>>,
    pub deallocation_count: Arc<Mutex<u64>>,
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            allocated_bytes: Arc::new(Mutex::new(0)),
            fragmentation_percent: Arc::new(Mutex::new(0.0)),
            allocation_count: Arc::new(Mutex::new(0)),
            deallocation_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl GPUMemoryPool {
    /// Create a new GPU memory pool
    pub fn new(device_id: u32, total_size: u64) -> Self {
        let metrics = MemoryMetrics::default();
        
        let mut initial_block = MemoryBlock {
            id: 0,
            device_id,
            offset: 0,
            size: total_size,
            allocated: false,
        };

        let blocks = vec![initial_block];

        GPUMemoryPool {
            device_id,
            total_size,
            blocks: Arc::new(Mutex::new(blocks)),
            allocation_counter: Arc::new(Mutex::new(1)),
            metrics,
        }
    }

    /// Allocate memory from the pool
    pub fn allocate(&self, size: u64) -> Result<MemoryBlock, MemoryPoolError> {
        let span = span!(Level::DEBUG, "memory_allocate", size = size);
        let _enter = span.enter();

        let mut blocks = self.blocks.lock();
        
        // Find first-fit available block
        for block in blocks.iter_mut() {
            if !block.allocated && block.size >= size {
                // Split block if necessary
                let mut allocated_block = block.clone();
                allocated_block.allocated = true;
                allocated_block.size = size;
                
                // Get next ID
                let mut counter = self.allocation_counter.lock();
                allocated_block.id = *counter;
                *counter += 1;

                // If block is larger than needed, split it
                if block.size > size {
                    let remaining_block = MemoryBlock {
                        id: *counter,
                        device_id: self.device_id,
                        offset: block.offset + size,
                        size: block.size - size,
                        allocated: false,
                    };
                    
                    *counter += 1;
                    
                    // Find position and insert
                    let pos = blocks.iter().position(|b| b.id == block.id).unwrap();
                    blocks[pos] = allocated_block.clone();
                    blocks.insert(pos + 1, remaining_block);
                } else {
                    block.allocated = true;
                }

                // Update metrics
                let mut allocated = self.metrics.allocated_bytes.lock();
                *allocated += size;
                
                let mut alloc_count = self.metrics.allocation_count.lock();
                *alloc_count += 1;

                debug!("✅ Allocated {} bytes (id: {})", size, allocated_block.id);
                return Ok(allocated_block);
            }
        }

        Err(MemoryPoolError::InsufficientMemory {
            requested: size,
            available: self.available_memory(),
        })
    }

    /// Deallocate memory and merge with adjacent free blocks
    pub fn deallocate(&self, block_id: u64) -> Result<(), MemoryPoolError> {
        let span = span!(Level::DEBUG, "memory_deallocate", block_id = block_id);
        let _enter = span.enter();

        let mut blocks = self.blocks.lock();
        
        // Find and mark block as free
        let mut block_pos = None;
        for (i, block) in blocks.iter_mut().enumerate() {
            if block.id == block_id {
                if !block.allocated {
                    return Err(MemoryPoolError::InvalidBlock);
                }
                block.allocated = false;
                block_pos = Some(i);
                
                // Update metrics
                let mut allocated = self.metrics.allocated_bytes.lock();
                *allocated = allocated.saturating_sub(block.size);
                
                let mut dealloc_count = self.metrics.deallocation_count.lock();
                *dealloc_count += 1;
                
                break;
            }
        }

        if block_pos.is_none() {
            return Err(MemoryPoolError::InvalidBlock);
        }

        let pos = block_pos.unwrap();

        // Defragmentation: merge with adjacent free blocks
        // Merge with right neighbor
        if pos + 1 < blocks.len() && !blocks[pos + 1].allocated {
            let next_size = blocks[pos + 1].size;
            blocks[pos].size += next_size;
            blocks.remove(pos + 1);
        }

        // Merge with left neighbor
        if pos > 0 && !blocks[pos - 1].allocated {
            blocks[pos - 1].size += blocks[pos].size;
            blocks.remove(pos);
        }

        // Recalculate fragmentation
        self.calculate_fragmentation(&blocks);

        debug!("✅ Deallocated block {}", block_id);
        Ok(())
    }

    /// Compact memory by moving allocations (simulated for now)
    pub fn compact(&self) -> Result<u64, MemoryPoolError> {
        let span = span!(Level::DEBUG, "memory_compact");
        let _enter = span.enter();

        let mut blocks = self.blocks.lock();
        
        // Collect allocated blocks and free blocks separately
        let mut allocated: Vec<_> = blocks.iter().filter(|b| b.allocated).collect();
        let mut free: Vec<_> = blocks.iter().filter(|b| !b.allocated).collect();

        if free.is_empty() {
            return Ok(0);
        }

        // Sort by offset
        allocated.sort_by_key(|b| b.offset);
        free.sort_by_key(|b| b.offset);

        let mut moved = 0u64;
        
        // Simulate compaction by rebuilding blocks list
        let mut new_offset = 0u64;
        let mut new_blocks = Vec::new();

        for block in allocated {
            let mut new_block = block.clone();
            new_block.offset = new_offset;
            new_offset += new_block.size;
            new_blocks.push(new_block.clone());
            moved += block.size;
        }

        // Add one large free block at the end
        if new_offset < self.total_size {
            new_blocks.push(MemoryBlock {
                id: free[0].id,
                device_id: self.device_id,
                offset: new_offset,
                size: self.total_size - new_offset,
                allocated: false,
            });
        }

        *blocks = new_blocks;

        debug!("✅ Compacted memory, moved {} bytes", moved);
        Ok(moved)
    }

    /// Get available memory
    pub fn available_memory(&self) -> u64 {
        let blocks = self.blocks.lock();
        blocks
            .iter()
            .filter(|b| !b.allocated)
            .map(|b| b.size)
            .sum()
    }

    /// Get memory allocation statistics
    pub fn stats(&self) -> MemoryStats {
        let blocks = self.blocks.lock();
        let allocated = blocks.iter().filter(|b| b.allocated).map(|b| b.size).sum::<u64>();
        let fragmentation = *self.metrics.fragmentation_percent.lock();

        MemoryStats {
            total_size: self.total_size,
            allocated_bytes: allocated,
            available_bytes: self.total_size - allocated,
            fragmentation_percent: fragmentation,
            block_count: blocks.len() as u32,
            allocations_total: *self.metrics.allocation_count.lock(),
            deallocations_total: *self.metrics.deallocation_count.lock(),
        }
    }

    fn calculate_fragmentation(&self, blocks: &[MemoryBlock]) {
        let free_blocks = blocks.iter().filter(|b| !b.allocated).count();
        let total_blocks = blocks.len();
        
        let fragmentation = if total_blocks > 0 {
            (free_blocks as f32 / total_blocks as f32) * 100.0
        } else {
            0.0
        };

        let mut frag = self.metrics.fragmentation_percent.lock();
        *frag = fragmentation;
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_size: u64,
    pub allocated_bytes: u64,
    pub available_bytes: u64,
    pub fragmentation_percent: f32,
    pub block_count: u32,
    pub allocations_total: u64,
    pub deallocations_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_and_deallocate() {
        let pool = GPUMemoryPool::new(0, 1024 * 1024);
        
        let block1 = pool.allocate(1024).unwrap();
        assert_eq!(block1.size, 1024);
        
        let block2 = pool.allocate(2048).unwrap();
        assert_eq!(block2.size, 2048);
        
        assert!(pool.deallocate(block1.id).is_ok());
        assert!(pool.deallocate(block2.id).is_ok());
        
        let stats = pool.stats();
        assert_eq!(stats.allocated_bytes, 0);
    }

    #[test]
    fn test_insufficient_memory() {
        let pool = GPUMemoryPool::new(0, 1024);
        
        let block = pool.allocate(2048);
        assert!(block.is_err());
    }

    #[test]
    fn test_compaction() {
        let pool = GPUMemoryPool::new(0, 10240);
        
        let b1 = pool.allocate(1024).unwrap();
        let b2 = pool.allocate(2048).unwrap();
        let b3 = pool.allocate(1024).unwrap();
        
        pool.deallocate(b1.id).unwrap();
        pool.deallocate(b2.id).unwrap();
        
        let moved = pool.compact().unwrap();
        assert!(moved > 0);
        
        let stats = pool.stats();
        assert!(stats.fragmentation_percent < 10.0);
    }
}
