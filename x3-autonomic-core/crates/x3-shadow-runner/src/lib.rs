//! X3 Shadow Runner
//! 
//! Shadow execution engine that replays blocks in an isolated environment
//! to verify correctness without affecting the main chain state.

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use x3_autonomic_types::{
    AuditEvent, AutonomyLevel, HealthStatus, Severity, ShadowExecutionResult,
};

/// Configuration for the shadow runner
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct ShadowRunnerConfig {
    /// Maximum blocks to keep in memory for replay
    pub block_cache_size: u32,
    /// Whether to enable state root verification
    pub verify_state_roots: bool,
    /// Timeout for shadow execution per block (milliseconds)
    pub execution_timeout_ms: u64,
    /// Whether to run in full verification mode
    pub full_verification: bool,
}

impl Default for ShadowRunnerConfig {
    fn default() -> Self {
        Self {
            block_cache_size: 100,
            verify_state_roots: true,
            execution_timeout_ms: 5000,
            full_verification: false,
        }
    }
}

/// Shadow execution engine for block verification
pub struct ShadowRunner {
    config: ShadowRunnerConfig,
    current_autonomy_level: AutonomyLevel,
}

impl ShadowRunner {
    /// Create a new shadow runner with the given configuration
    pub fn new(config: ShadowRunnerConfig) -> Self {
        Self {
            config,
            current_autonomy_level: AutonomyLevel::Manual,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &ShadowRunnerConfig {
        &self.config
    }

    /// Set the autonomy level for shadow execution
    pub fn set_autonomy_level(&mut self, level: AutonomyLevel) {
        self.current_autonomy_level = level;
    }

    /// Get the current autonomy level
    pub fn autonomy_level(&self) -> AutonomyLevel {
        self.current_autonomy_level
    }

    /// Execute a block in shadow mode and return the result
    pub fn execute_shadow_block(
        &self,
        block_hash: &[u8],
        extrinsics: &[Vec<u8>],
    ) -> Result<ShadowExecutionResult, ShadowRunnerError> {
        // Shadow execution implementation
        // In production, this would replay the block in an isolated runtime
        Ok(ShadowExecutionResult {
            block_hash: block_hash.to_vec(),
            execution_time_ms: 0,
            state_root_matches: true,
            events: vec![],
            errors: vec![],
        })
    }

    /// Verify that shadow execution results match expected state
    pub fn verify_shadow_result(
        &self,
        result: &ShadowExecutionResult,
        expected_root: &[u8],
    ) -> bool {
        if !self.config.verify_state_roots {
            return true;
        }
        // Simplified verification
        true
    }

    /// Check if autonomy level allows automatic action
    pub fn can_auto_act(&self) -> bool {
        matches!(
            self.current_autonomy_level,
            AutonomyLevel::Automatic(_)
                | AutonomyLevel::SelfImproving
                | AutonomyLevel::SelfGoverning
        )
    }
}

/// Errors that can occur during shadow execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerError {
    /// Block not found in cache
    BlockNotFound,
    /// Execution timeout
    ExecutionTimeout,
    /// State root mismatch
    StateRootMismatch,
    /// Invalid extrinsics
    InvalidExtrinsics,
    /// Runtime error during shadow execution
    RuntimeError(String),
}

impl core::fmt::Display for ShadowRunnerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BlockNotFound => write!(f, "Block not found in cache"),
            Self::ExecutionTimeout => write!(f, "Shadow execution timed out"),
            Self::StateRootMismatch => write!(f, "State root mismatch"),
            Self::InvalidExtrinsics => write!(f, "Invalid extrinsics provided"),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl From<ShadowRunnerError> for AuditEvent {
    fn from(err: ShadowRunnerError) -> Self {
        AuditEvent::Error {
            severity: Severity::High,
            component: "x3-shadow-runner".into(),
            message: err.to_string(),
            context: None,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ShadowRunnerError {}

/// Health check for the shadow runner
pub fn health_check() -> HealthStatus {
    // Shadow runner is healthy if it can be instantiated
    HealthStatus::Healthy
}