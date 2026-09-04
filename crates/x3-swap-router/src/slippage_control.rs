use crate::routing::SwapRoute;
use crate::{SwapParams, SwapRouterError};

#[derive(Debug, Clone, Default)]
pub struct SlippageController;

#[derive(Debug, Clone, Copy)]
pub enum ProtectionLevel {
    Strict,
    Balanced,
    Loose,
}

impl ProtectionLevel {
    pub fn to_bps(self) -> u16 {
        match self {
            ProtectionLevel::Strict => 25,   // 0.25%
            ProtectionLevel::Balanced => 75, // 0.75%
            ProtectionLevel::Loose => 150,   // 1.5%
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SlippageConfig {
    pub max_slippage_bps: u16,
}

#[derive(Debug, Clone)]
pub struct SlippageProtectedParams {
    pub params: SwapParams,
    pub slippage_bps: u16,
}

impl SlippageController {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn apply_protection(
        &self,
        params: &SwapParams,
        _route: &SwapRoute,
    ) -> Result<SlippageProtectedParams, SwapRouterError> {
        let limit = params.slippage_tolerance_bps;
        if limit == 0 {
            return Err(SwapRouterError::HighSlippage);
        }
        Ok(SlippageProtectedParams {
            params: params.clone(),
            slippage_bps: limit,
        })
    }
}
