//! Leader Schedule Module

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Leader schedule entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderEntry {
    pub slot: u64,
    pub leader_id: String,
}

/// Leader schedule for upcoming slots
#[derive(Debug, Clone)]
pub struct LeaderSchedule {
    /// Current slot
    current_slot: u64,
    /// Upcoming leaders
    schedule: VecDeque<LeaderEntry>,
}

impl LeaderSchedule {
    /// Create new leader schedule
    pub fn new() -> Self {
        Self {
            current_slot: 0,
            schedule: VecDeque::new(),
        }
    }

    /// Set current slot
    pub fn set_current_slot(&mut self, slot: u64) {
        self.current_slot = slot;
        
        // Remove old entries
        while let Some(entry) = self.schedule.front() {
            if entry.slot <= slot {
                self.schedule.pop_front();
            } else {
                break;
            }
        }
    }

    /// Update the schedule
    pub fn update_schedule(&mut self, entries: Vec<LeaderEntry>) {
        self.schedule = entries.into();
    }

    /// Get upcoming leaders
    pub fn get_upcoming_leaders(&self, count: usize) -> Vec<String> {
        self.schedule
            .iter()
            .take(count)
            .map(|e| e.leader_id.clone())
            .collect()
    }

    /// Get leader for a specific slot
    pub fn get_leader(&self, slot: u64) -> Option<String> {
        self.schedule
            .iter()
            .find(|e| e.slot == slot)
            .map(|e| e.leader_id.clone())
    }

    /// Get current leader
    pub fn get_current_leader(&self) -> Option<String> {
        self.get_leader(self.current_slot)
    }
}

impl Default for LeaderSchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple round-robin leader rotation for testing
pub struct RoundRobinLeaderRotation {
    leader_ids: Vec<String>,
    current_index: usize,
}

impl RoundRobinLeaderRotation {
    pub fn new(leader_ids: Vec<String>) -> Self {
        Self {
            leader_ids,
            current_index: 0,
        }
    }

    pub fn get_leader(&mut self) -> Option<String> {
        if self.leader_ids.is_empty() {
            return None;
        }

        let leader = self.leader_ids[self.current_index].clone();
        self.current_index = (self.current_index + 1) % self.leader_ids.len();
        Some(leader)
    }

    pub fn generate_schedule(&self, start_slot: u64, count: usize) -> Vec<LeaderEntry> {
        if self.leader_ids.is_empty() {
            return Vec::new();
        }

        (0..count)
            .map(|i| {
                let index = i % self.leader_ids.len();
                LeaderEntry {
                    slot: start_slot + i as u64,
                    leader_id: self.leader_ids[index].clone(),
                }
            })
            .collect()
    }
}