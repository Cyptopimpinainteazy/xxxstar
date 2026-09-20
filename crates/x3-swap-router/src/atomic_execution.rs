use crate::gas_optimization::ChainGasParams;
use crate::routing::SwapRoute;
use crate::slippage_control::SlippageProtectedParams;
use crate::SwapRouterError;
use sp_core::{H256, U256};

#[derive(Debug, Clone, Default)]
pub struct AtomicSwapExecutor;

#[derive(Debug, Clone, Default)]
pub struct SwapBundle;

#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    pub execution_id: H256,
    pub gas_used: U256,
    /// The slippage **bound the caller supplied**, in basis points.
    ///
    /// It was `slippage_achieved: f64`, set to `params.slippage_bps as f64 / 10_000.0` — a
    /// float fraction of the *declared* tolerance, labelled as the slippage *achieved* by a
    /// run that never happened. Two things were wrong with that and the second is why it is
    /// gone: an execution record has no business carrying a float (PHASE 43), and a record
    /// that echoes the caller's own bound under the name "achieved" is a measurement this
    /// crate did not take. The field is the unit the caller wrote — including its width, so
    /// it is a `u16` because `SlippageProtectedParams::slippage_bps` is (TICKET-094).
    pub slippage_bps: u16,
    pub execution_time_ms: u64,
    pub success: bool,
}

impl AtomicSwapExecutor {
    pub fn new() -> Result<Self, SwapRouterError> {
        Ok(Self)
    }

    pub async fn execute_swap_bundle(
        &self,
        route: &SwapRoute,
        gas_params: &ChainGasParams,
        params: &SlippageProtectedParams,
    ) -> Result<ExecutionResult, SwapRouterError> {
        // **Refused, not fabricated.** This used to return `Ok` with a record assembled from
        // constants: `execution_id: H256::zero()` for every call, `gas_used` set to the gas
        // *limit* rather than the gas used, `execution_time_ms: 10`, and
        // `success: !route.hops.is_empty() || amount_in > 0` — which is true for any
        // non-empty route, and no swap was executed. Its own comment said "Minimal
        // deterministic execution record; replace with real executor integration", and the
        // crate's test asserted the fabricated success, which is what made a no-op execution
        // path look intentional (TICKET-094; the project's rules forbid no-op execution paths
        // and placeholder logic, and this is both).
        //
        // A caller that reads `success: true` on an atomic swap bundle believes value moved.
        // Refusing is the only honest answer until there is real integration, and the error
        // says what is missing rather than reporting a failure the caller could mistake for a
        // market outcome.
        let _ = (route, gas_params, params);
        Err(SwapRouterError::NoExecutorConfigured)
    }
}
