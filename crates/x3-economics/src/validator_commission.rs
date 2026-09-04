//! Validator commission capping and governance
//!
//! Prevents validators from extracting excessive fees. Governance can adjust
//! the max commission cap (default 20%).

use sp_runtime::Permill;

/// Validator commission cap enforcement
#[derive(Clone, Debug)]
pub struct ValidatorCommissionCap {
    /// Maximum commission (parts per million)
    pub max_commission: Permill,
    /// Current active cap (may differ if governance changed it)
    pub active_cap: Permill,
    /// History of cap changes
    pub cap_history: Vec<(u32, Permill)>, // (block_height, cap)
}

impl ValidatorCommissionCap {
    /// Create new cap controller with 20% default
    pub fn new() -> Self {
        Self {
            max_commission: Permill::from_percent(20),
            active_cap: Permill::from_percent(20),
            cap_history: vec![(0, Permill::from_percent(20))],
        }
    }

    /// Check if proposed commission is valid
    pub fn is_valid_commission(&self, proposed: Permill) -> bool {
        proposed <= self.active_cap
    }

    /// Governance action: update max commission cap
    pub fn set_cap(&mut self, new_cap: Permill, block_height: u32) -> bool {
        // Cap itself capped at 50% to prevent governance abuse
        if new_cap > Permill::from_percent(50) {
            return false;
        }

        // If new cap is lower than some existing validators' commissions,
        // they'll need to reduce on next epoch
        self.active_cap = new_cap;
        self.cap_history.push((block_height, new_cap));

        true
    }

    /// Get the effective cap at a given height
    pub fn cap_at_height(&self, block_height: u32) -> Permill {
        self.cap_history
            .iter()
            .rev()
            .find(|(h, _)| *h <= block_height)
            .map(|(_, cap)| *cap)
            .unwrap_or(Permill::from_percent(20))
    }
}

/// Per-validator commission tracking
#[derive(Clone, Debug)]
pub struct ValidatorCommission {
    /// Validator account ID (as string for simplicity)
    pub validator: String,
    /// Claimed commission percentage
    pub commission: Permill,
    /// Is this commission capped?
    pub is_capped: bool,
    /// Last epoch commission was adjusted
    pub last_adjusted_epoch: u32,
}

impl ValidatorCommission {
    /// Create validator commission entry
    pub fn new(validator: String, commission: Permill) -> Self {
        Self {
            validator,
            commission,
            is_capped: false,
            last_adjusted_epoch: 0,
        }
    }

    /// Enforce cap: reduce validator commission if exceeds cap
    pub fn enforce_cap(&mut self, cap: Permill, current_epoch: u32) -> bool {
        if self.commission > cap {
            self.commission = cap;
            self.is_capped = true;
            self.last_adjusted_epoch = current_epoch;
            true // Commission was reduced
        } else {
            self.is_capped = false;
            false // No change needed
        }
    }

    /// Calculate validator earnings from commission pool
    pub fn calculate_take(&self, pool: u128) -> u128 {
        let percentage = self.commission.deconstruct() as u128;
        (pool * percentage) / 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commission_cap_creation() {
        let cap = ValidatorCommissionCap::new();
        assert_eq!(cap.active_cap, Permill::from_percent(20));
    }

    #[test]
    fn test_commission_validation() {
        let cap = ValidatorCommissionCap::new();

        // Valid commissions
        assert!(cap.is_valid_commission(Permill::from_percent(5)));
        assert!(cap.is_valid_commission(Permill::from_percent(20)));

        // Invalid (exceeds cap)
        assert!(!cap.is_valid_commission(Permill::from_percent(25)));
    }

    #[test]
    fn test_governance_cap_update() {
        let mut cap = ValidatorCommissionCap::new();

        // Governance lowers cap to 15%
        assert!(cap.set_cap(Permill::from_percent(15), 100));
        assert_eq!(cap.active_cap, Permill::from_percent(15));

        // New validators must comply
        assert!(!cap.is_valid_commission(Permill::from_percent(20)));
    }

    #[test]
    fn test_cap_prevents_abuse() {
        let mut cap = ValidatorCommissionCap::new();

        // Governance cannot set cap above 50% (circuit breaker)
        assert!(!cap.set_cap(Permill::from_percent(60), 100));
        assert_eq!(cap.active_cap, Permill::from_percent(20));
    }

    #[test]
    fn test_validator_commission_enforcement() {
        let mut validator = ValidatorCommission::new("alice".to_string(), Permill::from_percent(25));
        let cap = Permill::from_percent(20);

        // Enforce cap: commission reduced
        assert!(validator.enforce_cap(cap, 1));
        assert_eq!(validator.commission, Permill::from_percent(20));
        assert!(validator.is_capped);
    }

    #[test]
    fn test_validator_commission_take_calculation() {
        let validator = ValidatorCommission::new("bob".to_string(), Permill::from_percent(10));

        // Commission take from 1000 unit pool
        let take = validator.calculate_take(1000);
        assert_eq!(take, 100); // 10% of 1000
    }

    #[test]
    fn test_cap_history_tracking() {
        let mut cap = ValidatorCommissionCap::new();

        cap.set_cap(Permill::from_percent(15), 100);
        cap.set_cap(Permill::from_percent(10), 200);

        // Query historical cap
        assert_eq!(cap.cap_at_height(50), Permill::from_percent(20));
        assert_eq!(cap.cap_at_height(150), Permill::from_percent(15));
        assert_eq!(cap.cap_at_height(250), Permill::from_percent(10));
    }
}
