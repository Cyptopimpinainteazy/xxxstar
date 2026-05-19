//! Intent Management Module
//!
//! Handles atomic intent lifecycle from creation to finalization/refund.

use crate::types::{AssetSpec, ExternalChainId, IntentState, SettlementIntent};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{vec, vec::Vec};

/// Intent creation parameters
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct CreateIntentParams<AccountId> {
    /// Maker (initiator)
    pub maker: AccountId,
    /// Taker (counterparty)
    pub taker: AccountId,
    /// Asset maker is selling
    pub sell_asset: AssetSpec,
    /// Asset maker is buying
    pub buy_asset: AssetSpec,
    /// Secret hash for HTLC
    pub secret_hash: H256,
    /// Timeout in seconds from now
    pub timeout_seconds: u64,
}

/// Intent settlement plan
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct SettlementPlan {
    /// Ordered list of legs to execute
    pub legs: Vec<SettlementLeg>,
    /// Total expected time (seconds)
    pub estimated_time: u64,
    /// Risk assessment
    pub risk_level: RiskLevel,
}

/// Individual settlement leg
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct SettlementLeg {
    /// Leg index
    pub index: u32,
    /// Chain for this leg
    pub chain: ExternalChainId,
    /// Asset being moved
    pub asset: AssetSpec,
    /// Timeout for this leg
    pub timeout: u64,
    /// Required confirmations
    pub confirmations_required: u32,
}

/// Settlement risk level
#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum RiskLevel {
    /// Low risk (X3-internal swaps)
    Low,
    /// Medium risk (fast L2 chains)
    Medium,
    /// High risk (slow chains like BTC)
    High,
}

/// Intent planner - determines optimal settlement strategy
pub struct IntentPlanner;

impl IntentPlanner {
    /// Create settlement plan for an intent
    ///
    /// Rules:
    /// 1. Slow chain ALWAYS funds first (BTC, L1)
    /// 2. Fast chain funds second (L2, internal)
    /// 3. Fast chain claims first (revealing secret)
    /// 4. Slow chain claims second (using revealed secret)
    pub fn plan_settlement(
        sell_asset: &AssetSpec,
        buy_asset: &AssetSpec,
        timeout_seconds: u64,
    ) -> SettlementPlan {
        let sell_speed = Self::chain_speed(&sell_asset.chain);
        let buy_speed = Self::chain_speed(&buy_asset.chain);

        // Determine which chain is slower
        let (slow_chain, fast_chain) = if sell_speed <= buy_speed {
            (sell_asset, buy_asset)
        } else {
            (buy_asset, sell_asset)
        };

        // Calculate timeouts
        // Slow chain timeout > fast chain timeout (to prevent stuck funds)
        let slow_timeout = timeout_seconds;
        let fast_timeout = timeout_seconds / 2; // Fast chain has half the time

        let legs = vec![
            // Leg 0: Slow chain funds first
            SettlementLeg {
                index: 0,
                chain: slow_chain.chain.clone(),
                asset: slow_chain.clone(),
                timeout: slow_timeout,
                confirmations_required: Self::required_confirmations(&slow_chain.chain),
            },
            // Leg 1: Fast chain funds second
            SettlementLeg {
                index: 1,
                chain: fast_chain.chain.clone(),
                asset: fast_chain.clone(),
                timeout: fast_timeout,
                confirmations_required: Self::required_confirmations(&fast_chain.chain),
            },
        ];

        let risk_level = Self::assess_risk(sell_asset, buy_asset);

        SettlementPlan {
            legs,
            estimated_time: Self::estimate_time(sell_asset, buy_asset),
            risk_level,
        }
    }

    /// Get chain speed ranking (lower = slower)
    fn chain_speed(chain: &ExternalChainId) -> u32 {
        match chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => 1, // Slowest
            ExternalChainId::Ethereum => 2,
            ExternalChainId::Polygon => 3,
            ExternalChainId::Avalanche => 4,
            ExternalChainId::Bnb => 4,
            ExternalChainId::Arbitrum => 5,
            ExternalChainId::Base => 5,
            ExternalChainId::Optimism => 5,
            ExternalChainId::Solana | ExternalChainId::SolanaDevnet => 6,
            ExternalChainId::X3Native => 7,    // Fastest (internal)
            ExternalChainId::EvmChain(_) => 3, // Default for unknown EVM
        }
    }

    /// Get required confirmations for chain
    fn required_confirmations(chain: &ExternalChainId) -> u32 {
        match chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => 6,
            ExternalChainId::Ethereum => 12,
            ExternalChainId::Polygon => 128,
            ExternalChainId::Avalanche => 1, // Instant finality
            ExternalChainId::Bnb => 15,
            ExternalChainId::Arbitrum => 1,
            ExternalChainId::Base => 1,
            ExternalChainId::Optimism => 1,
            ExternalChainId::Solana | ExternalChainId::SolanaDevnet => 1,
            ExternalChainId::X3Native => 1,     // GRANDPA finality
            ExternalChainId::EvmChain(_) => 12, // Default for unknown EVM
        }
    }

    /// Estimate settlement time in seconds
    fn estimate_time(sell_asset: &AssetSpec, buy_asset: &AssetSpec) -> u64 {
        let sell_time = Self::chain_time(&sell_asset.chain);
        let buy_time = Self::chain_time(&buy_asset.chain);

        // Total time is sum of both chains (sequential funding + claiming)
        sell_time + buy_time
    }

    /// Get expected confirmation time for chain (seconds)
    fn chain_time(chain: &ExternalChainId) -> u64 {
        match chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => 3600, // 6 blocks * 10 min
            ExternalChainId::Ethereum => 180, // 12 blocks * 15 sec
            ExternalChainId::Polygon => 256,  // 128 blocks * 2 sec
            ExternalChainId::Avalanche => 2,  // Near instant
            ExternalChainId::Bnb => 45,       // 15 blocks * 3 sec
            ExternalChainId::Arbitrum => 15,
            ExternalChainId::Base => 15,
            ExternalChainId::Optimism => 15,
            ExternalChainId::Solana | ExternalChainId::SolanaDevnet => 2,
            ExternalChainId::X3Native => 6,      // 1 block
            ExternalChainId::EvmChain(_) => 180, // Default
        }
    }

    /// Assess risk level of swap
    fn assess_risk(sell_asset: &AssetSpec, buy_asset: &AssetSpec) -> RiskLevel {
        let involves_btc = matches!(
            sell_asset.chain,
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet
        ) || matches!(
            buy_asset.chain,
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet
        );

        let both_internal = matches!(sell_asset.chain, ExternalChainId::X3Native)
            && matches!(buy_asset.chain, ExternalChainId::X3Native);

        if both_internal {
            RiskLevel::Low
        } else if involves_btc {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        }
    }
}

/// Intent state machine transitions
pub struct IntentStateMachine;

impl IntentStateMachine {
    /// Get valid next states from current state
    pub fn valid_transitions(current: IntentState) -> Vec<IntentState> {
        match current {
            IntentState::Created => vec![
                IntentState::FundingInProgress,
                IntentState::Refunded, // Cancelled before funding
            ],
            IntentState::FundingInProgress => vec![
                IntentState::FullyFunded,
                IntentState::Refunded, // Timeout during funding
            ],
            IntentState::FullyFunded => vec![
                IntentState::ExecutingExternal,
                IntentState::Claiming,
                IntentState::Refunded,
            ],
            IntentState::ExecutingExternal => vec![IntentState::Claiming, IntentState::Refunded],
            IntentState::Claiming => vec![
                IntentState::Finalized,
                IntentState::Refunded, // Claim failed
            ],
            IntentState::Finalized => vec![], // Terminal state
            IntentState::Refunded => vec![],  // Terminal state
            IntentState::Halted => vec![
                IntentState::Refunded, // Only governance can resolve
            ],
        }
    }

    /// Check if transition is valid
    pub fn can_transition(from: IntentState, to: IntentState) -> bool {
        Self::valid_transitions(from).contains(&to)
    }

    /// Check if state is terminal
    pub fn is_terminal(state: IntentState) -> bool {
        matches!(state, IntentState::Finalized | IntentState::Refunded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenId;

    fn btc_asset(amount: u128) -> AssetSpec {
        AssetSpec {
            chain: ExternalChainId::Bitcoin,
            token: TokenId::Native,
            amount,
        }
    }

    fn arb_asset(amount: u128) -> AssetSpec {
        AssetSpec {
            chain: ExternalChainId::Arbitrum,
            token: TokenId::Native,
            amount,
        }
    }

    fn x3_asset(amount: u128) -> AssetSpec {
        AssetSpec {
            chain: ExternalChainId::X3Native,
            token: TokenId::Native,
            amount,
        }
    }

    #[test]
    fn test_slow_chain_first() {
        let plan = IntentPlanner::plan_settlement(&arb_asset(1000), &btc_asset(100000), 3600);

        // BTC should be leg 0 (funds first)
        assert_eq!(plan.legs[0].chain, ExternalChainId::Bitcoin);
        assert_eq!(plan.legs[1].chain, ExternalChainId::Arbitrum);
    }

    #[test]
    fn test_risk_assessment() {
        // BTC swap is high risk
        assert_eq!(
            IntentPlanner::assess_risk(&btc_asset(100), &arb_asset(1000)),
            RiskLevel::High
        );

        // Internal swap is low risk
        assert_eq!(
            IntentPlanner::assess_risk(&x3_asset(100), &x3_asset(1000)),
            RiskLevel::Low
        );

        // L2 to L2 is medium risk
        assert_eq!(
            IntentPlanner::assess_risk(
                &arb_asset(100),
                &AssetSpec {
                    chain: ExternalChainId::Base,
                    token: TokenId::Native,
                    amount: 1000,
                }
            ),
            RiskLevel::Medium
        );
    }

    #[test]
    fn test_state_transitions() {
        // Valid: Created -> FundingInProgress
        assert!(IntentStateMachine::can_transition(
            IntentState::Created,
            IntentState::FundingInProgress
        ));

        // Invalid: Created -> Finalized (skip states)
        assert!(!IntentStateMachine::can_transition(
            IntentState::Created,
            IntentState::Finalized
        ));

        // Terminal states have no transitions
        assert!(IntentStateMachine::valid_transitions(IntentState::Finalized).is_empty());
    }
}
