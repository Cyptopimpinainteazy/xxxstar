//! # X3 Chain Onboarding — Phase 11 Weekly Proving Cycle
//!
//! This crate defines the **canonical types** for the X3 chain onboarding pipeline
//! used by the Phase 11 weekly proving cycle. Every external chain considered for
//! integration with X3 must pass through all four scoring phases before approval.
//!
//! ## Scoring Model
//!
//! Each [`ChainOnboardingRecord`] carries four independent sub-scores (0–100) and a
//! [`ChainOnboardingRecord::compute_composite`] method that produces the weighted
//! average:
//!
//! | Dimension       | Weight |
//! |-----------------|--------|
//! | Technical proof | 30 %   |
//! | Economic score  | 30 %   |
//! | Liquidity       | 25 %   |
//! | Compliance      | 15 %   |
//!
//! Approval requires `composite >= 70`, `compliance >= 80`, and `technical >= 60`.
//!
//! ## `no_std` Compatibility
//!
//! This crate is `no_std` when the `std` feature is disabled and is safe to use
//! inside WASM runtimes or sidecar services compiled for embedded targets.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[cfg(test)]
mod tests;

// ─── Onboarding phase ────────────────────────────────────────────────────────

/// The current phase of a chain's onboarding journey.
///
/// Chains advance linearly through the technical, economic, liquidity, and
/// compliance phases before reaching a terminal state of [`OnboardingPhase::Approved`]
/// or [`OnboardingPhase::Rejected`].
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OnboardingPhase {
    /// Phase 0: Technical infrastructure and security proof under review.
    TechnicalProof = 0,
    /// Phase 1: Economic viability and tokenomics analysis.
    EconomicScore = 1,
    /// Phase 2: Liquidity depth and market quality assessment.
    LiquidityOpportunity = 2,
    /// Phase 3: Regulatory and compliance review.
    ComplianceReview = 3,
    /// Terminal — chain has passed all phases and is approved for integration.
    Approved = 4,
    /// Terminal — chain did not meet one or more thresholds and is rejected.
    Rejected = 5,
}

// ─── Chain risk tier ─────────────────────────────────────────────────────────

/// Risk classification assigned to a chain after scoring.
///
/// The risk tier influences liquidity limits, insurance reserve requirements,
/// and the frequency of ongoing review cycles.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum ChainRisk {
    /// Composite score ≥ 85 and no compliance red flags.
    Low = 0,
    /// Composite score 70–84 or minor compliance findings.
    Medium = 1,
    /// Composite score 50–69 or material compliance findings.
    High = 2,
    /// Composite score < 50 or critical compliance failure.
    Critical = 3,
}

// ─── Chain onboarding record ─────────────────────────────────────────────────

/// A snapshot of a single chain's onboarding state for a given ISO week.
///
/// Each field is bounded to 0–100 for the four sub-scores. The `composite_score`
/// field must be populated by the caller using [`Self::compute_composite`]; it is
/// stored separately so that it can be persisted and queried without recomputation.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ChainOnboardingRecord {
    /// CAIP-2 numeric chain identifier.
    pub chain_id: u32,
    /// Current onboarding phase.
    pub phase: OnboardingPhase,
    /// Technical infrastructure and security score (0–100).
    pub technical_score: u8,
    /// Economic viability and tokenomics score (0–100).
    pub economic_score: u8,
    /// Liquidity depth and market quality score (0–100).
    pub liquidity_score: u8,
    /// Regulatory and compliance score (0–100).
    pub compliance_score: u8,
    /// Weighted composite score (0–100). Populate via [`Self::compute_composite`].
    pub composite_score: u8,
    /// Risk classification derived from the composite score.
    pub risk_tier: ChainRisk,
    /// ISO 8601 week number during which this record was produced.
    pub week_number: u32,
}

impl ChainOnboardingRecord {
    /// Compute the weighted composite score from the four sub-scores.
    ///
    /// Weights: technical 30 %, economic 30 %, liquidity 25 %, compliance 15 %.
    /// The result is clamped to 100 and returned as a `u8`.
    ///
    /// # Note
    ///
    /// This method does **not** mutate `self.composite_score`. The caller must
    /// assign the return value to `self.composite_score` if persistence is
    /// required.
    ///
    /// # Example
    ///
    /// ```rust
    /// use x3_chain_onboarding::{ChainOnboardingRecord, ChainRisk, OnboardingPhase};
    ///
    /// let rec = ChainOnboardingRecord {
    ///     chain_id: 1,
    ///     phase: OnboardingPhase::ComplianceReview,
    ///     technical_score: 80,
    ///     economic_score: 90,
    ///     liquidity_score: 70,
    ///     compliance_score: 85,
    ///     composite_score: 0, // will be computed
    ///     risk_tier: ChainRisk::Low,
    ///     week_number: 20,
    /// };
    /// // 80*30 + 90*30 + 70*25 + 85*15 = 2400+2700+1750+1275 = 8125 / 100 = 81
    /// assert_eq!(rec.compute_composite(), 81);
    /// ```
    #[must_use]
    pub fn compute_composite(&self) -> u8 {
        let weighted = (u32::from(self.technical_score) * 30
            + u32::from(self.economic_score) * 30
            + u32::from(self.liquidity_score) * 25
            + u32::from(self.compliance_score) * 15)
            / 100;
        weighted.min(100) as u8
    }

    /// Returns `true` when all three hard approval thresholds are satisfied.
    ///
    /// | Threshold          | Minimum |
    /// |--------------------|---------|
    /// | `composite_score`  | ≥ 70    |
    /// | `compliance_score` | ≥ 80    |
    /// | `technical_score`  | ≥ 60    |
    ///
    /// This method reads `self.composite_score` as stored — the caller must
    /// ensure it has been populated via [`Self::compute_composite`] before
    /// checking approval.
    #[must_use]
    pub fn meets_approval_threshold(&self) -> bool {
        self.composite_score >= 70
            && self.compliance_score >= 80
            && self.technical_score >= 60
    }
}

// ─── Weekly proving cycle summary ────────────────────────────────────────────

/// Aggregate summary produced at the end of each weekly proving cycle.
///
/// The proving harness writes one of these per ISO week so that dashboards and
/// sidecar services can track onboarding throughput without scanning individual
/// [`ChainOnboardingRecord`]s.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WeeklyProvingCycleSummary {
    /// ISO 8601 week number this summary covers.
    pub week_number: u32,
    /// Total number of chains evaluated during the week.
    pub chains_evaluated: u32,
    /// Chains that passed all thresholds and moved to [`OnboardingPhase::Approved`].
    pub chains_approved: u32,
    /// Chains that failed one or more thresholds.
    pub chains_rejected: u32,
    /// Mean composite score across all evaluated chains (0–100).
    pub avg_composite_score: u8,
}
