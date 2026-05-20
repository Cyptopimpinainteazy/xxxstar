//! X3 Staking Analytics
//! 
//! Comprehensive staking metrics, APY calculations, and validator performance tracking

pub mod staking_ledger;
pub mod reward_calculator;
pub mod validator_stats;
pub mod slash_tracker;
pub mod staking_simulator;

pub use staking_ledger::{StakingLedger, StakingPosition};
pub use reward_calculator::{RewardCalculator, APYCalculation};
pub use validator_stats::{ValidatorStats, ValidatorPerformance};
pub use slash_tracker::{SlashTracker, SlashEvent};
pub use staking_simulator::StakingSimulator;

use serde::{Deserialize, Serialize};

/// Staking analytics error
#[derive(Debug, thiserror::Error)]
pub enum StakingError {
    #[error("Validator not found")]
    ValidatorNotFound,
    
    #[error("Position not found")]
    PositionNotFound,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Invalid calculation: {0}")]
    InvalidCalculation(String),
    
    #[error("Unbonding in progress")]
    UnbondingInProgress,
}

pub type Result<T> = std::result::Result<T, StakingError>;

/// Staking summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingSummary {
    pub total_staked: u128,
    pub total_rewards: u128,
    pub average_apy: f64,
    pub active_positions: u32,
    pub unbonding_positions: u32,
    pub claimable_rewards: u128,
}

/// Version info
pub const VERSION: &str = "1.0.0";
