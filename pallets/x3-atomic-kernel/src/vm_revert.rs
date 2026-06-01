//! VM reversion interface for atomic bundle rollback.
//!
//! Each VM type (EVM, SVM, X3VM) must implement `VmReverter` to support
//! state reversion during bundle rollback. The trait is opt-in: if no
//! reverter is configured (using `NoopVmReverter`), rollback proceeds
//! with pallet-level cleanup only and logs a warning that VM side effects
//! were NOT reverted — ensuring the runtime does not silently advertise
//! stronger atomicity than it can enforce.

use crate::proof::VmType;
use frame_support::{traits::Get, BoundedVec};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::traits::ConstU32;
use sp_std::vec::Vec;

/// Maximum encoded VM state diff bytes kept per executed leg.
pub type MaxStateDiffBytes = ConstU32<65_536>;

/// A diff of state changes produced by a single leg execution.
///
/// The exact encoding is VM-specific; the pallet treats it as opaque bytes.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct StateDiff(pub BoundedVec<u8, MaxStateDiffBytes>);

impl StateDiff {
    /// Build a bounded state diff, truncating over-large diffs to the storage cap.
    pub fn from_vec_lossy(bytes: Vec<u8>) -> Self {
        Self(bytes.try_into().unwrap_or_else(|bytes: Vec<u8>| {
            bytes
                .into_iter()
                .take(<MaxStateDiffBytes as Get<u32>>::get() as usize)
                .collect::<Vec<_>>()
                .try_into()
                .expect("truncated state diff fits bound")
        }))
    }

    /// Returns `true` if this diff contains no state changes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<u8>> for StateDiff {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_vec_lossy(bytes)
    }
}

/// Result of reverting a single leg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertOutcome {
    /// Leg was successfully reverted.
    Reverted,
    /// Leg had no side effects (e.g., read-only leg) or was never executed.
    NoSideEffects,
}

/// Error during leg reversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertError {
    /// The state diff is corrupt or unrecognized.
    InvalidStateDiff,
    /// The VM adapter is not configured or not available.
    VmNotAvailable(VmType),
    /// The revert operation itself failed.
    RevertFailed { reason: Vec<u8> },
}

/// Trait for reverting VM state changes from a single leg.
///
/// Production runtimes implement this per-VM-type. Test runtimes
/// can use `NoopVmReverter` for unit tests.
pub trait VmReverter {
    /// Attempt to revert the state changes from a single leg execution.
    ///
    /// # Arguments
    /// - `vm_type`: Which VM produced the state diff
    /// - `state_diff`: Opaque state diff captured at execution time
    ///
    /// # Returns
    /// `Ok(RevertOutcome)` on success, `Err(RevertError)` on failure.
    fn revert_leg(vm_type: VmType, state_diff: &StateDiff) -> Result<RevertOutcome, RevertError>;
}

/// No-op reverter for test/development use.
///
/// Logs a warning and returns `NoSideEffects` for every call, signalling
/// that VM state was NOT reverted. This ensures the runtime does not
/// silently advertise stronger atomicity than it can enforce.
pub struct NoopVmReverter;

impl VmReverter for NoopVmReverter {
    fn revert_leg(vm_type: VmType, _state_diff: &StateDiff) -> Result<RevertOutcome, RevertError> {
        log::warn!(
            target: "x3-atomic-kernel",
            "NoopVmReverter: leg on {:?} NOT reverted — no VM reverter configured. \
             VM side effects persist after rollback.",
            vm_type
        );
        Ok(RevertOutcome::NoSideEffects)
    }
}

/// Receipt from executing a single bundle leg.
///
/// Stored in `BundleLegReceipts` and used during rollback to determine
/// which legs need VM state reversion.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
)]
pub struct LegReceipt {
    /// Index of the leg within the bundle (0-based).
    pub leg_index: u32,
    /// VM type that executed this leg.
    pub vm_type: VmType,
    /// Whether this leg has been executed.
    pub executed: bool,
    /// Opaque state diff captured at execution time.
    /// Empty if the leg produced no side effects or has not been executed.
    pub state_diff: StateDiff,
}
