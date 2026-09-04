//! Position state management and persistence layer
//!
//! This module provides:
//! - Position state persistence
//! - Atomic state snapshots
//! - Fast state diffing
//! - State rollback capabilities

use crate::config::PositionManagerConfig;
use crate::error::{PositionManagerError, Result};
use crate::types::{PositionId, PositionState, PositionType, H160, H256, U256};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Position state manager for persistence and snapshots
#[derive(Debug, Clone)]
pub struct PositionStateManager {
    /// Current position states
    position_states: sp_std::collections::btree_map::BTreeMap<PositionId, PositionStateData>,
    /// State snapshots
    snapshots: Vec<StateSnapshot>,
    /// State history
    state_history: Vec<StateChange>,
    /// Configuration
    config: PositionManagerConfig,
}

impl PositionStateManager {
    /// Create a new position state manager
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            position_states: sp_std::collections::btree_map::BTreeMap::new(),
            snapshots: Vec::new(),
            state_history: Vec::new(),
            config: config.clone(),
        })
    }

    /// Start state manager
    pub async fn start(&mut self) -> Result<()> {
        // Initialize state storage
        Ok(())
    }

    /// Stop state manager
    pub async fn stop(&mut self) -> Result<()> {
        // Flush any pending state changes
        Ok(())
    }

    /// Get position state
    pub fn get_position_state(&self, position_id: &PositionId) -> Option<&PositionStateData> {
        self.position_states.get(position_id)
    }

    /// Set position state
    pub fn set_position_state(
        &mut self,
        position_id: PositionId,
        state: PositionStateData,
    ) -> Result<()> {
        // Record state change
        let change = StateChange {
            position_id: position_id.clone(),
            timestamp: sp_io::offchain::timestamp().unix_millis(),
            old_state: self.position_states.get(&position_id).map(|s| s.state),
            new_state: state.state,
            reason: "State update".to_string(),
        };

        self.state_history.push(change);

        // Update state
        self.position_states.insert(position_id, state);

        Ok(())
    }

    /// Update position state
    pub fn update_position_state(
        &mut self,
        position_id: &PositionId,
        new_state: PositionState,
        reason: &str,
    ) -> Result<()> {
        let old_state = self
            .position_states
            .get(position_id)
            .map(|s| s.state)
            .unwrap_or(PositionState::Active);

        // Record state change
        let change = StateChange {
            position_id: position_id.clone(),
            timestamp: sp_io::offchain::timestamp().unix_millis(),
            old_state: Some(old_state),
            new_state,
            reason: reason.to_string(),
        };

        self.state_history.push(change);

        // Update state
        if let Some(state_data) = self.position_states.get_mut(position_id) {
            state_data.state = new_state;
            state_data.last_updated = sp_io::offchain::timestamp().unix_millis();
        }

        Ok(())
    }

    /// Create a state snapshot
    pub fn create_snapshot(&mut self) -> Result<StateSnapshot> {
        let snapshot_id = self.generate_snapshot_id();
        let timestamp = sp_io::offchain::timestamp().unix_millis();

        let positions: Vec<PositionStateData> = self.position_states.values().cloned().collect();

        let snapshot = StateSnapshot {
            snapshot_id,
            timestamp,
            positions,
            total_positions: self.position_states.len(),
        };

        // Store snapshot
        self.snapshots.push(snapshot.clone());

        // Keep only last 100 snapshots
        if self.snapshots.len() > 100 {
            self.snapshots.remove(0);
        }

        Ok(snapshot)
    }

    /// Get latest snapshot
    pub fn get_latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.snapshots.last()
    }

    /// Get snapshot by ID
    pub fn get_snapshot(&self, snapshot_id: &H256) -> Option<&StateSnapshot> {
        self.snapshots
            .iter()
            .find(|s| s.snapshot_id == *snapshot_id)
    }

    /// Get snapshot history
    pub fn get_snapshot_history(&self, limit: Option<usize>) -> Vec<&StateSnapshot> {
        let limit = limit.unwrap_or(self.snapshots.len());
        let start = if self.snapshots.len() > limit {
            self.snapshots.len() - limit
        } else {
            0
        };
        self.snapshots[start..].iter().collect()
    }

    /// Calculate diff between two snapshots
    pub fn calculate_snapshot_diff(
        &self,
        snapshot1: &StateSnapshot,
        snapshot2: &StateSnapshot,
    ) -> SnapshotDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        // Find added positions
        for pos2 in &snapshot2.positions {
            if !snapshot1
                .positions
                .iter()
                .any(|p| p.position_id == pos2.position_id)
            {
                added.push(pos2.clone());
            }
        }

        // Find removed positions
        for pos1 in &snapshot1.positions {
            if !snapshot2
                .positions
                .iter()
                .any(|p| p.position_id == pos1.position_id)
            {
                removed.push(pos1.clone());
            }
        }

        // Find changed positions
        for pos1 in &snapshot1.positions {
            if let Some(pos2) = snapshot2
                .positions
                .iter()
                .find(|p| p.position_id == pos1.position_id)
            {
                if pos1.state != pos2.state || pos1.amount != pos2.amount {
                    changed.push(PositionDiff {
                        position_id: pos1.position_id.clone(),
                        old_state: pos1.state,
                        new_state: pos2.state,
                        old_amount: pos1.amount,
                        new_amount: pos2.amount,
                    });
                }
            }
        }

        SnapshotDiff {
            from_snapshot: snapshot1.snapshot_id,
            to_snapshot: snapshot2.snapshot_id,
            added,
            removed,
            changed,
            timestamp: sp_io::offchain::timestamp().unix_millis(),
        }
    }

    /// Rollback to a snapshot
    pub fn rollback_to_snapshot(&mut self, snapshot_id: &H256) -> Result<()> {
        let snapshot = self
            .get_snapshot(snapshot_id)
            .ok_or_else(|| PositionManagerError::SnapshotNotFound(*snapshot_id))?
            .clone();

        // Clear current state
        self.position_states.clear();

        // Restore from snapshot
        for position in snapshot.positions {
            self.position_states
                .insert(position.position_id.clone(), position);
        }

        // Record rollback
        let change = StateChange {
            position_id: PositionId::new(), // Placeholder
            timestamp: sp_io::offchain::timestamp().unix_millis(),
            old_state: None,
            new_state: PositionState::Active,
            reason: format!("Rollback to snapshot {:?}", snapshot_id),
        };

        self.state_history.push(change);

        Ok(())
    }

    /// Get state history
    pub fn get_state_history(
        &self,
        position_id: Option<&PositionId>,
        limit: Option<usize>,
    ) -> Vec<&StateChange> {
        let mut history: Vec<&StateChange> = if let Some(id) = position_id {
            self.state_history
                .iter()
                .filter(|c| c.position_id == *id)
                .collect()
        } else {
            self.state_history.iter().collect()
        };

        // Sort by timestamp (newest first)
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// Get positions by state
    pub fn get_positions_by_state(&self, state: PositionState) -> Vec<&PositionStateData> {
        self.position_states
            .values()
            .filter(|p| p.state == state)
            .collect()
    }

    /// Get positions by type
    pub fn get_positions_by_type(&self, position_type: PositionType) -> Vec<&PositionStateData> {
        self.position_states
            .values()
            .filter(|p| p.position_type == position_type)
            .collect()
    }

    /// Get positions by chain
    pub fn get_positions_by_chain(&self, chain_id: u64) -> Vec<&PositionStateData> {
        self.position_states
            .values()
            .filter(|p| p.chain_id == chain_id)
            .collect()
    }

    /// Get total positions count
    pub fn get_total_positions(&self) -> usize {
        self.position_states.len()
    }

    /// Get positions count by state
    pub fn get_positions_count_by_state(
        &self,
    ) -> sp_std::collections::btree_map::BTreeMap<PositionState, usize> {
        let mut counts = sp_std::collections::btree_map::BTreeMap::new();
        for state_data in self.position_states.values() {
            *counts.entry(state_data.state).or_insert(0) += 1;
        }
        counts
    }

    /// Generate snapshot ID
    fn generate_snapshot_id(&self) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(&sp_io::offchain::timestamp().unix_millis().to_le_bytes());
        hasher.hash(&self.position_states.len().to_le_bytes());
        hasher.hash(&self.snapshots.len().to_le_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }

    /// Clear old history
    pub fn clear_old_history(&mut self, max_age_ms: u64) {
        let cutoff = sp_io::offchain::timestamp().unix_millis() - max_age_ms;
        self.state_history.retain(|c| c.timestamp > cutoff);
    }

    /// Export state for backup
    pub fn export_state(&self) -> StateExport {
        StateExport {
            positions: self.position_states.values().cloned().collect(),
            snapshots: self.snapshots.clone(),
            history: self.state_history.clone(),
            exported_at: sp_io::offchain::timestamp().unix_millis(),
        }
    }

    /// Import state from backup
    pub fn import_state(&mut self, export: StateExport) -> Result<()> {
        self.position_states.clear();
        for position in export.positions {
            self.position_states
                .insert(position.position_id.clone(), position);
        }
        self.snapshots = export.snapshots;
        self.state_history = export.history;
        Ok(())
    }
}

/// Position state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStateData {
    pub position_id: PositionId,
    pub position_type: PositionType,
    pub state: PositionState,
    pub chain_id: u64,
    pub asset: H160,
    pub amount: U256,
    pub value_usd: U256,
    pub created_at: u64,
    pub last_updated: u64,
    pub metadata: sp_std::collections::btree_map::BTreeMap<String, String>,
}

/// State snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub snapshot_id: H256,
    pub timestamp: u64,
    pub positions: Vec<PositionStateData>,
    pub total_positions: usize,
}

/// State change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub position_id: PositionId,
    pub timestamp: u64,
    pub old_state: Option<PositionState>,
    pub new_state: PositionState,
    pub reason: String,
}

/// Snapshot diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub from_snapshot: H256,
    pub to_snapshot: H256,
    pub added: Vec<PositionStateData>,
    pub removed: Vec<PositionStateData>,
    pub changed: Vec<PositionDiff>,
    pub timestamp: u64,
}

/// Position diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionDiff {
    pub position_id: PositionId,
    pub old_state: PositionState,
    pub new_state: PositionState,
    pub old_amount: U256,
    pub new_amount: U256,
}

/// State export for backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateExport {
    pub positions: Vec<PositionStateData>,
    pub snapshots: Vec<StateSnapshot>,
    pub history: Vec<StateChange>,
    pub exported_at: u64,
}

/// Persistence layer for state storage
#[derive(Debug, Clone)]
pub struct PersistenceLayer {
    /// Storage backend
    storage_backend: StorageBackend,
    /// Configuration
    config: PositionManagerConfig,
}

impl PersistenceLayer {
    /// Create a new persistence layer
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            storage_backend: StorageBackend::Memory, // Default to memory
            config: config.clone(),
        })
    }

    /// Save state
    pub async fn save_state(&self, state: &PositionStateManager) -> Result<()> {
        match &self.storage_backend {
            StorageBackend::Memory => {
                // In-memory storage, nothing to do
                Ok(())
            }
            StorageBackend::File(path) => {
                // Save to file
                let serialized = serde_json::to_vec(state)
                    .map_err(|e| PositionManagerError::SerializationError(e.to_string()))?;
                sp_io::storage::set(path.as_bytes(), &serialized);
                Ok(())
            }
            StorageBackend::Database(url) => {
                // Save to database
                // Placeholder
                Ok(())
            }
        }
    }

    /// Load state
    pub async fn load_state(&self) -> Result<PositionStateManager> {
        match &self.storage_backend {
            StorageBackend::Memory => {
                // Return empty state
                PositionStateManager::new(&self.config)
            }
            StorageBackend::File(path) => {
                // Load from file
                if let Some(data) = sp_io::storage::get(path.as_bytes()) {
                    let state: PositionStateManager = serde_json::from_slice(&data)
                        .map_err(|e| PositionManagerError::DeserializationError(e.to_string()))?;
                    Ok(state)
                } else {
                    PositionStateManager::new(&self.config)
                }
            }
            StorageBackend::Database(url) => {
                // Load from database
                // Placeholder
                PositionStateManager::new(&self.config)
            }
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone)]
pub enum StorageBackend {
    Memory,
    File(String),
    Database(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager() {
        let config = PositionManagerConfig::default();
        let mut manager = PositionStateManager::new(&config).unwrap();

        assert_eq!(manager.get_total_positions(), 0);
    }

    #[test]
    fn test_snapshot() {
        let config = PositionManagerConfig::default();
        let mut manager = PositionStateManager::new(&config).unwrap();

        let snapshot = manager.create_snapshot().unwrap();
        assert_eq!(snapshot.total_positions, 0);
    }

    #[test]
    fn test_state_change() {
        let config = PositionManagerConfig::default();
        let mut manager = PositionStateManager::new(&config).unwrap();

        let position_id = PositionId::new();
        let state_data = PositionStateData {
            position_id: position_id.clone(),
            position_type: PositionType::Token,
            state: PositionState::Active,
            chain_id: 1,
            asset: H160::zero(),
            amount: U256::from(1000),
            value_usd: U256::from(1000),
            created_at: sp_io::offchain::timestamp().unix_millis(),
            last_updated: sp_io::offchain::timestamp().unix_millis(),
            metadata: sp_std::collections::btree_map::BTreeMap::new(),
        };

        manager
            .set_position_state(position_id.clone(), state_data)
            .unwrap();
        assert_eq!(manager.get_total_positions(), 1);
    }
}
