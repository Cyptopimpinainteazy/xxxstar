//! Supply invariant guard: enforces and audits the global supply conservation rule.
//!
//! The core invariant: for every asset,
//!   circulating + bridge_locked + pending_transfer + external_locked == total_issued
//!
//! This module provides the checker function and audit trail used by tests and
//! on-chain verification hooks to assert supply conservation at all times.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;
pub type Balance = u128;

/// Snapshot of all supply components for a single asset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct SupplySnapshot {
    pub asset_id: AssetId,
    pub total_issued: Balance,
    pub circulating: Balance,
    pub bridge_locked: Balance,
    pub pending_transfer: Balance,
    pub external_locked: Balance,
}

/// Result of an invariant check.
#[derive(Debug, PartialEq, Eq)]
pub enum InvariantResult {
    /// Invariant holds: all components sum to total_issued.
    Ok,
    /// Invariant violated: the delta (expected - actual sum).
    Violated {
        asset_id: AssetId,
        expected: Balance,
        actual_sum: Balance,
    },
}

/// Check the supply conservation invariant for a single asset snapshot.
pub fn check_supply_invariant(snap: &SupplySnapshot) -> InvariantResult {
    let actual_sum = snap
        .circulating
        .saturating_add(snap.bridge_locked)
        .saturating_add(snap.pending_transfer)
        .saturating_add(snap.external_locked);
    if actual_sum == snap.total_issued {
        InvariantResult::Ok
    } else {
        InvariantResult::Violated {
            asset_id: snap.asset_id,
            expected: snap.total_issued,
            actual_sum,
        }
    }
}

/// Check invariants across a batch of snapshots. Returns Err on first violation.
pub fn check_all(snapshots: &[SupplySnapshot]) -> Result<(), InvariantResult> {
    for snap in snapshots {
        match check_supply_invariant(snap) {
            InvariantResult::Ok => {}
            v => return Err(v),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(
        asset_id: AssetId,
        total_issued: Balance,
        circulating: Balance,
        bridge_locked: Balance,
        pending_transfer: Balance,
        external_locked: Balance,
    ) -> SupplySnapshot {
        SupplySnapshot {
            asset_id,
            total_issued,
            circulating,
            bridge_locked,
            pending_transfer,
            external_locked,
        }
    }

    #[test]
    fn test_invariant_holds_when_sum_matches() {
        let s = snap(1, 1000, 600, 200, 100, 100);
        assert_eq!(check_supply_invariant(&s), InvariantResult::Ok);
    }

    #[test]
    fn test_invariant_violated_when_sum_too_low() {
        let s = snap(1, 1000, 500, 200, 100, 100); // sum = 900
        assert_eq!(
            check_supply_invariant(&s),
            InvariantResult::Violated {
                asset_id: 1,
                expected: 1000,
                actual_sum: 900
            }
        );
    }

    #[test]
    fn test_invariant_violated_when_sum_too_high() {
        let s = snap(1, 1000, 700, 200, 100, 100); // sum = 1100
        assert_eq!(
            check_supply_invariant(&s),
            InvariantResult::Violated {
                asset_id: 1,
                expected: 1000,
                actual_sum: 1100
            }
        );
    }

    #[test]
    fn test_check_all_passes_clean_batch() {
        let snaps = vec![
            snap(1, 500, 500, 0, 0, 0),
            snap(2, 1000, 400, 300, 200, 100),
        ];
        assert!(check_all(&snaps).is_ok());
    }

    #[test]
    fn test_check_all_fails_on_violation() {
        let snaps = vec![
            snap(1, 500, 500, 0, 0, 0),
            snap(2, 1000, 400, 300, 0, 0), // sum = 700, expected 1000
        ];
        assert!(check_all(&snaps).is_err());
    }

    #[test]
    fn test_zero_supply_invariant() {
        let s = snap(99, 0, 0, 0, 0, 0);
        assert_eq!(check_supply_invariant(&s), InvariantResult::Ok);
    }

    #[test]
    fn test_all_in_external_locked() {
        let s = snap(5, 10_000, 0, 0, 0, 10_000);
        assert_eq!(check_supply_invariant(&s), InvariantResult::Ok);
    }
}
