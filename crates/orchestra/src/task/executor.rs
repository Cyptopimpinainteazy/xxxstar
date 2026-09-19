//! Task executor — approval gating, Score enforcement, execution dispatch.

use crate::agent::on_chain::OnChainAgent;
use crate::audit::{AuditEntry, AuditLog};
use crate::score::{ActionContext, ScoreEnforcer, TaskClassification};
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
    /// The backend that actually runs a task payload.
    dispatcher: Box<dyn TaskDispatcher>,
}

/// What a dispatcher produced for one task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchOutcome {
    /// The backend's output (or the reason it refused the task).
    pub output: String,
    /// Compute units the backend reports having consumed.
    pub compute_units: u64,
}

/// The backend a task payload is handed to.
///
/// The Orchestra deliberately does not contain a compute backend: the EL/CL
/// execution paths, the GPU swarm and the simulation sandbox live outside this
/// crate. An executor that invented a success for every task would make
/// [`TaskExecutor::process_task`] a no-op execution path, so the backend is a
/// required constructor argument instead.
pub trait TaskDispatcher {
    /// Run one task payload for `agent`.
    ///
    /// Returning `Err` means the task did not run. A dispatcher that ran but
    /// failed the task returns `Ok(DispatchOutcome)` and the executor records
    /// the outcome as a task failure.
    fn dispatch(&self, task: &TaskSpec, agent: &OnChainAgent) -> Result<DispatchOutcome, String>;
}

impl TaskExecutor {
    /// Build an executor around a task dispatcher.
    pub fn new(
        base_reward: u64,
        failure_penalty: i32,
        dispatcher: Box<dyn TaskDispatcher>,
    ) -> Self {
        Self {
            base_reward,
            failure_penalty,
            dispatcher,
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
            writes_to_chain: !matches!(task.metadata.task_type, crate::task::TaskType::Simulation),
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

        // Step 3: hand the payload to the dispatcher. A dispatcher that refuses
        // the task is recorded as a task failure — never as a success.
        let result = match self.dispatcher.dispatch(task, agent) {
            Ok(outcome) => ExecutionResult {
                task_id: task.metadata.id.clone(),
                success: true,
                output: outcome.output,
                compute_units: outcome.compute_units,
                reward: self.base_reward,
                completed_at: Utc::now(),
            },
            Err(reason) => ExecutionResult {
                task_id: task.metadata.id.clone(),
                success: false,
                output: reason,
                compute_units: 0,
                reward: 0,
                completed_at: Utc::now(),
            },
        };

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

    /// Test-only dispatcher: the Orchestra ships no backend, so the tests
    /// supply one and can make it refuse on demand.
    struct TestDispatcher {
        refuse_with: Option<String>,
        compute_units: u64,
    }

    impl TestDispatcher {
        fn accepting() -> Self {
            Self {
                refuse_with: None,
                compute_units: 7,
            }
        }

        fn refusing(reason: &str) -> Self {
            Self {
                refuse_with: Some(reason.to_string()),
                compute_units: 0,
            }
        }
    }

    impl TaskDispatcher for TestDispatcher {
        fn dispatch(
            &self,
            _task: &TaskSpec,
            agent: &OnChainAgent,
        ) -> Result<DispatchOutcome, String> {
            match &self.refuse_with {
                Some(reason) => Err(reason.clone()),
                None => Ok(DispatchOutcome {
                    output: format!("ran for agent {}", agent.identity.id),
                    compute_units: self.compute_units,
                }),
            }
        }
    }

    fn executor_with(dispatcher: TestDispatcher) -> TaskExecutor {
        TaskExecutor::new(100, 10, Box::new(dispatcher))
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
        let executor = executor_with(TestDispatcher::accepting());
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Execution, false);

        let result = executor.process_task(&task, &mut agent, &mut log).unwrap();
        assert!(result.success);
        assert_eq!(result.reward, 100);
        assert_eq!(result.compute_units, 7);
        assert_eq!(agent.identity.tasks_completed, 1);
        assert_eq!(
            log.iter().last().and_then(|e| e.task_event_name()),
            Some("execution_completed")
        );
    }

    #[test]
    fn major_task_blocked_without_approval() {
        let executor = executor_with(TestDispatcher::accepting());
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Law, false); // not approved

        let result = executor.process_task(&task, &mut agent, &mut log);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("jury approval"));
        assert_eq!(agent.identity.tasks_completed, 0, "nothing may have run");
    }

    #[test]
    fn approved_major_task_executes() {
        let executor = executor_with(TestDispatcher::accepting());
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Law, true); // approved by jury

        let result = executor.process_task(&task, &mut agent, &mut log).unwrap();
        assert!(result.success);
    }

    #[test]
    fn a_dispatcher_that_refuses_is_a_failed_task_not_a_success() {
        let executor = executor_with(TestDispatcher::refusing("no backend for task type"));
        let mut agent = make_agent();
        let mut log = AuditLog::new(1000);
        let task = make_task(TaskType::Execution, true);

        let result = executor.process_task(&task, &mut agent, &mut log).unwrap();

        assert!(
            !result.success,
            "a refusal must never be reported as success"
        );
        assert_eq!(result.reward, 0);
        assert_eq!(result.compute_units, 0);
        assert!(result.output.contains("no backend"));
        assert_eq!(agent.identity.tasks_completed, 0);
        assert_eq!(
            log.iter().last().and_then(|e| e.task_event_name()),
            Some("execution_failed")
        );
    }
}
