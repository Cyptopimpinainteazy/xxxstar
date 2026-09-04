//! Canonical supply tracking for X3 assets.
//!
//! Maintains a ledger of total issuance per asset, enforcing that mints and burns
//! are always reflected in canonical supply before any cross-VM execution proceeds.

use frame_support::pallet_prelude::*;

/// Asset identifier type.
pub type AssetId = u32;

/// Unsigned balance.
pub type Balance = u128;

/// Supply record for a single asset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct SupplyRecord {
    /// Total minted, including cross-VM locked amounts.
    pub total_issued: Balance,
    /// Currently circulating (issued minus externally locked).
    pub circulating: Balance,
    /// Amount locked in cross-VM bridge escrow.
    pub bridge_locked: Balance,
    /// Amount held in pending cross-VM transfers.
    pub pending_transfer: Balance,
}

impl SupplyRecord {
    /// Invariant: circulating + bridge_locked + pending_transfer == total_issued.
    pub fn check_invariant(&self) -> bool {
        self.circulating
            .saturating_add(self.bridge_locked)
            .saturating_add(self.pending_transfer)
            == self.total_issued
    }
}

/// Errors returned by supply operations.
#[derive(Debug, PartialEq, Eq)]
pub enum SupplyError {
    /// The asset does not have a registered supply record.
    AssetNotFound,
    /// A mint would overflow the total supply.
    MintOverflow,
    /// A burn would underflow the circulating supply.
    BurnUnderflow,
    /// Supply invariant is violated after the operation.
    InvariantViolation,
}

/// Update the supply record for a mint operation.
///
/// Adds `amount` to both `total_issued` and `circulating`.
pub fn apply_mint(record: &mut SupplyRecord, amount: Balance) -> Result<(), SupplyError> {
    record.total_issued = record
        .total_issued
        .checked_add(amount)
        .ok_or(SupplyError::MintOverflow)?;
    record.circulating = record
        .circulating
        .checked_add(amount)
        .ok_or(SupplyError::MintOverflow)?;
    if !record.check_invariant() {
        return Err(SupplyError::InvariantViolation);
    }
    Ok(())
}

/// Update the supply record for a burn operation.
///
/// Subtracts `amount` from both `total_issued` and `circulating`.
pub fn apply_burn(record: &mut SupplyRecord, amount: Balance) -> Result<(), SupplyError> {
    record.circulating = record
        .circulating
        .checked_sub(amount)
        .ok_or(SupplyError::BurnUnderflow)?;
    record.total_issued = record
        .total_issued
        .checked_sub(amount)
        .ok_or(SupplyError::BurnUnderflow)?;
    if !record.check_invariant() {
        return Err(SupplyError::InvariantViolation);
    }
    Ok(())
}

/// Move `amount` from circulating into bridge_locked (lock for cross-VM transfer).
pub fn lock_for_bridge(record: &mut SupplyRecord, amount: Balance) -> Result<(), SupplyError> {
    record.circulating = record
        .circulating
        .checked_sub(amount)
        .ok_or(SupplyError::BurnUnderflow)?;
    record.bridge_locked = record
        .bridge_locked
        .checked_add(amount)
        .ok_or(SupplyError::MintOverflow)?;
    if !record.check_invariant() {
        return Err(SupplyError::InvariantViolation);
    }
    Ok(())
}

/// Release `amount` from bridge_locked back into circulating (unlock after cross-VM settlement).
pub fn unlock_from_bridge(record: &mut SupplyRecord, amount: Balance) -> Result<(), SupplyError> {
    record.bridge_locked = record
        .bridge_locked
        .checked_sub(amount)
        .ok_or(SupplyError::BurnUnderflow)?;
    record.circulating = record
        .circulating
        .checked_add(amount)
        .ok_or(SupplyError::MintOverflow)?;
    if !record.check_invariant() {
        return Err(SupplyError::InvariantViolation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_record(issued: Balance) -> SupplyRecord {
        SupplyRecord {
            total_issued: issued,
            circulating: issued,
            bridge_locked: 0,
            pending_transfer: 0,
        }
    }

    #[test]
    fn test_mint_increases_supply() {
        let mut r = default_record(1000);
        apply_mint(&mut r, 500).unwrap();
        assert_eq!(r.total_issued, 1500);
        assert_eq!(r.circulating, 1500);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_burn_decreases_supply() {
        let mut r = default_record(1000);
        apply_burn(&mut r, 400).unwrap();
        assert_eq!(r.total_issued, 600);
        assert_eq!(r.circulating, 600);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_burn_underflow_rejected() {
        let mut r = default_record(100);
        assert_eq!(apply_burn(&mut r, 200), Err(SupplyError::BurnUnderflow));
    }

    #[test]
    fn test_lock_for_bridge() {
        let mut r = default_record(1000);
        lock_for_bridge(&mut r, 300).unwrap();
        assert_eq!(r.circulating, 700);
        assert_eq!(r.bridge_locked, 300);
        assert_eq!(r.total_issued, 1000);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_unlock_from_bridge() {
        let mut r = SupplyRecord {
            total_issued: 1000,
            circulating: 700,
            bridge_locked: 300,
            pending_transfer: 0,
        };
        unlock_from_bridge(&mut r, 300).unwrap();
        assert_eq!(r.circulating, 1000);
        assert_eq!(r.bridge_locked, 0);
        assert!(r.check_invariant());
    }

    #[test]
    fn test_invariant_holds_after_chain_of_ops() {
        let mut r = default_record(5000);
        apply_mint(&mut r, 1000).unwrap();
        lock_for_bridge(&mut r, 2000).unwrap();
        apply_burn(&mut r, 500).unwrap();
        unlock_from_bridge(&mut r, 1000).unwrap();
        assert!(r.check_invariant(), "invariant broken: {r:?}");
    }

    #[test]
    fn test_negative_supply_rejected() {
        let mut r = SupplyRecord {
            total_issued: 0,
            circulating: 0,
            bridge_locked: 0,
            pending_transfer: 0,
        };
        assert!(apply_burn(&mut r, 1).is_err());
    }
}
