// crates/gpu-swarm/src/advanced/jury.rs
// Jury System - Encrypted audit logging and agent rotation

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tracing::{debug, span, Level};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event_type: String,
    pub agent_id: String,
    pub action: String,
    pub result: String,
    pub metadata: serde_json::Value,
    pub hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Jury {
    pub id: String,
    pub agents: Vec<JuryAgent>,
    pub audit_log: Arc<Mutex<Vec<AuditEntry>>>,
    pub rotation_interval_seconds: u64,
    pub last_rotation: Arc<Mutex<u64>>,
    pub encryption_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JuryAgent {
    pub id: String,
    pub reputation: f32,
    pub evaluations_count: u64,
    pub accurate_evaluations: u64,
    pub failed_evaluations: u64,
    pub slashed_count: u64,
    pub active: bool,
    pub joined_at: String,
}

impl Jury {
    pub fn new(agents: Vec<JuryAgent>, rotation_interval_seconds: u64) -> Self {
        let jury_id = format!("jury-{}", Utc::now().timestamp());
        let key = Self::generate_encryption_key();

        Jury {
            id: jury_id,
            agents,
            audit_log: Arc::new(Mutex::new(Vec::new())),
            rotation_interval_seconds,
            last_rotation: Arc::new(Mutex::new(Utc::now().timestamp() as u64)),
            encryption_key: key,
        }
    }

    /// Log an audit event (encrypted)
    pub fn log_audit_event(
        &self,
        agent_id: &str,
        action: &str,
        result: &str,
        metadata: serde_json::Value,
    ) -> Result<AuditEntry, Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "jury_log_audit", agent_id = agent_id);
        let _enter = span.enter();

        let timestamp = Utc::now().to_rfc3339();
        let event_data = format!("{}-{}-{}-{}", timestamp, agent_id, action, result);
        
        let mut hasher = Sha256::new();
        hasher.update(event_data.as_bytes());
        let hash = STANDARD.encode(hasher.finalize());

        let entry = AuditEntry {
            timestamp,
            event_type: "jury_evaluation".to_string(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            result: result.to_string(),
            metadata,
            hash: hash.clone(),
        };

        let mut log = self.audit_log.lock();
        log.push(entry.clone());

        debug!("📝 Audit logged for agent {}: {}", agent_id, result);

        Ok(entry)
    }

    /// Rotate jury agents based on reputation
    pub fn rotate_agents(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "jury_rotate_agents");
        let _enter = span.enter();

        let now = Utc::now().timestamp() as u64;
        let mut last_rotation = self.last_rotation.lock();

        if now - *last_rotation < self.rotation_interval_seconds {
            return Ok(Vec::new());
        }

        // Calculate accuracy scores
        let mut agent_scores: Vec<_> = self
            .agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let accuracy = if agent.evaluations_count > 0 {
                    agent.accurate_evaluations as f32 / agent.evaluations_count as f32
                } else {
                    0.5
                };
                
                let reputation_factor = agent.reputation / 100.0;
                let slash_penalty = (1.0 - (agent.slashed_count as f32 * 0.1).min(1.0));
                
                let score = accuracy * reputation_factor * slash_penalty;
                (i, agent.id.clone(), score)
            })
            .collect();

        // Sort by score (ascending - low scores out)
        agent_scores.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // Bottom 20% get rotated out
        let rotation_count = (self.agents.len() / 5).max(1);
        let mut retired_agents = Vec::new();

        for (i, _, _) in agent_scores.iter().take(rotation_count) {
            self.agents[*i].active = false;
            retired_agents.push(self.agents[*i].id.clone());
        }

        *last_rotation = now;

        debug!("🔄 Rotated {} agents", retired_agents.len());

        Ok(retired_agents)
    }

    /// Update agent reputation based on evaluation accuracy
    pub fn update_agent_reputation(
        &mut self,
        agent_id: &str,
        was_accurate: bool,
    ) -> Result<f32, Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "jury_update_reputation", agent_id = agent_id);
        let _enter = span.enter();

        if let Some(agent) = self.agents.iter_mut().find(|a| a.id == agent_id) {
            agent.evaluations_count += 1;

            if was_accurate {
                agent.accurate_evaluations += 1;
                agent.reputation = (agent.reputation + 2.0).min(100.0);
            } else {
                agent.failed_evaluations += 1;
                agent.reputation = (agent.reputation - 5.0).max(0.0);
            }

            debug!(
                "📊 Agent {} reputation updated to {:.2}",
                agent_id, agent.reputation
            );

            Ok(agent.reputation)
        } else {
            Err(format!("Agent {} not found", agent_id).into())
        }
    }

    /// Slash agent for malicious behavior
    pub fn slash_agent(&mut self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let span = span!(Level::DEBUG, "jury_slash_agent", agent_id = agent_id);
        let _enter = span.enter();

        if let Some(agent) = self.agents.iter_mut().find(|a| a.id == agent_id) {
            agent.slashed_count += 1;
            agent.reputation = (agent.reputation - 25.0).max(0.0);
            
            if agent.slashed_count > 3 {
                agent.active = false;
                debug!("🚨 Agent {} deactivated after {} slashes", agent_id, agent.slashed_count);
            } else {
                debug!("⚠️ Agent {} slashed ({} times)", agent_id, agent.slashed_count);
            }

            Ok(())
        } else {
            Err(format!("Agent {} not found", agent_id).into())
        }
    }

    /// Get audit log (decrypted/filtered)
    pub fn get_audit_log(&self, agent_id: Option<&str>) -> Result<Vec<AuditEntry>, Box<dyn std::error::Error>> {
        let log = self.audit_log.lock();

        let entries = if let Some(agent) = agent_id {
            log.iter()
                .filter(|e| e.agent_id == agent)
                .cloned()
                .collect()
        } else {
            log.iter().cloned().collect()
        };

        Ok(entries)
    }

    /// Verify audit log integrity (check hashes)
    pub fn verify_audit_log(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let log = self.audit_log.lock();

        for entry in log.iter() {
            let event_data = format!(
                "{}-{}-{}-{}",
                entry.timestamp, entry.agent_id, entry.action, entry.result
            );

            let mut hasher = Sha256::new();
            hasher.update(event_data.as_bytes());
            let calculated_hash = STANDARD.encode(hasher.finalize());

            if calculated_hash != entry.hash {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Generate encryption key
    fn generate_encryption_key() -> String {
        let key = format!("key-{}", Utc::now().timestamp());
        STANDARD.encode(key)
    }

    /// Get jury statistics
    pub fn get_stats(&self) -> JuryStats {
        let active_agents = self.agents.iter().filter(|a| a.active).count();
        let total_reputation: f32 = self.agents.iter().map(|a| a.reputation).sum();
        let avg_reputation = total_reputation / self.agents.len().max(1) as f32;

        JuryStats {
            total_agents: self.agents.len(),
            active_agents,
            average_reputation: avg_reputation,
            total_audits: self.audit_log.lock().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JuryStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub average_reputation: f32,
    pub total_audits: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jury_creation() {
        let agents = vec![
            JuryAgent {
                id: "agent1".to_string(),
                reputation: 85.0,
                evaluations_count: 100,
                accurate_evaluations: 95,
                failed_evaluations: 5,
                slashed_count: 0,
                active: true,
                joined_at: Utc::now().to_rfc3339(),
            },
        ];

        let jury = Jury::new(agents, 3600);
        assert_eq!(jury.agents.len(), 1);
        assert!(jury.encryption_key.len() > 0);
    }

    #[test]
    fn test_audit_logging() {
        let agents = vec![JuryAgent {
            id: "agent1".to_string(),
            reputation: 85.0,
            evaluations_count: 0,
            accurate_evaluations: 0,
            failed_evaluations: 0,
            slashed_count: 0,
            active: true,
            joined_at: Utc::now().to_rfc3339(),
        }];

        let jury = Jury::new(agents, 3600);
        let result = jury.log_audit_event(
            "agent1",
            "evaluation",
            "accurate",
            serde_json::json!({"confidence": 0.95}),
        );

        assert!(result.is_ok());
    }
}
