//! Canonical trading IR and compiled policy.
use alloc::{collections::BTreeSet, string::String};
use serde::{Deserialize, Serialize};
/// Stable typed asset identity carried by trading IR.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetKey {
    pub vm_family: String,
    pub chain: String,
    pub canonical_id: String,
    pub symbol: String,
    pub decimals: u8,
}

/// Stable submission/privacy requirement for economic execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SubmissionProfile {
    Public,
    Protected,
    Private,
}

/// Strength of the state commitment bound into an economic proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StateBindingMode {
    None,
    HostCommitment,
    Exact,
}

/// Stable cost categories understood by economic policy version 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CostKind {
    /// Native execution gas, in the chain's native fee unit.
    Gas,
    /// A venue's explicit liquidity-provider fee.
    LiquidityFee,
    /// A fee charged by a flash-loan / flash-liquidity provider.
    FlashLiquidityFee,
    /// A fee charged by a solver / execution infrastructure provider.
    SolverInfrastructureFee,
    /// A fee charged for generating or verifying a proof.
    ProofFee,
    /// A fee charged for a cross-domain (bridge) transfer.
    CrossDomainFee,
    /// The realized shortfall between a host's quoted expected output and the
    /// swap's actual output, in basis points of the expected output:
    /// `max(0, expected - actual) / expected * 10_000`. Zero when `actual >=
    /// expected`. The host's `quote()` is the reference point, taken
    /// immediately before the swap so this measures *realized* slippage, not
    /// an estimate.
    Slippage,
    /// The adverse price movement caused by the trade's own size against the
    /// venue's liquidity, in basis points. It is the difference between the
    /// spot price before the trade and the execution price the trade receives,
    /// excluding fees. For a constant-product pool with reserves `(x, y)`, a
    /// swap of `dx` has price impact `dx / (x + dx)` before fees. It is
    /// **distinct from slippage**: slippage is measured against a host's
    /// quote, while price impact is measured against the venue's own depth. A
    /// host must report it from the venue's real liquidity, never derive it
    /// from the program.
    PriceImpact,
    /// The value extracted from the trade by adversarial third-party
    /// reordering — front-running and sandwiching — in basis points: the
    /// shortfall between the price the trade would have received without
    /// adversarial reordering and the price it actually received, attributable
    /// to MEV. It is a host/block-context fact (read from mempool or block
    /// ordering) and cannot be derived from the program itself.
    MevLeakage,
}

impl CostKind {
    /// Every cost kind understood by economic policy version 1, in a fixed
    /// order so callers can iterate deterministically.
    pub const ALL: [CostKind; 9] = [
        CostKind::Gas,
        CostKind::LiquidityFee,
        CostKind::FlashLiquidityFee,
        CostKind::SolverInfrastructureFee,
        CostKind::ProofFee,
        CostKind::CrossDomainFee,
        CostKind::Slippage,
        CostKind::PriceImpact,
        CostKind::MevLeakage,
    ];

    /// Canonical, stable name for this cost kind.
    ///
    /// Hosts report committed costs as `CommittedCost { kind: String }`, and
    /// receipts persist those reports. Before this mapping existed the wire
    /// string was never compared against anything, so a policy's
    /// `allowed_cost_kinds` allowlist could not be enforced: any string a
    /// host sent was accepted, and receipts stored the placeholder
    /// `"committed"` instead of the real category. This is the single
    /// source of truth for both directions.
    pub fn as_str(self) -> &'static str {
        match self {
            CostKind::Gas => "gas",
            CostKind::LiquidityFee => "liquidity_fee",
            CostKind::FlashLiquidityFee => "flash_liquidity_fee",
            CostKind::SolverInfrastructureFee => "solver_infrastructure_fee",
            CostKind::ProofFee => "proof_fee",
            CostKind::CrossDomainFee => "cross_domain_fee",
            CostKind::Slippage => "slippage",
            CostKind::PriceImpact => "price_impact",
            CostKind::MevLeakage => "mev_leakage",
        }
    }

    /// Parse a host- or receipt-reported cost kind.
    ///
    /// Returns `None` for anything unrecognized. Callers in the execution and
    /// verification paths must treat `None` as a hard failure rather than a
    /// default: an unclassifiable cost cannot be checked against a policy
    /// allowlist, and silently accepting it would let a host bypass the
    /// allowlist by inventing a category.
    pub fn from_str(name: &str) -> Option<CostKind> {
        CostKind::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

/// Reference to a literal base-unit amount or a prior trading binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ValueRef {
    Literal(u128),
    Binding(String),
}

/// Immutable, compile-time snapshot of the risk policy carried by a trade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledTradingPolicy {
    pub policy_id: String,
    pub policy_version: u16,
    pub chain: String,
    pub max_slippage_bps: u16,
    pub max_gas: u128,
    /// Asset `max_gas` is denominated in. Without this, `max_gas` is an
    /// unenforceable bare number: the VM has no way to know which of a
    /// trade's several accrued costs it's supposed to cap.
    pub max_gas_asset: AssetKey,
    pub max_flash_fee_bps: u16,
    pub deadline_blocks: u64,
    /// Legacy migration field. It must agree with `submission_profile`.
    pub require_private_submission: bool,
    pub minimum_net_profit: Option<u128>,
    /// Asset `minimum_net_profit` is denominated in.
    ///
    /// The same reason `max_gas_asset` exists: without it the floor is an
    /// unenforceable bare number, and the check that consumed it took the
    /// *maximum* delta across every asset — passing a trade that lost on the
    /// settlement asset whenever a side asset gained.
    #[serde(default)]
    pub minimum_net_profit_asset: Option<AssetKey>,
    /// Maximum age, in blocks, of the venue quote a swap is allowed to have
    /// been taken at. `None` means no freshness requirement — the same opt-in
    /// shape as `max_oracle_deviation_bps`.
    ///
    /// Three sibling ceilings were removed here rather than left unenforced:
    /// `max_total_cost` (a bare amount with no denomination asset, hardcoded to
    /// `max_gas`, so it duplicated the gas ceiling and could mean nothing
    /// else), and `max_price_impact_bps` / `max_mev_leakage_bps` (both
    /// hardcoded to `max_slippage_bps`; the host boundary carries no
    /// price-impact or MEV-leakage evidence, so enforcing them would have
    /// required inventing host fields and comparing fabricated numbers). No
    /// source program could set any of the three, and nothing enforced them —
    /// they were read only by `EconomicPolicy::validate_not_weaker_than`,
    /// which compared each one against a copy of itself.
    pub quote_freshness_blocks: Option<u64>,
    pub submission_profile: SubmissionProfile,
    pub state_binding: StateBindingMode,
    pub allowed_cost_kinds: BTreeSet<CostKind>,
    pub allow_mint: bool,
    pub allow_burn: bool,
    /// Oracle-firewall ceiling: maximum allowed disagreement, in basis
    /// points, between the venue's primary quote and any other independent
    /// price source the host reports. `None` means no cross-source check
    /// is required for this trade.
    pub max_oracle_deviation_bps: Option<u16>,
    /// Cross-trade circuit breaker: maximum realized loss, in
    /// `max_cumulative_loss_asset`, the VM instance executing this trade
    /// may have accumulated across every trade it has already committed
    /// before this one is allowed to commit too. `None` means no
    /// cross-trade ceiling is enforced. Always `Some` exactly when
    /// `max_cumulative_loss_asset` is `Some` — the VM has no way to
    /// enforce an amount with no asset to measure it in.
    pub max_cumulative_loss: Option<u128>,
    /// Asset `max_cumulative_loss` is denominated in.
    pub max_cumulative_loss_asset: Option<AssetKey>,
    /// Optional ceiling on the venue-reported price impact, in basis points.
    pub max_price_impact_bps: Option<u16>,
    /// Optional ceiling on the host-reported MEV leakage, in basis points.
    pub max_mev_leakage_bps: Option<u16>,
}

impl CompiledTradingPolicy {
    /// Returns true only when the legacy privacy flag and versioned profile agree.
    pub fn submission_profile_is_consistent(&self) -> bool {
        self.require_private_submission == (self.submission_profile == SubmissionProfile::Private)
    }
}

/// Trading Core v1 IR operations. Field names are stable and explicit for
/// deterministic receipt and artifact hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TradingOperation {
    BeginAtomicTrade {
        trade_id: String,
        policy: CompiledTradingPolicy,
    },
    OpenDebt {
        debt_id: String,
        provider: String,
        asset: AssetKey,
        principal: u128,
    },
    ExecuteSwap {
        binding: String,
        venue: String,
        from: AssetKey,
        to: AssetKey,
        input: ValueRef,
        min_output: u128,
    },
    CloseDebt {
        debt_id: String,
    },
    /// Move a settled amount to another chain through a bridge. No
    /// `min_output`: a bridge transfer is proven by a cryptographic
    /// inclusion/finality proof at settlement time (see
    /// `x3_lang_vm::bridge`), not subject to venue-side slippage the way
    /// a DEX swap quote is.
    Bridge {
        via: String,
        from: AssetKey,
        to: AssetKey,
        input: ValueRef,
        receiver: String,
    },
    AssertMinNetProfit {
        settlement_asset: AssetKey,
        minimum: u128,
    },
    AssertAllDebtsClosed,
    AssertInvariant {
        kind: InvariantKind,
    },
    EmitTradeReceipt,
    CommitAtomicTrade,
    AbortAtomicTrade,
}

/// IR-level mirror of `x3_lang_ast::InvariantKind`. Kept as its own type
/// (matching how `AssetKey`/`CompiledTradingPolicy` mirror their AST
/// counterparts) so bytecode encoding stays stable independent of AST shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InvariantKind {
    /// Every asset touched by the trade nets to a non-negative delta at
    /// commit time.
    Solvent,
}

impl InvariantKind {
    pub fn as_str(self) -> &'static str {
        match self {
            InvariantKind::Solvent => "solvent",
        }
    }
}
