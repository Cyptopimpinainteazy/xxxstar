//! # X3 Swap Router
//!
//! Cross-VM DEX swap routing: quoting, route optimization, slippage protection,
//! MEV protection, gas estimation, fee calculation, and atomic execution across
//! X3VM, EVM, and SVM legs.

mod atomic_execution;
mod fee_calculator;
mod gas_optimization;
mod mev_protection;
mod optimization;
mod quote_engine;
mod routing;
mod slippage_control;

#[cfg(test)]
mod tests;

pub use atomic_execution::{AtomicSwapExecutor, ExecutionResult, ExecutionStatus, SwapBundle};
pub use fee_calculator::{FeeCalculator, FeeStructure, ProtocolFees};
pub use gas_optimization::{ChainGasParams, GasEstimate, GasOptimizer};
pub use mev_protection::{
    Hop, MEVProtectionConfig, MEVProtectionError, MEVProtector, ProtectedRoute, ProtectionMetrics,
    ProtectionStrategy, SandwichAttack, SandwichProtection,
};
pub use optimization::{OptimizationParams, RouteOptimizer, RouteScore};
pub use quote_engine::{PriceOracle, PriceSource, QuoteEngine, QuoteResult};
pub use routing::{HopInfo, RouteConstraints, RouteFinder, SwapRoute};
pub use slippage_control::{
    ProtectionLevel, SlippageConfig, SlippageController, SlippageProtectedParams,
};

use sp_core::{H160, U256};

/// Which virtual machine a chain in a swap leg belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VmType {
    /// VM not identified from the chain ID.
    Unknown,
    /// X3 native VM.
    X3Vm,
    /// EVM-compatible chain.
    Evm,
    /// SVM-compatible chain.
    Svm,
}

/// Parameters describing a requested swap, possibly crossing VMs and chains.
#[derive(Debug, Clone)]
pub struct SwapParams {
    pub token_in: H160,
    pub token_out: H160,
    pub amount_in: U256,
    pub min_amount_out: U256,
    pub chain_in: u64,
    pub chain_out: u64,
    pub deadline: u64,
    pub recipient: H160,
    pub slippage_tolerance_bps: u16,
    pub gas_price_limit: Option<U256>,
    pub source_vm: VmType,
    pub destination_vm: VmType,
}

/// Errors produced by the swap router pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapRouterError {
    /// No route satisfied the requested parameters.
    RouteNotFound,
    /// Slippage tolerance was zero or otherwise unacceptable.
    HighSlippage,
    /// Route or params failed validation.
    InvalidParameters,
    /// Route execution failed.
    ExecutionFailed,
}

impl core::fmt::Display for SwapRouterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SwapRouterError::RouteNotFound => write!(f, "no route found"),
            SwapRouterError::HighSlippage => write!(f, "slippage tolerance too high or unset"),
            SwapRouterError::InvalidParameters => write!(f, "invalid swap parameters"),
            SwapRouterError::ExecutionFailed => write!(f, "route execution failed"),
        }
    }
}

impl std::error::Error for SwapRouterError {}
