//! Intent Management Module
//!
//! Handles atomic intent lifecycle from creation to finalization/refund.

use crate::types::{AssetSpec, ExternalChainId, IntentState, SettlementIntent, TokenId};
use codec::{
    alloc::string::{String, ToString},
    Decode, DecodeWithMemTracking, Encode,
};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{vec, vec::Vec};
use x3_crosschain_intent::{AssetRef, ChainKind, CrossChainIntent};

/// Adapter boundary error: produced when a `CrossChainIntent` cannot
/// be safely projected onto the settlement runtime's `SettlementIntent`.
///
/// These mirror the `AdapterError` cases in
/// `x3-crosschain-intent::adapter`, but the settlement runtime
/// surfaces them as a separate type so the pallet does not leak the
/// intent crate's error model into runtime APIs.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum FromIntentError {
    /// The intent crate reports an unsupported chain on either leg.
    UnsupportedChain { chain: String },
    /// The intent is empty (no source or destination).
    EmptyIntent,
    /// The intent's timeout is zero — `SettlementIntent::timeout`
    /// must be a positive Unix timestamp.
    ZeroTimeout,
    /// The intent's hash did not match the recomputed canonical
    /// hash. The pallet refuses to ingest an intent with a stale or
    /// tampered hash because doing so would let a malicious caller
    /// pass through a hashed-empty intent and substitute the body at
    /// execution time.
    HashMismatch {
        stored: [u8; 32],
        recomputed: [u8; 32],
    },
}

impl core::fmt::Display for FromIntentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FromIntentError::UnsupportedChain { chain } => {
                write!(f, "settlement: unsupported chain '{chain}'")
            }
            FromIntentError::EmptyIntent => write!(f, "settlement: empty intent"),
            FromIntentError::ZeroTimeout => write!(f, "settlement: zero timeout"),
            FromIntentError::HashMismatch { stored, recomputed } => write!(
                f,
                "settlement: hash mismatch (stored={stored:?}, recomputed={recomputed:?})"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FromIntentError {}

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
                chain: slow_chain.chain,
                asset: slow_chain.clone(),
                timeout: slow_timeout,
                confirmations_required: Self::required_confirmations(&slow_chain.chain),
            },
            // Leg 1: Fast chain funds second
            SettlementLeg {
                index: 1,
                chain: fast_chain.chain,
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

// ============================================================================
// Adapter boundary: CrossChainIntent (intent crate) → SettlementIntent (pallet)
// ============================================================================

/// Bridge the intent crate's [`ChainKind`] into the settlement pallet's
/// [`ExternalChainId`].
///
/// This is the runtime side of the cross-chain intent contract: every
/// `AssetRef.chain` from the intent crate must project onto a valid
/// `ExternalChainId` before the pallet will accept the intent.
/// Unknown / unsupported chains fail closed at the boundary.
pub fn external_chain_from_kind(kind: ChainKind) -> Result<ExternalChainId, FromIntentError> {
    Ok(match kind {
        ChainKind::X3 => ExternalChainId::X3Native,
        ChainKind::Ethereum => ExternalChainId::Ethereum,
        ChainKind::Bitcoin => ExternalChainId::Bitcoin,
        ChainKind::Solana => ExternalChainId::Solana,
        ChainKind::Base => ExternalChainId::Base,
        ChainKind::Arbitrum => ExternalChainId::Arbitrum,
        ChainKind::Optimism => ExternalChainId::Optimism,
        ChainKind::Bsc => ExternalChainId::Bnb,
        ChainKind::Polygon => ExternalChainId::Polygon,
        ChainKind::Avalanche => ExternalChainId::Avalanche,
        // Cosmos is recognized by the intent crate but the settlement
        // pallet does not yet have a `Cosmos` ExternalChainId variant.
        // Surface it as an explicit unsupported-chain error rather than
        // silently re-routing to X3Native.
        ChainKind::Cosmos => {
            return Err(FromIntentError::UnsupportedChain {
                chain: "cosmos".to_string(),
            });
        }
    })
}

/// Project an `AssetRef` to an `AssetSpec` (chain + token + amount).
///
/// The symbol is encoded into the 32-byte contract slot of `TokenId`
/// so the pallet can later look up the on-chain token by its hash
/// (the canonical 32-byte symbol hash). Native assets use
/// `TokenId::Native` so the planner can skip ERC-20/SPL lookup.
pub fn asset_spec_from_ref(asset: &AssetRef, amount: u128) -> Result<AssetSpec, FromIntentError> {
    let chain = external_chain_from_kind(asset.chain)?;
    // We treat the symbol as a string token contract id. If the
    // symbol is empty we fail closed. Native-style tickers ("ETH",
    // "BTC", "SOL", "X3") map to `TokenId::Native`.
    let token = if asset.symbol.is_empty() {
        return Err(FromIntentError::EmptyIntent);
    } else if is_native_symbol(&asset.symbol) {
        TokenId::Native
    } else {
        let mut bytes = [0u8; 32];
        let sym = asset.symbol.as_bytes();
        let len = sym.len().min(32);
        bytes[..len].copy_from_slice(&sym[..len]);
        TokenId::Contract(bytes)
    };
    Ok(AssetSpec {
        chain,
        token,
        amount,
    })
}

/// Heuristic: tickers that the runtime should treat as the chain's
/// native asset rather than a token contract.
fn is_native_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "ETH" | "BTC" | "SOL" | "X3" | "MATIC" | "BNB" | "AVAX" | "ATOM" | "X3.NATIVE"
    )
}

/// Adapter: convert a `CrossChainIntent` (intent crate) into a
/// `SettlementIntent<AccountId>` (settlement pallet).
///
/// This is **the** boundary function between the intent compiler
/// layer and the runtime. The settlement pallet MUST NOT accept a
/// `CrossChainIntent` from any other entry point: the language
/// compiler, the cross-chain intent compiler, and the settlement
/// engine all share this one function so the three layers cannot
/// drift into parallel state machines.
///
/// The function:
///
/// 1. Verifies the intent hash matches the recomputed canonical
///    hash. A mismatch is a hard error — the pallet refuses to
///    ingest a tampered intent.
/// 2. Verifies the timeout is non-zero (the runtime interprets
///    `timeout == 0` as "no timeout" which is unsafe).
/// 3. Projects source and destination `AssetRef` values onto
///    `AssetSpec`. Unsupported chains (e.g. `cosmos` while the
///    pallet does not yet have that variant) fail closed.
/// 4. Derives the `intent_id` as the H256 of the canonical intent
///    hash, so the runtime's `SettlementIntent.intent_id` is
///    deterministically tied to the intent body.
/// 5. Derives a `secret_hash` placeholder. In a real flow the
///    cross-chain intent crate emits a `X3Instruction::HashSecret`
///    plan; the runtime enforces the secret reveal when claims
///    arrive. The hash placeholder here is the first 32 bytes of
///    the intent hash (collision-resistant for binding to a single
///    intent body) so the settlement intent is self-contained.
pub fn from_crosschain_intent<AccountId>(
    intent: &CrossChainIntent,
    maker: AccountId,
    taker: AccountId,
    created_at: u64,
) -> Result<SettlementIntent<AccountId>, FromIntentError>
where
    AccountId: Encode + Decode + Clone + PartialEq + Debug,
{
    // 1. Hash check.
    let stored = intent.intent_hash;
    let recomputed = intent.compute_hash();
    if stored != recomputed {
        return Err(FromIntentError::HashMismatch { stored, recomputed });
    }

    // 2. Timeout check.
    if intent.timeout.timeout_secs == 0 {
        return Err(FromIntentError::ZeroTimeout);
    }

    // 3. Asset projection.
    let asset_a = asset_spec_from_ref(&intent.source.asset, intent.source.amount)?;
    // The destination leg uses the planned delivery amount. The
    // intent crate stores this as `min_amount: Option<u128>`; if it
    // is absent we fall back to the source amount so the runtime
    // still has a positive asset_b.
    let dest_amount = intent
        .destination
        .min_amount
        .unwrap_or(intent.source.amount);
    let asset_b = asset_spec_from_ref(&intent.destination.asset, dest_amount)?;

    // 4. intent_id = H256(intent_hash)
    let intent_id = H256::from(stored);

    // 5. secret_hash placeholder (see doc above).
    let mut secret_hash = [0u8; 32];
    secret_hash.copy_from_slice(&stored);
    let secret_hash = H256::from(secret_hash);

    // timeout_secs is a duration; convert to an absolute Unix
    // timestamp by adding `created_at`. The runtime expects
    // `timeout` to be an absolute deadline.
    let timeout = created_at.saturating_add(intent.timeout.timeout_secs);

    Ok(SettlementIntent {
        intent_id,
        maker,
        taker,
        asset_a,
        asset_b,
        secret_hash,
        timeout,
        created_at,
        legs_total: 2,
        legs_locked: 0,
        legs_claimed: 0,
    })
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

    // ----------------------------------------------------------------
    // Adapter boundary tests: CrossChainIntent → SettlementIntent
    // ----------------------------------------------------------------

    use x3_crosschain_intent::ReceiverAuthorization;
    use x3_crosschain_intent::{
        AssetRef, ChainKind, DestinationSpec, FailureAction, ReceiptSpec, Requirements, RouteSpec,
        SourceSpec, TimeoutSpec,
    };

    fn build_eth_x3_intent(name: &str, amount: u128, timeout_secs: u64) -> CrossChainIntent {
        let source = SourceSpec {
            asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
            amount,
            owner: "alice.eth".to_string(),
            lock_contract: Some("0xBridge".to_string()),
        };
        let destination = DestinationSpec {
            asset: AssetRef::new(ChainKind::X3, "USDC.e"),
            receiver: "alice.x3".to_string(),
            min_amount: Some(amount),
        };
        let mut intent = CrossChainIntent {
            id: 1,
            name: name.to_string(),
            source,
            destination,
            route: RouteSpec::default(),
            requirements: Requirements {
                receiver_authorization: ReceiverAuthorization::MappedAccount {
                    source_chain: ChainKind::Ethereum,
                    source_owner: "alice.eth".to_string(),
                    dest_chain: ChainKind::X3,
                    dest_account: "alice.x3".to_string(),
                },
                ..Requirements::default()
            },
            timeout: TimeoutSpec {
                timeout_secs,
                on_fail: vec![FailureAction::RefundSource],
            },
            receipt: ReceiptSpec::default(),
            intent_hash: [0u8; 32],
        };
        intent.recompute_and_store_hash();
        intent
    }

    #[test]
    fn adapter_chain_bridge_covers_all_known_kinds() {
        assert_eq!(
            external_chain_from_kind(ChainKind::X3).unwrap(),
            ExternalChainId::X3Native
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Ethereum).unwrap(),
            ExternalChainId::Ethereum
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Bitcoin).unwrap(),
            ExternalChainId::Bitcoin
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Solana).unwrap(),
            ExternalChainId::Solana
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Base).unwrap(),
            ExternalChainId::Base
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Arbitrum).unwrap(),
            ExternalChainId::Arbitrum
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Optimism).unwrap(),
            ExternalChainId::Optimism
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Bsc).unwrap(),
            ExternalChainId::Bnb
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Polygon).unwrap(),
            ExternalChainId::Polygon
        );
        assert_eq!(
            external_chain_from_kind(ChainKind::Avalanche).unwrap(),
            ExternalChainId::Avalanche
        );
        // Cosmos is not (yet) supported by the pallet — must fail closed.
        assert!(matches!(
            external_chain_from_kind(ChainKind::Cosmos),
            Err(FromIntentError::UnsupportedChain { .. })
        ));
    }

    #[test]
    fn adapter_projects_native_and_contract_symbols() {
        let native =
            asset_spec_from_ref(&AssetRef::new(ChainKind::Ethereum, "ETH"), 1_000).unwrap();
        assert_eq!(native.chain, ExternalChainId::Ethereum);
        assert_eq!(native.token, TokenId::Native);
        assert_eq!(native.amount, 1_000);

        let contract =
            asset_spec_from_ref(&AssetRef::new(ChainKind::Ethereum, "USDC"), 5_000).unwrap();
        assert_eq!(contract.chain, ExternalChainId::Ethereum);
        assert!(matches!(contract.token, TokenId::Contract(_)));
        assert_eq!(contract.amount, 5_000);
    }

    #[test]
    fn adapter_rejects_empty_symbol() {
        let bad = asset_spec_from_ref(&AssetRef::new(ChainKind::Ethereum, ""), 1);
        assert!(matches!(bad, Err(FromIntentError::EmptyIntent)));
    }

    #[test]
    fn from_crosschain_intent_produces_runtime_intent() {
        let intent = build_eth_x3_intent("bridge_usdc_x3", 500_000_000, 1800);
        let rt_intent: SettlementIntent<u64> =
            from_crosschain_intent(&intent, 1u64, 2u64, 1_000_000)
                .expect("adapter must produce a runtime intent");

        // Hash must round-trip.
        assert_eq!(rt_intent.intent_id.0, intent.intent_hash);

        // Source is eth.USDC → ExternalChainId::Ethereum.
        assert_eq!(rt_intent.asset_a.chain, ExternalChainId::Ethereum);
        assert_eq!(rt_intent.asset_a.amount, 500_000_000);

        // Destination is x3.USDC.e → ExternalChainId::X3Native.
        assert_eq!(rt_intent.asset_b.chain, ExternalChainId::X3Native);
        assert_eq!(rt_intent.asset_b.amount, 500_000_000);

        // timeout is absolute deadline = created_at + timeout_secs.
        assert_eq!(rt_intent.created_at, 1_000_000);
        assert_eq!(rt_intent.timeout, 1_001_800);

        // Fresh settlement intent starts with 0 locked / 0 claimed.
        assert_eq!(rt_intent.legs_total, 2);
        assert_eq!(rt_intent.legs_locked, 0);
        assert_eq!(rt_intent.legs_claimed, 0);
    }

    #[test]
    fn from_crosschain_intent_rejects_tampered_hash() {
        let mut intent = build_eth_x3_intent("tampered", 100, 1800);
        // Mutate a field without recomputing the hash.
        intent.source.amount = 999_999_999;
        let err: Result<SettlementIntent<u64>, _> = from_crosschain_intent(&intent, 1u64, 2u64, 0);
        assert!(matches!(err, Err(FromIntentError::HashMismatch { .. })));
    }

    #[test]
    fn from_crosschain_intent_rejects_zero_timeout() {
        let mut intent = build_eth_x3_intent("no_timeout", 100, 0);
        intent.recompute_and_store_hash();
        let err: Result<SettlementIntent<u64>, _> = from_crosschain_intent(&intent, 1u64, 2u64, 0);
        assert!(matches!(err, Err(FromIntentError::ZeroTimeout)));
    }

    #[test]
    fn from_crosschain_intent_supports_btc_leg() {
        let source = SourceSpec {
            asset: AssetRef::new(ChainKind::Bitcoin, "BTC"),
            amount: 10_000,
            owner: "bc1qalice".to_string(),
            lock_contract: None,
        };
        let destination = DestinationSpec {
            asset: AssetRef::new(ChainKind::X3, "BTC.e"),
            receiver: "alice.x3".to_string(),
            min_amount: Some(10_000),
        };
        let mut intent = CrossChainIntent {
            id: 7,
            name: "btc_bridge".to_string(),
            source,
            destination,
            route: RouteSpec::default(),
            requirements: Requirements {
                receiver_authorization: ReceiverAuthorization::MappedAccount {
                    source_chain: ChainKind::Bitcoin,
                    source_owner: "bc1qalice".to_string(),
                    dest_chain: ChainKind::X3,
                    dest_account: "alice.x3".to_string(),
                },
                ..Requirements::default()
            },
            timeout: TimeoutSpec {
                timeout_secs: 3600,
                on_fail: vec![FailureAction::RefundSource, FailureAction::Quarantine],
            },
            receipt: ReceiptSpec::default(),
            intent_hash: [0u8; 32],
        };
        intent.recompute_and_store_hash();
        let rt: SettlementIntent<u64> =
            from_crosschain_intent(&intent, 1u64, 2u64, 0).expect("btc leg must project");
        assert_eq!(rt.asset_a.chain, ExternalChainId::Bitcoin);
        assert_eq!(rt.asset_a.token, TokenId::Native);
        assert_eq!(rt.asset_b.chain, ExternalChainId::X3Native);
        assert_eq!(rt.timeout, 3600);
    }

    #[test]
    fn intent_planner_plans_canonical_legs_from_runtime_intent() {
        // End-to-end: take a CrossChainIntent, project it to a
        // SettlementIntent, then drive IntentPlanner::plan_settlement
        // with the projected assets. The planner must see the same
        // chains as the intent declared.
        let intent = build_eth_x3_intent("plan_legs", 1_000, 1800);
        let rt: SettlementIntent<u64> =
            from_crosschain_intent(&intent, 1u64, 2u64, 1_700_000_000).unwrap();
        let plan = IntentPlanner::plan_settlement(&rt.asset_a, &rt.asset_b, rt.timeout);
        // Both legs must reference chains the intent declared.
        let chains: Vec<ExternalChainId> = plan.legs.iter().map(|l| l.chain).collect();
        assert!(chains.contains(&ExternalChainId::Ethereum));
        assert!(chains.contains(&ExternalChainId::X3Native));
        // The intent's timeout must be respected.
        for leg in &plan.legs {
            assert!(leg.timeout > 0, "leg timeout must be positive");
        }
    }
}
