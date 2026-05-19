//! Task executor — approval gating, Score enforcement, execution dispatch.

use crate::agent::identity::AgentId;
use crate::agent::on_chain::OnChainAgent;
use crate::audit::{AuditEntry, AuditLog};
use crate::score::{ActionContext, ScoreEnforcer, TaskClassification};
use crate::task::queue::{TaskQueue, TaskStatus};
use crate::task::spec::TaskSpec;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Task ID.
    pub task_id: String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Output or error message.
    pub output: String,
    /// Compute units consumed.
    pub compute_units: u64,
    /// Reward earned (if successful).
    pub reward: u64,
    /// Timestamp of completion.
    pub completed_at: chrono::DateTime<Utc>,
}

/// The task executor — routes tasks through approval gates and dispatches execution.
pub struct TaskExecutor {
    /// Reward per successful task execution.
    pub base_reward: u64,
    /// penalty per failed task execution (alignment score decrease).
    pub failure_penalty: i32,
}

impl TaskExecutor {
    pub fn new(base_reward: u64, failure_penalty: i32) -> Self {
        Self {
            base_reward,
            failure_penalty,
        }
    }

    /// Process a single task: check classification, gate approval, execute.
    ///
    /// Returns:
    /// - `Ok(ExecutionResult)` if the task executed (success or failure)
    /// - `Err(String)` if the task cannot proceed (needs jury, blocked, etc.)
    pub fn process_task(
        &self,
        task: &TaskSpec,
        agent: &mut OnChainAgent,
        audit_log: &mut AuditLog,
    ) -> Result<ExecutionResult, String> {
        // Step 1: Score validation
        let ctx = ActionContext {
            action_type: format!(
                "execute_task:{}:{}",
                task.metadata.id,
                match task.metadata.task_type {
                    crate::task::TaskType::Law => "law",
                    crate::task::TaskType::Execution => "execution",
                    crate::task::TaskType::Simulation => "simulation",
                }
            ),
            is_protocol_bound: true,
            claims_sovereignty: false,
            is_loggable: true,
            writes_to_chain: !matches!(
                task.metadata.task_type,
                crate::task::TaskType::Simulation
            ),
        };

        if let Err(violation) = ScoreEnforcer::validate_on_chain_action(agent.identity.id, &ctx) {
            agent.slash(&violation, self.failure_penalty);
            audit_log.append(AuditEntry::violation(
                agent.identity.id,
                violation.commandment,
                violation.detail.clone(),
            ));
            return Err(format!("Score violation: {}", violation));
        }

        // Step 2: Classification gate
        if task.classification == TaskClassification::Major && !task.approved {
            // Major tasks must be approved by jury first
            audit_log.append(AuditEntry::task_event(
                agent.identity.id,
                &task.metadata.id,
                "staged_for_jury",
                "Major task requires jury approval before execution",
            ));
            return Err(format!(
                "Task {} requires jury approval (classification: Major)",
                task.metadata.id
            ));
        }

        // Step 3: Execute
        audit_log.append(AuditEntry::task_event(
            agent.identity.id,
            &task.metadata.id,
            "execution_started",
            &format!(
                "Executing task type={:?}, priority={:?}",
                task.metadata.task_type, task.metadata.priority
            ),
        ));

        // Simulate execution — in production this dispatches to the actual compute backend
        let result = self.execute_task_payload(task, agent);

        // Step 4: Record result
        if result.success {
            agent.complete_task(result.reward);
            audit_log.append(AuditEntry::task_event(
                agent.identity.id,
                &task.metadata.id,
                "execution_completed",
                &format!(
                    "Success: compute_units={}, reward={}",
                    result.compute_units, result.reward
                ),
            ));
        } else {
            agent.fail_task(self.failure_penalty);
            audit_log.append(AuditEntry::task_event(
                agent.identity.id,
                &task.metadata.id,
                "execution_failed",
                &result.output,
            ));
        }

        Ok(result)
    }

    /// Execute the actual task payload. In production, this dispatches to:
    /// - GPU swarm for compute tasks
    /// - EVM/SVM/X3VM for contract tasks
    /// - Simulation sandbox for simulation tasks
    fn execute_task_payload(&self, task: &TaskSpec, agent: &OnChainAgent) -> ExecutionResult {
        let compute_units = match task.metadata.priority {
            crate::task::spec::TaskPriority::High => 1000,
            crate::task::spec::TaskPriority::Medium => 500,
            crate::task::spec::TaskPriority::Low => 100,
        };

        // For now, all tasks succeed (real implementation dispatches to VM backends)
        ExecutionResult {
            task_id: task.metadata.id.clone(),
            success: true,
            output: format!(
                "Task executed by agent {} in section {}",
                agent.identity.id, agent.identity.section
            ),
            compute_units,
            reward: self.base_reward,
            completed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::OrchestraSection;
    use crate::agent::on_chain::OnChainAgent;
    use crate::task::spec::*;

    fn make_agent() -> OnChainAgent {
        OnChainAgent::new(1, "executor-1".into(), OrchestraSection::Strings)
    }

    fn make_task(task_type: TaskType, approved: bool) -> TaskSpec {
        TaskSpec {
            metadata: TaskMetadata {
                id: "test-task".into(),
                priority: TaskPriority::High,
                section: OrchestraSection::Strings,
                proposer: 42,
                timestamp: Utc::now(),
                task_type,
            },
            body: "Test".into(),
            simulation_output: None,
            source_path: None,
            approved,
            classification: task_type.classification(),
        }
    }

    #[test]
    fn minor_task_executes_without_jury() {
        let executor = TaskExecutor::new(100, 10);
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Execution, false);

        let result = executor.process_task(&task, &mut agent, &mut log).unwrap();
        assert!(result.success);
        assert_eq!(result.reward, 100);
        assert_eq!(agent.identity.tasks_completed, 1);
    }

    #[test]
    fn major_task_blocked_without_approval() {
        let executor = TaskExecutor::new(100, 10);
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Law, false); // not approved

        let result = executor.process_task(&task, &mut agent, &mut log);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("jury approval"));
    }

    #[test]
    fn approved_major_task_executes() {
        let executor = TaskExecutor::new(100, 10);
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Law, true); // approved by jury

        let result = executor.process_task(&task, &mut agent, &mut log).unwrap();
        assert!(result.success);
    }
}
