use crate::routing::SwapRoute;
use crate::{SwapParams, SwapRouterError};
use serde::{Deserialize, Serialize};
use sp_core::U256;

pub struct QuoteEngine;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteResult {
    pub route: SwapRoute,
    pub estimated_output: U256,
    pub gas_cost: U256,
}

pub struct PriceOracle;
pub struct PriceSource;

impl QuoteEngine {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn get_comprehensive_quotes(
        &self,
        _params: &SwapParams,
    ) -> Result<Vec<QuoteResult>, SwapRouterError> {
        // A minimum output is a user constraint, not an observed market price.
        // Refuse until a real source supplies a quote and an executable route.
        Err(SwapRouterError::NoLiquiditySourceConfigured)
    }
}
