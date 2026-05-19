//! Error types for the X3 flashloan engine.

use crate::executor::ContextId;
use crate::types::{AssetId, ChainKind, FlashloanId};

/// Flashloan engine error.
#[derive(Debug, thiserror::Error)]
pub enum FlashloanError {
    #[error("insufficient liquidity on {chain}: requested {requested}, available {available} of {asset}")]
    InsufficientLiquidity {
        chain: ChainKind,
        asset: AssetId,
        requested: u128,
        available: u128,
    },

    #[error("insufficient repayment: owed {owed}, paid {paid}")]
    InsufficientRepayment { owed: u128, paid: u128 },

    #[error("concurrent borrow rejected on {chain} for {asset}: outstanding flashloan exists")]
    ConcurrentBorrowRejected { chain: ChainKind, asset: AssetId },

    #[error("unknown flashloan: {0}")]
    UnknownFlashloan(FlashloanId),

    #[error("invalid plan: {0}")]
    InvalidPlan(String),

    #[error("atomic revert: context {context_id} failed ({} of {total_legs} legs failed)", failed_legs.len())]
    AtomicRevert {
        context_id: ContextId,
        failed_legs: Vec<(ChainKind, String)>,
        total_legs: usize,
    },

    #[error("unknown execution context: {0}")]
    UnknownContext(ContextId),

    #[error("execution context already finalized: {0}")]
    AlreadyFinalized(ContextId),

    #[error("deadline exceeded")]
    DeadlineExceeded,

    #[error("settlement failed: {0}")]
    SettlementFailed(String),
}
