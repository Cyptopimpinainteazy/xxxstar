//! VM storage: deterministic key-value store for X3VM contract state.
//!
//! Provides snapshot/restore semantics for atomic window rollback and a
//! journal of all writes made during execution (for cross-VM delta sync).

use crate::prelude::*;
use alloc::collections::BTreeMap;

/// Storage key: 32-byte hash.
pub type StorageKey = [u8; 32];
/// Storage value: up to 32 bytes.
pub type StorageValue = [u8; 32];

/// A single write record in the storage journal.
#[derive(Clone, Debug)]
pub struct WriteRecord {
    pub key: StorageKey,
    pub old_value: Option<StorageValue>,
    pub new_value: Option<StorageValue>,
}

/// Errors from storage operations.
#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// Snapshot stack underflow (too many restores).
    SnapshotUnderflow,
    /// Storage size limit exceeded.
    StorageLimitExceeded,
}

/// Maximum number of keys in a single contract's storage.
pub const MAX_STORAGE_KEYS: usize = 65_536;

/// One atomic window's snapshot: the store's contents, plus how long the journal
/// was when the window opened.
///
/// The journal length is what makes the rollback *transactional*. Rolling back
/// used to call `journal.clear()`, which dropped every write recorded before the
/// window as well as the ones inside it — so a caller that applies the journal
/// after execution would lose pre-window state changes, and the journal stopped
/// describing the delta it documents ("all writes since last flush"). Measured
/// 2026-09-26: writing `A`, snapshotting, writing `B` and rolling back left
/// `data = {A}` and an empty journal.
struct Snapshot {
    data: BTreeMap<StorageKey, StorageValue>,
    journal_len: usize,
}

/// In-memory deterministic storage with snapshot/restore support.
pub struct VmStorage {
    data: BTreeMap<StorageKey, StorageValue>,
    /// Stack of snapshots for nested atomic windows.
    snapshots: Vec<Snapshot>,
    /// Journal of all writes since last flush.
    journal: Vec<WriteRecord>,
}

impl VmStorage {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            snapshots: Vec::new(),
            journal: Vec::new(),
        }
    }

    /// Read a value by key.
    pub fn get(&self, key: &StorageKey) -> Option<&StorageValue> {
        self.data.get(key)
    }

    /// Write a value by key. Appends to journal.
    pub fn set(
        &mut self,
        key: StorageKey,
        value: Option<StorageValue>,
    ) -> Result<(), StorageError> {
        let old_value = self.data.get(&key).copied();
        self.record(key, value, old_value)
    }

    /// Load a slot the chain already holds, without journaling it.
    ///
    /// A seed is the state an execution *starts* from, not a change it makes, so it must not appear
    /// in the journal a receipt is built from. It does have to be in `data`: that is what makes a
    /// later `set` report the chain's value as the write's `old_value` (instead of `None`), and what
    /// makes an atomic window's rollback restore the chain's value rather than delete the slot.
    ///
    /// Refuses past the same key limit `set` enforces, so an execution cannot be handed more state
    /// than a contract's storage may hold.
    pub fn seed_slot(&mut self, key: StorageKey, value: StorageValue) -> Result<(), StorageError> {
        if self.data.len() >= MAX_STORAGE_KEYS && !self.data.contains_key(&key) {
            return Err(StorageError::StorageLimitExceeded);
        }
        self.data.insert(key, value);
        Ok(())
    }

    /// The one place a mutation is applied and journaled, so every write path records the same
    /// `old_value` rule.
    fn record(
        &mut self,
        key: StorageKey,
        value: Option<StorageValue>,
        old_value: Option<StorageValue>,
    ) -> Result<(), StorageError> {
        if self.data.len() >= MAX_STORAGE_KEYS && !self.data.contains_key(&key) {
            return Err(StorageError::StorageLimitExceeded);
        }
        match value {
            Some(v) => {
                self.data.insert(key, v);
            }
            None => {
                self.data.remove(&key);
            }
        }
        self.journal.push(WriteRecord {
            key,
            old_value,
            new_value: value,
        });
        Ok(())
    }

    /// Begin an atomic window: push a snapshot of current state.
    pub fn snapshot(&mut self) {
        self.snapshots.push(Snapshot {
            data: self.data.clone(),
            journal_len: self.journal.len(),
        });
    }

    /// Commit the current atomic window: pop snapshot without restoring.
    pub fn commit(&mut self) -> Result<(), StorageError> {
        if self.snapshots.is_empty() {
            return Err(StorageError::SnapshotUnderflow);
        }
        self.snapshots.pop();
        Ok(())
    }

    /// Abort the current atomic window: restore from snapshot.
    pub fn rollback(&mut self) -> Result<(), StorageError> {
        let snap = self
            .snapshots
            .pop()
            .ok_or(StorageError::SnapshotUnderflow)?;
        self.data = snap.data;
        // Truncate to the length the journal had when this window opened: the writes
        // inside the window are abandoned, the writes before it are not.
        self.journal.truncate(snap.journal_len);
        Ok(())
    }

    /// Drain the journal (for cross-VM delta sync).
    pub fn drain_journal(&mut self) -> Vec<WriteRecord> {
        core::mem::take(&mut self.journal)
    }

    /// Number of keys currently stored.
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Default for VmStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(b: u8) -> StorageKey {
        [b; 32]
    }
    fn val(b: u8) -> StorageValue {
        [b; 32]
    }

    #[test]
    fn test_set_and_get() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(0xAB))).unwrap();
        assert_eq!(s.get(&key(1)), Some(&val(0xAB)));
    }

    #[test]
    fn test_delete_removes_key() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap();
        s.set(key(1), None).unwrap();
        assert_eq!(s.get(&key(1)), None);
    }

    #[test]
    fn test_snapshot_and_rollback() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap();
        s.snapshot();
        s.set(key(1), Some(val(99))).unwrap();
        assert_eq!(s.get(&key(1)), Some(&val(99)));
        s.rollback().unwrap();
        assert_eq!(s.get(&key(1)), Some(&val(1)));
    }

    #[test]
    fn test_snapshot_and_commit_keeps_changes() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap();
        s.snapshot();
        s.set(key(1), Some(val(99))).unwrap();
        s.commit().unwrap();
        assert_eq!(s.get(&key(1)), Some(&val(99)));
    }

    #[test]
    fn test_rollback_underflow() {
        let mut s = VmStorage::new();
        assert_eq!(s.rollback(), Err(StorageError::SnapshotUnderflow));
    }

    #[test]
    fn test_journal_tracking() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap();
        s.set(key(2), Some(val(2))).unwrap();
        let journal = s.drain_journal();
        assert_eq!(journal.len(), 2);
    }

    #[test]
    fn test_nested_snapshots() {
        let mut s = VmStorage::new();
        s.snapshot();
        s.set(key(1), Some(val(1))).unwrap();
        s.snapshot();
        s.set(key(2), Some(val(2))).unwrap();
        s.rollback().unwrap(); // inner rollback
        assert_eq!(s.get(&key(2)), None);
        assert_eq!(s.get(&key(1)), Some(&val(1)));
        s.commit().unwrap(); // outer commit
    }

    /// A rollback abandons the writes inside the window, and only those.
    ///
    /// This is the regression test for the rollback that cleared the whole journal:
    /// the write made before the window is still in the store, so it has to still be
    /// in the journal — otherwise whoever applies the journal loses it.
    #[test]
    fn test_rollback_keeps_the_journal_of_writes_that_predate_the_window() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap(); // before the window
        s.snapshot();
        s.set(key(2), Some(val(2))).unwrap(); // inside the window
        s.rollback().unwrap();

        assert_eq!(s.get(&key(1)), Some(&val(1)));
        assert_eq!(s.get(&key(2)), None);
        let journal = s.drain_journal();
        assert_eq!(
            journal.len(),
            1,
            "only the pre-window write survives, got {journal:?}"
        );
        assert_eq!(journal[0].key, key(1));
        assert_eq!(journal[0].new_value, Some(val(1)));
    }

    /// An inner rollback must not drop the outer window's writes either.
    #[test]
    fn test_nested_rollback_truncates_the_journal_to_the_inner_window() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap(); // before everything
        s.snapshot(); // outer
        s.set(key(2), Some(val(2))).unwrap(); // outer scope
        s.snapshot(); // inner
        s.set(key(3), Some(val(3))).unwrap(); // inner scope
        s.rollback().unwrap(); // inner

        assert_eq!(s.get(&key(3)), None);
        assert_eq!(s.get(&key(2)), Some(&val(2)));
        let journal = s.drain_journal();
        assert_eq!(
            journal.iter().map(|w| w.key).collect::<Vec<_>>(),
            vec![key(1), key(2)],
            "the inner rollback must keep both writes outside the inner window"
        );
        s.commit().unwrap();
    }

    /// Rolling back an outer window after an inner commit still abandons everything
    /// the outer window did — including the inner writes it committed.
    #[test]
    fn test_rollback_of_the_outer_window_abandons_a_committed_inner_window() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap(); // before everything
        s.snapshot(); // outer
        s.set(key(2), Some(val(2))).unwrap();
        s.snapshot(); // inner
        s.set(key(3), Some(val(3))).unwrap();
        s.commit().unwrap(); // inner commits into the outer window
        s.rollback().unwrap(); // outer aborts anyway

        assert_eq!(s.get(&key(2)), None);
        assert_eq!(s.get(&key(3)), None);
        assert_eq!(s.get(&key(1)), Some(&val(1)));
        let journal = s.drain_journal();
        assert_eq!(
            journal.len(),
            1,
            "only the pre-window write is left in the delta, got {journal:?}"
        );
    }

    /// A delete inside the window is reverted *and* removed from the journal, so the
    /// delta never claims a delete that did not happen.
    #[test]
    fn test_rollback_of_a_delete_restores_the_key_and_the_journal() {
        let mut s = VmStorage::new();
        s.set(key(1), Some(val(1))).unwrap();
        let _ = s.drain_journal(); // flush, so the next delta starts empty
        s.snapshot();
        s.set(key(1), None).unwrap(); // delete inside the window
        s.rollback().unwrap();

        assert_eq!(s.get(&key(1)), Some(&val(1)));
        assert!(
            s.drain_journal().is_empty(),
            "a reverted delete must not appear in the write delta"
        );
    }
}
