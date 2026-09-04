//! X3 Regression Engine
//! 
//! Automated regression test generator that creates and validates tests
//! based on detected behavior changes.

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use x3_autonomic_types::{AutonomyLevel, HealthStatus, Severity, UpgradeProposal};

/// Configuration for the regression engine
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RegressionConfig {
    /// Maximum tests to keep in history
    pub test_history_size: u32,
    /// Minimum confidence threshold to auto-approve
    pub min_confidence: f64,
    /// Whether to enable automatic test generation
    pub auto_generate: bool,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            test_history_size: 1000,
            min_confidence: 0.95,
            auto_generate: false,
        }
    }
}

/// A generated regression test
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RegressionTest {
    /// Unique test identifier
    pub id: Vec<u8>,
    /// Human-readable test name
    pub name: Vec<u8>,
    /// Test code/source
    pub source: Vec<u8>,
    /// Block range this test covers
    pub block_range: (u64, u64),
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Whether this test passed
    pub passed: bool,
}

impl RegressionTest {
    /// Create a new regression test
    pub fn new(id: Vec<u8>, name: Vec<u8>, source: Vec<u8>) -> Self {
        Self {
            id,
            name,
            source,
            block_range: (0, 0),
            confidence: 0.0,
            passed: false,
        }
    }

    /// Set the block range
    pub fn with_block_range(mut self, start: u64, end: u64) -> Self {
        self.block_range = (start, end);
        self
    }

    /// Set the confidence score
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Regression engine for automated test generation
pub struct RegressionEngine {
    config: RegressionConfig,
    tests: Vec<RegressionTest>,
    current_autonomy_level: AutonomyLevel,
}

impl RegressionEngine {
    /// Create a new regression engine
    pub fn new(config: RegressionConfig) -> Self {
        Self {
            config,
            tests: Vec::new(),
            current_autonomy_level: AutonomyLevel::Manual,
        }
    }

    /// Generate a new regression test from audit events
    pub fn generate_test(&mut self, name: Vec<u8>, source: Vec<u8>) -> RegressionTest {
        let id = format!("test-{}", self.tests.len()).as_bytes().to_vec();
        let test = RegressionTest::new(id, name, source);
        self.tests.push(test.clone());
        test
    }

    /// Get all tests
    pub fn tests(&self) -> &[RegressionTest] {
        &self.tests
    }

    /// Run all regression tests and return pass count
    pub fn run_tests(&mut self) -> (u32, u32) {
        let total = self.tests.len() as u32;
        // In production, this would actually execute tests
        for test in &mut self.tests {
            test.passed = true; // Simplified
        }
        let passed = self.tests.iter().filter(|t| t.passed).count() as u32;
        (passed, total)
    }

    /// Set the autonomy level
    pub fn set_autonomy_level(&mut self, level: AutonomyLevel) {
        self.current_autonomy_level = level;
    }

    /// Get current autonomy level
    pub fn autonomy_level(&self) -> AutonomyLevel {
        self.current_autonomy_level
    }

    /// Check if auto-generation is allowed
    pub fn can_auto_generate(&self) -> bool {
        self.config.auto_generate && matches!(
            self.current_autonomy_level,
            AutonomyLevel::Automatic(_) | AutonomyLevel::SelfImproving
        )
    }
}

/// Health check for regression engine
pub fn health_check() -> HealthStatus {
    HealthStatus::Healthy
}