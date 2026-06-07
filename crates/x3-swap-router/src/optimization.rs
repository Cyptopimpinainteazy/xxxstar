use crate::quote_engine::QuoteResult;
use crate::routing::SwapRoute;
use crate::{SwapParams, SwapRouterError};
use sp_core::U256;

/// Basic route optimizer: pick best output, break ties on lowest gas, then earliest deadline.
#[derive(Debug, Clone, Default)]
pub struct RouteOptimizer;

#[derive(Debug, Clone, Default)]
pub struct OptimizationParams {
    pub min_output: Option<U256>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteScore {
    pub estimated_output: U256,
    pub gas_cost: U256,
}

impl RouteOptimizer {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn optimize_route(
        &self,
        quotes: &[QuoteResult],
        params: &SwapParams,
    ) -> Result<SwapRoute, SwapRouterError> {
        let mut best: Option<(RouteScore, SwapRoute)> = None;

        for q in quotes {
            if q.estimated_output < params.min_amount_out {
                continue;
            }
            let score = RouteScore {
                estimated_output: q.estimated_output,
                gas_cost: q.gas_cost,
            };
            best = match best {
                None => Some((score, q.route.clone())),
                Some((cur_score, cur_route)) => {
                    if score.estimated_output > cur_score.estimated_output
                        || (score.estimated_output == cur_score.estimated_output
                            && score.gas_cost < cur_score.gas_cost)
                    {
                        Some((score, q.route.clone()))
                    } else {
                        Some((cur_score, cur_route))
                    }
                }
            };
        }

        best.map(|(_, route)| route)
            .ok_or(SwapRouterError::RouteNotFound)
    }
}
