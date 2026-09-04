//! Swap execution and LP withdrawal for cross-VM settlement.
//!
//! Wraps the spot-swap path from `x3_dex::AMMPool::swap` adding bounds
//! checks required by the IXL settlement contract before calling into
//! the DEX execution layer.

/// Request to execute a swap (settle a cross-VM liquidity transfer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettleRequest {
    pub pool_id: u64,
    pub amount_in: u128,
    /// Minimum acceptable output (slippage guard).
    pub min_out: u128,
}

/// Errors returned by `Settlement`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettleError {
    /// `amount_in` is zero.
    ZeroAmount,
    /// `min_out` exceeds `amount_in` — impossible to satisfy.
    InvertedBounds,
}

/// Cross-VM settlement executor.
pub struct Settlement;

impl Settlement {
    /// Validate and build a settle request.
    pub fn build(
        pool_id: u64,
        amount_in: u128,
        min_out: u128,
    ) -> Result<SettleRequest, SettleError> {
        if amount_in == 0 {
            return Err(SettleError::ZeroAmount);
        }
        if min_out > amount_in {
            return Err(SettleError::InvertedBounds);
        }
        Ok(SettleRequest {
            pool_id,
            amount_in,
            min_out,
        })
    }
}
