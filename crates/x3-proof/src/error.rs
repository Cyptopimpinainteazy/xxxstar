//! Error types for the proof engine.

use crate::types::BlockHeight;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("too many state diffs in single proof (max: {max})")]
    TooManyDiffs { max: usize },

    #[error("cannot nest atomic blocks")]
    NestedAtomic,

    #[error("not inside an atomic block")]
    NotInAtomic,

    #[error("cannot emit proof during atomic block — commit or rollback first")]
    EmitDuringAtomic,

    #[error("uncommitted atomic block at finalization")]
    UncommittedAtomic,

    #[error("block height mismatch: expected {expected}, got {got}")]
    BlockHeightMismatch {
        expected: BlockHeight,
        got: BlockHeight,
    },

    #[error("program hash mismatch in proof chain")]
    ProgramHashMismatch,

    #[error("invalid proof hash at index {index}")]
    InvalidProofHash { index: usize },

    #[error("state discontinuity at proof index {proof_index}")]
    StateDiscontinuity { proof_index: usize },

    #[error("chain hash mismatch — chain has been tampered with")]
    ChainHashMismatch,

    #[error("duplicate key in state diffs within single proof")]
    DuplicateKeyInDiffs,

    #[error("replay target hash does not match original proof")]
    ReplayTargetMismatch,
}
