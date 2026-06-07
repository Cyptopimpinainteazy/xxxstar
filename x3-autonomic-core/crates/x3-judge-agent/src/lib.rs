//! X3 Judge Agent
//! 
//! AI judge agent that evaluates upgrade proposals and makes autonomous
//! decisions about system improvements for the X3 Autonomic Core.

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use x3_autonomic_types::{AutonomyLevel, HealthStatus, Severity, UpgradeProposal};

/// Configuration for the judge agent
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct JudgeConfig {
    /// Minimum approval threshold (0.0 - 1.0)
    pub approval_threshold: f64,
    /// Enable autonomous decision making
    pub autonomous_enabled: bool,
    /// Maximum proposal age in blocks
    pub max_proposal_age: u64,
    /// Require unanimous consent for critical upgrades
    pub require_unanimous_critical: bool,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 0.7,
            autonomous_enabled: false,
            max_proposal_age: 10080, // ~1 week at 6s blocks
            require_unanimous_critical: true,
        }
    }
}

/// Decision made by the judge agent
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum JudgeDecision {
    /// Proposal approved for implementation
    Approved,
    /// Proposal rejected
    Rejected,
    /// More information needed
    NeedsMoreInfo,
    /// Deferred for later review
    Deferred,
    /// Proposal requires human review
    RequiresHumanReview,
}

/// Justification for a judge decision
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct DecisionJustification {
    /// Decision made
    pub decision: JudgeDecision,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Reasoning summary
    pub reasoning: Vec<u8>,
    /// Supporting evidence
    pub evidence: Vec<Vec<u8>>,
}

/// Judge agent for upgrade decisions
pub struct JudgeAgent {
    config: JudgeConfig,
    current_autonomy_level: AutonomyLevel,
}

impl JudgeAgent {
    /// Create a new judge agent
    pub fn new(config: JudgeConfig) -> Self {
        Self {
            config,
            current_autonomy_level: AutonomyLevel::Manual,
        }
    }

    /// Evaluate an upgrade proposal
    pub fn evaluate(&self, proposal: &UpgradeProposal) -> DecisionJustification {
        // Simplified evaluation - in production this would use AI/ML
        let decision = if self.can_auto_decide() {
            JudgeDecision::Approved
        } else {
            JudgeDecision::RequiresHumanReview
        };

        DecisionJustification {
            decision,
            confidence: 0.85,
            reasoning: b"Simplified evaluation".to_vec(),
            evidence: vec![],
        }
    }

    /// Set the autonomy level
    pub fn set_autonomy_level(&mut self, level: AutonomyLevel) {
        self.current_autonomy_level = level;
    }

    /// Get current autonomy level
    pub fn autonomy_level(&self) -> AutonomyLevel {
        self.current_autonomy_level
    }

    /// Check if autonomous decisions are enabled
    pub fn can_auto_decide(&self) -> bool {
        self.config.autonomous_enabled && matches!(
            self.current_autonomy_level,
            AutonomyLevel::Automatic(_) | AutonomyLevel::SelfImproving | AutonomyLevel::SelfGoverning
        )
    }

    /// Check if human review is required
    pub fn requires_human_review(&self, proposal: &UpgradeProposal) -> bool {
        proposal.severity == Severity::Critical && self.config.require_unanimous_critical
    }
}

/// Health check for judge agent
pub fn health_check() -> HealthStatus {
    HealthStatus::Healthy
}