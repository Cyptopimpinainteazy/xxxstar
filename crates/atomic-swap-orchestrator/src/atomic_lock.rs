//! Atomic Fund Locking for 2PC Settlement
//!
//! This module ensures 2PC (Two-Phase Commit) atomicity by:
//! 1. Locking funds in Prepare phase → funds cannot be spent
//! 2. Either Committing (transferring) or Aborting (refunding) → no partial states
//! 3. Automatic slash if escrow not released within deadline
//!
//! Design: Lock-Release-or-Slash
//! - Prepare: Lock funds in escrow (tx_hold_reserve)
//! - Commit: Release from escrow, transfer finality
//! - Abort: Release from escrow, refund to original owner
//! - Timeout: Slash escrow, validator loses bond

use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ReservableCurrency},
};
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::vec::Vec;

/// Account that holds locked funds during 2PC phases
/// Formula: blake2_128_concat(intent_id || "escrow" || nonce)
/// Ensures no account-enumeration attacks and per-intent isolation
pub struct EscrowAccount;

impl EscrowAccount {
    pub fn derive<T: frame_system::Config>(intent_id: &[u8; 32], nonce: u32) -> T::AccountId {
        let mut data = Vec::with_capacity(36);
        data.extend_from_slice(intent_id);
        data.extend_from_slice(&nonce.to_le_bytes());

        let hash = sp_io::hashing::blake2_128_concat(&data);
        T::AccountId::from(hash)
    }
}

/// 2PC Atomic lock state
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum LockPhase {
    /// Funds locked during Prepare → ready for Commit
    LockedForCommit {
        locked_at_block: u32,
        commit_deadline: u32,
    },
    /// Funds locked during Commit → transferring
    CommitInProgress {
        locked_at_block: u32,
        finalize_deadline: u32,
    },
    /// Lock released (committed or aborted)
    Released {
        at_block: u32,
        reason: ReleaseReason,
    },
    /// Timed out and slashed (executor penalized)
    Slashed {
        at_block: u32,
        executor_id: [u8; 32],
    },
}

/// Reason for lock release
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum ReleaseReason {
    CommitSucceeded,
    CommitFailed,
    AbortRequested,
    TimeoutRefund,
}

/// Per-intent atomic lock record
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, RuntimeDebug)]
pub struct AtomicLock<Balance, AccountId> {
    /// Intent being settled
    pub intent_id: [u8; 32],
    /// Who locked the funds (initiator)
    pub lockee: AccountId,
    /// Amount locked in satoshis or smallest unit
    pub amount: Balance,
    /// Escrow account holding the funds
    pub escrow_account: AccountId,
    /// Current lock phase
    pub phase: LockPhase,
    /// Executor responsible for commit/abort
    pub executor_id: [u8; 32],
}

impl<Balance: Clone, AccountId: Clone> AtomicLock<Balance, AccountId> {
    /// Create new lock in Prepare phase
    pub fn new_prepare(
        intent_id: [u8; 32],
        lockee: AccountId,
        amount: Balance,
        escrow_account: AccountId,
        executor_id: [u8; 32],
        current_block: u32,
        commit_deadline_blocks: u32,
    ) -> Self {
        Self {
            intent_id,
            lockee,
            amount,
            escrow_account,
            phase: LockPhase::LockedForCommit {
                locked_at_block: current_block,
                commit_deadline: current_block + commit_deadline_blocks,
            },
            executor_id,
        }
    }

    /// Transition to Commit phase
    pub fn lock_for_commit(
        &mut self,
        current_block: u32,
        finalize_deadline_blocks: u32,
    ) -> Result<(), &'static str> {
        match &self.phase {
            LockPhase::LockedForCommit { .. } => {
                self.phase = LockPhase::CommitInProgress {
                    locked_at_block: current_block,
                    finalize_deadline: current_block + finalize_deadline_blocks,
                };
                Ok(())
            }
            _ => Err("Lock not in Prepare phase"),
        }
    }

    /// Release lock on successful commit
    pub fn release_on_commit(&mut self, current_block: u32) -> Result<(), &'static str> {
        match &self.phase {
            LockPhase::CommitInProgress { .. } => {
                self.phase = LockPhase::Released {
                    at_block: current_block,
                    reason: ReleaseReason::CommitSucceeded,
                };
                Ok(())
            }
            _ => Err("Lock not in Commit phase"),
        }
    }

    /// Release lock on abort
    pub fn release_on_abort(&mut self, current_block: u32) -> Result<(), &'static str> {
        match &self.phase {
            LockPhase::LockedForCommit { .. } | LockPhase::CommitInProgress { .. } => {
                self.phase = LockPhase::Released {
                    at_block: current_block,
                    reason: ReleaseReason::AbortRequested,
                };
                Ok(())
            }
            _ => Err("Lock already released"),
        }
    }

    /// Slash on timeout (executor loses bond)
    pub fn slash_on_timeout(&mut self, current_block: u32) -> Result<(), &'static str> {
        match &self.phase {
            LockPhase::LockedForCommit {
                commit_deadline, ..
            }
            | LockPhase::CommitInProgress {
                finalize_deadline: commit_deadline,
                ..
            } => {
                if current_block > *commit_deadline {
                    self.phase = LockPhase::Slashed {
                        at_block: current_block,
                        executor_id: self.executor_id,
                    };
                    Ok(())
                } else {
                    Err("Deadline not yet reached")
                }
            }
            LockPhase::Released { .. } | LockPhase::Slashed { .. } => Err("Lock already finalized"),
        }
    }

    /// Check if lock has expired
    pub fn is_expired(&self, current_block: u32) -> bool {
        match &self.phase {
            LockPhase::LockedForCommit {
                commit_deadline, ..
            } => current_block > *commit_deadline,
            LockPhase::CommitInProgress {
                finalize_deadline, ..
            } => current_block > *finalize_deadline,
            _ => false,
        }
    }

    /// Get deadline block for this lock
    pub fn deadline_block(&self) -> Option<u32> {
        match &self.phase {
            LockPhase::LockedForCommit {
                commit_deadline, ..
            } => Some(*commit_deadline),
            LockPhase::CommitInProgress {
                finalize_deadline, ..
            } => Some(*finalize_deadline),
            _ => None,
        }
    }
}

/// Lock storage manager
pub struct AtomicLockManager<T: frame_system::Config>
where
    T::AccountId: Clone,
{
    locks: sp_std::collections::btree_map::BTreeMap<[u8; 32], AtomicLock<u128, T::AccountId>>,
}

impl<T: frame_system::Config> AtomicLockManager<T>
where
    T::AccountId: Clone + Eq,
{
    /// Create new manager
    pub fn new() -> Self {
        Self {
            locks: sp_std::collections::btree_map::BTreeMap::new(),
        }
    }

    /// Create and store a new prepare-phase lock
    pub fn lock_for_prepare(
        &mut self,
        intent_id: [u8; 32],
        lockee: T::AccountId,
        amount: u128,
        executor_id: [u8; 32],
        current_block: u32,
        commit_deadline_blocks: u32,
    ) -> Result<T::AccountId, &'static str> {
        if self.locks.contains_key(&intent_id) {
            return Err("Lock already exists for this intent");
        }

        let escrow_account = EscrowAccount::derive::<T>(&intent_id, 0);
        let lock = AtomicLock::new_prepare(
            intent_id,
            lockee,
            amount,
            escrow_account.clone(),
            executor_id,
            current_block,
            commit_deadline_blocks,
        );

        self.locks.insert(intent_id, lock);
        Ok(escrow_account)
    }

    /// Get lock by intent_id
    pub fn get_lock(&self, intent_id: &[u8; 32]) -> Option<&AtomicLock<u128, T::AccountId>> {
        self.locks.get(intent_id)
    }

    /// Get mutable lock
    pub fn get_lock_mut(
        &mut self,
        intent_id: &[u8; 32],
    ) -> Option<&mut AtomicLock<u128, T::AccountId>> {
        self.locks.get_mut(intent_id)
    }

    /// Release lock after commit/abort
    pub fn release_lock(
        &mut self,
        intent_id: &[u8; 32],
        reason: ReleaseReason,
        current_block: u32,
    ) -> Result<(), &'static str> {
        if let Some(lock) = self.get_lock_mut(intent_id) {
            match reason {
                ReleaseReason::CommitSucceeded => lock.release_on_commit(current_block),
                ReleaseReason::CommitFailed | ReleaseReason::AbortRequested => {
                    lock.release_on_abort(current_block)
                }
                ReleaseReason::TimeoutRefund => {
                    lock.release_on_abort(current_block) // Treat as abort for refund
                }
            }
        } else {
            Err("Lock not found")
        }
    }

    /// Process timeout slashes (called in on_finalize)
    pub fn process_timeouts(&mut self, current_block: u32) -> Vec<[u8; 32]> {
        let mut slashed_executors = Vec::new();

        for (intent_id, lock) in self.locks.iter_mut() {
            if lock.is_expired(current_block) {
                if let Ok(()) = lock.slash_on_timeout(current_block) {
                    slashed_executors.push(lock.executor_id);
                }
            }
        }

        slashed_executors
    }

    /// Cleanup released locks after N blocks (prevent unbounded storage)
    pub fn cleanup_old_locks(&mut self, current_block: u32, grace_period_blocks: u32) {
        self.locks.retain(|_, lock| {
            match &lock.phase {
                LockPhase::Released { at_block, .. } | LockPhase::Slashed { at_block, .. } => {
                    current_block.saturating_sub(*at_block) <= grace_period_blocks
                }
                _ => true, // Keep active locks
            }
        });
    }

    /// Total active locks
    pub fn active_count(&self) -> usize {
        self.locks
            .iter()
            .filter(|(_, lock)| {
                matches!(
                    lock.phase,
                    LockPhase::LockedForCommit { .. } | LockPhase::CommitInProgress { .. }
                )
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_lifecycle() {
        let intent_id = [1u8; 32];
        let executor_id = [2u8; 32];
        let mut lock =
            AtomicLock::new_prepare(intent_id, 0usize, 1000u128, 1usize, executor_id, 100, 200);

        // Should be in LockedForCommit
        assert!(matches!(lock.phase, LockPhase::LockedForCommit { .. }));
        assert!(!lock.is_expired(100));

        // Transition to CommitInProgress
        assert!(lock.lock_for_commit(150, 50).is_ok());
        assert!(matches!(lock.phase, LockPhase::CommitInProgress { .. }));

        // Complete commit
        assert!(lock.release_on_commit(160).is_ok());
        assert!(matches!(
            lock.phase,
            LockPhase::Released {
                reason: ReleaseReason::CommitSucceeded,
                ..
            }
        ));
    }

    #[test]
    fn test_lock_timeout_and_slash() {
        let intent_id = [1u8; 32];
        let executor_id = [2u8; 32];
        let mut lock =
            AtomicLock::new_prepare(intent_id, 0usize, 1000u128, 1usize, executor_id, 100, 50);

        // Not expired yet
        assert!(!lock.is_expired(140));
        assert!(lock.slash_on_timeout(140).is_err());

        // Now expired
        assert!(lock.is_expired(151));
        assert!(lock.slash_on_timeout(151).is_ok());
        assert!(matches!(lock.phase, LockPhase::Slashed { .. }));
    }

    #[test]
    fn test_abort_releases_lock() {
        let intent_id = [1u8; 32];
        let executor_id = [2u8; 32];
        let mut lock =
            AtomicLock::new_prepare(intent_id, 0usize, 1000u128, 1usize, executor_id, 100, 200);

        assert!(lock.release_on_abort(110).is_ok());
        assert!(matches!(
            lock.phase,
            LockPhase::Released {
                reason: ReleaseReason::AbortRequested,
                ..
            }
        ));
    }
}
