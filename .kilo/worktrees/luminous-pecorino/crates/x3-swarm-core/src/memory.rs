use crate::AgentKind;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Persistent memory entry for agent learnings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwarmMemoryEntry {
    pub id: String,
    pub agent: AgentKind,
    pub feature: String,
    pub finding: String,
    pub severity: Severity,
    pub test_added: Option<String>,
    pub fix_commit: Option<String>,
    pub result: ResultState,
    pub timestamp: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultState {
    Observed,
    Passed,
    Failed,
    Skipped,
}

impl Default for ResultState {
    fn default() -> Self {
        Self::Observed
    }
}

impl SwarmMemoryEntry {
    pub fn new(id: String, agent: AgentKind, feature: String, finding: String) -> Self {
        Self {
            id,
            agent,
            feature,
            finding,
            severity: Severity::Medium,
            test_added: None,
            fix_commit: None,
            result: ResultState::Observed,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// In-memory store (append-only).
#[derive(Debug, Default)]
pub struct AgentMemory {
    entries: Vec<SwarmMemoryEntry>,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: SwarmMemoryEntry) {
        self.entries.push(entry);
    }

    pub fn query(&self, agent: Option<AgentKind>, feature: Option<&str>) -> Vec<&SwarmMemoryEntry> {
        self.entries
            .iter()
            .filter(|e| match &agent {
                Some(a) => &e.agent == a,
                None => true,
            })
            .filter(|e| match feature {
                Some(f) => e.feature == f,
                None => true,
            })
            .collect()
    }

    pub fn entries(&self) -> &[SwarmMemoryEntry] {
        &self.entries
    }
}

pub fn append_memory_entry(entries: &mut Vec<SwarmMemoryEntry>, entry: SwarmMemoryEntry) {
    // Compatibility helper for callers that operate on raw memory vectors.
    entries.push(entry);
}

pub fn load_memory_entries(entries: &[SwarmMemoryEntry]) -> Vec<SwarmMemoryEntry> {
    // Compatibility helper for snapshotting raw memory vectors.
    entries.to_vec()
}
