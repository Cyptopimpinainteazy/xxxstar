//! Access list: tracks read/write keys touched by a transaction.
//!
//! Every transaction submits an `AccessList` before scheduling.  The
//! conflict detector uses these lists to determine whether two transactions
//! can execute in parallel.  If their write sets intersect with each other's
//! read or write sets they are conflicting and must be serialised.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;

/// A storage key: 32-byte canonical form matching Substrate's StorageKey.
pub type StorageKey = [u8; 32];

/// Access list declared by a transaction prior to scheduling.
///
/// Declarations are *advisory* — the executor validates them post-execution
/// and escalates undeclared writes to a conflict.  This design mirrors the
/// approach used in Block-STM and Aptos parallel execution:
/// optimistic parallelism with deterministic serial fallback.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct AccessList {
    /// Keys the transaction intends to read (but not write).
    pub reads: Vec<StorageKey>,
    /// Keys the transaction intends to write (implicitly read too).
    pub writes: Vec<StorageKey>,
}

impl AccessList {
    pub fn new(reads: Vec<StorageKey>, writes: Vec<StorageKey>) -> Self {
        Self { reads, writes }
    }

    /// Returns true if `other`'s write set intersects this list's read or
    /// write set.  That constitutes a RAW (read-after-write) or WAW
    /// (write-after-write) hazard and requires serialisation.
    pub fn conflicts_with(&self, other: &AccessList) -> bool {
        let my_reads: BTreeSet<_> = self.reads.iter().collect();
        let my_writes: BTreeSet<_> = self.writes.iter().collect();

        for w in &other.writes {
            if my_reads.contains(w) || my_writes.contains(w) {
                return true;
            }
        }

        // WAR (write-after-read): this tx writes into something `other` reads.
        let other_reads: BTreeSet<_> = other.reads.iter().collect();
        for w in &self.writes {
            if other_reads.contains(w) {
                return true;
            }
        }

        false
    }

    pub fn is_empty(&self) -> bool {
        self.reads.is_empty() && self.writes.is_empty()
    }
}
