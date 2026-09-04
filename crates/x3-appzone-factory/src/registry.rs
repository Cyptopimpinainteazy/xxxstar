//! AppZone registry: tracks deployed zones and their status.
//!
//! In production this state lives in a pallet's `StorageMap`.  This
//! in-memory version is used for CLI tooling, integration tests, and the
//! devnet deployment harness.

use alloc::collections::BTreeMap;
use alloc::string::String;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

use crate::deploy::DeployRequest;

/// Unique zone identifier derived from the deploy commitment.
pub type ZoneId = H256;

/// Lifecycle state of a deployed zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum ZoneStatus {
    /// Deployment submitted but not yet confirmed on-chain.
    Pending,
    /// Deployment confirmed; zone is active and accepting traffic.
    Active,
    /// Zone has been paused by its operator.
    Paused,
    /// Zone has been permanently decommissioned.
    Decommissioned,
}

/// A registered AppZone entry.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct ZoneEntry {
    pub id: ZoneId,
    pub name: String,
    pub deploy_request: DeployRequest,
    pub status: ZoneStatus,
}

/// In-memory zone registry (real impl: pallet StorageMap).
#[derive(Default)]
pub struct ZoneRegistry {
    zones: BTreeMap<ZoneId, ZoneEntry>,
}

/// Registry errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    AlreadyRegistered,
    NotFound,
    InvalidTransition { from: ZoneStatus, to: ZoneStatus },
}

impl ZoneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, request: DeployRequest) -> Result<ZoneId, RegistryError> {
        let id = request.commitment;
        if self.zones.contains_key(&id) {
            return Err(RegistryError::AlreadyRegistered);
        }
        let entry = ZoneEntry {
            id,
            name: request.zone_name.clone(),
            deploy_request: request,
            status: ZoneStatus::Pending,
        };
        self.zones.insert(id, entry);
        Ok(id)
    }

    pub fn activate(&mut self, id: &ZoneId) -> Result<(), RegistryError> {
        self.transition(id, ZoneStatus::Active)
    }

    pub fn pause(&mut self, id: &ZoneId) -> Result<(), RegistryError> {
        self.transition(id, ZoneStatus::Paused)
    }

    pub fn decommission(&mut self, id: &ZoneId) -> Result<(), RegistryError> {
        self.transition(id, ZoneStatus::Decommissioned)
    }

    pub fn get(&self, id: &ZoneId) -> Option<&ZoneEntry> {
        self.zones.get(id)
    }

    pub fn len(&self) -> usize {
        self.zones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    fn transition(&mut self, id: &ZoneId, to: ZoneStatus) -> Result<(), RegistryError> {
        let entry = self.zones.get_mut(id).ok_or(RegistryError::NotFound)?;
        let from = entry.status;
        // Allowed transitions.
        let ok = match (from, to) {
            (ZoneStatus::Pending, ZoneStatus::Active) => true,
            (ZoneStatus::Active, ZoneStatus::Paused) => true,
            (ZoneStatus::Paused, ZoneStatus::Active) => true,
            (ZoneStatus::Active, ZoneStatus::Decommissioned) => true,
            (ZoneStatus::Paused, ZoneStatus::Decommissioned) => true,
            _ => false,
        };
        if !ok {
            return Err(RegistryError::InvalidTransition { from, to });
        }
        entry.status = to;
        Ok(())
    }
}
