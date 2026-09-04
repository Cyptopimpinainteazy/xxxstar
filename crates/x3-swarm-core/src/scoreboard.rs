use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmScoreboard {
    agent_scores: HashMap<String, f64>,
    task_success_rate: f64,
    total_tasks_succeeded: u64,
    total_tasks_failed: u64,
}

impl Default for SwarmScoreboard {
    fn default() -> Self {
        Self {
            agent_scores: HashMap::new(),
            task_success_rate: 0.0,
            total_tasks_succeeded: 0,
            total_tasks_failed: 0,
        }
    }
}

impl SwarmScoreboard {
    pub fn record_result(&mut self, agent: &str, success: bool) {
        let score = self.agent_scores.entry(agent.to_string()).or_insert(0.0);
        if success {
            *score += 1.0;
            self.total_tasks_succeeded += 1;
        } else {
            *score -= 1.0;
            self.total_tasks_failed += 1;
        }
        debug_assert!(self.total_tasks_succeeded + self.total_tasks_failed > 0);
        self.task_success_rate = self.total_tasks_succeeded as f64
            / (self.total_tasks_succeeded + self.total_tasks_failed) as f64;
    }

    pub fn success_rate(&self) -> f64 {
        self.task_success_rate
    }

    pub fn agent_scores(&self) -> &HashMap<String, f64> {
        &self.agent_scores
    }
    pub fn total_tasks_succeeded(&self) -> u64 {
        self.total_tasks_succeeded
    }
    pub fn total_tasks_failed(&self) -> u64 {
        self.total_tasks_failed
    }
}
