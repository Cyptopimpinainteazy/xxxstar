use crate::routing::SwapRoute;
use crate::SwapRouterError;
use serde::{Deserialize, Serialize};
use sp_core::U256;

/// Very small heuristic gas estimator; replace with per-chain oracle when available.
#[derive(Debug, Clone, Default)]
pub struct GasOptimizer;

impl GasOptimizer {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn calculate_gas(
        &self,
        route: &SwapRoute,
    ) -> Result<ChainGasParams, SwapRouterError> {
        // Heuristic: base 21000 + 50_000 per hop; gas price placeholder 1 gwei.
        let hop_cost = U256::from(50_000u64 * route.hops.len() as u64 + 21_000u64);
        let gas_price = U256::from(1_000_000_000u64); // 1 gwei default
        Ok(ChainGasParams {
            gas_price,
            gas_limit: hop_cost,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ChainGasParams {
    pub gas_price: U256,
    pub gas_limit: U256,
}

pub type GasEstimate = ChainGasParams;
