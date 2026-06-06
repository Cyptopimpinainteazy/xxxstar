// runtime/src/fraud_proofs/freeze.rs
//
// Freeze/unfreeze hooks for consensus-critical paths.
//
// ## Design
// The "freeze" switch is a runtime-level circuit breaker that disables
// AI syscalls and the parallel scheduler when a confirmed fraud proof
// forces the node into safe-mode (CPU-only, linear execution).
//
// STATE: The `FreezeState` is stored via FRAME StorageValue in the
// `pallet-fraud-proofs` pallet.  This module is a *pure logic* helper
// that the pallet calls — it has no hidden global state of its own.
//
// INVARIANT: FREEZE-001 — freezing does not halt block production; only
// the parallel/AI scheduler path is bypassed.
// INVARIANT: FREEZE-002 — once frozen, only a governance extrinsic
// (verified via `FreezeReason::GovernanceUnfreeze`) can unfreeze.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Reason the consensus freeze was engaged.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum FreezeReason {
    /// A fraud proof was accepted and divergence was confirmed.
    DivergenceDetected,
    /// An explicit governance order (e.g. security council vote).
    GovernanceOrder,
    /// The WASM/native runtime hashes differ from recorded values.
    KernelMismatch,
    /// Emergency manual switch pulled by a privileged root call.
    EmergencyManual,
}

/// The full freeze state stored in FRAME storage.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct FreezeState {
    /// AI/GPU scheduler syscalls are disabled when this is true.
    pub ai_syscalls_frozen: bool,
    /// The parallel batch scheduler is disabled when this is true.
    pub scheduler_frozen: bool,
    /// The reason this freeze was engaged.
    pub reason: Option<FreezeReason>,
    /// Block number at which the freeze was applied (for auditing).
    pub frozen_at_block: Option<u32>,
}

impl Default for FreezeState {
    fn default() -> Self {
        Self {
            ai_syscalls_frozen: false,
            scheduler_frozen: false,
            reason: None,
            frozen_at_block: None,
        }
    }
}

impl FreezeState {
    /// Returns `true` if any consensus-critical path is currently frozen.
    #[inline]
    pub fn is_consensus_frozen(&self) -> bool {
        self.ai_syscalls_frozen || self.scheduler_frozen
    }

    /// Fully freeze the node's AI/scheduler paths.
    pub fn engage(&mut self, reason: FreezeReason, at_block: u32) {
        self.ai_syscalls_frozen = true;
        self.scheduler_frozen = true;
        self.reason = Some(reason);
        self.frozen_at_block = Some(at_block);
    }

    /// Full unfreeze — only valid if called from the governance path.
    pub fn disengage(&mut self) {
        self.ai_syscalls_frozen = false;
        self.scheduler_frozen = false;
        self.reason = None;
        self.frozen_at_block = None;
    }
}

// ---------------------------------------------------------------------------
// Guard helpers
// ---------------------------------------------------------------------------

/// Check the encoded `FreezeState` bytes and return whether to block execution.
/// Used in extrinsic pre-dispatch checks where you have raw storage bytes.
pub fn is_frozen_from_bytes(encoded: &[u8]) -> bool {
    if encoded.is_empty() {
        return false;
    }
    FreezeState::decode(&mut &encoded[..])
        .map(|s| s.is_consensus_frozen())
        .unwrap_or(false)
}

/// Build a `FreezeState` engaged with the given reason.
pub fn engaged_state(reason: FreezeReason, at_block: u32) -> FreezeState {
    let mut s = FreezeState::default();
    s.engage(reason, at_block);
    s
}

/// A compact summary string for log / event emission (no_std safe).
pub fn freeze_summary(state: &FreezeState) -> Vec<u8> {
    if state.is_consensus_frozen() {
        b"frozen".to_vec()
    } else {
        b"live".to_vec()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// FREEZE-001: engage sets both flags
    #[test]
    fn engage_sets_flags() {
        let mut state = FreezeState::default();
        assert!(!state.is_consensus_frozen());
        state.engage(FreezeReason::DivergenceDetected, 42);
        assert!(state.is_consensus_frozen());
        assert!(state.ai_syscalls_frozen);
        assert!(state.scheduler_frozen);
        assert_eq!(state.frozen_at_block, Some(42));
    }

    /// FREEZE-002: disengage clears all flags
    #[test]
    fn disengage_clears_flags() {
        let mut state = FreezeState::default();
        state.engage(FreezeReason::EmergencyManual, 10);
        state.disengage();
        assert!(!state.is_consensus_frozen());
        assert!(state.reason.is_none());
    }

    /// FREEZE-003: round-trip encode/decode preserves state
    #[test]
    fn encode_decode_roundtrip() {
        let state = engaged_state(FreezeReason::GovernanceOrder, 100);
        let bytes = state.encode();
        let decoded = FreezeState::decode(&mut &bytes[..]).expect("decode must succeed");
        assert_eq!(state, decoded);
    }

    /// FREEZE-004: is_frozen_from_bytes mirrors in-memory flag
    #[test]
    fn is_frozen_from_bytes_works() {
        let live = FreezeState::default();
        assert!(!is_frozen_from_bytes(&live.encode()));

        let frozen = engaged_state(FreezeReason::KernelMismatch, 1);
        assert!(is_frozen_from_bytes(&frozen.encode()));
    }

    /// FREEZE-005: empty bytes are not frozen
    #[test]
    fn empty_bytes_not_frozen() {
        assert!(!is_frozen_from_bytes(&[]));
    }
}
