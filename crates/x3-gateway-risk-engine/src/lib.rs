#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Gateway Risk Engine
//!
//! Risk classification for cross-chain operations using oracle data and anti-rug scores.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::SaturatedConversion;
use sp_std::vec::Vec;
// Note: Would integrate with oracle pallet for price data

/// Risk levels for transactions/operations
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum RiskLevel {
    /// Low risk - proceed normally
    Low,
    /// Medium risk - additional verification required
    Medium,
    /// High risk - manual review required
    High,
    /// Critical risk - block operation
    Critical,
}

/// Risk assessment result
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct RiskAssessment {
    /// Overall risk level
    pub level: RiskLevel,
    /// Risk score (0-10000, representing 0.00%-100.00%)
    pub score: u16,
    /// Risk factors identified
    pub factors: Vec<RiskFactor>,
    /// Recommended actions
    pub recommendations: Vec<RiskRecommendation>,
}

/// Risk factors that contribute to assessment
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum RiskFactor {
    /// Large transaction amount relative to liquidity
    LargeTransactionAmount,
    /// Price manipulation detected
    PriceManipulation,
    /// Low anti-rug score for involved assets
    LowAntiRugScore,
    /// Unusual transaction pattern
    UnusualPattern,
    /// High price volatility
    HighVolatility,
    /// Cross-chain operation with insufficient finality
    InsufficientFinality,
}

/// Recommended actions based on risk assessment
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum RiskRecommendation {
    /// Require additional confirmations
    RequireAdditionalConfirmations,
    /// Reduce maximum transaction amount
    ReduceMaxAmount,
    /// Require manual approval
    RequireManualApproval,
    /// Temporarily suspend operations
    SuspendOperations,
    /// Block this specific operation
    BlockOperation,
}

/// Transaction data for risk assessment
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct TransactionData {
    /// Transaction amount
    pub amount: U256,
    /// Asset ID being transacted
    pub asset_id: u32,
    /// Source chain ID
    pub source_chain: u32,
    /// Destination chain ID
    pub dest_chain: u32,
    /// Transaction type
    pub tx_type: TransactionType,
}

/// Transaction types
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub enum TransactionType {
    /// Asset transfer
    Transfer,
    /// Swap operation
    Swap,
    /// Liquidity provision
    LiquidityProvision,
    /// Contract call
    ContractCall,
}

/// Risk classifier interface
pub trait RiskClassifier {
    /// Classify risk for a transaction
    fn classify_risk(tx: &TransactionData) -> RiskAssessment;
}

/// AI-powered risk classifier
pub struct AiRiskClassifier;

impl RiskClassifier for AiRiskClassifier {
    fn classify_risk(tx: &TransactionData) -> RiskAssessment {
        let mut factors = Vec::new();
        let mut recommendations = Vec::new();
        let mut score = 0u16;

        // Check transaction amount vs liquidity (simplified)
        if Self::is_large_transaction(tx) {
            factors.push(RiskFactor::LargeTransactionAmount);
            score += 3000; // +30.00%
            recommendations.push(RiskRecommendation::RequireAdditionalConfirmations);
        }

        // Check anti-rug score (simplified - would integrate with actual scores)
        if Self::has_low_anti_rug_score(tx.asset_id) {
            factors.push(RiskFactor::LowAntiRugScore);
            score += 2500; // +25.00%
            recommendations.push(RiskRecommendation::ReduceMaxAmount);
        }

        // Check price volatility
        if Self::has_high_volatility(tx.asset_id) {
            factors.push(RiskFactor::HighVolatility);
            score += 2000; // +20.00%
            recommendations.push(RiskRecommendation::RequireManualApproval);
        }

        // Check for unusual patterns
        if Self::is_unusual_pattern(tx) {
            factors.push(RiskFactor::UnusualPattern);
            score += 1500; // +15.00%
        }

        // Cross-chain specific checks
        if tx.source_chain != tx.dest_chain {
            if Self::has_insufficient_finality(tx) {
                factors.push(RiskFactor::InsufficientFinality);
                score += 4000; // +40.00%
                recommendations.push(RiskRecommendation::BlockOperation);
            }
        }

        // Determine risk level based on score
        let level = if score >= 8000 {
            RiskLevel::Critical
        } else if score >= 6000 {
            RiskLevel::High
        } else if score >= 4000 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        RiskAssessment {
            level,
            score,
            factors,
            recommendations,
        }
    }
}

impl AiRiskClassifier {
    /// Check if transaction amount is considered large (simplified)
    fn is_large_transaction(tx: &TransactionData) -> bool {
        // Simplified check - in production would check against pool liquidity
        let amount_u128 = tx.amount.saturated_into::<u128>();
        amount_u128 > 1000000 // Arbitrary threshold
    }

    /// Check if asset has low anti-rug score (simplified)
    fn has_low_anti_rug_score(_asset_id: u32) -> bool {
        // In production, would query anti-rug scores from pallet
        // For demo, assume some assets have issues
        false
    }

    /// Check if asset has high volatility (simplified)
    fn has_high_volatility(_asset_id: u32) -> bool {
        // TODO: Check recent price movements from oracle
        // Stubbed until x3-oracle pallet is implemented
        // if let Some(price_data) = AssetPrices::get(asset_id) {
        //     let current_price = price_data.price;
        //     current_price > 2000000
        // }
        false
    }

    /// Check for unusual transaction patterns (simplified)
    fn is_unusual_pattern(_tx: &TransactionData) -> bool {
        // Would analyze transaction history, time patterns, etc.
        false
    }

    /// Check for insufficient finality in cross-chain ops (simplified)
    fn has_insufficient_finality(tx: &TransactionData) -> bool {
        // Would check bridge finality proofs, validator confirmations, etc.
        // For demo, flag certain chain combinations
        tx.source_chain == 999 || tx.dest_chain == 999 // Mock risky chains
    }
}

/// Rate limiting component
pub struct RateLimiter {
    /// Maximum transactions per time window
    pub max_per_window: u32,
    /// Time window in blocks
    pub window_blocks: u32,
}

impl RateLimiter {
    /// Check if operation should be rate limited
    pub fn should_limit(&self, _account: &[u8], _current_count: u32) -> bool {
        // Simplified rate limiting logic
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_classification_low() {
        let tx = TransactionData {
            amount: U256::from(1000),
            asset_id: 0,
            source_chain: 1,
            dest_chain: 1,
            tx_type: TransactionType::Transfer,
        };

        let assessment = AiRiskClassifier::classify_risk(&tx);
        assert_eq!(assessment.level, RiskLevel::Low);
        assert!(assessment.factors.is_empty());
    }

    #[test]
    fn test_risk_classification_critical() {
        let tx = TransactionData {
            amount: U256::from(10000000), // Very large amount
            asset_id: 0,
            source_chain: 1,
            dest_chain: 999, // Risky destination
            tx_type: TransactionType::Transfer,
        };

        let assessment = AiRiskClassifier::classify_risk(&tx);
        assert_eq!(assessment.level, RiskLevel::Critical);
        assert!(!assessment.factors.is_empty());
    }
}

/// Decision from the route risk engine.
#[derive(Clone, Debug)]
pub struct RouteRiskDecision {
    /// Whether the route is allowed.
    pub allow_route: bool,
    /// Human-readable reason.
    pub reason: String,
}

/// Input for route-level risk evaluation.
#[derive(Clone, Debug, Default)]
pub struct RouteRiskInput {
    /// Estimated value in USD of the pending operations.
    pub value_usd: u64,
    /// Number of recent failures observed.
    pub recent_failures: u32,
    /// Whether verifier quorum has been met.
    pub verifier_quorum_met: bool,
}

/// Simple risk policy configuration.
#[derive(Clone, Debug, Default)]
pub struct RiskPolicy {
    /// Maximum USD value allowed per route batch.
    pub max_value_usd: u64,
    /// Maximum tolerated recent failures before blocking.
    pub max_recent_failures: u32,
}

/// Stateful gateway risk engine that evaluates route safety.
#[derive(Clone, Debug, Default)]
pub struct GatewayRiskEngine {
    policy: RiskPolicy,
}

impl GatewayRiskEngine {
    /// Create a new engine with the given policy.
    pub fn new(policy: RiskPolicy) -> Self {
        Self { policy }
    }

    /// Evaluate whether a route is safe to proceed.
    pub fn evaluate(&self, input: RouteRiskInput) -> RouteRiskDecision {
        if self.policy.max_value_usd > 0 && input.value_usd > self.policy.max_value_usd {
            return RouteRiskDecision {
                allow_route: false,
                reason: format!(
                    "value_usd {} exceeds limit {}",
                    input.value_usd, self.policy.max_value_usd
                ),
            };
        }
        if self.policy.max_recent_failures > 0
            && input.recent_failures > self.policy.max_recent_failures
        {
            return RouteRiskDecision {
                allow_route: false,
                reason: format!(
                    "recent_failures {} exceeds limit {}",
                    input.recent_failures, self.policy.max_recent_failures
                ),
            };
        }
        if !input.verifier_quorum_met {
            return RouteRiskDecision {
                allow_route: false,
                reason: "verifier quorum not met".to_string(),
            };
        }
        RouteRiskDecision {
            allow_route: true,
            reason: "ok".to_string(),
        }
    }
}
