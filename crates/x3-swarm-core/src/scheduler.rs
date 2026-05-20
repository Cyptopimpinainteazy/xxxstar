use crate::{AgentKind, AgentTask, TaskStatus};
use std::collections::{HashMap, VecDeque};

/// Swarm task scheduler.
#[derive(Debug, Default)]
pub struct SwarmScheduler {
    tasks: HashMap<String, AgentTask>,
    task_order: VecDeque<String>,
    /// Reserved for future active-agent accounting.
    _active_agents: Vec<AgentKind>,
}

impl SwarmScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, task: AgentTask) -> Option<AgentTask> {
        if !self.tasks.contains_key(&task.id) {
            self.task_order.push_back(task.id.clone());
        }
        self.tasks.insert(task.id.clone(), task)
    }

    pub fn next_task(&self, agent: AgentKind) -> Option<&AgentTask> {
        for task_id in &self.task_order {
            let Some(task) = self.tasks.get(task_id) else {
                continue;
            };
            if task.agent == agent && task.status == TaskStatus::Pending {
                return Some(task);
            }
        }
        None
    }

    pub fn update_status(&mut self, task_id: &str, status: TaskStatus) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = status;
            return true;
        }
        false
    }

    pub fn count_tasks(&self) -> usize {
        self.tasks.len()
    }
}
