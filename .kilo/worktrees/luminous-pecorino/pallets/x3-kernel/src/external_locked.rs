//! External locked accounting: tracks assets locked in external bridges/protocols.
//!
//! Assets locked in external systems (wrapped, bridged out) must be accounted for
//! separately from circulating supply. This module enforces that external lock
//! and unlock operations are always balanced.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;
pub type Balance = u128;

/// Record of externally locked assets for a single asset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct ExternalLockRecord {
    /// Total amount currently locked in external protocols.
    pub total_locked: Balance,
    /// Total amount historically locked (for audit).
    pub cumulative_locked: Balance,
    /// Total amount historically unlocked (for audit).
    pub cumulative_unlocked: Balance,
}

impl ExternalLockRecord {
    /// Invariant: cumulative_unlocked <= cumulative_locked.
    pub fn check_invariant(&self) -> bool {
        self.cumulative_unlocked <= self.cumulative_locked
            && self.total_locked
                == self
                    .cumulative_locked
                    .saturating_sub(self.cumulative_unlocked)
    }
}

/// Errors returned by external locked accounting operations.
#[derive(Debug, PartialEq, Eq)]
pub enum ExternalLockError {
    /// Lock amount is zero.
    ZeroAmount,
    /// Unlock would exceed current locked balance.
    UnlockExceedsLocked,
    /// Arithmetic overflow during lock accumulation.
    Overflow,
}

/// Apply an external lock (asset is being sent to an external protocol).
pub fn apply_lock(
    record: &mut ExternalLockRecord,
    amount: Balance,
) -> Result<(), ExternalLockError> {
    if amount == 0 {
        return Err(ExternalLockError::ZeroAmount);
    }
    record.total_locked = record
        .total_locked
        .checked_add(amount)
        .ok_or(ExternalLockError::Overflow)?;
    record.cumulative_locked = record
        .cumulative_locked
        .checked_add(amount)
        .ok_or(ExternalLockError::Overflow)?;
    debug_assert!(record.check_invariant());
    Ok(())
}

/// Apply an external unlock (asset is returning from an external protocol).
pub fn apply_unlock(
    record: &mut ExternalLockRecord,
    amount: Balance,
) -> Result<(), ExternalLockError> {
    if amount == 0 {
        return Err(ExternalLockError::ZeroAmount);
    }
    record.total_locked = record
        .total_locked
        .checked_sub(amount)
        .ok_or(ExternalLockError::UnlockExceedsLocked)?;
    record.cumulative_unlocked = record
        .cumulative_unlocked
        .checked_add(amount)
        .ok_or(ExternalLockError::Overflow)?;
    debug_assert!(record.check_invariant());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_and_unlock() {
        let mut r = ExternalLockRecord::default();
        apply_lock(&mut r, 1000).unwrap();
        assert_eq!(r.total_locked, 1000);
        apply_unlock(&mut r, 400).unwrap();
        assert_eq!(r.total_locked, 600);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_unlock_exceeds_locked_rejected() {
        let mut r = ExternalLockRecord::default();
        apply_lock(&mut r, 100).unwrap();
        assert_eq!(
            apply_unlock(&mut r, 200),
            Err(ExternalLockError::UnlockExceedsLocked)
        );
    }

    #[test]
    fn test_zero_lock_rejected() {
        let mut r = ExternalLockRecord::default();
        assert_eq!(apply_lock(&mut r, 0), Err(ExternalLockError::ZeroAmount));
    }

    #[test]
    fn test_zero_unlock_rejected() {
        let mut r = ExternalLockRecord::default();
        apply_lock(&mut r, 100).unwrap();
        assert_eq!(apply_unlock(&mut r, 0), Err(ExternalLockError::ZeroAmount));
    }

    #[test]
    fn test_cumulative_tracking() {
        let mut r = ExternalLockRecord::default();
        apply_lock(&mut r, 500).unwrap();
        apply_lock(&mut r, 500).unwrap();
        apply_unlock(&mut r, 300).unwrap();
        assert_eq!(r.cumulative_locked, 1000);
        assert_eq!(r.cumulative_unlocked, 300);
        assert_eq!(r.total_locked, 700);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_full_unlock_returns_to_zero() {
        let mut r = ExternalLockRecord::default();
        apply_lock(&mut r, 888).unwrap();
        apply_unlock(&mut r, 888).unwrap();
        assert_eq!(r.total_locked, 0);
        assert!(r.check_invariant());
    }
}
