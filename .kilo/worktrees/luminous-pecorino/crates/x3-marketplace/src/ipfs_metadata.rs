//! IPFS Metadata Manager — Decentralized plugin metadata storage
//!
//! Manages IPFS pinning for plugin metadata and documentation

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::Result;

/// IPFS pin record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPFSPin {
    pub hash: String,
    pub plugin_id: String,
    pub content_type: String, // "metadata", "docs", "icon", "readme"
    pub size_bytes: u32,
    pub pinned_at: DateTime<Utc>,
    pub pins: u32, // number of pinning nodes
    pub accessible: bool,
}

impl IPFSPin {
    /// Is pin sufficiently backed (3+nodes)
    pub fn is_well_replicated(&self) -> bool {
        self.pins >= 3
    }

    /// Days since pinned
    pub fn days_pinned(&self) -> i64 {
        (Utc::now() - self.pinned_at).num_days()
    }
}

/// IPFS Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPFSManager {
    pins: HashMap<String, IPFSPin>,
    by_plugin: HashMap<String, Vec<String>>,
    pin_counter: u32,
}

impl IPFSManager {
    pub fn new() -> Self {
        IPFSManager {
            pins: HashMap::new(),
            by_plugin: HashMap::new(),
            pin_counter: 0,
        }
    }

    /// Pin metadata to IPFS
    pub fn pin_metadata(
        &mut self,
        plugin_id: &str,
        ipfs_hash: String,
        size_bytes: u32,
    ) -> Result<String> {
        let pin = IPFSPin {
            hash: ipfs_hash,
            plugin_id: plugin_id.to_string(),
            content_type: "metadata".to_string(),
            size_bytes,
            pinned_at: Utc::now(),
            pins: 1, // Default 1 node, increases through PoI
            accessible: true,
        };

        let key = pin.hash.clone();
        self.pins.insert(key.clone(), pin);
        self.by_plugin
            .entry(plugin_id.to_string())
            .or_insert_with(Vec::new)
            .push(key.clone());

        Ok(key)
    }

    /// Increase pin replication
    pub fn increase_replication(&mut self, hash: &str) -> Result<()> {
        if let Some(pin) = self.pins.get_mut(hash) {
            pin.pins = (pin.pins + 1).min(10); // Cap at 10 nodes
            Ok(())
        } else {
            Err(crate::MarketplaceError::IPFSError(
                "Pin not found".to_string(),
            ))
        }
    }

    /// Get pin info
    pub fn get_pin(&self, hash: &str) -> Option<IPFSPin> {
        self.pins.get(hash).cloned()
    }

    /// Get all pins for plugin
    pub fn plugin_pins(&self, plugin_id: &str) -> Vec<IPFSPin> {
        self.by_plugin
            .get(plugin_id)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|h| self.pins.get(h))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Mark pin as inaccessible
    pub fn mark_inaccessible(&mut self, hash: &str) -> Result<()> {
        if let Some(pin) = self.pins.get_mut(hash) {
            pin.accessible = false;
            Ok(())
        } else {
            Err(crate::MarketplaceError::IPFSError(
                "Pin not found".to_string(),
            ))
        }
    }

    /// Get accessible pins
    pub fn accessible_pins(&self) -> Vec<IPFSPin> {
        self.pins.values().filter(|p| p.accessible).cloned().collect()
    }

    /// Total pinned size
    pub fn total_storage(&self) -> u64 {
        self.pins.values().map(|p| p.size_bytes as u64).sum()
    }

    /// Count pins
    pub fn pin_count(&self) -> u32 {
        self.pins.len() as u32
    }
}

impl Default for IPFSManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_metadata() {
        let mut manager = IPFSManager::new();
        let hash = manager
            .pin_metadata("plugin1", "QmTest123".to_string(), 1024)
            .unwrap();

        assert_eq!(hash, "QmTest123");
        assert_eq!(manager.pin_count(), 1);
    }

    #[test]
    fn test_increase_replication() {
        let mut manager = IPFSManager::new();
        manager
            .pin_metadata("plugin1", "QmTest123".to_string(), 1024)
            .unwrap();

        manager.increase_replication("QmTest123").unwrap();
        let pin = manager.get_pin("QmTest123").unwrap();
        assert_eq!(pin.pins, 2);
    }

    #[test]
    fn test_well_replicated() {
        let pin = IPFSPin {
            hash: "QmTest".to_string(),
            plugin_id: "plugin1".to_string(),
            content_type: "metadata".to_string(),
            size_bytes: 1024,
            pinned_at: Utc::now(),
            pins: 3,
            accessible: true,
        };

        assert!(pin.is_well_replicated());
    }

    #[test]
    fn test_plugin_pins() {
        let mut manager = IPFSManager::new();
        manager
            .pin_metadata("plugin1", "QmTest1".to_string(), 1024)
            .unwrap();
        manager
            .pin_metadata("plugin1", "QmTest2".to_string(), 2048)
            .unwrap();

        let pins = manager.plugin_pins("plugin1");
        assert_eq!(pins.len(), 2);
    }

    #[test]
    fn test_total_storage() {
        let mut manager = IPFSManager::new();
        manager
            .pin_metadata("plugin1", "QmTest1".to_string(), 1024)
            .unwrap();
        manager
            .pin_metadata("plugin1", "QmTest2".to_string(), 2048)
            .unwrap();

        let total = manager.total_storage();
        assert_eq!(total, 3072);
    }
}
