//! Primitive types for the cross-chain intent system.
//!
//! Every type here is serializable and hash-stable. No runtime state lives here —
//! only the declarative structure the user (or compiler) provides.

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Chain identification
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical chain identifier.
///
/// Used in asset references and finality requirements.
/// All chain variants must be explicitly listed here — unknown chains are a
/// compiler error (X3-INTENT-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainKind {
    Ethereum,
    Solana,
    Bitcoin,
    X3,
    Base,
    Arbitrum,
    Optimism,
    Bsc,
    Polygon,
    Avalanche,
    Cosmos,
}

impl ChainKind {
    /// Returns the canonical lowercase string identifier used in x3-lang syntax.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainKind::Ethereum => "eth",
            ChainKind::Solana => "sol",
            ChainKind::Bitcoin => "btc",
            ChainKind::X3 => "x3",
            ChainKind::Base => "base",
            ChainKind::Arbitrum => "arb",
            ChainKind::Optimism => "op",
            ChainKind::Bsc => "bsc",
            ChainKind::Polygon => "poly",
            ChainKind::Avalanche => "avax",
            ChainKind::Cosmos => "cosmos",
        }
    }

    /// Parse from the canonical lowercase identifier used in x3-lang.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "eth" | "ethereum" => Some(ChainKind::Ethereum),
            "sol" | "solana" => Some(ChainKind::Solana),
            "btc" | "bitcoin" => Some(ChainKind::Bitcoin),
            "x3" => Some(ChainKind::X3),
            "base" => Some(ChainKind::Base),
            "arb" | "arbitrum" => Some(ChainKind::Arbitrum),
            "op" | "optimism" => Some(ChainKind::Optimism),
            "bsc" => Some(ChainKind::Bsc),
            "poly" | "polygon" => Some(ChainKind::Polygon),
            "avax" | "avalanche" => Some(ChainKind::Avalanche),
            "cosmos" => Some(ChainKind::Cosmos),
            _ => None,
        }
    }

    /// True if this chain uses probabilistic finality (block confirmations).
    pub fn is_probabilistic(&self) -> bool {
        matches!(
            self,
            ChainKind::Ethereum | ChainKind::Bitcoin | ChainKind::Bsc | ChainKind::Polygon
        )
    }

    /// Default safe confirmation count for this chain.
    /// Compiler uses this as a floor when validating `require finality` clauses.
    pub fn default_safe_confirmations(&self) -> u32 {
        match self {
            ChainKind::Ethereum => 12,
            ChainKind::Bitcoin => 6,
            ChainKind::Solana => 0, // uses commitment level instead
            ChainKind::X3 => 1,     // X3 uses BFT finality
            ChainKind::Base | ChainKind::Arbitrum | ChainKind::Optimism => 1, // L2 sequencer finality
            ChainKind::Bsc => 15,
            ChainKind::Polygon => 128,
            ChainKind::Avalanche => 1, // PoS snowball
            ChainKind::Cosmos => 1,    // Tendermint BFT
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Asset references
// ─────────────────────────────────────────────────────────────────────────────

/// A fully-qualified asset reference: `{chain}.{symbol}` e.g. `eth.USDC`.
///
/// The compiler validates that the chain is known (X3-INTENT-008) and that the
/// asset ticker is not ambiguous across chains (X3-INTENT-009).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetRef {
    /// Which chain this asset lives on.
    pub chain: ChainKind,
    /// Asset ticker / symbol (e.g. "USDC", "SOL", "USDC.e").
    pub symbol: String,
}

impl AssetRef {
    pub fn new(chain: ChainKind, symbol: impl Into<String>) -> Self {
        Self {
            chain,
            symbol: symbol.into(),
        }
    }

    /// Display as `chain.SYMBOL`, e.g. `eth.USDC`.
    pub fn display(&self) -> String {
        format!("{}.{}", self.chain.as_str(), self.symbol)
    }

    /// True if this is a canonical X3-wrapped asset (e.g. x3.USDC.e).
    pub fn is_canonical_wrapped(&self) -> bool {
        self.chain == ChainKind::X3
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Source and destination descriptors
// ─────────────────────────────────────────────────────────────────────────────

/// The source side of an intent: which asset, how much, who owns it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpec {
    /// Asset to spend.
    pub asset: AssetRef,
    /// Amount in the asset's base units (no decimals ambiguity — see X3-INTENT-012).
    pub amount: u128,
    /// Owner address (chain-native format, validated at compile time).
    pub owner: String,
    /// Optional: explicit lock contract address to use.
    pub lock_contract: Option<String>,
}

/// The destination side of an intent: which asset the user wants, where to send it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationSpec {
    /// Asset the user wants to receive.
    pub asset: AssetRef,
    /// Recipient address (chain-native format).
    pub receiver: String,
    /// Minimum acceptable output amount in base units.
    pub min_amount: Option<u128>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Finality requirements
// ─────────────────────────────────────────────────────────────────────────────

/// Finality model for a specific chain within this intent.
///
/// X3 rejects intents that bridge without explicit finality requirements
/// (error X3-INTENT-001). A bridge goblin only gets fed when finality is skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinalityLevel {
    /// Wait for N block confirmations (probabilistic chains).
    Confirmations(u32),
    /// Wait for Solana's "finalized" commitment level.
    Finalized,
    /// Wait for Solana's "confirmed" commitment level.
    Confirmed,
    /// BFT finality — one block = final (Tendermint / X3 consensus).
    Bft,
}

/// A finality requirement for one chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityRequirement {
    pub chain: ChainKind,
    pub level: FinalityLevel,
}

impl FinalityRequirement {
    /// True if the requirement is safe (at or above the chain's recommended floor).
    pub fn is_safe(&self) -> bool {
        match &self.level {
            FinalityLevel::Confirmations(n) => {
                *n >= self.chain.default_safe_confirmations()
            }
            FinalityLevel::Finalized => true,
            FinalityLevel::Confirmed => {
                // Confirmed is weaker than Finalized on Solana; safe for most flows.
                true
            }
            FinalityLevel::Bft => true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Proof requirements
// ─────────────────────────────────────────────────────────────────────────────

/// Type of on-chain event proof required before proceeding.
///
/// Every bridge action must have a `ProofRequirement` or the compiler rejects
/// the intent (X3-INTENT-002). "Trust the relayer" is not acceptable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofKind {
    /// Prove a specific on-chain event fired.
    EventProof {
        /// Event name (e.g. "BridgeLock", "Transfer").
        event: String,
        /// Contract address that must emit the event.
        contract: String,
        /// Minimum confirmations before proof is valid.
        confirmations: u32,
    },
    /// Merkle inclusion proof (token in block/state).
    MerkleProof {
        root_type: String,
    },
    /// Light-client state proof (IBC-style).
    LightClientProof {
        client_id: String,
    },
    /// Validator quorum signature (multisig bridge).
    ValidatorQuorum {
        /// Minimum fraction as basis points (e.g. 7143 = 5/7).
        threshold_bps: u32,
    },
    /// Zero-knowledge proof of execution / state transition.
    ZkProof {
        circuit: String,
    },
    /// Bitcoin SPV proof.
    SpvProof {
        confirmations: u32,
    },
    /// GPU batch receipt (X3-native validator batch).
    GpuBatchReceipt,
}

/// A proof requirement tied to a specific chain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequirement {
    /// Chain where the proof must be collected.
    pub chain: ChainKind,
    /// Logical label for this proof (e.g. "eth.lock_event", "sol.release_receipt").
    pub label: String,
    /// Type and parameters of the proof.
    pub kind: ProofKind,
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety requirements
// ─────────────────────────────────────────────────────────────────────────────

/// All safety requirements declared in a cross-chain intent.
///
/// The compiler checks every field that can introduce risk. Missing fields
/// that are required for bridge or swap operations produce compile errors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Requirements {
    /// Per-chain finality requirements.
    pub finality: Vec<FinalityRequirement>,
    /// Maximum slippage in basis points (e.g. 50 = 0.5%).
    /// Required for any swap operation (X3-INTENT-006).
    pub max_slippage_bps: Option<u32>,
    /// Maximum total fee the user will pay (in source asset base units).
    /// Required for all intents (X3-INTENT-007).
    pub max_total_fee: Option<u128>,
    /// Receiver must equal wallet owner (prevents drain attacks).
    pub require_receiver_is_owner: bool,
    /// On-chain proofs that must be verified before proceeding.
    pub proofs: Vec<ProofRequirement>,
    /// Canonical supply invariant must hold after any mint.
    pub require_canonical_supply_valid: bool,
    /// Route simulation must succeed before execution.
    pub require_route_simulated: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Route specification
// ─────────────────────────────────────────────────────────────────────────────

/// Route optimization objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteObjective {
    /// Choose the path that maximizes what the user receives.
    MaximizeOutput,
    /// Choose the path that minimizes total cost (fees + slippage).
    MinimizeTotalCost,
    /// Minimize bridge/execution time.
    MinimizeLatency,
    /// Best = MaximizeOutput with MinimizeTotalCost as tiebreaker.
    Best,
}

impl Default for RouteObjective {
    fn default() -> Self {
        RouteObjective::Best
    }
}

/// An explicitly allowed or denied venue in the route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VenuePolicy {
    Allow(String),
    Deny(String),
}

/// The route specification: how X3 should find the execution path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSpec {
    pub objective: RouteObjective,
    /// Venues (DEXes, bridges, relayers) that are explicitly allowed.
    pub allow: Vec<String>,
    /// Venues that are explicitly forbidden. `deny bridge.unknown` is default.
    pub deny: Vec<String>,
}

impl Default for RouteSpec {
    fn default() -> Self {
        Self {
            objective: RouteObjective::Best,
            allow: Vec::new(),
            deny: vec!["bridge.unknown".to_string()],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timeout and failure handling
// ─────────────────────────────────────────────────────────────────────────────

/// What to do with funds if execution fails or times out.
///
/// Must be present for any cross-chain operation (X3-INTENT-003, X3-INTENT-004).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureAction {
    /// Return funds to the source chain in the source asset.
    RefundSource,
    /// Return as canonical X3-wrapped asset (e.g. x3.USDC.e).
    RefundX3 {
        asset: AssetRef,
        to: String,
    },
    /// Return a stable asset on the destination chain.
    RefundDestinationStable {
        asset: AssetRef,
        to: String,
    },
    /// Rollback all completed steps if the system supports atomic rollback.
    RollbackIfPossible,
    /// Hold for manual review by the security council.
    Quarantine,
    /// File a claim against the bridge insurance fund.
    InsuranceClaim,
}

/// Timeout and on-failure configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutSpec {
    /// Maximum execution duration in seconds.
    pub timeout_secs: u64,
    /// Priority-ordered list of recovery actions.
    /// The first action that can be executed will be used.
    /// Falls through to the next on failure.
    pub on_fail: Vec<FailureAction>,
}

impl TimeoutSpec {
    /// 30-minute timeout with source refund — the standard safe default.
    pub fn default_30m_source_refund() -> Self {
        Self {
            timeout_secs: 30 * 60,
            on_fail: vec![FailureAction::RefundSource, FailureAction::Quarantine],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Receipt specification
// ─────────────────────────────────────────────────────────────────────────────

/// What to include in the final cross-chain receipt exposed to the explorer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReceiptSpec {
    pub include_route: bool,
    pub include_fees: bool,
    pub include_proofs: bool,
    pub include_state_transitions: bool,
}

impl ReceiptSpec {
    pub fn verbose() -> Self {
        Self {
            include_route: true,
            include_fees: true,
            include_proofs: true,
            include_state_transitions: true,
        }
    }
}
