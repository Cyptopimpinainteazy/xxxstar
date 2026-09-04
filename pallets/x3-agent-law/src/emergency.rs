//! Emergency action module for x3-agent-law pallet.
//!
//! Provides hard-switch capability to pause, degrade, quarantine, or kill
//! the entire swarm or individual agents. All actions generate immutable audit trails.

#![allow(unused_imports)]

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use sp_core::{hashing::blake2_256, H256};
use sp_runtime::traits::Zero;
use sp_std::vec::Vec;

/// Emergency action enum
#[derive(Clone, Copy, Encode, Decode, Debug, TypeInfo, PartialEq, Eq)]
pub enum EmergencyAction {
    /// Pause all swarm operations (reversible)
    Pause,
    /// Degrade to read-only mode (reversible)
    Degrade,
    /// Quarantine a specific agent (reversible)
    Quarantine,
    /// Permanent kill-switch for agent (irreversible)
    Kill,
}

/// Immutable audit trail entry for emergency actions
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq, Eq)]
pub struct EmergencyEvent<BlockNumber, AccountId> {
    /// Action taken
    pub action: EmergencyAction,
    /// Block when triggered
    pub triggered_at_block: BlockNumber,
    /// Account that triggered (governance/Proof-Forge)
    pub triggered_by: AccountId,
    /// Reason in bounded bytes (max 256)
    pub reason: BoundedVec<u8, ConstU32<256>>,
    /// SHA2-256 hash of this event for tamper-evidence
    pub audit_hash: [u8; 32],
}

impl<BlockNumber: Clone + Encode, AccountId: Clone + Encode>
    EmergencyEvent<BlockNumber, AccountId>
{
    /// Create a new emergency event with deterministic audit hash.
    /// Hash covers: action || block || account || reason
    pub fn new(
        action: EmergencyAction,
        triggered_at_block: BlockNumber,
        triggered_by: AccountId,
        reason: BoundedVec<u8, ConstU32<256>>,
    ) -> Self {
        // Construct canonical bytes for hashing
        let mut canonical = Vec::new();

        // Encode action (1 byte discriminant)
        canonical.push(match action {
            EmergencyAction::Pause => 0u8,
            EmergencyAction::Degrade => 1u8,
            EmergencyAction::Quarantine => 2u8,
            EmergencyAction::Kill => 3u8,
        });

        // Encode block and account
        canonical.extend(triggered_at_block.encode());
        canonical.extend(triggered_by.encode());
        canonical.extend(reason.encode());

        // Compute deterministic audit hash
        let audit_hash = blake2_256(&canonical);

        Self {
            action,
            triggered_at_block,
            triggered_by,
            reason,
            audit_hash,
        }
    }

    /// Verify this emergency event's audit hash is correct
    pub fn verify_audit_hash(&self) -> bool {
        let mut canonical = Vec::new();
        canonical.push(match self.action {
            EmergencyAction::Pause => 0u8,
            EmergencyAction::Degrade => 1u8,
            EmergencyAction::Quarantine => 2u8,
            EmergencyAction::Kill => 3u8,
        });
        canonical.extend(self.triggered_at_block.encode());
        canonical.extend(self.triggered_by.encode());
        canonical.extend(self.reason.encode());

        blake2_256(&canonical) == self.audit_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_event_audit_hash_deterministic() {
        let reason = BoundedVec::try_from(b"Test emergency".to_vec()).unwrap();

        let event1 = EmergencyEvent::new(
            EmergencyAction::Pause,
            0u32,
            0u32, // simplified AccountId
            reason.clone(),
        );

        let event2 = EmergencyEvent::new(EmergencyAction::Pause, 0u32, 0u32, reason.clone());

        // Same inputs → same hash
        assert_eq!(event1.audit_hash, event2.audit_hash);
    }

    #[test]
    fn emergency_event_audit_hash_changes_with_action() {
        let reason = BoundedVec::try_from(b"Test".to_vec()).unwrap();

        let pause_event = EmergencyEvent::new(EmergencyAction::Pause, 0u32, 0u32, reason.clone());

        let kill_event = EmergencyEvent::new(EmergencyAction::Kill, 0u32, 0u32, reason.clone());

        // Different actions → different hashes
        assert_ne!(pause_event.audit_hash, kill_event.audit_hash);
    }

    #[test]
    fn emergency_event_verify_audit_hash() {
        let reason = BoundedVec::try_from(b"Verification test".to_vec()).unwrap();

        let event = EmergencyEvent::new(EmergencyAction::Quarantine, 42u32, 99u32, reason);

        // Verification should pass
        assert!(event.verify_audit_hash());
    }
}
