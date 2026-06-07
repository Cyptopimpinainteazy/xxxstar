//! Canonical supply invariant snapshot.
//!
//! Mirrors the X3 Universal Asset Kernel accounting model. The sum of all
//! tracked balances (native, EVM-side, SVM-side, X3VM-side, externally
//! locked, in-flight) MUST equal the canonical supply at all times.

use serde::{Deserialize, Serialize};

use crate::{OrchestratorError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalSupplySnapshot {
    pub native: u128,
    pub evm: u128,
    pub svm: u128,
    pub x3vm: u128,
    pub external_locked: u128,
    pub pending: u128,
    pub canonical_supply: u128,
}

impl CanonicalSupplySnapshot {
    /// Sum of every tracked compartment.
    pub fn tracked_total(&self) -> u128 {
        self.native
            .saturating_add(self.evm)
            .saturating_add(self.svm)
            .saturating_add(self.x3vm)
            .saturating_add(self.external_locked)
            .saturating_add(self.pending)
    }

    /// Returns `Ok(())` iff the canonical supply equals the tracked total.
    pub fn validate(&self) -> Result<()> {
        if self.tracked_total() != self.canonical_supply {
            return Err(OrchestratorError::InvariantFailed);
        }
        Ok(())
    }
}
