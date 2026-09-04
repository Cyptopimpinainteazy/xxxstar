//! Cross-VM router metering: tracks resource consumption across VM boundaries.
//!
//! Cross-VM calls must account for gas/compute-units consumed in both the source
//! and target VM. This module provides the unified metering view and enforces
//! that the total cost never exceeds the Comit budget.

use frame_support::pallet_prelude::*;

/// Gas consumed on the source VM side.
pub type SourceGas = u64;
/// Gas consumed on the target VM side.
pub type TargetGas = u64;

/// A cross-VM metering record for a single call.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct CrossVmMeteringRecord {
    /// Gas used by the routing/dispatch layer.
    pub routing_overhead: u64,
    /// Gas consumed on the source VM before the cross-VM call.
    pub source_gas_pre_call: SourceGas,
    /// Gas consumed by the target VM.
    pub target_gas_consumed: TargetGas,
    /// Gas consumed on the source VM after the cross-VM call returns.
    pub source_gas_post_call: SourceGas,
}

impl CrossVmMeteringRecord {
    /// Total gas consumed across both VMs plus routing.
    pub fn total(&self) -> u64 {
        self.routing_overhead
            .saturating_add(self.source_gas_pre_call)
            .saturating_add(self.target_gas_consumed)
            .saturating_add(self.source_gas_post_call)
    }
}

/// Errors from metering operations.
#[derive(Debug, PartialEq, Eq)]
pub enum MeteringError {
    /// Cross-VM call would exceed the Comit budget.
    BudgetExceeded { budget: u64, required: u64 },
    /// Metering record is incomplete (target gas not yet set).
    Incomplete,
}

/// A live metering context for a cross-VM call.
pub struct CrossVmMeteringCtx {
    budget: u64,
    record: CrossVmMeteringRecord,
    finalized: bool,
}

impl CrossVmMeteringCtx {
    /// Create a new metering context with the given budget.
    pub fn new(budget: u64) -> Self {
        Self {
            budget,
            record: CrossVmMeteringRecord::default(),
            finalized: false,
        }
    }

    /// Charge routing overhead (dispatch, serialization).
    pub fn charge_routing(&mut self, amount: u64) -> Result<(), MeteringError> {
        let new_total = self.record.total().saturating_add(amount);
        if new_total > self.budget {
            return Err(MeteringError::BudgetExceeded {
                budget: self.budget,
                required: new_total,
            });
        }
        self.record.routing_overhead = self.record.routing_overhead.saturating_add(amount);
        Ok(())
    }

    /// Record pre-call source gas consumption.
    pub fn record_source_pre(&mut self, amount: u64) -> Result<(), MeteringError> {
        let new_total = self.record.total().saturating_add(amount);
        if new_total > self.budget {
            return Err(MeteringError::BudgetExceeded {
                budget: self.budget,
                required: new_total,
            });
        }
        self.record.source_gas_pre_call = self.record.source_gas_pre_call.saturating_add(amount);
        Ok(())
    }

    /// Record target VM gas consumption (received from the target after execution).
    pub fn record_target(&mut self, amount: u64) -> Result<(), MeteringError> {
        let new_total = self.record.total().saturating_add(amount);
        if new_total > self.budget {
            return Err(MeteringError::BudgetExceeded {
                budget: self.budget,
                required: new_total,
            });
        }
        self.record.target_gas_consumed = self.record.target_gas_consumed.saturating_add(amount);
        Ok(())
    }

    /// Record post-call source gas consumption and finalize.
    pub fn finalize(&mut self, post_call_gas: u64) -> Result<CrossVmMeteringRecord, MeteringError> {
        let new_total = self.record.total().saturating_add(post_call_gas);
        if new_total > self.budget {
            return Err(MeteringError::BudgetExceeded {
                budget: self.budget,
                required: new_total,
            });
        }
        self.record.source_gas_post_call = post_call_gas;
        self.finalized = true;
        Ok(self.record.clone())
    }

    /// Remaining budget.
    pub fn remaining(&self) -> u64 {
        self.budget.saturating_sub(self.record.total())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_gas_accounting() {
        let r = CrossVmMeteringRecord {
            routing_overhead: 100,
            source_gas_pre_call: 200,
            target_gas_consumed: 500,
            source_gas_post_call: 50,
        };
        assert_eq!(r.total(), 850);
    }

    #[test]
    fn test_metering_ctx_happy_path() {
        let mut ctx = CrossVmMeteringCtx::new(10_000);
        ctx.charge_routing(100).unwrap();
        ctx.record_source_pre(500).unwrap();
        ctx.record_target(2000).unwrap();
        let record = ctx.finalize(100).unwrap();
        assert_eq!(record.total(), 2700);
        assert_eq!(ctx.remaining(), 7300);
    }

    #[test]
    fn test_budget_exceeded_on_routing() {
        let mut ctx = CrossVmMeteringCtx::new(50);
        assert_eq!(
            ctx.charge_routing(100),
            Err(MeteringError::BudgetExceeded {
                budget: 50,
                required: 100
            })
        );
    }

    #[test]
    fn test_budget_exceeded_on_target() {
        let mut ctx = CrossVmMeteringCtx::new(1000);
        ctx.charge_routing(100).unwrap();
        assert_eq!(
            ctx.record_target(1000),
            Err(MeteringError::BudgetExceeded {
                budget: 1000,
                required: 1100
            })
        );
    }

    #[test]
    fn test_remaining_decreases() {
        let mut ctx = CrossVmMeteringCtx::new(5000);
        assert_eq!(ctx.remaining(), 5000);
        ctx.charge_routing(500).unwrap();
        assert_eq!(ctx.remaining(), 4500);
    }

    #[test]
    fn test_exact_budget_ok() {
        let mut ctx = CrossVmMeteringCtx::new(1000);
        ctx.charge_routing(1000).unwrap();
        assert_eq!(ctx.remaining(), 0);
    }
}
