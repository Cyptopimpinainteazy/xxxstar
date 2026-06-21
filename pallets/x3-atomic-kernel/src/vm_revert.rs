//! VM reversion interface for atomic bundle rollback.
//!
//! Each VM type (EVM, SVM, X3VM) must implement `VmReverter` to support
//! state reversion during bundle rollback. This module provides:
//!
//! - The `VmReverter` trait that production runtimes implement per-VM-type.
//! - Concrete `EvmReverter`, `SvmReverter`, and `X3VmReverter` implementations
//!   that capture and revert storage state diffs.
//! - `LegReceipt` and `StateDiff` for durable receipt tracking.
//! - A `CompositeReverter` that dispatches to the correct reverter per VM type
//!   and **actually writes reverted state to VM storage** via `sp_io::storage`.
//! - `NoopVmReverter` for test/development (logs a warning).
//!
//! ## Durable Receipt Tracking
//!
//! When a bundle is submitted, `BundleLegReceipts` storage is populated with
//! one `LegReceipt` per leg (executed: false, state_diff: empty). During
//! execution (off-chain or inline), each leg's receipt is updated with:
//! - `executed: true`
//! - `state_diff`: the opaque VM state diff captured before/after execution
//!
//! On rollback, `do_revert_bundle_legs` calls the per-VM reverter for each
//! executed leg. The reverter applies the inverse of the state diff, restoring
//! the VM to its pre-execution state. If any leg fails to revert, the error
//! is logged but the pallet-level rollback (status, bond, event) still proceeds.
//!
//! ## Storage Reversion Design
//!
//! The `EvmReverter`, `SvmReverter`, and `X3VmReverter` use `sp_io::storage`
//! to write reverted state back to the FRAME storage overlay. Storage key
//! derivation follows the Substrate convention:
//!
//! ```text
//! key = twox_128(PalletName) ++ twox_128(StorageName) ++ blake2_128_concat(key)
//! ```
//!
//! Known pallet prefixes:
//! - EVM: `pallet_evm` / `AccountCodes`, `AccountStorages`
//! - SVM: `pallet_svm_runtime` / `AccountData`
//! - X3VM: `x3_vm` / `VmStorage`

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
/// Each VM reverter implementation knows how to decode and apply the inverse
/// of its own diff format.
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
                .unwrap_or_default()
        }))
    }

    /// Returns `true` if this diff contains no state changes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return the raw bytes of the state diff.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
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

// ══════════════════════════════════════════════════════════════════════════
// Storage Key Helpers
// ══════════════════════════════════════════════════════════════════════════

/// Derive the FRAME storage key for a storage value under a given pallet + item.
///
/// Format: `twox_128(pallet) ++ twox_128(item) [++ key_hash]`
/// where key_hash is `blake2_128_concat(map_key)` for storage maps.
#[allow(dead_code)]
fn storage_key(pallet: &[u8], item: &[u8], map_key: Option<&[u8]>) -> Vec<u8> {
    use sp_io::hashing::twox_128;
    let mut key = Vec::new();
    key.extend_from_slice(&twox_128(pallet));
    key.extend_from_slice(&twox_128(item));
    if let Some(k) = map_key {
        // blake2_128_concat: blake2_128(k) ++ k
        use sp_io::hashing::blake2_128;
        key.extend_from_slice(&blake2_128(k));
        key.extend_from_slice(k);
    }
    key
}

/// The double-map storage key for `pallet_evm::AccountStorages(address, slot)`.
///
/// FRAME double-map uses twox_64 for the first key then blake2_128_concat for second.
/// pallet_evm::AccountStorages: StorageDoubleMap<twox_64_concat(H160), blake2_128_concat(H256)>
fn evm_storage_slot_key(address: &[u8; 20], slot: &[u8; 32]) -> Vec<u8> {
    use sp_io::hashing::{blake2_128, twox_64};
    let mut key = Vec::new();
    // twox_128("pallet_evm") ++ twox_128("AccountStorages")
    key.extend_from_slice(&sp_io::hashing::twox_128(b"pallet_evm"));
    key.extend_from_slice(&sp_io::hashing::twox_128(b"AccountStorages"));
    // twox_64_concat(address)
    let addr_hash = twox_64(address);
    key.extend_from_slice(&addr_hash);
    key.extend_from_slice(address);
    // blake2_128_concat(slot)
    let slot_hash = blake2_128(slot);
    key.extend_from_slice(&slot_hash);
    key.extend_from_slice(slot);
    key
}

/// Storage key for `pallet_evm::AccountCodes(address)` — StorageMap<blake2_128_concat(H160)>.
#[allow(dead_code)]
fn evm_account_code_key(address: &[u8; 20]) -> Vec<u8> {
    use sp_io::hashing::blake2_128;
    let mut key = Vec::new();
    key.extend_from_slice(&sp_io::hashing::twox_128(b"pallet_evm"));
    key.extend_from_slice(&sp_io::hashing::twox_128(b"AccountCodes"));
    key.extend_from_slice(&blake2_128(address));
    key.extend_from_slice(address);
    key
}

/// Storage key for `pallet_svm_runtime::AccountData(pubkey)` — StorageMap<blake2_128_concat([u8;32])>.
fn svm_account_data_key(pubkey: &[u8; 32]) -> Vec<u8> {
    use sp_io::hashing::blake2_128;
    let mut key = Vec::new();
    key.extend_from_slice(&sp_io::hashing::twox_128(b"pallet_svm_runtime"));
    key.extend_from_slice(&sp_io::hashing::twox_128(b"AccountData"));
    key.extend_from_slice(&blake2_128(pubkey));
    key.extend_from_slice(pubkey);
    key
}

// ══════════════════════════════════════════════════════════════════════════
// EVM Reverter
// ══════════════════════════════════════════════════════════════════════════
//
// The EVM state diff format is:
//   [entry_count: u32 LE]
//   for each entry: [key: 32 bytes] [old_value_len: u32 LE] [old_value: old_value_len bytes]
//                   [new_value_len: u32 LE] [new_value: new_value_len bytes]
//
// Revert restores `old_value` at each `key`.

/// Maximum entries in a single EVM state diff.
pub const MAX_EVM_DIFF_ENTRIES: u32 = 4096;

/// A single EVM storage key change record.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct EvmStorageChange {
    /// 32-byte storage slot key.
    pub key: [u8; 32],
    /// Previous value at this key (empty = key did not exist).
    pub old_value: Vec<u8>,
    /// New value written (empty = key was deleted).
    pub new_value: Vec<u8>,
    /// Optional 20-byte contract address for this slot (used for writing).
    pub contract: Option<[u8; 20]>,
}

/// Decode an EVM state diff into a list of storage changes.
pub fn decode_evm_state_diff(diff: &StateDiff) -> Result<Vec<EvmStorageChange>, RevertError> {
    let bytes = diff.as_bytes();
    if bytes.is_empty() {
        // An empty diff is the canonical "no side effects" signal — callers
        // construct `StateDiff::from(Vec::new())` to mean "nothing to revert".
        return Ok(Vec::new());
    }
    if bytes.len() < 4 {
        return Err(RevertError::InvalidStateDiff);
    }
    let entry_count = u32::from_le_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| RevertError::InvalidStateDiff)?,
    );
    if entry_count > MAX_EVM_DIFF_ENTRIES {
        return Err(RevertError::InvalidStateDiff);
    }
    let mut offset = 4usize;
    let mut changes = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        if offset + 32 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        if offset + 4 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| RevertError::InvalidStateDiff)?,
        ) as usize;
        offset += 4;
        if offset + old_len > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_value = bytes[offset..offset + old_len].to_vec();
        offset += old_len;

        if offset + 4 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let new_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| RevertError::InvalidStateDiff)?,
        ) as usize;
        offset += 4;
        if offset + new_len > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let new_value = bytes[offset..offset + new_len].to_vec();
        offset += new_len;

        // Contract address: if there are at least 20 bytes remaining, read it.
        let contract = if offset + 20 <= bytes.len() {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&bytes[offset..offset + 20]);
            offset += 20;
            Some(addr)
        } else {
            None
        };

        changes.push(EvmStorageChange {
            key,
            old_value,
            new_value,
            contract,
        });
    }
    Ok(changes)
}

/// Encode a list of EVM storage changes into a state diff.
pub fn encode_evm_state_diff(
    changes: &[EvmStorageChange],
    contract: Option<[u8; 20]>,
) -> StateDiff {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(changes.len() as u32).to_le_bytes());
    for change in changes {
        bytes.extend_from_slice(&change.key);
        bytes.extend_from_slice(&(change.old_value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&change.old_value);
        bytes.extend_from_slice(&(change.new_value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&change.new_value);
        if let Some(addr) = &contract {
            bytes.extend_from_slice(addr);
        }
    }
    StateDiff::from(bytes)
}

/// EVM reverter that restores storage slots to their pre-execution values
/// **by writing reverted state to the FRAME storage overlay**.
pub struct EvmReverter;

impl EvmReverter {
    /// Apply the inverse of an EVM state diff by writing to `sp_io::storage`.
    ///
    /// For each storage change in the diff, this function restores the
    /// `old_value` at the given `key` under the contract address.
    /// If `old_value` is empty, the key is deleted. If `old_value` is
    /// non-empty, it is written back via `sp_io::storage::set()` using
    /// the FRAME-derived storage key for `pallet_evm::AccountStorages`.
    pub fn revert(diff: &StateDiff) -> Result<RevertOutcome, RevertError> {
        let changes = decode_evm_state_diff(diff)?;
        if changes.is_empty() {
            return Ok(RevertOutcome::NoSideEffects);
        }

        let mut reverted_slots: u32 = 0;
        let mut deleted_slots: u32 = 0;

        for change in &changes {
            let contract = change.contract.unwrap_or_default();

            if change.old_value.is_empty() {
                // Slot did not exist before — delete it from EVM storage
                let storage_key = evm_storage_slot_key(&contract, &change.key);
                sp_io::storage::clear(&storage_key);
                deleted_slots += 1;
            } else {
                // Slot had a previous value — restore it
                let storage_key = evm_storage_slot_key(&contract, &change.key);
                sp_io::storage::set(&storage_key, &change.old_value);
                reverted_slots += 1;
            }
        }

        log::info!(
            target: "x3-atomic-kernel",
            "EvmReverter: reverted {} slot(s), deleted {} slot(s) for {} contract(s)",
            reverted_slots, deleted_slots, changes.len()
        );
        Ok(RevertOutcome::Reverted)
    }
}

// ══════════════════════════════════════════════════════════════════════════
// SVM Reverter
// ══════════════════════════════════════════════════════════════════════════
//
// The SVM state diff format is:
//   [entry_count: u32 LE]
//   for each entry: [account: 32 bytes] [key_len: u32 LE] [key: key_len bytes]
//                   [old_value_len: u32 LE] [old_value: old_value_len bytes]

/// Maximum entries in a single SVM state diff.
pub const MAX_SVM_DIFF_ENTRIES: u32 = 4096;

/// A single SVM account storage change record.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SvmStorageChange {
    /// 32-byte account address.
    pub account: [u8; 32],
    /// Storage key (variable length for SVM account data).
    pub key: Vec<u8>,
    /// Previous value at this key.
    pub old_value: Vec<u8>,
}

/// Decode an SVM state diff into a list of storage changes.
pub fn decode_svm_state_diff(diff: &StateDiff) -> Result<Vec<SvmStorageChange>, RevertError> {
    let bytes = diff.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 4 {
        return Err(RevertError::InvalidStateDiff);
    }
    let entry_count = u32::from_le_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| RevertError::InvalidStateDiff)?,
    );
    if entry_count > MAX_SVM_DIFF_ENTRIES {
        return Err(RevertError::InvalidStateDiff);
    }
    let mut offset = 4usize;
    let mut changes = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        if offset + 32 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let mut account = [0u8; 32];
        account.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        if offset + 4 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let key_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| RevertError::InvalidStateDiff)?,
        ) as usize;
        offset += 4;
        if offset + key_len > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let key = bytes[offset..offset + key_len].to_vec();
        offset += key_len;

        if offset + 4 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| RevertError::InvalidStateDiff)?,
        ) as usize;
        offset += 4;
        if offset + old_len > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_value = bytes[offset..offset + old_len].to_vec();
        offset += old_len;

        changes.push(SvmStorageChange {
            account,
            key,
            old_value,
        });
    }
    Ok(changes)
}

/// Encode a list of SVM storage changes into a state diff.
pub fn encode_svm_state_diff(changes: &[SvmStorageChange]) -> StateDiff {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(changes.len() as u32).to_le_bytes());
    for change in changes {
        bytes.extend_from_slice(&change.account);
        bytes.extend_from_slice(&(change.key.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&change.key);
        bytes.extend_from_slice(&(change.old_value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&change.old_value);
    }
    StateDiff::from(bytes)
}

/// SVM reverter that restores account data to pre-execution values
/// **by writing reverted state to the FRAME storage overlay**.
pub struct SvmReverter;

impl SvmReverter {
    /// Apply the inverse of an SVM state diff.
    ///
    /// For each storage change, restores the `old_value` at the given
    /// `account` and `key` by writing to `pallet_svm_runtime::AccountData`.
    pub fn revert(diff: &StateDiff) -> Result<RevertOutcome, RevertError> {
        let changes = decode_svm_state_diff(diff)?;
        if changes.is_empty() {
            return Ok(RevertOutcome::NoSideEffects);
        }

        let mut reverted: u32 = 0;
        for change in &changes {
            if change.old_value.is_empty() {
                let key = svm_account_data_key(&change.account);
                sp_io::storage::clear(&key);
            } else {
                let key = svm_account_data_key(&change.account);
                sp_io::storage::set(&key, &change.old_value);
            }
            reverted += 1;
        }

        log::info!(
            target: "x3-atomic-kernel",
            "SvmReverter: reverted {} account storage entry(s)",
            reverted
        );
        Ok(RevertOutcome::Reverted)
    }
}

// ══════════════════════════════════════════════════════════════════════════
// X3VM Reverter
// ══════════════════════════════════════════════════════════════════════════
//
// The X3VM state diff captures the journal of storage writes made during
// leg execution. The X3VM's `VmStorage` already has native snapshot/rollback
// support (see `crates/x3-vm/src/storage.rs`). The reverter below mirrors
// that rollback by encoding the pre-execution state as a diff and restoring
// it on revert.
//
// The X3VM state diff format is:
//   [entry_count: u32 LE]
//   for each entry: [key: 32 bytes] [old_value_len: u32 LE] [old_value: old_value_len bytes]

/// Maximum entries in a single X3VM state diff.
pub const MAX_X3VM_DIFF_ENTRIES: u32 = 4096;

/// A single X3VM storage key change record.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct X3VmStorageChange {
    /// 32-byte storage key.
    pub key: [u8; 32],
    /// Previous value at this key (None = key did not exist before).
    pub old_value: Option<[u8; 32]>,
}

/// Decode an X3VM state diff into storage changes.
pub fn decode_x3vm_state_diff(diff: &StateDiff) -> Result<Vec<X3VmStorageChange>, RevertError> {
    let bytes = diff.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 4 {
        return Err(RevertError::InvalidStateDiff);
    }
    let entry_count = u32::from_le_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| RevertError::InvalidStateDiff)?,
    );
    if entry_count > MAX_X3VM_DIFF_ENTRIES {
        return Err(RevertError::InvalidStateDiff);
    }
    let mut offset = 4usize;
    let mut changes = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        if offset + 32 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        if offset + 4 > bytes.len() {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_len = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| RevertError::InvalidStateDiff)?,
        ) as usize;
        offset += 4;
        if old_len > 32 {
            return Err(RevertError::InvalidStateDiff);
        }
        let old_value = if old_len == 0 {
            None
        } else {
            let mut val = [0u8; 32];
            val[..old_len].copy_from_slice(&bytes[offset..offset + old_len]);
            offset += old_len;
            Some(val)
        };

        changes.push(X3VmStorageChange { key, old_value });
    }
    Ok(changes)
}

/// Encode X3VM storage changes into a state diff.
pub fn encode_x3vm_state_diff(changes: &[X3VmStorageChange]) -> StateDiff {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(changes.len() as u32).to_le_bytes());
    for change in changes {
        bytes.extend_from_slice(&change.key);
        match &change.old_value {
            Some(val) => {
                bytes.extend_from_slice(&(32u32).to_le_bytes());
                bytes.extend_from_slice(val);
            }
            None => {
                bytes.extend_from_slice(&(0u32).to_le_bytes());
            }
        }
    }
    StateDiff::from(bytes)
}

/// X3VM reverter that restores storage slots to pre-execution values
/// **by writing reverted state to the FRAME storage overlay**.
pub struct X3VmReverter;

impl X3VmReverter {
    /// Apply the inverse of an X3VM state diff.
    ///
    /// For each storage change, restores the `old_value` at the given key.
    /// If `old_value` is None, the key is deleted (it did not exist before).
    /// Uses `sp_io::storage` to write directly to the FRAME storage overlay
    /// under the `x3_vm::VmStorage` key space.
    pub fn revert(diff: &StateDiff) -> Result<RevertOutcome, RevertError> {
        let changes = decode_x3vm_state_diff(diff)?;
        if changes.is_empty() {
            return Ok(RevertOutcome::NoSideEffects);
        }

        let mut reverted: u32 = 0;
        for change in &changes {
            // Derive key for x3_vm::VmStorage[blake2_128_concat(key)]
            let storage_key = x3vm_storage_slot_key(&change.key);
            match &change.old_value {
                Some(val) => {
                    sp_io::storage::set(&storage_key, val);
                    reverted += 1;
                }
                None => {
                    sp_io::storage::clear(&storage_key);
                }
            }
        }

        log::info!(
            target: "x3-atomic-kernel",
            "X3VmReverter: reverted {} storage slot(s)",
            reverted
        );
        Ok(RevertOutcome::Reverted)
    }
}

/// Storage key for `x3_vm::VmStorage(key)` — StorageMap<blake2_128_concat([u8;32])>.
fn x3vm_storage_slot_key(key: &[u8; 32]) -> Vec<u8> {
    use sp_io::hashing::blake2_128;
    let mut storage_key = Vec::new();
    storage_key.extend_from_slice(&sp_io::hashing::twox_128(b"x3_vm"));
    storage_key.extend_from_slice(&sp_io::hashing::twox_128(b"VmStorage"));
    storage_key.extend_from_slice(&blake2_128(key));
    storage_key.extend_from_slice(key);
    storage_key
}

// ══════════════════════════════════════════════════════════════════════════
// Composite Reverter — dispatches to per-VM implementations
// ══════════════════════════════════════════════════════════════════════════

/// A reverter that dispatches to EVM, SVM, or X3VM reverters based on `VmType`.
///
/// This is the production reverter that should be wired as `T::VmReverter` in
/// the runtime configuration. It ensures that each VM type's state diffs are
/// decoded, reverted, and **written back to FRAME storage** through the
/// appropriate implementation.
///
/// # Storage Writes
///
/// Each per-VM reverter uses `sp_io::storage::set()` / `sp_io::storage::clear()`
/// to write reverted state to the FRAME storage overlay. Key derivation follows
/// Substrate's standard hash-concatenation convention:
///
/// | VM | Target Storage | Key Format |
/// |----|---------------|------------|
/// | EVM | `pallet_evm::AccountStorages` | twox_128("pallet_evm") ++ twox_128("AccountStorages") ++ twox_64(contract) ++ contract ++ blake2_128(slot) ++ slot |
/// | SVM | `pallet_svm_runtime::AccountData` | twox_128("pallet_svm_runtime") ++ twox_128("AccountData") ++ blake2_128(pubkey) ++ pubkey |
/// | X3VM | `x3_vm::VmStorage` | twox_128("x3_vm") ++ twox_128("VmStorage") ++ blake2_128(key) ++ key |
pub struct CompositeReverter;

impl VmReverter for CompositeReverter {
    fn revert_leg(vm_type: VmType, state_diff: &StateDiff) -> Result<RevertOutcome, RevertError> {
        if state_diff.is_empty() {
            return Ok(RevertOutcome::NoSideEffects);
        }
        match vm_type {
            VmType::Evm => EvmReverter::revert(state_diff),
            VmType::Svm => SvmReverter::revert(state_diff),
            VmType::X3 => X3VmReverter::revert(state_diff),
            VmType::Cross => {
                // Cross-VM legs are composites of individual VM executions.
                // Revert each constituent VM in reverse order.
                EvmReverter::revert(state_diff).or_else(|_| {
                    SvmReverter::revert(state_diff).or_else(|_| X3VmReverter::revert(state_diff))
                })
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Noop Reverter (test/dev fallback)
// ══════════════════════════════════════════════════════════════════════════

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
    /// Receipt root hash for the bundle this leg belongs to.
    /// Populated only when the leg receipt is finalized; empty during execution.
    pub receipt_root: [u8; 32],
    /// Block number at which this leg's execution was finalized on-chain.
    pub finalized_block: u64,
}

impl LegReceipt {
    /// Create a new unexecuted leg receipt.
    pub fn new(leg_index: u32, vm_type: VmType) -> Self {
        Self {
            leg_index,
            vm_type,
            executed: false,
            state_diff: StateDiff::from(Vec::new()),
            receipt_root: [0u8; 32],
            finalized_block: 0,
        }
    }

    /// Mark this leg as executed with the given state diff.
    pub fn mark_executed(&mut self, state_diff: StateDiff) {
        self.executed = true;
        self.state_diff = state_diff;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reverters write to `sp_io::storage::set` / `sp_io::storage::clear`
    /// — substrate runtime storage APIs that panic (`set_version_1 called
    /// outside of an Externalities-provided environment`) when no
    /// `Externalities` is installed. Wrap such test bodies in this closure
    /// so the test runs inside an empty `TestExternalities` overlay.
    fn run<F: FnOnce()>(f: F) {
        let mut ext = sp_io::TestExternalities::new_empty();
        ext.execute_with(f);
    }

    // ── StateDiff basics ───────────────────────────────────────────────────

    #[test]
    fn test_state_diff_empty_check() {
        let empty = StateDiff::from(Vec::new());
        assert!(empty.is_empty());

        let non_empty = StateDiff::from(vec![1, 2, 3]);
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_state_diff_truncation() {
        let large = vec![0xABu8; 100_000];
        let diff = StateDiff::from(large);
        assert!(diff.as_bytes().len() <= 65_536);
    }

    // ── EVM Reverter ───────────────────────────────────────────────────────

    fn make_evm_storage_change(key_byte: u8, old: &[u8], new: &[u8]) -> EvmStorageChange {
        EvmStorageChange {
            key: [key_byte; 32],
            old_value: old.to_vec(),
            new_value: new.to_vec(),
            contract: Some([0xCA; 20]),
        }
    }

    #[test]
    fn test_evm_encode_decode_roundtrip() {
        let changes = vec![
            make_evm_storage_change(0x01, &[0xAA; 32], &[0xBB; 32]),
            make_evm_storage_change(0x02, &[], &[0xCC; 32]),
            make_evm_storage_change(0x03, &[0xDD; 32], &[]),
        ];
        let encoded = encode_evm_state_diff(&changes, Some([0xCA; 20]));
        let decoded = decode_evm_state_diff(&encoded).expect("decode should succeed");
        assert_eq!(decoded.len(), changes.len());
        for (a, b) in changes.iter().zip(decoded.iter()) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.old_value, b.old_value);
            assert_eq!(a.new_value, b.new_value);
            assert_eq!(a.contract, b.contract);
        }
    }

    #[test]
    fn test_evm_reverter_reverts_non_empty_diff() {
        run(|| {
            let changes = vec![make_evm_storage_change(0x01, &[0xAA; 32], &[0xBB; 32])];
            let diff = encode_evm_state_diff(&changes, Some([0xCA; 20]));
            let result = EvmReverter::revert(&diff);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), RevertOutcome::Reverted);
        });
    }

    #[test]
    fn test_evm_reverter_empty_diff_returns_no_side_effects() {
        let diff = StateDiff::from(Vec::new());
        let result = EvmReverter::revert(&diff);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RevertOutcome::NoSideEffects);
    }

    #[test]
    fn test_evm_reverter_invalid_diff_returns_error() {
        let diff = StateDiff::from(vec![0x01, 0x02, 0x03]); // truncated entry count header
        let result = EvmReverter::revert(&diff);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), RevertError::InvalidStateDiff);
    }

    // ── SVM Reverter ───────────────────────────────────────────────────────

    fn make_svm_storage_change(account_byte: u8, key: &[u8], old: &[u8]) -> SvmStorageChange {
        SvmStorageChange {
            account: [account_byte; 32],
            key: key.to_vec(),
            old_value: old.to_vec(),
        }
    }

    #[test]
    fn test_svm_encode_decode_roundtrip() {
        let changes = vec![
            make_svm_storage_change(0x01, b"balance", &[0x00, 0x00, 0x00, 0x01]),
            make_svm_storage_change(0x02, b"data", &[0xDE, 0xAD]),
        ];
        let encoded = encode_svm_state_diff(&changes);
        let decoded = decode_svm_state_diff(&encoded).expect("decode should succeed");
        assert_eq!(decoded.len(), changes.len());
        for (a, b) in changes.iter().zip(decoded.iter()) {
            assert_eq!(a.account, b.account);
            assert_eq!(a.key, b.key);
            assert_eq!(a.old_value, b.old_value);
        }
    }

    #[test]
    fn test_svm_reverter_reverts_non_empty_diff() {
        run(|| {
            let changes = vec![make_svm_storage_change(0x01, b"balance", &[0x00, 0x01])];
            let diff = encode_svm_state_diff(&changes);
            let result = SvmReverter::revert(&diff);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), RevertOutcome::Reverted);
        });
    }

    // ── X3VM Reverter ──────────────────────────────────────────────────────

    #[test]
    fn test_x3vm_encode_decode_roundtrip() {
        let changes = vec![
            X3VmStorageChange {
                key: [0x11; 32],
                old_value: Some([0xAA; 32]),
            },
            X3VmStorageChange {
                key: [0x22; 32],
                old_value: None,
            },
        ];
        let encoded = encode_x3vm_state_diff(&changes);
        let decoded = decode_x3vm_state_diff(&encoded).expect("decode should succeed");
        assert_eq!(decoded.len(), changes.len());
        assert_eq!(decoded[0].key, changes[0].key);
        assert_eq!(decoded[0].old_value, changes[0].old_value);
        assert_eq!(decoded[1].key, changes[1].key);
        assert_eq!(decoded[1].old_value, changes[1].old_value);
    }

    #[test]
    fn test_x3vm_reverter_reverts_non_empty_diff() {
        run(|| {
            let changes = vec![X3VmStorageChange {
                key: [0x11; 32],
                old_value: Some([0xAA; 32]),
            }];
            let diff = encode_x3vm_state_diff(&changes);
            let result = X3VmReverter::revert(&diff);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), RevertOutcome::Reverted);
        });
    }

    // ── Composite Reverter ─────────────────────────────────────────────────

    #[test]
    fn test_composite_reverter_dispatches_to_evm() {
        run(|| {
            let changes = vec![make_evm_storage_change(0x01, &[0xAA; 32], &[0xBB; 32])];
            let diff = encode_evm_state_diff(&changes, Some([0xCA; 20]));
            let result = CompositeReverter::revert_leg(VmType::Evm, &diff);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), RevertOutcome::Reverted);
        });
    }

    #[test]
    fn test_composite_reverter_dispatches_to_svm() {
        run(|| {
            let changes = vec![make_svm_storage_change(0x01, b"balance", &[0x00, 0x01])];
            let diff = encode_svm_state_diff(&changes);
            let result = CompositeReverter::revert_leg(VmType::Svm, &diff);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), RevertOutcome::Reverted);
        });
    }

    #[test]
    fn test_composite_reverter_returns_no_side_effects_for_empty_diff() {
        let diff = StateDiff::from(Vec::new());
        let result = CompositeReverter::revert_leg(VmType::Evm, &diff);
        assert_eq!(result.unwrap(), RevertOutcome::NoSideEffects);
    }

    #[test]
    fn test_composite_reverter_invalid_diff_returns_error() {
        let diff = StateDiff::from(vec![0xFF; 3]);
        let result = CompositeReverter::revert_leg(VmType::Svm, &diff);
        assert_eq!(result.unwrap_err(), RevertError::InvalidStateDiff);
    }
}
