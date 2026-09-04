//! Error types for the slashing engine.

use crate::types::{Amount, BondId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SlashError {
    #[error("insufficient bond: required {required}, provided {provided}")]
    InsufficientBond { required: Amount, provided: Amount },

    #[error("bond not found: {0:?}")]
    BondNotFound(BondId),

    #[error("bond is not in a slashable state: {0:?}")]
    BondNotSlashable(BondId),

    #[error("bond is not in a releasable state: {0:?}")]
    BondNotReleasable(BondId),

    #[error("agent not bonded — must post bond before execution")]
    AgentNotBonded,

    #[error("slash event hashing failed")]
    HashingFailed,
}
