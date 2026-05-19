//! Gas metering for X3VM execution.
//!
//! Gas is charged per instruction before execution. If the gas limit is exceeded,
//! execution halts with `GasExhausted`. All arithmetic is saturating to prevent
//! integer overflow in gas accounting.

/// Gas cost per instruction category.
pub mod cost {
    /// Simple arithmetic or nop.
    pub const TRIVIAL: u64 = 1;
    /// Memory read/write.
    pub const MEMORY: u64 = 3;
    /// Jump or branch.
    pub const JUMP: u64 = 2;
    /// Function call setup.
    pub const CALL: u64 = 10;
    /// Atomic window boundary.
    pub const ATOMIC: u64 = 5;
    /// Hash computation.
    pub const HASH: u64 = 50;
    /// Cross-VM hostcall.
    pub const HOSTCALL: u64 = 100;
}

/// Gas meter tracking remaining and consumed gas for a single execution.
#[derive(Clone, Debug)]
pub struct GasMeter {
    limit: u64,
    consumed: u64,
}

/// Errors from gas metering.
#[derive(Debug, PartialEq, Eq)]
pub enum GasError {
    /// Execution exceeded the gas limit.
    GasExhausted { limit: u64, required: u64 },
}

impl GasMeter {
    /// Create a new meter with the given limit.
    pub fn new(limit: u64) -> Self {
        Self { limit, consumed: 0 }
    }

    /// Charge `amount` gas units. Returns Err if this would exceed the limit.
    pub fn charge(&mut self, amount: u64) -> Result<(), GasError> {
        let new_consumed = self.consumed.saturating_add(amount);
        if new_consumed > self.limit {
            return Err(GasError::GasExhausted {
                limit: self.limit,
                required: new_consumed,
            });
        }
        self.consumed = new_consumed;
        Ok(())
    }

    /// Gas remaining before the limit is hit.
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    /// Total gas consumed so far.
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Refund up to `amount` gas (used for SSTORE refunds etc.).
    pub fn refund(&mut self, amount: u64) {
        self.consumed = self.consumed.saturating_sub(amount);
    }

    /// Whether gas has been exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.consumed >= self.limit
    }
}

/// Estimate gas for a bytecode module based on instruction counts.
/// Returns a conservative upper bound.
pub fn estimate_gas(instruction_count: usize, hostcall_count: usize) -> u64 {
    let base = (instruction_count as u64).saturating_mul(cost::TRIVIAL + cost::JUMP);
    let hostcalls = (hostcall_count as u64).saturating_mul(cost::HOSTCALL);
    base.saturating_add(hostcalls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_charge() {
        let mut m = GasMeter::new(100);
        m.charge(30).unwrap();
        assert_eq!(m.consumed(), 30);
        assert_eq!(m.remaining(), 70);
    }

    #[test]
    fn test_exhaustion() {
        let mut m = GasMeter::new(50);
        m.charge(40).unwrap();
        assert_eq!(
            m.charge(20),
            Err(GasError::GasExhausted {
                limit: 50,
                required: 60
            })
        );
    }

    #[test]
    fn test_exact_limit_ok() {
        let mut m = GasMeter::new(100);
        m.charge(100).unwrap();
        assert_eq!(m.remaining(), 0);
        assert!(m.is_exhausted());
    }

    #[test]
    fn test_refund() {
        let mut m = GasMeter::new(100);
        m.charge(80).unwrap();
        m.refund(30);
        assert_eq!(m.consumed(), 50);
        assert_eq!(m.remaining(), 50);
    }

    #[test]
    fn test_refund_does_not_go_negative() {
        let mut m = GasMeter::new(100);
        m.charge(10).unwrap();
        m.refund(50); // more than consumed
        assert_eq!(m.consumed(), 0);
    }

    #[test]
    fn test_estimate_gas_non_zero() {
        let est = estimate_gas(1000, 5);
        assert!(est > 0);
    }

    #[test]
    fn test_zero_limit_exhausted_immediately() {
        let mut m = GasMeter::new(0);
        assert!(m.is_exhausted());
        assert!(m.charge(1).is_err());
    }
}
