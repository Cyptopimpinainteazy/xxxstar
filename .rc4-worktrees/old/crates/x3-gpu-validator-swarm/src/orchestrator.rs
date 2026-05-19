//! Swarm Orchestrator for X3 GPU Validator Swarm
//!
//! Coordinates multiple validators, distributes tasks, and manages the swarm.

use crate::config::SwarmConfig;
#[cfg(test)]
use crate::crypto::HashAlgorithm;
use crate::deterministic::DeterministicTask;
#[cfg(test)]
use crate::deterministic::TaskType;
use crate::error::SwarmResult;
use crate::metrics::{MetricsCollector, SwarmMetrics};
use crate::network::NetworkManager;
use crate::protocol::TaskResult;
use crate::quarantine::{QuarantineManager, QuarantineReason};
use crate::validator::Validator;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Orchestrator event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorEvent {
    /// Event type
    pub event_type: String,
    /// Timestamp
    pub timestamp: i64,
    /// Data
    pub data: HashMap<String, serde_json::Value>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Pending
    Pending,
    /// Assigned
    Assigned,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// Pending task
#[derive(Debug, Clone)]
struct PendingTask {
    /// Task
    task: DeterministicTask,
    /// Assigned validator
    assigned_to: Option<String>,
    /// Status
    status: TaskStatus,
    /// Created at
    _created_at: Instant,
    /// Assigned at
    assigned_at: Option<Instant>,
}

/// Swarm Orchestrator
pub struct SwarmOrchestrator {
    /// Orchestrator ID
    orchestrator_id: String,
    /// Configuration
    _config: SwarmConfig,
    /// Validators
    validators: RwLock<HashMap<String, Arc<Validator>>>,
    /// Pending tasks
    pending_tasks: RwLock<VecDeque<PendingTask>>,
    /// Completed tasks
    completed_tasks: RwLock<HashMap<String, TaskResult>>,
    /// Network manager
    _network: RwLock<Option<NetworkManager>>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Quarantine manager
    quarantine: Arc<QuarantineManager>,
    /// Task assignment strategy
    assignment_strategy: AssignmentStrategy,
    /// Start time
    start_time: Instant,
}

/// Assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentStrategy {
    /// Round robin
    RoundRobin,
    /// Least loaded
    LeastLoaded,
    /// Random
    Random,
    /// Priority based
    Priority,
}

impl Default for AssignmentStrategy {
    fn default() -> Self {
        Self::LeastLoaded
    }
}

impl SwarmOrchestrator {
    /// Create a new orchestrator
    pub fn new(config: SwarmConfig) -> Self {
        Self {
            orchestrator_id: Uuid::new_v4().to_string(),
            _config: config,
            validators: RwLock::new(HashMap::new()),
            pending_tasks: RwLock::new(VecDeque::new()),
            completed_tasks: RwLock::new(HashMap::new()),
            _network: RwLock::new(None),
            metrics: Arc::new(MetricsCollector::new()),
            quarantine: Arc::new(QuarantineManager::new(3, 1800, true)),
            assignment_strategy: AssignmentStrategy::LeastLoaded,
            start_time: Instant::now(),
        }
    }

    /// Register a validator
    pub fn register_validator(&self, validator: Arc<Validator>) {
        let id = validator.id().to_string();
        self.validators.write().insert(id, validator);

        // Update metrics
        let total = self.validators.read().len() as u64;
        let active = self.get_active_validators() as u64;
        let quarantined = self.get_quarantined_validators() as u64;
        self.metrics
            .update_validator_count(total, active, quarantined);
    }

    /// Unregister a validator
    pub fn unregister_validator(&self, validator_id: &str) {
        self.validators.write().remove(validator_id);
    }

    /// Get active validators
    pub fn get_active_validators(&self) -> usize {
        self.validators
            .read()
            .values()
            .filter(|v| v.state() == crate::validator::ValidatorState::Running)
            .count()
    }

    /// Get quarantined validators
    pub fn get_quarantined_validators(&self) -> usize {
        self.validators
            .read()
            .values()
            .filter(|v| {
                v.get_quarantine_status()
                    .map(|s| s.is_quarantined)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Submit a task
    pub fn submit_task(&self, task: DeterministicTask) -> String {
        let task_id = task.task_id.clone();
        let pending = PendingTask {
            task,
            assigned_to: None,
            status: TaskStatus::Pending,
            _created_at: Instant::now(),
            assigned_at: None,
        };

        self.pending_tasks.write().push_back(pending);
        task_id
    }

    /// Submit multiple tasks (batch)
    pub fn submit_batch(&self, tasks: Vec<DeterministicTask>) -> Vec<String> {
        let mut task_ids = Vec::new();

        for task in tasks {
            let task_id = self.submit_task(task);
            task_ids.push(task_id);
        }

        task_ids
    }

    /// Process pending tasks
    pub fn process_pending_tasks(&self) -> usize {
        let mut processed = 0;
        let mut tasks = self.pending_tasks.write();

        while let Some(mut pending) = tasks.pop_front() {
            if pending.status == TaskStatus::Pending {
                // Find a validator
                if let Some(validator_id) = self.select_validator() {
                    // Get validator
                    let validators = self.validators.read();
                    if let Some(validator) = validators.get(&validator_id) {
                        // Process task
                        let result = validator.process_task(pending.task.clone());

                        // Store result
                        let task_result = TaskResult {
                            assignment_id: pending.task.task_id.clone(),
                            validator_id: validator_id.clone(),
                            outputs: result.outputs,
                            execution_time_ms: result.execution_time_us,
                            verification_result: format!("{:?}", result.verification),
                            cpu_fallback: result.cpu_fallback_used,
                            signature: None,
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        self.completed_tasks
                            .write()
                            .insert(pending.task.task_id.clone(), task_result);

                        pending.status = TaskStatus::Completed;
                        pending.assigned_to = Some(validator_id);
                        pending.assigned_at = Some(Instant::now());

                        processed += 1;
                    }
                }
            }

            // Re-queue if not completed
            if pending.status != TaskStatus::Completed {
                tasks.push_back(pending);
            }
        }

        processed
    }

    /// Select a validator based on strategy
    fn select_validator(&self) -> Option<String> {
        let validators = self.validators.read();

        // Filter active, non-quarantined validators
        let available: Vec<_> = validators
            .iter()
            .filter(|(_, v)| {
                v.state() == crate::validator::ValidatorState::Running
                    && !v
                        .get_quarantine_status()
                        .map(|s| s.is_quarantined)
                        .unwrap_or(false)
            })
            .map(|(id, v)| {
                let metrics = v.get_metrics();
                (id.clone(), metrics)
            })
            .collect();

        if available.is_empty() {
            return None;
        }

        match self.assignment_strategy {
            AssignmentStrategy::RoundRobin => {
                // Simple round-robin: return first available
                available.first().map(|(id, _)| id.clone())
            }
            AssignmentStrategy::LeastLoaded => {
                // Return validator with lowest load
                available
                    .into_iter()
                    .min_by(|a, b| a.1.total_tasks.cmp(&b.1.total_tasks))
                    .map(|(id, _)| id)
            }
            AssignmentStrategy::Random => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..available.len());
                available.into_iter().nth(idx).map(|(id, _)| id)
            }
            AssignmentStrategy::Priority => {
                // Return first (could be sorted by priority)
                available.first().map(|(id, _)| id.clone())
            }
        }
    }

    /// Get task result
    pub fn get_task_result(&self, task_id: &str) -> Option<TaskResult> {
        self.completed_tasks.read().get(task_id).cloned()
    }

    /// Get pending task count
    pub fn pending_task_count(&self) -> usize {
        self.pending_tasks.read().len()
    }

    /// Get completed task count
    pub fn completed_task_count(&self) -> usize {
        self.completed_tasks.read().len()
    }

    /// Get swarm metrics
    pub fn get_swarm_metrics(&self) -> SwarmMetrics {
        self.metrics.get_swarm_metrics()
    }

    /// Set assignment strategy
    pub fn set_assignment_strategy(&mut self, strategy: AssignmentStrategy) {
        self.assignment_strategy = strategy;
    }

    /// Get validator metrics
    pub fn get_validator_metrics(&self, validator_id: &str) -> Option<serde_json::Value> {
        let validators = self.validators.read();
        validators.get(validator_id).map(|v| {
            serde_json::json!({
                "validator_id": v.id(),
                "state": format!("{:?}", v.state()),
                "mode": format!("{:?}", v.current_mode()),
                "uptime_secs": v.uptime().as_secs(),
            })
        })
    }

    /// Get all validator metrics
    pub fn get_all_validator_metrics(&self) -> Vec<serde_json::Value> {
        self.validators
            .read()
            .values()
            .map(|v| {
                serde_json::json!({
                    "validator_id": v.id(),
                    "state": format!("{:?}", v.state()),
                    "mode": format!("{:?}", v.current_mode()),
                    "uptime_secs": v.uptime().as_secs(),
                    "metrics": v.get_metrics(),
                })
            })
            .collect()
    }

    /// Get orchestrator ID
    pub fn id(&self) -> &str {
        &self.orchestrator_id
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Export state as JSON
    pub fn export_state_json(&self) -> SwarmResult<String> {
        let state = serde_json::json!({
            "orchestrator_id": self.orchestrator_id,
            "uptime_secs": self.uptime().as_secs(),
            "validators": {
                "total": self.validators.read().len(),
                "active": self.get_active_validators(),
                "quarantined": self.get_quarantined_validators(),
            },
            "tasks": {
                "pending": self.pending_task_count(),
                "completed": self.completed_task_count(),
            },
            "metrics": self.get_swarm_metrics(),
            "validators": self.get_all_validator_metrics(),
        });

        serde_json::to_string_pretty(&state).map_err(|e| e.into())
    }

    /// Quarantine a validator
    pub fn quarantine_validator(&self, validator_id: &str, reason: QuarantineReason) {
        self.quarantine.quarantine(validator_id.to_string(), reason);
    }

    /// Release a validator from quarantine
    pub fn release_validator(
        &self,
        validator_id: &str,
        caller: &str,
        auth_token: &crate::quarantine::AuthToken,
    ) -> SwarmResult<bool> {
        self.quarantine.release(validator_id, caller, auth_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = SwarmConfig::default();
        let orchestrator = SwarmOrchestrator::new(config);

        assert!(orchestrator.id().len() > 0);
        assert_eq!(orchestrator.pending_task_count(), 0);
    }

    #[test]
    fn test_validator_registration() {
        let config = SwarmConfig::default();
        let orchestrator = SwarmOrchestrator::new(config);

        let validator = Arc::new(Validator::new(
            SwarmConfig::default(),
            "test-validator".to_string(),
        ));

        orchestrator.register_validator(validator);

        assert_eq!(orchestrator.get_active_validators(), 0); // Not started
    }

    #[test]
    fn test_task_submission() {
        let config = SwarmConfig::default();
        let orchestrator = SwarmOrchestrator::new(config);

        let task = DeterministicTask::new(
            TaskType::BatchHash,
            vec![b"hello".to_vec()],
            HashAlgorithm::Keccak256,
        );

        let task_id = orchestrator.submit_task(task);
        assert!(!task_id.is_empty());
        assert_eq!(orchestrator.pending_task_count(), 1);
    }
}
