//! X3 Autonomic CLI
//! 
//! Command-line interface for the X3 Autonomic Core system management.

#![cfg_attr(not(feature = "std"), no_std)]

use x3_autonomic_types::AutonomyLevel;

/// CLI configuration
#[derive(Debug, Clone)]
pub struct CliConfig {
    /// Default autonomy level
    pub default_autonomy: AutonomyLevel,
    /// Enable verbose output
    pub verbose: bool,
    /// RPC endpoint
    pub rpc_endpoint: Vec<u8>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_autonomy: AutonomyLevel::Manual,
            verbose: false,
            rpc_endpoint: b"ws://localhost:9944".to_vec(),
        }
    }
}

/// CLI command types
#[derive(Debug, Clone)]
pub enum Command {
    /// Set autonomy level
    SetAutonomy(AutonomyLevel),
    /// Check health status
    HealthCheck,
    /// Run audit
    RunAudit,
    /// List invariants
    ListInvariants,
    /// View performance metrics
    ViewMetrics,
    /// Generate upgrade proposal
    ProposeUpgrade(Vec<u8>),
}

impl CliConfig {
    /// Create a new CLI config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set autonomy level
    pub fn with_autonomy(mut self, level: AutonomyLevel) -> Self {
        self.default_autonomy = level;
        self
    }

    /// Set verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}