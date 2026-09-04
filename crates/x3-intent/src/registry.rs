//! Intent registry — tracks all intents in the system.

use crate::error::IntentError;
use crate::intent::ArbIntent;
use crate::types::*;
use std::collections::HashMap;
use x3_proof::types::{BlockHeight, IntentId};

/// Registry of all intents — active and historical.
pub struct IntentRegistry {
    /// All intents by ID.
    intents: HashMap<u128, ArbIntent>,
    /// Intents by agent pubkey.
    by_agent: HashMap<[u8; 32], Vec<u128>>,
    /// Active (non-terminal) intent IDs.
    active: Vec<u128>,
    /// Next intent ID.
    next_id: u128,
}

impl IntentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            intents: HashMap::new(),
            by_agent: HashMap::new(),
            active: Vec::new(),
            next_id: 1,
        }
    }

    /// Allocate the next intent ID.
    pub fn next_id(&mut self) -> IntentId {
        let id = IntentId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Register a new intent.
    pub fn register(&mut self, intent: ArbIntent) -> Result<(), IntentError> {
        let id = intent.id.0;
        if self.intents.contains_key(&id) {
            return Err(IntentError::DuplicateId(intent.id));
        }

        self.by_agent
            .entry(intent.agent_id.pubkey)
            .or_default()
            .push(id);

        if !intent.is_terminal() {
            self.active.push(id);
        }

        self.intents.insert(id, intent);
        Ok(())
    }

    /// Get an intent by ID.
    pub fn get(&self, id: IntentId) -> Option<&ArbIntent> {
        self.intents.get(&id.0)
    }

    /// Get a mutable intent by ID.
    pub fn get_mut(&mut self, id: IntentId) -> Option<&mut ArbIntent> {
        self.intents.get_mut(&id.0)
    }

    /// Get all intents for an agent.
    pub fn get_by_agent(&self, pubkey: &[u8; 32]) -> Vec<&ArbIntent> {
        self.by_agent
            .get(pubkey)
            .map(|ids| ids.iter().filter_map(|id| self.intents.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all active (non-terminal) intents.
    pub fn active_intents(&self) -> Vec<&ArbIntent> {
        self.active
            .iter()
            .filter_map(|id| self.intents.get(id))
            .filter(|i| !i.is_terminal())
            .collect()
    }

    /// Process expired intents at the given block height.
    pub fn process_expiries(&mut self, current_block: BlockHeight) -> Vec<IntentId> {
        let mut expired = Vec::new();

        for id in &self.active {
            if let Some(intent) = self.intents.get(id) {
                if intent.is_expired(current_block) && !intent.is_terminal() {
                    expired.push(IntentId(*id));
                }
            }
        }

        for id in &expired {
            if let Some(intent) = self.intents.get_mut(&id.0) {
                intent.state = IntentState::Expired;
            }
        }

        // Clean up active list
        self.active.retain(|id| {
            self.intents
                .get(id)
                .map(|i| !i.is_terminal())
                .unwrap_or(false)
        });

        expired
    }

    /// Get counts by state.
    pub fn state_counts(&self) -> HashMap<IntentState, usize> {
        let mut counts = HashMap::new();
        for intent in self.intents.values() {
            *counts.entry(intent.state).or_insert(0) += 1;
        }
        counts
    }

    /// Total number of intents (all states).
    pub fn total_count(&self) -> usize {
        self.intents.len()
    }

    /// Number of active intents.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

impl Default for IntentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
