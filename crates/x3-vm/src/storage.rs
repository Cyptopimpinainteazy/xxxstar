//! VM storage: deterministic key-value store for X3VM contract state.
//!
//! Provides snapshot/restore semantics for atomic window rollback and a
//! journal of all writes made during execution (for cross-VM delta sync).

use std::collections::{BTreeMap, HashMap};

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

/// In-memory deterministic storage with snapshot/restore support.
pub struct VmStorage {
    data: BTreeMap<StorageKey, StorageValue>,
    /// Stack of snapshots for nested atomic windows.
    snapshots: Vec<BTreeMap<StorageKey, StorageValue>>,
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
        if self.data.len() >= MAX_STORAGE_KEYS && !self.data.contains_key(&key) {
            return Err(StorageError::StorageLimitExceeded);
        }
        let old_value = self.data.get(&key).copied();
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
        self.snapshots.push(self.data.clone());
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
        self.data = snap;
        // Truncate journal entries since snapshot
        self.journal.clear();
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
}
