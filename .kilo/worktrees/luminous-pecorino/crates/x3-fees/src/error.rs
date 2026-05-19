//! Error types for the fee engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeeError {
    #[error("execution must have at least one leg")]
    ZeroLegs,

    #[error("fee calculation overflow")]
    Overflow,

    #[error("fee cap exceeded: computed {computed}, cap {cap}")]
    FeeCapExceeded { computed: u128, cap: u128 },
}
