//! Core types for the X3 flashloan engine.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Unique flashloan identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlashloanId(pub String);

impl FlashloanId {
    pub fn new() -> Self {
        let mut hasher = Sha256::new();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        hasher.update(nonce.to_le_bytes());
        hasher.update(b"x3-flashloan");
        let hash = hasher.finalize();
        Self(hex::encode(&hash[..16]))
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for FlashloanId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FlashloanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Chain kind — unified EVM + SVM + X3VM addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainKind {
    /// EVM-compatible chain with chain ID.
    Evm(u64),
    /// Solana Virtual Machine.
    Svm,
    /// X3 Chain native VM (WASM-based, ~200ms finality).
    X3,
}

impl fmt::Display for ChainKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainKind::Evm(id) => write!(f, "evm:{}", id),
            ChainKind::Svm => write!(f, "svm"),
            ChainKind::X3 => write!(f, "x3"),
        }
    }
}

/// Asset identifier (symbol-based, resolved to address per chain).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl AssetId {
    pub fn new(symbol: &str) -> Self {
        Self(symbol.to_uppercase())
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Borrow request — a single flashloan borrow on a single chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowRequest {
    pub id: FlashloanId,
    pub chain: ChainKind,
    pub asset: AssetId,
    /// Amount in the asset's smallest unit.
    pub amount: u128,
    pub purpose: BorrowPurpose,
}

/// Why capital is being borrowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorrowPurpose {
    /// Arbitrage execution.
    ArbExecution,
    /// Liquidation.
    Liquidation,
    /// Collateral swap.
    CollateralSwap,
    /// Debt refinancing.
    DebtRefinance,
}

/// Receipt issued after a successful borrow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowReceipt {
    pub id: FlashloanId,
    pub chain: ChainKind,
    pub asset: AssetId,
    /// Principal amount borrowed.
    pub principal: u128,
    /// Premium owed (calculated via x3-fees curve).
    pub premium: u128,
    /// Timestamp of borrow (ms).
    pub borrowed_at: u64,
    /// Hard deadline for repayment (ms).
    pub must_repay_by: u64,
}

impl BorrowReceipt {
    /// Total amount that must be repaid (principal + premium).
    pub fn total_owed(&self) -> u128 {
        self.principal + self.premium
    }
}

/// A complete flashloan plan spanning one or more chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashloanPlan {
    pub intent_id: String,
    pub borrows: Vec<BorrowRequest>,
    pub legs: Vec<ExecutionLeg>,
    /// Deadline in milliseconds from plan creation.
    pub deadline_ms: u64,
}

/// A validated flashloan plan ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedPlan {
    pub intent_id: String,
    pub borrows: Vec<BorrowRequest>,
    pub legs: Vec<ExecutionLeg>,
    pub deadline_ms: u64,
    /// Total premium across all borrows.
    pub total_premium: u128,
    /// Estimated total gas across all legs.
    pub estimated_gas: u64,
    /// Hash commitment of the plan.
    pub plan_hash: String,
}

/// A single execution leg within a flashloan plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLeg {
    pub chain: ChainKind,
    pub action: LegAction,
    pub gas_limit: u64,
}

/// Action to perform in an execution leg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegAction {
    Swap {
        from: AssetId,
        to: AssetId,
        amount: u128,
    },
    Deposit {
        asset: AssetId,
        amount: u128,
        protocol: String,
    },
    Withdraw {
        asset: AssetId,
        amount: u128,
        protocol: String,
    },
    Repay {
        asset: AssetId,
        amount: u128,
        debt_protocol: String,
    },
}

/// Outcome of a single leg execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LegOutcome {
    Success {
        chain: ChainKind,
        gas_used: u64,
        output_amount: u128,
    },
    Failure {
        chain: ChainKind,
        reason: String,
    },
}

impl LegOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, LegOutcome::Success { .. })
    }

    pub fn chain(&self) -> ChainKind {
        match self {
            LegOutcome::Success { chain, .. } => *chain,
            LegOutcome::Failure { chain, .. } => *chain,
        }
    }
}

/// Settlement status for a flashloan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    /// Pending execution.
    Pending,
    /// Successfully settled (principal + premium repaid).
    Settled,
    /// Atomically reverted (no capital lost).
    Reverted,
    /// Defaulted (agent slashed).
    Defaulted,
}

/// Record of a completed flashloan settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub flashloan_id: FlashloanId,
    pub status: SettlementStatus,
    /// Amount repaid.
    pub amount_repaid: u128,
    /// Proof hash binding borrow→use→repay.
    pub proof_hash: String,
    /// Timestamp of settlement.
    pub settled_at: u64,
}
