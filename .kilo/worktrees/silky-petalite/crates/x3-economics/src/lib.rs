//! X3 Economic Mechanisms
//!
//! Fee markets, token distribution, governance, staking.

pub mod validator_commission;
pub mod stake_compounding;
pub mod inflation_schedule;

pub use validator_commission::{ValidatorCommissionCap, ValidatorCommission};
pub use stake_compounding::{DelegationConfig, NominatorStake, ValidatorPool};
pub use inflation_schedule::{InflationCurve, InflationSchedule};
