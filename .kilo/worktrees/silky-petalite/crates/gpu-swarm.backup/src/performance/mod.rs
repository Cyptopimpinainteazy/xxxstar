// crates/gpu-swarm/src/performance/mod.rs
// Performance optimization module

pub mod memory_pooling;
pub mod batch_optimization;
pub mod network_tuning;

pub use memory_pooling::{GPUMemoryPool, MemoryBlock, MemoryStats, MemoryPoolError};
pub use batch_optimization::TaskBatchOptimizer;
pub use network_tuning::NetworkTuner;

/// Initialize all performance optimizations
pub async fn init_performance_optimizations(
    device_count: u32,
    memory_per_device: u64,
) -> Result<PerformanceOptimizers, Box<dyn std::error::Error>> {
    tracing::info!("Initializing performance optimizations...");

    // Initialize memory pools for each device
    let mut memory_pools = Vec::new();
    for device_id in 0..device_count {
        let pool = GPUMemoryPool::new(device_id, memory_per_device);
        memory_pools.push(pool);
    }

    // Initialize batch optimizer
    let batch_optimizer = TaskBatchOptimizer::new(16, 256);

    // Initialize network tuner
    let network_tuner = NetworkTuner::new();

    tracing::info!("✅ Performance optimizations initialized");

    Ok(PerformanceOptimizers {
        memory_pools,
        batch_optimizer,
        network_tuner,
    })
}

pub struct PerformanceOptimizers {
    pub memory_pools: Vec<GPUMemoryPool>,
    pub batch_optimizer: TaskBatchOptimizer,
    pub network_tuner: NetworkTuner,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_optimizers() {
        let opts = init_performance_optimizations(2, 8 * 1024 * 1024 * 1024).await;
        assert!(opts.is_ok());
        
        let opts = opts.unwrap();
        assert_eq!(opts.memory_pools.len(), 2);
    }
}
