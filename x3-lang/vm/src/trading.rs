//! Atomic trading execution and the capability-controlled host boundary.
//!
//! The VM owns accounting, guard evaluation, atomic rollback, and receipt
//! readiness. Venue and provider behavior is supplied through an explicit
//! [`TradingHost`] capability implementation.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::profit::Profit;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x3_lang_compiler::ir::{AssetKey, CompiledTradingPolicy, CostKind, InvariantKind, TradingOperation, ValueRef};

pub use x3_lang_execution::trading::*;

/// Helper for constructing a deterministic capability manifest in tests or
/// fixture execution.
pub fn fixture_manifest(state_commitment: [u8; 32]) -> CapabilityManifest {
    CapabilityManifest {
        mode: CapabilityMode::Fixture,
        version: "trading-policy-v1".to_string(),
        chain: "ethereum".to_string(),
        state_commitment,
        private_submission: false,
        providers: BTreeSet::new(),
        venues: BTreeSet::new(),
        bridges: BTreeSet::new(),
    }
}

/// Success or failure classification carried by a trade receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeOutcome {
    Success,
    Failure { reason: String },
}

/// Typed amount inside a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedReceiptAmount {
    pub asset: AssetKey,
    pub amount: u128,
}

/// Final per-asset delta committed by the trade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDelta {
    pub asset: AssetKey,
    pub delta: i128,
}

/// Debt repayment status recorded in a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtReceipt {
    pub debt_id: String,
    pub asset: AssetKey,
    pub principal: u128,
    pub fee: u128,
    pub repaid: bool,
}

/// Deterministic, tamper-evident trading receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeReceipt {
    pub format_version: u16,
    pub compiler_version: String,
    pub artifact_hash: [u8; 32],
    pub trade_id: String,
    pub policy_id: String,
    pub state_commitment: [u8; 32],
    pub operations: Vec<TradingOperation>,
    pub costs: Vec<CommittedCost>,
    pub debts: Vec<DebtReceipt>,
    pub deltas: Vec<AssetDelta>,
    /// The quote window of every swap leg in `operations`, in the same order.
    ///
    /// Carried because `quote_freshness` bounds the age of the price a leg traded
    /// against, and an age is only checkable from the two blocks it is the
    /// difference of. Absent for a policy that declares no ceiling, and then
    /// informational — `verify_receipt_economics` refuses a receipt whose leg was
    /// priced outside the ceiling (TICKET-017).
    pub legs: Vec<LegQuoteWindow>,
    pub realized_net_profit: Option<TypedReceiptAmount>,
    pub outcome: TradeOutcome,
    pub receipt_hash: [u8; 32],
    pub attestation: Option<ReceiptAttestation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptAttestation {
    pub key_id: String,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

/// Receipt encoding and validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptError {
    Encoding(String),
    HashMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    OpenDebtInSuccessfulReceipt(String),
    ProfitInFailedReceipt,
    EmptyOperationList,
    EconomicReplayMismatch(String),
    MissingAttestation,
    UntrustedAttestor(String),
    /// The attestor is in the key registry and has been revoked: a key that may be compromised,
    /// so nothing it signed is accepted, including receipts signed before the revocation.
    RevokedAttestor(String),
    InvalidAttestation,
    /// This exact receipt (by `receipt_hash`) has already been accepted by
    /// this `ReceiptReplayLedger` once before. Distinct from
    /// `EconomicReplayMismatch`, which is about re-deriving a receipt's
    /// reported numbers from its own operations — this is about the same
    /// valid, correctly signed receipt being presented for settlement a
    /// second time.
    ReceiptAlreadySettled([u8; 32]),
    /// The receipt's realized net for `asset` is below the profit floor its operation sequence
    /// states. `AssertMinNetProfit` is the source-level floor the trade ran under; a receipt is
    /// checked against it rather than against its own arithmetic alone.
    ProfitBelowCompiledFloor {
        asset: AssetKey,
        floor: u128,
        realized: i128,
    },
    /// The compiled policy states a profit floor without the asset it is denominated in, so the
    /// floor cannot be read against any realized figure. Fail closed rather than assume an asset.
    ProfitFloorWithoutAsset,
    /// A swap leg the receipt records carries no realized slippage figure, so the artifact's
    /// `max_slippage_bps` ceiling cannot be re-derived for it. A receipt written before format
    /// version 3 is such a receipt.
    MissingRealizedSlippage {
        venue: String,
    },
    /// The receipt's realized slippage for a leg exceeds the artifact's `max_slippage_bps`.
    SlippageExceeded {
        venue: String,
        ceiling_bps: u16,
        actual_bps: u128,
    },
    /// The artifact declares `max_oracle_deviation_bps`, and the receipt's window for a leg
    /// carries no oracle deviation (a receipt before format version 4, or one that dropped it).
    MissingOracleDeviation {
        venue: String,
    },
    /// The receipt's largest oracle deviation for a leg exceeds the artifact's ceiling.
    OracleDeviationExceeded {
        venue: String,
        ceiling_bps: u16,
        actual_bps: u128,
    },
    /// The receipt's costs in the policy's gas asset sum past the artifact's `max_gas`.
    GasCeilingExceeded {
        ceiling: u128,
        actual: u128,
    },
    /// The receipt's fee on a debt exceeds the artifact's `max_flash_fee_bps`.
    FeeCeilingExceeded {
        debt_id: String,
        ceiling_bps: u16,
        actual_bps: u128,
    },
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encoding(message) => write!(f, "receipt encoding error: {message}"),
            Self::HashMismatch { expected, actual } => {
                write!(f, "receipt hash mismatch: expected {expected:?}, actual {actual:?}")
            }
            Self::OpenDebtInSuccessfulReceipt(debt) => {
                write!(f, "successful receipt contains open debt '{debt}'")
            }
            Self::ProfitInFailedReceipt => write!(f, "failed receipt must not report realized profit"),
            Self::EmptyOperationList => write!(f, "receipt must contain at least one operation"),
            Self::EconomicReplayMismatch(message) => write!(f, "receipt economic replay mismatch: {message}"),
            Self::MissingAttestation => write!(f, "receipt is missing a trusted attestation"),
            Self::UntrustedAttestor(key_id) => write!(f, "receipt attestor '{key_id}' is not trusted"),
            Self::RevokedAttestor(key_id) => {
                write!(f, "receipt attestor '{key_id}' is revoked in the key registry")
            }
            Self::InvalidAttestation => write!(f, "receipt attestation signature is invalid"),
            Self::ReceiptAlreadySettled(hash) => {
                write!(f, "receipt {hash:?} has already been settled once")
            }
            Self::ProfitBelowCompiledFloor { asset, floor, realized } => write!(
                f,
                "receipt's realized net for {} is {realized}, below the compiled profit floor {floor}",
                asset.symbol
            ),
            Self::ProfitFloorWithoutAsset => write!(
                f,
                "the compiled policy states a profit floor without the asset it is denominated in"
            ),
            Self::MissingOracleDeviation { venue } => write!(
                f,
                "receipt records no oracle deviation for the leg on venue '{venue}', so the \
                 compiled max_oracle_deviation_bps ceiling cannot be re-derived"
            ),
            Self::OracleDeviationExceeded {
                venue,
                ceiling_bps,
                actual_bps,
            } => write!(
                f,
                "leg on venue '{venue}' reports an oracle deviation of {actual_bps} bps, exceeding \
                 the compiled ceiling of {ceiling_bps} bps"
            ),
            Self::GasCeilingExceeded { ceiling, actual } => write!(
                f,
                "receipt costs in the policy's gas asset total {actual}, exceeding the compiled \
                 max_gas ceiling {ceiling}"
            ),
            Self::MissingRealizedSlippage { venue } => write!(
                f,
                "receipt records no realized slippage for the leg on venue '{venue}', so the \
                 compiled max_slippage_bps ceiling cannot be re-derived"
            ),
            Self::SlippageExceeded {
                venue,
                ceiling_bps,
                actual_bps,
            } => write!(
                f,
                "leg on venue '{venue}' slipped {actual_bps} bps, exceeding the compiled \
                 max_slippage_bps ceiling of {ceiling_bps} bps"
            ),
            Self::FeeCeilingExceeded {
                debt_id,
                ceiling_bps,
                actual_bps,
            } => write!(
                f,
                "debt '{debt_id}' was charged {actual_bps} bps, exceeding the compiled \
                 max_flash_fee_bps ceiling of {ceiling_bps} bps"
            ),
        }
    }
}

impl Error for ReceiptError {}

/// Canonical receipt bytes with `receipt_hash` zeroed.
pub fn canonical_receipt_bytes(receipt: &TradeReceipt) -> Result<Vec<u8>, ReceiptError> {
    let mut canonical = receipt.clone();
    canonical.receipt_hash = [0u8; 32];
    canonical.attestation = None;
    serde_json::to_vec(&canonical).map_err(|err| ReceiptError::Encoding(err.to_string()))
}

/// Compute the deterministic receipt hash over canonical bytes.
pub fn compute_receipt_hash(receipt: &TradeReceipt) -> Result<[u8; 32], ReceiptError> {
    let bytes = canonical_receipt_bytes(receipt)?;
    let digest = Sha256::digest(bytes);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    Ok(hash)
}

/// Fill in a receipt's hash from its canonical encoding.
pub fn finalize_receipt(mut receipt: TradeReceipt) -> Result<TradeReceipt, ReceiptError> {
    receipt.receipt_hash = compute_receipt_hash(&receipt)?;
    Ok(receipt)
}

/// Verify a receipt hash and its accounting/tamper invariants.
pub fn verify_receipt(receipt: &TradeReceipt) -> Result<(), ReceiptError> {
    if receipt.operations.is_empty() {
        return Err(ReceiptError::EmptyOperationList);
    }
    let actual = compute_receipt_hash(receipt)?;
    if actual != receipt.receipt_hash {
        return Err(ReceiptError::HashMismatch {
            expected: receipt.receipt_hash,
            actual,
        });
    }
    match &receipt.outcome {
        TradeOutcome::Failure { .. } => {
            if receipt.realized_net_profit.is_some() {
                return Err(ReceiptError::ProfitInFailedReceipt);
            }
        }
        TradeOutcome::Success => {
            if let Some(debt) = receipt.debts.iter().find(|debt| !debt.repaid) {
                return Err(ReceiptError::OpenDebtInSuccessfulReceipt(debt.debt_id.clone()));
            }
        }
    }
    Ok(())
}

pub fn verify_receipt_economics(receipt: &TradeReceipt) -> Result<(), ReceiptError> {
    let first = receipt.operations.first().ok_or(ReceiptError::EmptyOperationList)?;
    let (trade_id, compiled_policy) = match first {
        TradingOperation::BeginAtomicTrade { trade_id, policy } => (trade_id, policy),
        _ => {
            return Err(ReceiptError::EconomicReplayMismatch(
                "operation sequence does not begin with BeginAtomicTrade".to_string(),
            ))
        }
    };
    if trade_id != &receipt.trade_id {
        return Err(ReceiptError::EconomicReplayMismatch(
            "receipt trade_id does not match compiled operation sequence".to_string(),
        ));
    }
    if compiled_policy.policy_id != receipt.policy_id {
        return Err(ReceiptError::EconomicReplayMismatch(
            "receipt policy_id does not match compiled operation sequence".to_string(),
        ));
    }
    if !matches!(receipt.operations.last(), Some(TradingOperation::CommitAtomicTrade)) {
        return Err(ReceiptError::EconomicReplayMismatch(
            "successful receipt operation sequence must terminate in CommitAtomicTrade".to_string(),
        ));
    }

    let mut open_debts: BTreeMap<String, (AssetKey, u128)> = BTreeMap::new();
    let mut closed_debts: BTreeSet<String> = BTreeSet::new();
    let mut saw_profit_guard = false;
    let mut saw_all_debts_guard = false;
    let mut saw_receipt_emit = false;
    // The floors this receipt's own operation sequence states, in the order it states them. Each
    // is enforced against the figures below; recording only that a guard was *present* was the
    // hole that let a receipt net below the floor it claims to have run under (TICKET-150).
    let mut profit_floors: Vec<(AssetKey, u128)> = Vec::new();

    for operation in &receipt.operations {
        match operation {
            TradingOperation::BeginAtomicTrade { .. } => {}
            TradingOperation::OpenDebt {
                debt_id,
                asset,
                principal,
                ..
            } => {
                if open_debts
                    .insert(debt_id.clone(), (asset.clone(), *principal))
                    .is_some()
                    || closed_debts.contains(debt_id)
                {
                    return Err(ReceiptError::EconomicReplayMismatch(format!(
                        "debt '{debt_id}' opens more than once"
                    )));
                }
            }
            TradingOperation::CloseDebt { debt_id } => {
                if open_debts.remove(debt_id).is_none() || !closed_debts.insert(debt_id.clone()) {
                    return Err(ReceiptError::EconomicReplayMismatch(format!(
                        "debt '{debt_id}' closes without a matching open debt"
                    )));
                }
            }
            TradingOperation::AssertMinNetProfit {
                settlement_asset,
                minimum,
            } => {
                saw_profit_guard = true;
                profit_floors.push((settlement_asset.clone(), *minimum));
            }
            TradingOperation::AssertAllDebtsClosed => {
                if !open_debts.is_empty() {
                    return Err(ReceiptError::EconomicReplayMismatch(
                        "all-debts guard appears while debts remain open".to_string(),
                    ));
                }
                saw_all_debts_guard = true;
            }
            TradingOperation::AssertInvariant { .. } => {}
            TradingOperation::EmitTradeReceipt => saw_receipt_emit = true,
            TradingOperation::CommitAtomicTrade => {}
            TradingOperation::AbortAtomicTrade => {
                return Err(ReceiptError::EconomicReplayMismatch(
                    "successful receipt cannot contain AbortAtomicTrade".to_string(),
                ))
            }
            TradingOperation::ExecuteSwap { .. } => {}
            TradingOperation::Bridge { .. } => {}
        }
    }

    if !open_debts.is_empty() {
        return Err(ReceiptError::EconomicReplayMismatch(
            "receipt operation sequence leaves debt open".to_string(),
        ));
    }
    if !saw_profit_guard || !saw_all_debts_guard || !saw_receipt_emit {
        return Err(ReceiptError::EconomicReplayMismatch(
            "receipt operation sequence is missing required final guards/receipt emission".to_string(),
        ));
    }

    // A policy-level floor is a second way the same requirement reaches the artifact (the source
    // can state `min_profit` in the risk policy instead of a `require net_profit` guard), and the
    // VM enforces it at commit. Both are read here, so a receipt cannot report a net that satisfies
    // the guard it carries while missing the floor its policy states.
    if let Some(minimum) = compiled_policy.minimum_net_profit {
        let asset = compiled_policy
            .minimum_net_profit_asset
            .clone()
            .ok_or(ReceiptError::ProfitFloorWithoutAsset)?;
        profit_floors.push((asset, minimum));
    }

    // Quote freshness, re-derived rather than trusted. The receipt carries the two
    // blocks each leg was priced between and the compiled policy carries the
    // ceiling, so a receipt whose leg was priced outside the ceiling is refused
    // here even though execution let it through — which is the point of replay: a
    // receipt is checked against the policy, not against the run that produced it.
    //
    // The windows are matched to the `ExecuteSwap` operations in order, and the
    // counts have to agree in both directions. A receipt that dropped a window
    // would otherwise be replayed as though that leg had no quote to age.
    if let Some(ceiling_blocks) = compiled_policy.quote_freshness_blocks {
        let mut windows = receipt.legs.iter();
        for operation in &receipt.operations {
            let TradingOperation::ExecuteSwap { venue, .. } = operation else {
                continue;
            };
            let Some(window) = windows.next() else {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "receipt records no quote window for the executed leg on venue '{venue}', so the \
                     age the compiled quote_freshness ceiling of {ceiling_blocks} blocks bounds \
                     cannot be re-derived"
                )));
            };
            if &window.venue != venue {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "receipt's quote window names venue '{}' where the executed leg is on '{venue}'",
                    window.venue
                )));
            }
            let age_blocks = window.age_blocks();
            if age_blocks > ceiling_blocks {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "leg on venue '{venue}' was priced {age_blocks} blocks before it executed, \
                     exceeding the compiled quote_freshness ceiling of {ceiling_blocks} blocks"
                )));
            }
        }
        if let Some(unused) = windows.next() {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "receipt carries a quote window for venue '{}' with no executed leg to age",
                unused.venue
            )));
        }
    }

    if let Some(ceiling) = compiled_policy.max_price_impact_bps {
        let mut windows = receipt.legs.iter();
        for operation in &receipt.operations {
            let TradingOperation::ExecuteSwap { venue, .. } = operation else {
                continue;
            };
            let Some(window) = windows.next() else {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "receipt records no risk window for the executed leg on venue '{venue}', so \
                     max_price_impact cannot be re-derived"
                )));
            };
            let measurement = window.price_impact.as_ref().ok_or_else(|| {
                ReceiptError::EconomicReplayMismatch(format!(
                    "leg on venue '{venue}' has no max_price_impact measurement, but the compiled policy \
                     requires one"
                ))
            })?;
            if measurement.bps > ceiling as u128 {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "leg on venue '{venue}' reports max_price_impact {} bps, exceeding the compiled ceiling \
                     {ceiling} bps",
                    measurement.bps
                )));
            }
        }
    }
    if let Some(ceiling) = compiled_policy.max_mev_leakage_bps {
        let mut windows = receipt.legs.iter();
        for operation in &receipt.operations {
            let TradingOperation::ExecuteSwap { venue, .. } = operation else {
                continue;
            };
            let Some(window) = windows.next() else {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "receipt records no risk window for the executed leg on venue '{venue}', so \
                     max_mev_leakage cannot be re-derived"
                )));
            };
            let measurement = window.mev_leakage.as_ref().ok_or_else(|| {
                ReceiptError::EconomicReplayMismatch(format!(
                    "leg on venue '{venue}' has no max_mev_leakage measurement, but the compiled policy \
                     requires one"
                ))
            })?;
            if measurement.bps > ceiling as u128 {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "leg on venue '{venue}' reports max_mev_leakage {} bps, exceeding the compiled ceiling \
                     {ceiling} bps",
                    measurement.bps
                )));
            }
        }
    }

    // The ceiling on what a leg may lose against its own quote. Only the run can measure it, which
    // is why the window carries the figure; a window without one (a receipt written before format
    // version 3) leaves the ceiling unverifiable, and that is a refusal rather than a pass.
    {
        let mut windows = receipt.legs.iter();
        for operation in &receipt.operations {
            let TradingOperation::ExecuteSwap { venue, .. } = operation else {
                continue;
            };
            let Some(window) = windows.next() else {
                return Err(ReceiptError::EconomicReplayMismatch(format!(
                    "receipt records no window for the executed leg on venue '{venue}', so \
                     max_slippage_bps cannot be re-derived"
                )));
            };
            let actual_bps = window
                .realized_slippage_bps
                .ok_or_else(|| ReceiptError::MissingRealizedSlippage { venue: venue.clone() })?;
            if actual_bps > compiled_policy.max_slippage_bps as u128 {
                return Err(ReceiptError::SlippageExceeded {
                    venue: venue.clone(),
                    ceiling_bps: compiled_policy.max_slippage_bps,
                    actual_bps,
                });
            }
        }
    }

    // The oracle firewall, re-derived. Only the run can consult the price sources, so the window
    // carries the largest deviation the run measured; a window without it (format version 3 or
    // earlier) leaves a declared ceiling unverifiable, which is a refusal (X3-LANG-002).
    if let Some(ceiling_bps) = compiled_policy.max_oracle_deviation_bps {
        let mut windows = receipt.legs.iter();
        for operation in &receipt.operations {
            let TradingOperation::ExecuteSwap { venue, .. } = operation else {
                continue;
            };
            let actual_bps = windows
                .next()
                .and_then(|window| window.oracle_deviation_bps)
                .ok_or_else(|| ReceiptError::MissingOracleDeviation { venue: venue.clone() })?;
            if actual_bps > ceiling_bps as u128 {
                return Err(ReceiptError::OracleDeviationExceeded {
                    venue: venue.clone(),
                    ceiling_bps,
                    actual_bps,
                });
            }
        }
    }

    // The gas ceiling, re-derived. Every cost the run accrued is on the receipt with its asset
    // (the same ledger the VM's per-asset rollup is summed from), so the total in the policy's gas
    // asset is a figure the receipt fully determines (X3-LANG-002).
    let gas_total = receipt
        .costs
        .iter()
        .filter(|cost| cost.asset == compiled_policy.max_gas_asset)
        .try_fold(0u128, |total, cost| total.checked_add(cost.amount))
        .ok_or_else(|| ReceiptError::EconomicReplayMismatch("gas cost overflow".to_string()))?;
    if gas_total > compiled_policy.max_gas {
        return Err(ReceiptError::GasCeilingExceeded {
            ceiling: compiled_policy.max_gas,
            actual: gas_total,
        });
    }

    let mut expected_debts: BTreeMap<String, (&AssetKey, u128)> = BTreeMap::new();
    for operation in &receipt.operations {
        if let TradingOperation::OpenDebt {
            debt_id,
            asset,
            principal,
            ..
        } = operation
        {
            expected_debts.insert(debt_id.clone(), (asset, *principal));
        }
    }
    for debt in &receipt.debts {
        let Some((asset, principal)) = expected_debts.get(&debt.debt_id) else {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "receipt reports unknown debt '{}'",
                debt.debt_id
            )));
        };
        if *asset != &debt.asset || *principal != debt.principal {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "receipt debt '{}' does not match compiled operation",
                debt.debt_id
            )));
        }
        if debt.repaid && debt.fee > debt.principal {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "receipt debt '{}' reports an implausible fee above principal",
                debt.debt_id
            )));
        }
        // The flash fee the run paid is in the receipt already (principal and fee on the debt
        // record), so the artifact's ceiling on it is re-derivable without the host. Computed
        // through the same helper execution used, so the two figures cannot drift.
        if debt.repaid {
            let actual_bps = basis_points_of(debt.fee, debt.principal)
                .ok_or_else(|| ReceiptError::EconomicReplayMismatch("fee conversion overflow".to_string()))?;
            if actual_bps > compiled_policy.max_flash_fee_bps as u128 {
                return Err(ReceiptError::FeeCeilingExceeded {
                    debt_id: debt.debt_id.clone(),
                    ceiling_bps: compiled_policy.max_flash_fee_bps,
                    actual_bps,
                });
            }
        }
    }
    if expected_debts.len() != receipt.debts.len() {
        return Err(ReceiptError::EconomicReplayMismatch(
            "receipt debt set does not match compiled operation sequence".to_string(),
        ));
    }

    // The reported deltas are read where they are needed (`realized_net`) rather than accumulated
    // into a map first: the previous accumulator added each cost to a total nothing read, which
    // invited a reader to believe costs were being counted twice when in fact `TradingState`'s
    // `accrue_cost` had already netted them into the deltas.
    for cost in &receipt.costs {
        // A cost the policy cannot classify is a cost the policy cannot
        // bound, so an unknown category is a replay failure rather than a
        // value to be carried through the totals.
        let kind = CostKind::from_str(&cost.kind).ok_or_else(|| {
            ReceiptError::EconomicReplayMismatch(format!("receipt reports unknown cost kind '{}'", cost.kind))
        })?;
        if !compiled_policy.allowed_cost_kinds.contains(&kind) {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "receipt reports cost kind '{}', which the compiled policy's allowed_cost_kinds allowlist does not permit",
                kind.as_str()
            )));
        }
    }

    if let Some(profit) = &receipt.realized_net_profit {
        let replayed = receipt
            .deltas
            .iter()
            .find(|delta| delta.asset == profit.asset)
            .map(|delta| u128::try_from(delta.delta.max(0)).unwrap_or(0))
            .unwrap_or(0);
        if replayed != profit.amount {
            return Err(ReceiptError::EconomicReplayMismatch(format!(
                "reported profit {} does not equal replayed delta {} for {}",
                profit.amount, replayed, profit.asset.symbol
            )));
        }
    }

    for debt in &receipt.debts {
        if matches!(receipt.outcome, TradeOutcome::Success) && !debt.repaid {
            return Err(ReceiptError::OpenDebtInSuccessfulReceipt(debt.debt_id.clone()));
        }
    }

    // The floors the artifact states, each against the net it is denominated in. A receipt that
    // reports no delta for that asset realized nothing there — that is 0, and 0 is below any
    // positive floor — so a missing figure is refused rather than read as "no evidence, pass".
    if matches!(receipt.outcome, TradeOutcome::Success) {
        for (asset, floor) in &profit_floors {
            let realized = realized_net(&receipt.deltas, asset)?;
            let floor_i128 = i128::try_from(*floor)
                .map_err(|_| ReceiptError::EconomicReplayMismatch("profit floor conversion overflow".to_string()))?;
            if realized < floor_i128 {
                return Err(ReceiptError::ProfitBelowCompiledFloor {
                    asset: asset.clone(),
                    floor: *floor,
                    realized,
                });
            }
        }
    }

    Ok(())
}

/// The net a receipt reports for one asset, summed across the deltas that name it.
///
/// A `TradingState` nets costs into `net_deltas` as they accrue, so a receipt's deltas are already
/// net of the cost ledger beside them and this is the realized figure a floor is checked against.
fn realized_net(deltas: &[AssetDelta], asset: &AssetKey) -> Result<i128, ReceiptError> {
    let mut total: i128 = 0;
    for delta in deltas.iter().filter(|delta| &delta.asset == asset) {
        total = total
            .checked_add(delta.delta)
            .ok_or_else(|| ReceiptError::EconomicReplayMismatch("delta overflow".to_string()))?;
    }
    Ok(total)
}

pub fn sign_receipt(
    mut receipt: TradeReceipt,
    key_id: &str,
    signing_key: &SigningKey,
) -> Result<TradeReceipt, ReceiptError> {
    receipt.attestation = None;
    receipt.receipt_hash = compute_receipt_hash(&receipt)?;
    let signature = signing_key.sign(&receipt.receipt_hash);
    receipt.attestation = Some(ReceiptAttestation {
        key_id: key_id.to_string(),
        public_key: signing_key.verifying_key().to_bytes(),
        signature: signature.to_bytes().to_vec(),
    });
    Ok(receipt)
}

pub fn verify_receipt_attestation(
    receipt: &TradeReceipt,
    trusted_keys: &BTreeMap<String, [u8; 32]>,
) -> Result<(), ReceiptError> {
    let attestation = receipt.attestation.as_ref().ok_or(ReceiptError::MissingAttestation)?;
    let trusted = trusted_keys
        .get(&attestation.key_id)
        .ok_or_else(|| ReceiptError::UntrustedAttestor(attestation.key_id.clone()))?;
    if trusted != &attestation.public_key {
        return Err(ReceiptError::UntrustedAttestor(attestation.key_id.clone()));
    }
    let verifying_key = VerifyingKey::from_bytes(trusted).map_err(|_| ReceiptError::InvalidAttestation)?;
    let signature_bytes: [u8; 64] = attestation
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| ReceiptError::InvalidAttestation)?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(&receipt.receipt_hash, &signature)
        .map_err(|_| ReceiptError::InvalidAttestation)
}

pub fn verify_receipt_trusted(
    receipt: &TradeReceipt,
    trusted_keys: &BTreeMap<String, [u8; 32]>,
) -> Result<(), ReceiptError> {
    verify_receipt(receipt)?;
    verify_receipt_economics(receipt)?;
    verify_receipt_attestation(receipt, trusted_keys)
}

/// Where a receipt-signing key stands in its lifecycle.
///
/// Receipts carry no signing time a verifier could trust, so the lifecycle is by state, not by
/// date: rotating a key moves it from `Active` to `Retired`, and it keeps verifying what it signed
/// (those receipts are genuine); revoking a key, for compromise, refuses everything it ever signed,
/// because a forger holding it can produce receipts indistinguishable from the old ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKeyStatus {
    /// Signs new receipts and verifies.
    Active,
    /// Rotated out: verifies the receipts it signed, must not sign new ones.
    Retired,
    /// Compromised or withdrawn: verifies nothing.
    Revoked,
}

/// One entry of a receipt key registry, as written in its JSON file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredReceiptKey {
    pub key_id: String,
    /// The ed25519 public key, 64 hex characters.
    pub public_key: String,
    pub status: ReceiptKeyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptKeyRegistryFile {
    keys: Vec<RegisteredReceiptKey>,
}

/// The keys a settlement trusts to sign receipts, and where each is in its lifecycle
/// (X3-LANG-003: `--trusted` named keys per invocation, with no record of which keys were
/// retired or revoked, so a compromised key stayed trusted wherever an old invocation named it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiptKeyRegistry {
    keys: BTreeMap<String, ([u8; 32], ReceiptKeyStatus)>,
}

impl ReceiptKeyRegistry {
    /// Build a registry, refusing one a verifier could misread: an empty or duplicated key id,
    /// a key that is not 32 bytes of hex, one public key under two ids (a revocation of one id
    /// would leave the key trusted under the other), or no active key at all.
    pub fn from_entries(entries: Vec<RegisteredReceiptKey>) -> Result<Self, String> {
        let mut keys = BTreeMap::new();
        let mut seen_public: BTreeMap<[u8; 32], String> = BTreeMap::new();
        for entry in entries {
            if entry.key_id.trim().is_empty() {
                return Err("a registry entry names no key id".to_string());
            }
            let public_key = decode_public_key_hex(&entry.public_key)
                .map_err(|reason| format!("key '{}': {reason}", entry.key_id))?;
            if let Some(other) = seen_public.insert(public_key, entry.key_id.clone()) {
                return Err(format!(
                    "keys '{other}' and '{}' are the same public key: revoking one id would leave \
                     the key trusted under the other",
                    entry.key_id
                ));
            }
            if keys.insert(entry.key_id.clone(), (public_key, entry.status)).is_some() {
                return Err(format!("key id '{}' is listed twice", entry.key_id));
            }
        }
        if !keys.values().any(|(_, status)| *status == ReceiptKeyStatus::Active) {
            return Err("the registry has no active key".to_string());
        }
        Ok(Self { keys })
    }

    /// Parse a registry file: `{"keys": [{"key_id", "public_key", "status"}, ...]}`.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: ReceiptKeyRegistryFile = serde_json::from_str(json).map_err(|e| format!("key registry: {e}"))?;
        Self::from_entries(file.keys)
    }

    pub fn status(&self, key_id: &str) -> Option<ReceiptKeyStatus> {
        self.keys.get(key_id).map(|(_, status)| *status)
    }

    /// The public key the registry lists under `key_id`, whatever its status.
    pub fn public_key(&self, key_id: &str) -> Option<[u8; 32]> {
        self.keys.get(key_id).map(|(key, _)| *key)
    }

    /// Whether `key_id` may sign a new receipt: only an active key may.
    pub fn may_sign(&self, key_id: &str) -> bool {
        self.status(key_id) == Some(ReceiptKeyStatus::Active)
    }

    /// Everything `verify_receipt_trusted` checks, with the trusted set taken from this registry:
    /// an active or retired signer verifies, a revoked one is refused by name.
    pub fn verify(&self, receipt: &TradeReceipt) -> Result<(), ReceiptError> {
        verify_receipt(receipt)?;
        verify_receipt_economics(receipt)?;
        let attestation = receipt.attestation.as_ref().ok_or(ReceiptError::MissingAttestation)?;
        if self.status(&attestation.key_id) == Some(ReceiptKeyStatus::Revoked) {
            return Err(ReceiptError::RevokedAttestor(attestation.key_id.clone()));
        }
        // Revoked signers were refused above; every remaining listed key is a verifier.
        let verifying: BTreeMap<String, [u8; 32]> = self.keys.iter().map(|(id, (key, _))| (id.clone(), *key)).collect();
        verify_receipt_attestation(receipt, &verifying)
    }
}

/// Domain separator for a registry signature, so it can never be mistaken for a receipt's.
pub const REGISTRY_SIGNATURE_DOMAIN: &[u8] = b"x3-receipt-key-registry-v1";

/// A detached signature over a receipt key registry file, written next to it as
/// `<registry>.sig.json` (X3-LANG-003).
///
/// The registry says which keys may sign receipts; without this, anyone who could edit the file
/// could add a key of their own and every verifier reading it would trust that key. The signature
/// covers the file's **exact bytes** (not a re-serialization, so no two encodings of one registry
/// can disagree about what was signed) and is checked against a root key the *verifier* names —
/// the `root_public_key` carried here is a label for a reader, never a trust decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySignature {
    /// The signing root's ed25519 public key, 64 hex characters.
    pub root_public_key: String,
    /// sha256 of the registry file's bytes, 64 hex characters.
    pub registry_sha256: String,
    /// ed25519 signature over [`registry_signing_digest`], 128 hex characters.
    pub signature: String,
}

/// The digest a registry root signs: the domain, then the registry file's sha256.
pub fn registry_signing_digest(registry_bytes: &[u8]) -> [u8; 32] {
    let file_digest: [u8; 32] = Sha256::digest(registry_bytes).into();
    let mut hasher = Sha256::new();
    hasher.update(REGISTRY_SIGNATURE_DOMAIN);
    hasher.update(file_digest);
    hasher.finalize().into()
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn bytes_of_hex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if !hex.is_ascii() || hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// Sign a registry file's bytes with `root`.
pub fn sign_registry(registry_bytes: &[u8], root: &SigningKey) -> RegistrySignature {
    RegistrySignature {
        root_public_key: hex_of(&root.verifying_key().to_bytes()),
        registry_sha256: hex_of(&Sha256::digest(registry_bytes)),
        signature: hex_of(&root.sign(&registry_signing_digest(registry_bytes)).to_bytes()),
    }
}

/// Check `signature` over `registry_bytes` against the root key the verifier trusts. Every refusal
/// names what broke: another root, another file, or a signature that does not verify.
pub fn verify_registry_signature(
    registry_bytes: &[u8],
    signature: &RegistrySignature,
    trusted_root: &[u8; 32],
) -> Result<(), String> {
    let claimed_root = bytes_of_hex(&signature.root_public_key)
        .ok_or_else(|| "registry signature: root_public_key is not hex".to_string())?;
    if claimed_root.as_slice() != trusted_root {
        return Err(format!(
            "the registry is signed by root {}, not the root you trust ({})",
            signature.root_public_key,
            hex_of(trusted_root)
        ));
    }
    let file_digest = hex_of(&Sha256::digest(registry_bytes));
    if !signature.registry_sha256.eq_ignore_ascii_case(&file_digest) {
        return Err("the registry signature is about a different file: its sha256 does not match".to_string());
    }
    let key = VerifyingKey::from_bytes(trusted_root)
        .map_err(|_| "the trusted registry root is not a valid ed25519 key".to_string())?;
    let bytes = bytes_of_hex(&signature.signature)
        .and_then(|bytes| <[u8; 64]>::try_from(bytes.as_slice()).ok())
        .ok_or_else(|| "registry signature: signature is not 64 bytes of hex".to_string())?;
    key.verify(&registry_signing_digest(registry_bytes), &Signature::from_bytes(&bytes))
        .map_err(|_| "the registry signature does not verify under the trusted root".to_string())
}

impl ReceiptKeyRegistry {
    /// Parse a registry file only after its detached signature verifies under `trusted_root`.
    pub fn from_signed_json(
        registry_bytes: &[u8],
        signature: &RegistrySignature,
        trusted_root: &[u8; 32],
    ) -> Result<Self, String> {
        verify_registry_signature(registry_bytes, signature, trusted_root)?;
        let json = core::str::from_utf8(registry_bytes).map_err(|_| "key registry: not UTF-8".to_string())?;
        Self::from_json(json)
    }
}

fn decode_public_key_hex(hex: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    // Checked before slicing: a multi-byte character would put a byte index off a char boundary.
    if !hex.is_ascii() {
        return Err("public key is not hex".to_string());
    }
    if hex.len() != 64 {
        return Err(format!("public key must be 64 hex characters, got {}", hex.len()));
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| "public key is not hex".to_string())?;
    }
    Ok(out)
}

/// Replay protection for receipt settlement.
///
/// `verify_receipt_trusted` alone checks that a receipt is well-formed,
/// economically consistent, and signed by a trusted key — but it is a pure
/// function with no memory: the identical valid, correctly signed receipt
/// verifies successfully every single time it's presented. A settlement
/// service that used `verify_receipt_trusted` as its sole admission check
/// would accept (and presumably act on) the same trade's receipt twice.
///
/// This ledger closes that gap: it wraps `verify_receipt_trusted` with a
/// record of every `receipt_hash` already accepted, so the second
/// presentation of an identical receipt is rejected even though every
/// other check about it still passes. It is deliberately not persisted or
/// distributed by this crate — a real settlement service is expected to
/// back this (or an equivalent check) with whatever durable, possibly
/// shared storage its deployment actually needs; this type documents and
/// enforces the invariant in-process.
#[derive(Debug, Clone, Default)]
pub struct ReceiptReplayLedger {
    seen: BTreeSet<[u8; 32]>,
}

impl ReceiptReplayLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify `receipt` exactly as `verify_receipt_trusted` does, and
    /// additionally reject it if this ledger has already accepted the same
    /// `receipt_hash`. On success, the hash is recorded, so a later replay
    /// of the identical receipt is rejected even though it would otherwise
    /// still pass every other check.
    pub fn verify_and_record(
        &mut self,
        receipt: &TradeReceipt,
        trusted_keys: &BTreeMap<String, [u8; 32]>,
    ) -> Result<(), ReceiptError> {
        verify_receipt_trusted(receipt, trusted_keys)?;
        if !self.seen.insert(receipt.receipt_hash) {
            return Err(ReceiptError::ReceiptAlreadySettled(receipt.receipt_hash));
        }
        Ok(())
    }

    /// Whether `receipt_hash` has already been accepted by this ledger.
    pub fn has_settled(&self, receipt_hash: &[u8; 32]) -> bool {
        self.seen.contains(receipt_hash)
    }
}

/// Build and finalize a canonical receipt from an execution result.
#[allow(clippy::too_many_arguments)]
pub fn build_receipt(
    compiler_version: &str,
    artifact_hash: [u8; 32],
    trade_id: &str,
    policy_id: &str,
    state_commitment: [u8; 32],
    operations: &[TradingOperation],
    state: &TradingState,
    settlement_asset: Option<&AssetKey>,
    outcome: TradeOutcome,
) -> Result<TradeReceipt, ReceiptError> {
    // The ledger, not the per-asset rollup: the rollup cannot say which
    // category each cost belonged to, and an independent verifier needs that
    // category to check the cost against the policy's
    // `allowed_cost_kinds` allowlist.
    let costs = state.cost_ledger.clone();
    let mut debts: Vec<DebtReceipt> = state
        .closed_debt_records
        .iter()
        .map(|(debt_id, record)| DebtReceipt {
            debt_id: debt_id.clone(),
            asset: record.asset.clone(),
            principal: record.principal,
            fee: record.fee,
            repaid: true,
        })
        .collect();
    for (debt_id, record) in &state.open_debts {
        debts.push(DebtReceipt {
            debt_id: debt_id.clone(),
            asset: record.asset.clone(),
            principal: record.principal,
            fee: record.fee,
            repaid: false,
        });
    }
    let deltas = state
        .net_deltas
        .iter()
        .map(|(asset, delta)| AssetDelta {
            asset: asset.clone(),
            delta: *delta,
        })
        .collect();
    let realized_net_profit = match outcome {
        TradeOutcome::Success => match settlement_asset {
            Some(asset) => {
                let raw = state.net_deltas.get(asset).copied().unwrap_or(0).max(0);
                Some(TypedReceiptAmount {
                    asset: asset.clone(),
                    amount: u128::try_from(raw)
                        .map_err(|_| ReceiptError::EconomicReplayMismatch("profit conversion overflow".to_string()))?,
                })
            }
            None => None,
        },
        TradeOutcome::Failure { .. } => None,
    };

    finalize_receipt(TradeReceipt {
        // 3: the receipt carries the quote window of every swap leg — including the leg's realized
        // slippage — which is what makes `quote_freshness` and `max_slippage_bps` re-derivable at
        // replay (TICKET-017, TICKET-150). A reader of version 1 sees no windows; a version 2
        // receipt's windows have no slippage figure, and replay refuses such a leg when the
        // artifact declares a `max_slippage_bps` ceiling rather than treating the missing figure as
        // a satisfied one.
        // 4: each window also carries the leg's largest oracle deviation, so
        // `max_oracle_deviation_bps` is re-derivable at replay; a version 3 window has none, and
        // replay refuses such a leg when the artifact declares that ceiling.
        format_version: 4,
        compiler_version: compiler_version.to_string(),
        artifact_hash,
        trade_id: trade_id.to_string(),
        policy_id: policy_id.to_string(),
        state_commitment,
        operations: operations.to_vec(),
        costs,
        debts,
        deltas,
        legs: state.leg_quote_windows.clone(),
        realized_net_profit,
        outcome,
        receipt_hash: [0u8; 32],
        attestation: None,
    })
}
