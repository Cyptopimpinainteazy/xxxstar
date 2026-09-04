//! Shared types for X3 Autonomic Core
//!
//! Common data structures, traits, and type definitions used across
//! all autonomic core components.

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

/// 32-byte hash type
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct H256(pub [u8; 32]);

impl Default for H256 {
    fn default() -> Self {
        H256([0u8; 32])
    }
}

/// Autonomy level representing system self-governance capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AutonomyLevel {
    /// Level 0: Fully manual, human-only control
    Manual = 0,
    /// Level 1: Automated monitoring and alerting
    Monitored = 1,
    /// Level 2: Automated detection with human approval
    DetectedHumanApproval = 2,
    /// Level 3: Automated detection with staged rollout
    StagedRollout = 3,
    /// Level 4: Automated detection with canary deployment
    Canary = 4,
    /// Level 5: Fully autonomous self-improvement
    FullyAutonomous = 5,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        AutonomyLevel::Monitored
    }
}

/// Severity level for invariant violations and findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Severity {
    Info = 0,
    Warning = 1,
    Critical = 2,
    Emergency = 3,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Info
    }
}

/// Health status of the X3 runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
    Emergency,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Healthy
    }
}

/// Audit event types
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuditEvent {
    /// Invariant was checked
    InvariantChecked {
        invariant_id: u32,
        passed: bool,
        actual_value: u128,
        expected_range_min: u128,
        expected_range_max: u128,
    },
    /// Health metric updated
    HealthMetricUpdated {
        metric_id: Vec<u8>,
        value: f64,
        threshold: f64,
    },
    /// Block shadow execution completed
    ShadowExecutionCompleted {
        block_hash: H256,
        matches_production: bool,
        execution_time_ms: u64,
    },
    /// Regression test generated
    RegressionTestGenerated {
        test_name: Vec<u8>,
        failure_description: Vec<u8>,
        block_hash: H256,
    },
    /// Upgrade proposal created
    UpgradeProposed {
        proposal_id: H256,
        autonomy_level: AutonomyLevel,
        description: Vec<u8>,
    },
}

/// Result of a shadow execution comparison
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ShadowExecutionResult {
    pub block_hash: Vec<u8>,
    pub execution_time_ms: u64,
    pub state_root_matches: bool,
    pub events: Vec<Vec<u8>>,
    pub errors: Vec<Vec<u8>>,
}

/// Performance benchmark metrics
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct PerformanceMetrics {
    pub block_time_avg_ms: f64,
    pub block_time_p95_ms: f64,
    pub block_time_p99_ms: f64,
    pub tx_throughput_per_block: f64,
    pub storage_read_avg_us: f64,
    pub storage_write_avg_us: f64,
    pub vm_execution_evm_ms: f64,
    pub vm_execution_svm_ms: f64,
    pub memory_usage_mb: f64,
}

/// Upgrade proposal for governance
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct UpgradeProposal {
    pub id: Vec<u8>,
    pub description: Vec<u8>,
    pub proposed_by: Vec<u8>,
    pub autonomy_level: AutonomyLevel,
    pub target_block: Option<u32>,
    pub code_hash: Option<Vec<u8>>,
    pub canary_percentage: u8,
    pub created_at: u64,
    pub status: ProposalStatus,
    pub severity: Severity,
}

/// Status of an upgrade proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Staged { current_percentage: u8 },
    RolledBack,
    Executed,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        ProposalStatus::Pending
    }
}

/// Invariant definition
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct InvariantDefinition {
    pub id: u32,
    pub name: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    pub check_interval_blocks: u32,
    pub enabled: bool,
}

/// Health metric definition
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HealthMetricDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub check_interval_blocks: u32,
    pub enabled: bool,
}

/// Predefined invariant IDs
pub mod invariants {
    use super::*;

    pub const INVARIANT_TOTAL_SUPPLY: u32 = 1;
    pub const INVARIANT_BALANCE_NON_NEGATIVE: u32 = 2;
    pub const INVARIANT_STAKING_REWARDS_CAPPED: u32 = 3;
    pub const INVARIANT_GOVERNANCE_QUORUM: u32 = 4;
    pub const INVARIANT_CROSS_VM_STATE_CONSISTENCY: u32 = 5;
    pub const INVARIANT_BRIDGE_ESCROW_BALANCE: u32 = 6;
    pub const INVARIANT_DEX_RESERVES_CONSISTENCY: u32 = 7;
    pub const INVARIANT_NFT_TOTAL_SUPPLY: u32 = 8;
    pub const INVARIANT_FEE_BALANCE_NON_NEGATIVE: u32 = 9;
    pub const INVARIANT_AUTHORITY_SET_SIZE: u32 = 10;
    pub const INVARIANT_SCHEDULED_QUEUE_ORDER: u32 = 11;
    pub const INVARIANT_BLOCK_AUTHOR_REWARD: u32 = 12;
}

/// Predefined health metric IDs
pub mod health_metrics {
    pub const METRIC_BLOCK_TIME: &str = "block_time";
    pub const METRIC_TX_THROUGHPUT: &str = "tx_throughput";
    pub const METRIC_STORAGE_GROWTH: &str = "storage_growth";
    pub const METRIC_MEMORY_USAGE: &str = "memory_usage";
    pub const METRIC_PEER_COUNT: &str = "peer_count";
    pub const METRIC_SYNC_LAG: &str = "sync_lag";
    pub const METRIC_INVARIANT_VIOLATIONS: &str = "invariant_violations";
    pub const METRIC_UPGRADE_SUCCESS_RATE: &str = "upgrade_success_rate";
}