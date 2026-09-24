//! Agent Bridge — maps Python AGI agent identity to GPU swarm tasks.
//!
//! This module provides the integration layer between the Python
//! swarm agents (via JSON over HTTP/FFI) and the Rust GPU task
//! execution pipeline.
//!
//! # Architecture
//!
//! ```text
//! Python Agent                        Rust GPU Swarm
//! ┌──────────────┐                    ┌──────────────────────┐
//! │ GpuTaskClient│ ──JSON/HTTP──►     │   AgentBridge        │
//! │              │                    │   ├─ agent_map        │
//! │ agent_id     │                    │   │  (agent→node)     │
//! │ GpuTask      │                    │   ├─ submit_task()    │
//! └──────────────┘                    │   ├─ poll_result()    │
//!                                     │   └─ cancel_task()    │
//!                                     └──────────┬───────────┘
//!                                                │
//!                                     ┌──────────▼───────────┐
//!                                     │  SwarmCoordinator     │
//!                                     │  ├─ TaskScheduler     │
//!                                     │  ├─ NodeRegistry      │
//!                                     │  └─ ExecutionVerifier │
//!                                     └──────────────────────┘
//! ```

use crate::coordinator::SwarmCoordinator;
use crate::error::{SwarmError, SwarmResult};
use crate::node::NodeId;
use crate::task::{Task, TaskId, TaskPriority, TaskStatus, TaskType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unique identifier for a Python swarm agent.
pub type AgentId = String;

/// Maps agent-centric task requests to the internal task pipeline.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTaskRequest {
    /// Python agent's unique ID
    pub agent_id: AgentId,
    /// Task type string matching GpuTaskType enum values
    pub task_type: String,
    /// Priority: "Low", "Normal", "High", "Critical"
    pub priority: String,
    /// Epoch when the agent submitted this task
    pub epoch: u64,
    /// JSON payload (opaque to bridge, passed through to GPU)
    pub payload: serde_json::Value,
    /// How many verification nodes required
    pub verification_count: u32,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Optional metadata
    pub metadata: Option<AgentTaskMetadata>,
}

/// Agent-level metadata for scheduling hints.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTaskMetadata {
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub required_capabilities: Vec<String>,
}

/// Response sent back to the Python agent after submission.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTaskResponse {
    pub task_id: String,
    pub status: String,
    pub agent_id: AgentId,
    pub assigned_node: Option<String>,
}

/// Result returned to the Python agent upon task completion.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTaskResult {
    pub task_id: String,
    pub agent_id: AgentId,
    pub status: String,
    pub result_data: serde_json::Value,
    pub result_hash: String,
    pub error: Option<String>,
    pub compute_units_used: u64,
    pub executor_node: Option<String>,
}

/// Registration record binding an agent to a preferred GPU node.
#[derive(Debug, Clone)]
struct AgentRegistration {
    agent_id: AgentId,
    preferred_node: Option<NodeId>,
    tasks_submitted: u64,
    tasks_completed: u64,
    total_compute_used: u64,
}

/// The AgentBridge manages the mapping between Python agents and GPU tasks.
///
/// It holds a registry of agent→node affinities, submits tasks to the
/// coordinator, and translates results back to the Python-compatible format.
pub struct AgentBridge {
    /// Agent identity registry
    agents: Arc<RwLock<HashMap<AgentId, AgentRegistration>>>,
    /// Reference to the coordinator (shared ownership)
    coordinator: Arc<SwarmCoordinator>,
    /// Track in-flight tasks → agent mapping
    task_owners: Arc<RwLock<HashMap<TaskId, AgentId>>>,
}

impl AgentBridge {
    /// Create a new AgentBridge connected to the given coordinator.
    pub fn new(coordinator: Arc<SwarmCoordinator>) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            coordinator,
            task_owners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a Python agent so it can submit GPU tasks.
    pub async fn register_agent(
        &self,
        agent_id: AgentId,
        preferred_node: Option<NodeId>,
    ) -> SwarmResult<()> {
        let mut agents = self.agents.write().await;
        agents.insert(
            agent_id.clone(),
            AgentRegistration {
                agent_id,
                preferred_node,
                tasks_submitted: 0,
                tasks_completed: 0,
                total_compute_used: 0,
            },
        );
        Ok(())
    }

    /// Submit a task on behalf of a Python agent.
    ///
    /// Converts the AgentTaskRequest into a Rust Task, submits it
    /// to the coordinator, and returns an AgentTaskResponse.
    pub async fn submit_task(&self, request: AgentTaskRequest) -> SwarmResult<AgentTaskResponse> {
        // Verify agent is registered
        {
            let agents = self.agents.read().await;
            if !agents.contains_key(&request.agent_id) {
                return Err(SwarmError::InvalidTask(format!(
                    "Agent {} is not registered",
                    request.agent_id
                )));
            }
        }

        // Convert to internal Task
        let task_type = Self::parse_task_type(&request.task_type)?;
        let priority = Self::parse_priority(&request.priority)?;

        let task = Task::new(task_type, [0u8; 32], 0).with_priority(priority);
        let task_id = task.id;

        // Record ownership
        {
            let mut owners = self.task_owners.write().await;
            owners.insert(task_id.clone(), request.agent_id.clone());
        }

        // Increment submitted count
        {
            let mut agents = self.agents.write().await;
            if let Some(reg) = agents.get_mut(&request.agent_id) {
                reg.tasks_submitted += 1;
            }
        }

        // Submit to coordinator
        self.coordinator.submit_task(task).await?;

        Ok(AgentTaskResponse {
            task_id: task_id.to_string(),
            status: "Pending".to_string(),
            agent_id: request.agent_id,
            assigned_node: None,
        })
    }

    /// Poll for a task result.
    ///
    /// Returns None if the task is still in progress.
    pub async fn poll_result(&self, task_id: &str) -> SwarmResult<Option<AgentTaskResult>> {
        let tid: TaskId = TaskId::parse_str(task_id)
            .map_err(|e| SwarmError::InvalidTask(format!("Invalid task ID: {}", e)))?;

        // Get the agent owning this task
        let agent_id = {
            let owners = self.task_owners.read().await;
            owners.get(&tid).cloned()
        };

        let agent_id = agent_id.unwrap_or_default();

        // Query the coordinator for status
        match self.coordinator.get_task_status(&tid).await {
            Some(TaskStatus::Completed) => {
                // Fetch result from coordinator
                if let Some(result) = self.coordinator.get_task_result(&tid).await {
                    // Update agent stats
                    {
                        let mut agents = self.agents.write().await;
                        if let Some(reg) = agents.get_mut(&agent_id) {
                            reg.tasks_completed += 1;
                            reg.total_compute_used += result.compute_units;
                        }
                    }

                    Ok(Some(AgentTaskResult {
                        task_id: task_id.to_string(),
                        agent_id,
                        status: "Completed".to_string(),
                        result_data: serde_json::from_slice(&result.result_data)
                            .unwrap_or(serde_json::Value::Null),
                        result_hash: hex::encode(result.result_hash),
                        error: None,
                        compute_units_used: result.compute_units,
                        executor_node: Some(hex::encode(result.executor)),
                    }))
                } else {
                    Ok(None)
                }
            }
            Some(TaskStatus::Failed) => Ok(Some(AgentTaskResult {
                task_id: task_id.to_string(),
                agent_id,
                status: "Failed".to_string(),
                result_data: serde_json::Value::Null,
                result_hash: String::new(),
                error: Some("Task execution failed".to_string()),
                compute_units_used: 0,
                executor_node: None,
            })),
            Some(_) => Ok(None), // Still in progress
            None => Err(SwarmError::TaskNotFound(tid)),
        }
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, task_id: &str) -> SwarmResult<bool> {
        let tid: TaskId = TaskId::parse_str(task_id)
            .map_err(|e| SwarmError::InvalidTask(format!("Invalid task ID: {}", e)))?;
        self.coordinator.cancel_task(&tid).await
    }

    /// List all pending tasks for a specific agent.
    pub async fn list_agent_tasks(&self, agent_id: &str) -> Vec<String> {
        let owners = self.task_owners.read().await;
        owners
            .iter()
            .filter(|(_, aid)| aid.as_str() == agent_id)
            .map(|(tid, _)| tid.to_string())
            .collect()
    }

    /// Get stats for a specific agent.
    pub async fn agent_stats(&self, agent_id: &str) -> Option<AgentStats> {
        let agents = self.agents.read().await;
        agents.get(agent_id).map(|reg| AgentStats {
            agent_id: reg.agent_id.clone(),
            tasks_submitted: reg.tasks_submitted,
            tasks_completed: reg.tasks_completed,
            total_compute_used: reg.total_compute_used,
        })
    }

    // ---- Internal helpers ----

    fn parse_task_type(s: &str) -> SwarmResult<TaskType> {
        match s {
            "X3Bytecode" => Ok(TaskType::X3Bytecode {
                bytecode: vec![],
                input: vec![],
                gas_budget: 0,
            }),
            "MempoolSimulation" => Ok(TaskType::MempoolSimulation {
                chain_id: 1,
                tx_count: 0,
                rpc_endpoint: String::new(),
            }),
            "RouteOptimization" => Ok(TaskType::RouteOptimization {
                source_token: String::new(),
                dest_token: String::new(),
                amount: String::new(),
                chains: vec![],
                max_hops: 3,
            }),
            "MLTraining" => Ok(TaskType::MLTraining {
                model_id: String::new(),
                training_data_hash: String::new(),
                epochs: 1,
                batch_size: 32,
            }),
            "ProofGeneration" => Ok(TaskType::ProofGeneration {
                circuit_id: String::new(),
                public_inputs: vec![],
                private_inputs: vec![],
            }),
            "ArbitrageSearch" => Ok(TaskType::ArbitrageSearch {
                pairs: vec![],
                min_profit_bps: 0,
                max_gas: 0,
            }),
            "Custom" | "CausalAnalysis" | "AgentEvaluation" | "PredictionBatch"
            | "Counterfactual" => Ok(TaskType::Custom {
                task_type: s.to_string(),
                payload: vec![],
            }),
            _ => Err(SwarmError::InvalidTask(format!("Unknown task type: {}", s))),
        }
    }

    fn parse_priority(s: &str) -> SwarmResult<TaskPriority> {
        match s {
            "Low" => Ok(TaskPriority::Low),
            "Normal" => Ok(TaskPriority::Normal),
            "High" => Ok(TaskPriority::High),
            "Critical" => Ok(TaskPriority::Critical),
            _ => Ok(TaskPriority::Normal),
        }
    }
}

/// Public-facing agent stats.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentStats {
    pub agent_id: AgentId,
    pub tasks_submitted: u64,
    pub tasks_completed: u64,
    pub total_compute_used: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_type() {
        assert!(matches!(
            AgentBridge::parse_task_type("X3Bytecode"),
            Ok(TaskType::X3Bytecode { .. })
        ));
        assert!(matches!(
            AgentBridge::parse_task_type("Custom"),
            Ok(TaskType::Custom { .. })
        ));
        assert!(matches!(
            AgentBridge::parse_task_type("CausalAnalysis"),
            Ok(TaskType::Custom { .. })
        ));
        assert!(AgentBridge::parse_task_type("BadType").is_err());
    }

    #[test]
    fn test_parse_priority() {
        assert!(matches!(
            AgentBridge::parse_priority("Critical"),
            Ok(TaskPriority::Critical)
        ));
        assert!(matches!(
            AgentBridge::parse_priority("Unknown"),
            Ok(TaskPriority::Normal)
        ));
    }

    #[test]
    fn test_agent_task_request_serde() {
        let req = AgentTaskRequest {
            agent_id: "agent-001".to_string(),
            task_type: "MLTraining".to_string(),
            priority: "High".to_string(),
            epoch: 42,
            payload: serde_json::json!({"model": "bert", "epochs": 10}),
            verification_count: 2,
            timeout_secs: 300,
            metadata: Some(AgentTaskMetadata {
                description: Some("Train BERT on agent data".to_string()),
                tags: vec!["ml".to_string(), "training".to_string()],
                required_capabilities: vec!["cuda".to_string()],
            }),
        };

        let json = serde_json::to_string(&req).expect("serialize");
        let parsed: AgentTaskRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.agent_id, "agent-001");
        assert_eq!(parsed.epoch, 42);
    }
}
