// crates/gpu-swarm/src/performance/batch_optimization.rs
// Task batch optimization for GPU efficiency

use std::collections::VecDeque;
use tokio::time::{Duration, timeout};
use tracing::{debug, span, Level};

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub estimated_duration_ms: u32,
    pub memory_required: u64,
    pub priority: u32,
}

#[derive(Clone, Debug)]
pub struct Batch {
    pub id: u64,
    pub tasks: Vec<Task>,
    pub total_memory: u64,
    pub estimated_duration_ms: u32,
}

pub struct TaskBatchOptimizer {
    min_batch_size: usize,
    max_batch_size: usize,
    batch_timeout_ms: u64,
    batch_queue: parking_lot::Mutex<VecDeque<Batch>>,
    batch_counter: std::sync::atomic::AtomicU64,
}

impl TaskBatchOptimizer {
    pub fn new(min_batch_size: usize, max_batch_size: usize) -> Self {
        TaskBatchOptimizer {
            min_batch_size,
            max_batch_size,
            batch_timeout_ms: 100,
            batch_queue: parking_lot::Mutex::new(VecDeque::new()),
            batch_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Add a task and potentially create a batch
    pub fn add_task(&self, task: Task) -> Option<Batch> {
        let span = span!(Level::DEBUG, "batch_add_task", task_id = &task.id);
        let _enter = span.enter();

        let mut queue = self.batch_queue.lock();
        
        // Check if last batch is incomplete
        if let Some(last_batch) = queue.back_mut() {
            if last_batch.tasks.len() < self.max_batch_size {
                last_batch.tasks.push(task.clone());
                last_batch.total_memory += task.memory_required;
                last_batch.estimated_duration_ms = 
                    last_batch.estimated_duration_ms.max(task.estimated_duration_ms);
                
                // Return batch if full
                if last_batch.tasks.len() >= self.min_batch_size {
                    let batch = queue.pop_back().unwrap();
                    debug!("📦 Created batch with {} tasks", batch.tasks.len());
                    return Some(batch);
                }
                return None;
            }
        }

        // Create new batch
        let batch_id = self.batch_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let batch = Batch {
            id: batch_id,
            tasks: vec![task],
            total_memory: task.memory_required,
            estimated_duration_ms: task.estimated_duration_ms,
        };

        queue.push_back(batch);
        None
    }

    /// Flush pending batch (use after timeout)
    pub fn flush_batch(&self) -> Option<Batch> {
        let span = span!(Level::DEBUG, "batch_flush");
        let _enter = span.enter();

        let mut queue = self.batch_queue.lock();
        if let Some(batch) = queue.pop_front() {
            debug!("🚀 Flushed batch with {} tasks", batch.tasks.len());
            return Some(batch);
        }
        None
    }

    /// Optimize batch by reordering tasks for better GPU cache locality
    pub fn optimize_batch_order(batch: &mut Batch) {
        let span = span!(Level::DEBUG, "batch_optimize_order", batch_id = batch.id);
        let _enter = span.enter();

        // Sort by task type for cache locality
        batch.tasks.sort_by(|a, b| {
            // First sort by type
            match a.task_type.cmp(&b.task_type) {
                std::cmp::Ordering::Equal => {
                    // Then by memory requirement (ascending - process small tasks first)
                    a.memory_required.cmp(&b.memory_required)
                }
                other => other,
            }
        });

        // Sort by priority within same type
        let mut groups: std::collections::HashMap<String, Vec<Task>> = 
            std::collections::HashMap::new();
        
        for task in batch.tasks.drain(..) {
            groups.entry(task.task_type.clone())
                .or_insert_with(Vec::new)
                .push(task);
        }

        for tasks in groups.values_mut() {
            tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        for tasks in groups.values() {
            batch.tasks.extend(tasks.iter().cloned());
        }

        debug!("✅ Optimized batch task order");
    }

    /// Estimate GPU utilization improvement for batch
    pub fn estimate_utilization_gain(batch: &Batch) -> f32 {
        if batch.tasks.is_empty() {
            return 0.0;
        }

        // Batch gain = (sum of individual times - max time) / (sum of individual times)
        let total_individual = batch
            .tasks
            .iter()
            .map(|t| t.estimated_duration_ms as u32)
            .sum::<u32>();

        let max_task = batch
            .tasks
            .iter()
            .map(|t| t.estimated_duration_ms)
            .max()
            .unwrap_or(0);

        if total_individual > 0 {
            ((total_individual - max_task) as f32 / total_individual as f32) * 100.0
        } else {
            0.0
        }
    }

    /// Get pending batch queue size
    pub fn pending_batch_count(&self) -> usize {
        self.batch_queue.lock().len()
    }

    /// Get total tasks waiting in queue
    pub fn pending_task_count(&self) -> usize {
        self.batch_queue
            .lock()
            .iter()
            .map(|b| b.tasks.len())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_creation() {
        let optimizer = TaskBatchOptimizer::new(2, 4);

        let task1 = Task {
            id: "task1".to_string(),
            task_type: "matmul".to_string(),
            estimated_duration_ms: 100,
            memory_required: 1024,
            priority: 1,
        };

        let task2 = Task {
            id: "task2".to_string(),
            task_type: "conv".to_string(),
            estimated_duration_ms: 150,
            memory_required: 2048,
            priority: 1,
        };

        assert!(optimizer.add_task(task1).is_none());
        let batch = optimizer.add_task(task2);
        assert!(batch.is_some());

        let batch = batch.unwrap();
        assert_eq!(batch.tasks.len(), 2);
    }

    #[test]
    fn test_batch_optimization() {
        let mut batch = Batch {
            id: 0,
            tasks: vec![
                Task {
                    id: "task1".to_string(),
                    task_type: "conv".to_string(),
                    estimated_duration_ms: 100,
                    memory_required: 2048,
                    priority: 2,
                },
                Task {
                    id: "task2".to_string(),
                    task_type: "matmul".to_string(),
                    estimated_duration_ms: 50,
                    memory_required: 1024,
                    priority: 1,
                },
            ],
            total_memory: 3072,
            estimated_duration_ms: 100,
        };

        TaskBatchOptimizer::optimize_batch_order(&mut batch);
        assert_eq!(batch.tasks[0].task_type, "conv");
    }
}
