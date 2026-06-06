//! Permanent slash record — the immutable historical ledger of all slashing events.

use crate::types::SlashEvent;
use std::collections::HashMap;

/// Permanent, append-only record of all slashing events.
/// This is the "criminal record" of the jurisdiction.
/// Records are never deleted, never amended, never sealed.
pub struct SlashRecord {
    /// All events in chronological order.
    events: Vec<SlashEvent>,
    /// Events indexed by agent pubkey.
    by_agent: HashMap<[u8; 32], Vec<usize>>,
    /// Events indexed by bond ID.
    by_bond: HashMap<u64, Vec<usize>>,
}

impl SlashRecord {
    /// Create a new empty record.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            by_agent: HashMap::new(),
            by_bond: HashMap::new(),
        }
    }

    /// Record a slash event. Permanent. Irrevocable.
    pub fn record(&mut self, event: SlashEvent) {
        let idx = self.events.len();

        self.by_agent
            .entry(event.agent_id.pubkey)
            .or_default()
            .push(idx);

        self.by_bond.entry(event.bond_id.0).or_default().push(idx);

        self.events.push(event);
    }

    /// Get the complete history for an agent.
    pub fn get_agent_history(&self, pubkey: &[u8; 32]) -> Vec<&SlashEvent> {
        self.by_agent
            .get(pubkey)
            .map(|indices| indices.iter().filter_map(|&i| self.events.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get all events for a specific bond.
    pub fn get_bond_events(&self, bond_id: u64) -> Vec<&SlashEvent> {
        self.by_bond
            .get(&bond_id)
            .map(|indices| indices.iter().filter_map(|&i| self.events.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get total amount slashed for an agent across all events.
    pub fn total_slashed(&self, pubkey: &[u8; 32]) -> u128 {
        self.get_agent_history(pubkey)
            .iter()
            .map(|e| e.amount_slashed)
            .sum()
    }

    /// Get the total number of slash events for an agent.
    pub fn slash_count(&self, pubkey: &[u8; 32]) -> usize {
        self.by_agent.get(pubkey).map(|v| v.len()).unwrap_or(0)
    }

    /// Get all events in chronological order.
    pub fn all_events(&self) -> &[SlashEvent] {
        &self.events
    }

    /// Total number of recorded events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the record is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Check if an agent has any critical slashes.
    pub fn has_critical_slash(&self, pubkey: &[u8; 32]) -> bool {
        use crate::types::SlashSeverity;
        self.get_agent_history(pubkey)
            .iter()
            .any(|e| e.severity == SlashSeverity::Critical)
    }
}

impl Default for SlashRecord {
    fn default() -> Self {
        Self::new()
    }
}
