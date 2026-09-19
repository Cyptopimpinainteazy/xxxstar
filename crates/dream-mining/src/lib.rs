#![allow(unused, dead_code, deprecated)]

//! Dream Mining - Idle GPU Optimization for Background Computation
//!
//! Dream Mining enables X3 Chain to utilize idle GPU resources during
//! periods of low user activity (e.g., sleep hours) for productive tasks:
//!
//! - **Model Training**: Train AI models for Evolution Core
//! - **Route Optimization**: Pre-compute optimal swap routes
//! - **ZK Proof Generation**: Generate zero-knowledge proofs in batch
//! - **Index Building**: Build search indexes for faster queries
//! - **Network Analysis**: Analyze blockchain network for insights
//!
//! # Safety Features
//!
//! - **Activity Detection**: Pauses on user activity
//! - **Temperature Monitoring**: Throttles if GPU overheats
//! - **Battery Awareness**: Respects laptop battery on mobile devices
//! - **Configurable Schedule**: User-defined sleep/work hours
//! - **Resource Limits**: Never exceeds configured GPU/CPU usage

#![allow(unused_variables)]

pub mod config;
pub mod error;
pub mod monitor;
pub mod scheduler;
pub mod tasks;

pub use config::DreamConfig;
pub use error::{DreamError, DreamResult};
pub use monitor::SystemMonitor;
pub use scheduler::DreamScheduler;
pub use tasks::{DreamTask, TaskPriority, TaskResult};

/// Dream Mining version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Main Dream Mining orchestrator
pub struct DreamMiner {
    config: DreamConfig,
    scheduler: DreamScheduler,
    monitor: SystemMonitor,
    running: std::sync::atomic::AtomicBool,
}

impl DreamMiner {
    /// Create a new Dream Miner with default configuration
    pub fn new() -> Self {
        Self::with_config(DreamConfig::default())
    }

    /// Create a new Dream Miner with custom configuration
    pub fn with_config(config: DreamConfig) -> Self {
        Self {
            scheduler: DreamScheduler::new(&config),
            monitor: SystemMonitor::new(&config),
            config,
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Start the Dream Mining service
    pub async fn start(&self) -> DreamResult<()> {
        use std::sync::atomic::Ordering;

        if self.running.swap(true, Ordering::SeqCst) {
            return Err(DreamError::AlreadyRunning);
        }

        tracing::info!("Dream Mining service starting...");

        // Start background monitoring
        self.monitor.start().await?;

        // Main loop
        while self.running.load(Ordering::SeqCst) {
            // Check if we should be mining
            if !self.should_mine().await? {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }

            // Get next task from scheduler
            if let Some(task) = self.scheduler.next_task().await? {
                // Execute task with monitoring
                match self.execute_task(&task).await {
                    Ok(result) => {
                        tracing::info!(task = %task.id, "Task completed: {:?}", result);
                        self.scheduler.complete_task(task.id, result).await?;
                    }
                    Err(e) => {
                        tracing::warn!(task = %task.id, "Task failed: {}", e);
                        self.scheduler.fail_task(task.id, e.to_string()).await?;
                    }
                }
            } else {
                // No tasks available, sleep
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        }

        Ok(())
    }

    /// Stop the Dream Mining service
    pub fn stop(&self) {
        use std::sync::atomic::Ordering;
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("Dream Mining service stopping...");
    }

    /// Check if mining should be active
    async fn should_mine(&self) -> DreamResult<bool> {
        // Check schedule
        if !self.is_within_schedule() {
            return Ok(false);
        }

        // Check system resources
        let stats = self.monitor.current_stats().await?;

        // Don't mine if user is active
        if stats.user_active {
            return Ok(false);
        }

        // Don't mine if GPU is too hot
        if let Some(temp) = stats.gpu_temperature {
            if temp > self.config.max_gpu_temp {
                tracing::warn!(temp, "GPU too hot, pausing mining");
                return Ok(false);
            }
        }

        // Don't mine on battery if disabled
        if !self.config.mine_on_battery && stats.on_battery {
            return Ok(false);
        }

        // Don't mine if battery too low
        if let Some(level) = stats.battery_level {
            if level < self.config.min_battery_level {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if current time is within configured schedule
    fn is_within_schedule(&self) -> bool {
        let hour = chrono::Local::now().hour();
        Self::hour_within_schedule(hour, self.config.schedule_start, self.config.schedule_end)
    }

    /// Pure schedule-window check, split out from `is_within_schedule` so
    /// the cross-midnight branch can be tested against explicit hours
    /// instead of depending on when the test happens to run.
    fn hour_within_schedule(hour: u32, start: u32, end: u32) -> bool {
        if start <= end {
            // Normal range (e.g., 09:00 to 17:00 doesn't cross midnight)
            hour >= start && hour < end
        } else {
            // Crosses midnight (e.g., 23:00 to 07:00)
            hour >= start || hour < end
        }
    }

    /// Execute a mining task
    async fn execute_task(&self, task: &DreamTask) -> DreamResult<TaskResult> {
        match &task.task_type {
            tasks::TaskType::ModelTraining(model_task) => {
                self.execute_model_training(model_task).await
            }
            tasks::TaskType::RouteOptimization(route_task) => {
                self.execute_route_optimization(route_task).await
            }
            tasks::TaskType::ZkProofGeneration(zk_task) => {
                self.execute_zk_generation(zk_task).await
            }
            tasks::TaskType::IndexBuilding(index_task) => {
                self.execute_index_building(index_task).await
            }
            tasks::TaskType::NetworkAnalysis(network_task) => {
                self.execute_network_analysis(network_task).await
            }
        }
    }

    async fn execute_model_training(
        &self,
        task: &tasks::ModelTrainingTask,
    ) -> DreamResult<TaskResult> {
        tracing::info!(model = %task.model_name, "Starting model training");

        // Simulate training progress
        let mut progress = 0.0;
        while progress < 1.0 {
            // Check if we should pause
            if !self.should_mine().await? {
                return Ok(TaskResult::Paused { progress });
            }

            // Training step
            progress += 0.01;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(TaskResult::Completed {
            output: format!("Model {} trained successfully", task.model_name),
            metrics: Default::default(),
        })
    }

    async fn execute_route_optimization(
        &self,
        task: &tasks::RouteOptimizationTask,
    ) -> DreamResult<TaskResult> {
        tracing::info!(pairs = %task.pairs.len(), "Optimizing swap routes");

        let mut optimized = 0;
        for pair in &task.pairs {
            // Optimize route for each pair
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            optimized += 1;
        }

        Ok(TaskResult::Completed {
            output: format!("Optimized {} routes", optimized),
            metrics: Default::default(),
        })
    }

    async fn execute_zk_generation(&self, task: &tasks::ZkProofTask) -> DreamResult<TaskResult> {
        tracing::info!(proofs = %task.proof_count, "Generating ZK proofs");

        let mut generated = 0;
        for _ in 0..task.proof_count {
            // Generate proof
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            generated += 1;
        }

        Ok(TaskResult::Completed {
            output: format!("Generated {} proofs", generated),
            metrics: Default::default(),
        })
    }

    async fn execute_index_building(
        &self,
        task: &tasks::IndexBuildTask,
    ) -> DreamResult<TaskResult> {
        tracing::info!(index = %task.index_name, "Building search index");

        // Build index
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        Ok(TaskResult::Completed {
            output: format!("Index {} built", task.index_name),
            metrics: Default::default(),
        })
    }

    async fn execute_network_analysis(
        &self,
        task: &tasks::NetworkAnalysisTask,
    ) -> DreamResult<TaskResult> {
        tracing::info!(chains = ?task.chains, "Analyzing network");

        // Analyze network
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        Ok(TaskResult::Completed {
            output: format!("Analyzed {} chains", task.chains.len()),
            metrics: Default::default(),
        })
    }

    /// Queue a new task
    pub async fn queue_task(&self, task: DreamTask) -> DreamResult<uuid::Uuid> {
        self.scheduler.add_task(task).await
    }

    /// Get current mining stats
    pub async fn stats(&self) -> DreamResult<MiningStats> {
        let system = self.monitor.current_stats().await?;
        let tasks = self.scheduler.stats().await?;

        Ok(MiningStats {
            is_mining: self.running.load(std::sync::atomic::Ordering::SeqCst)
                && !system.user_active,
            tasks_completed: tasks.completed,
            tasks_pending: tasks.pending,
            current_task: tasks.current_task,
            gpu_usage: system.gpu_usage,
            cpu_usage: system.cpu_usage,
            uptime_hours: tasks.uptime_hours,
        })
    }
}

impl Default for DreamMiner {
    fn default() -> Self {
        Self::new()
    }
}

/// Mining statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiningStats {
    pub is_mining: bool,
    pub tasks_completed: u64,
    pub tasks_pending: u64,
    pub current_task: Option<String>,
    pub gpu_usage: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub uptime_hours: f64,
}

// Re-export chrono::Timelike for hour()
use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_check() {
        // `field_reassign_with_default` wants the struct built in one
        // expression rather than mutating a `Default::default()`.
        let config = DreamConfig {
            schedule_start: 23,
            schedule_end: 7,
            ..Default::default()
        };

        // Smoke test: the config wires through to a real miner without
        // panicking. The actual schedule logic is asserted deterministically
        // below, since `is_within_schedule` depends on wall-clock time and
        // this doesn't.
        let _miner = DreamMiner::with_config(config);
    }

    /// `is_within_schedule`'s cross-midnight branch (23:00 to 07:00 wraps
    /// past midnight rather than being an always-false empty range), tested
    /// against explicit hours via `hour_within_schedule` instead of
    /// depending on when the test happens to run.
    #[test]
    fn test_schedule_crosses_midnight() {
        let (start, end) = (23, 7);

        assert!(
            DreamMiner::hour_within_schedule(23, start, end),
            "23:00 starts the window"
        );
        assert!(
            DreamMiner::hour_within_schedule(0, start, end),
            "midnight is inside the window"
        );
        assert!(
            DreamMiner::hour_within_schedule(6, start, end),
            "06:00 is inside the window"
        );
        assert!(
            !DreamMiner::hour_within_schedule(7, start, end),
            "07:00 ends the window (exclusive)"
        );
        assert!(
            !DreamMiner::hour_within_schedule(12, start, end),
            "noon is outside the window"
        );
        assert!(
            !DreamMiner::hour_within_schedule(22, start, end),
            "22:00 is still outside the window"
        );
    }

    /// The non-crossing branch (e.g. a 09:00-17:00 workday window), so both
    /// branches of the start<=end check are covered, not just cross-midnight.
    #[test]
    fn test_schedule_does_not_cross_midnight() {
        let (start, end) = (9, 17);

        assert!(!DreamMiner::hour_within_schedule(8, start, end));
        assert!(DreamMiner::hour_within_schedule(9, start, end));
        assert!(DreamMiner::hour_within_schedule(16, start, end));
        assert!(!DreamMiner::hour_within_schedule(17, start, end));
    }
}
