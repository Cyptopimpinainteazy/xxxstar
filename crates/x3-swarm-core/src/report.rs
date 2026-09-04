use crate::memory::AgentMemory;
use crate::scheduler::SwarmScheduler;
use crate::scoreboard::SwarmScoreboard;
use serde::{Deserialize, Serialize};

/// Report types supported by the swarm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    Health,
    Tasks,
    Memory,
}

/// Swarm readiness report generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmReport {
    pub health_status: String,
    pub task_count: usize,
    pub success_rate: f64,
    pub top_findings: Vec<String>,
}

impl SwarmReport {
    pub fn generate(
        scoreboard: &SwarmScoreboard,
        memory: &AgentMemory,
        scheduler: &SwarmScheduler,
    ) -> Self {
        Self {
            health_status: if scheduler.count_tasks() == 0 {
                "IDLE"
            } else {
                "ACTIVE"
            }
            .to_string(),
            task_count: scheduler.count_tasks(),
            success_rate: scoreboard.success_rate(),
            top_findings: memory
                .entries()
                .iter()
                .take(5)
                .map(|e| e.finding.clone())
                .collect(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn to_markdown(&self) -> String {
        let findings = if self.top_findings.is_empty() {
            "- none".to_string()
        } else {
            self.top_findings
                .iter()
                .map(|finding| format!("- {}", finding))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!("# X3 Swarm Report\n\nHealth Status: {}\nSuccess Rate: {:.2}%\nTasks: {}\n\nTop Findings:\n{}", self.health_status, self.success_rate * 100.0, self.task_count, findings)
    }
}
