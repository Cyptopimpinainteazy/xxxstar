//! Error types for the agent system.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("cannot register with an ephemeral key — use a persistent identity")]
    CannotRegisterEphemeral,

    #[error("insufficient bond: required {required}, provided {provided}")]
    InsufficientBond { required: u128, provided: u128 },

    #[error("agent already registered: {}", hex::encode(.0))]
    AlreadyRegistered([u8; 32]),

    #[error("agent not registered: {}", hex::encode(.0))]
    NotRegistered([u8; 32]),

    #[error("agent is not active: {}", hex::encode(.0))]
    NotActive([u8; 32]),

    #[error("agent already deregistered: {}", hex::encode(.0))]
    AlreadyDeregistered([u8; 32]),

    #[error("too many ephemeral keys (max: {max})")]
    TooManyEphemeralKeys { max: usize },

    #[error("ephemeral key already linked to another agent: {}", hex::encode(.0))]
    EphemeralKeyAlreadyLinked([u8; 32]),
}
