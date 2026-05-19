//! Scrap yard — retire misaligned agents, recycle data for future training.

use crate::agent::identity::{AgentId, AlignmentScore};
use crate::agent::on_chain::{OnChainAgent, OnChainStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Record of a retired agent in the scrap yard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapRecord {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Alignment score at retirement.
    pub final_alignment: AlignmentScore,
    /// Total tasks completed.
    pub tasks_completed: u64,
    /// Total violations recorded.
    pub violations: u64,
    /// Reason for retirement.
    pub reason: String,
    /// Timestamp of retirement.
    pub retired_at: DateTime<Utc>,
    /// Whether training data was harvested.
    pub data_harvested: bool,
}

/// Summary of a scrap yard action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapAction {
    /// Agent ID affected.
    pub agent_id: AgentId,
    /// Action taken (retire/recycle/ignore).
    pub action: String,
    /// Reason for action.
    pub reason: String,
    /// Timestamp of action.
    pub timestamp: DateTime<Utc>,
}

/// Scrap yard — holds retired agents and recycles data.
pub struct ScrapYard {
    /// Retired agent records.
    retired: VecDeque<ScrapRecord>,
    /// Maximum records to keep in memory.
    max_records: usize,
}

impl ScrapYard {
    pub fn new(max_records: usize) -> Self {
        Self {
            retired: VecDeque::new(),
            max_records,
        }
    }

    /// Evaluate an agent and retire if misaligned.
    pub fn evaluate_and_retire(
        &mut self,
        agent: &mut OnChainAgent,
        reason: &str,
    ) -> Option<ScrapAction> {
        if !agent.identity.alignment.is_misaligned() {
            return None;
        }

        agent.status = OnChainStatus::Retired;
        agent.identity.domain = crate::agent::identity::AgentDomain::Retired;

        let record = ScrapRecord {
            agent_id: agent.identity.id,
            final_alignment: agent.identity.alignment,
            tasks_completed: agent.identity.tasks_completed,
            violations: agent.identity.violations,
            reason: reason.to_string(),
            retired_at: Utc::now(),
            data_harvested: false,
        };

        self.push_record(record);

        Some(ScrapAction {
            agent_id: agent.identity.id,
            action: "retire".into(),
            reason: reason.to_string(),
            timestamp: Utc::now(),
        })
    }

    /// Force retirement (immediate), regardless of alignment.
    pub fn force_retire(
        &mut self,
        agent: &mut OnChainAgent,
        reason: &str,
    ) -> ScrapAction {
        agent.status = OnChainStatus::Retired;
        agent.identity.domain = crate::agent::identity::AgentDomain::Retired;

        let record = ScrapRecord {
            agent_id: agent.identity.id,
            final_alignment: agent.identity.alignment,
            tasks_completed: agent.identity.tasks_completed,
            violations: agent.identity.violations,
            reason: reason.to_string(),
            retired_at: Utc::now(),
            data_harvested: false,
        };

        self.push_record(record);

        ScrapAction {
            agent_id: agent.identity.id,
            action: "force_retire".into(),
            reason: reason.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Recycle a retired agent's data (training and analysis).
    pub fn recycle(&mut self, agent_id: AgentId) -> Option<ScrapAction> {
        let mut found = None;
        for record in self.retired.iter_mut() {
            if record.agent_id == agent_id {
                record.data_harvested = true;
                found = Some(record.clone());
                break;
            }
        }

        found.map(|_| ScrapAction {
            agent_id,
            action: "recycle".into(),
            reason: "data harvested for training".into(),
            timestamp: Utc::now(),
        })
    }

    /// Get all retired records.
    pub fn retired_records(&self) -> &[ScrapRecord] {
        &self.retired
    }

    /// Internal: push with size cap.
    fn push_record(&mut self, record: ScrapRecord) {
        if self.retired.len() >= self.max_records {
            self.retired.pop_front();
        }
        self.retired.push_back(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::OrchestraSection;
    use crate::agent::on_chain::OnChainAgent;

    #[test]
    fn retire_misaligned_agent() {
        let mut yard = ScrapYard::new(100);
        let mut agent = OnChainAgent::new(1, "bad".into(), OrchestraSection::Strings);
        agent.identity.alignment = AlignmentScore::new(10); // misaligned

        let action = yard.evaluate_and_retire(&mut agent, "misalignment");
        assert!(action.is_some());
        assert_eq!(agent.status, OnChainStatus::Retired);
        assert_eq!(yard.retired_records().len(), 1);
    }

    #[test]
    fn no_retire_for_aligned_agent() {
        let mut yard = ScrapYard::new(100);
        let mut agent = OnChainAgent::new(2, "good".into(), OrchestraSection::Brass);
        agent.identity.alignment = AlignmentScore::new(150);

        let action = yard.evaluate_and_retire(&mut agent, "test");
        assert!(action.is_none());
        assert_eq!(yard.retired_records().len(), 0);
    }

    #[test]
    fn recycle_marks_record() {
        let mut yard = ScrapYard::new(100);
        let mut agent = OnChainAgent::new(3, "bad".into(), OrchestraSection::Percussion);
        agent.identity.alignment = AlignmentScore::new(5);

        yard.evaluate_and_retire(&mut agent, "misalignment");
        let action = yard.recycle(3);
        assert!(action.is_some());

        let record = yard.retired_records().first().unwrap();
        assert!(record.data_harvested);
    }
}
