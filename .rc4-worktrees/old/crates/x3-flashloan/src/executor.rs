//! Atomic executor — ensures all-or-nothing execution across chains.
//!
//! If any leg fails, the entire context reverts. No partial fills.

use std::collections::HashMap;

use crate::error::FlashloanError;
use crate::types::{ChainKind, LegOutcome};

/// Unique execution context identifier.
pub type ContextId = u64;

/// State of an execution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    /// Actively recording legs.
    Active,
    /// All legs succeeded — ready to finalize.
    Succeeded,
    /// At least one leg failed — must revert.
    Failed,
    /// Finalized (committed or reverted).
    Finalized,
}

/// Record of all legs in an execution context.
#[derive(Debug)]
struct ExecutionContext {
    state: ContextState,
    legs: Vec<LegOutcome>,
    total_gas: u64,
    total_output: u128,
}

/// Atomic executor: enforces all-or-nothing semantics for flashloan execution.
///
/// Usage:
/// ```rust
/// use x3_flashloan::{AtomicExecutor, LegOutcome, ChainKind};
///
/// let mut executor = AtomicExecutor::new();
/// let ctx = executor.begin();
/// executor.record_leg(ctx, LegOutcome::Success {
///     chain: ChainKind::Evm(1),
///     gas_used: 100_000,
///     output_amount: 50_000,
/// });
/// executor.record_leg(ctx, LegOutcome::Success {
///     chain: ChainKind::Evm(137),
///     gas_used: 80_000,
///     output_amount: 45_000,
/// });
/// let result = executor.finalize(ctx);
/// assert!(result.is_ok());
/// ```
pub struct AtomicExecutor {
    next_id: ContextId,
    contexts: HashMap<ContextId, ExecutionContext>,
}

impl AtomicExecutor {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            contexts: HashMap::new(),
        }
    }

    /// Begin a new atomic execution context.
    pub fn begin(&mut self) -> ContextId {
        let id = self.next_id;
        self.next_id += 1;

        self.contexts.insert(
            id,
            ExecutionContext {
                state: ContextState::Active,
                legs: Vec::new(),
                total_gas: 0,
                total_output: 0,
            },
        );

        id
    }

    /// Record the outcome of an execution leg.
    pub fn record_leg(&mut self, ctx_id: ContextId, outcome: LegOutcome) {
        if let Some(ctx) = self.contexts.get_mut(&ctx_id) {
            if ctx.state != ContextState::Active {
                return; // Context already finalized
            }

            match &outcome {
                LegOutcome::Success {
                    gas_used,
                    output_amount,
                    ..
                } => {
                    ctx.total_gas += gas_used;
                    ctx.total_output += output_amount;
                }
                LegOutcome::Failure { .. } => {
                    ctx.state = ContextState::Failed;
                }
            }

            ctx.legs.push(outcome);
        }
    }

    /// Finalize the execution context.
    ///
    /// Returns `Ok(FinalizedContext)` if all legs succeeded.
    /// Returns `Err(FlashloanError::AtomicRevert)` if any leg failed.
    pub fn finalize(&mut self, ctx_id: ContextId) -> Result<FinalizedContext, FlashloanError> {
        let ctx = self
            .contexts
            .get_mut(&ctx_id)
            .ok_or(FlashloanError::UnknownContext(ctx_id))?;

        if ctx.state == ContextState::Finalized {
            return Err(FlashloanError::AlreadyFinalized(ctx_id));
        }

        // Check for any failures
        if ctx.state == ContextState::Failed {
            let failed_legs: Vec<(ChainKind, String)> = ctx
                .legs
                .iter()
                .filter_map(|l| match l {
                    LegOutcome::Failure { chain, reason } => Some((*chain, reason.clone())),
                    _ => None,
                })
                .collect();

            ctx.state = ContextState::Finalized;

            return Err(FlashloanError::AtomicRevert {
                context_id: ctx_id,
                failed_legs,
                total_legs: ctx.legs.len(),
            });
        }

        // All legs succeeded
        ctx.state = ContextState::Finalized;

        Ok(FinalizedContext {
            context_id: ctx_id,
            total_gas: ctx.total_gas,
            total_output: ctx.total_output,
            leg_count: ctx.legs.len(),
        })
    }

    /// Get the state of a context.
    pub fn state(&self, ctx_id: ContextId) -> Option<ContextState> {
        self.contexts.get(&ctx_id).map(|c| c.state)
    }

    /// Get the number of legs recorded for a context.
    pub fn leg_count(&self, ctx_id: ContextId) -> usize {
        self.contexts
            .get(&ctx_id)
            .map(|c| c.legs.len())
            .unwrap_or(0)
    }
}

impl Default for AtomicExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Successfully finalized execution context.
#[derive(Debug, Clone)]
pub struct FinalizedContext {
    pub context_id: ContextId,
    pub total_gas: u64,
    pub total_output: u128,
    pub leg_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_legs_succeed() {
        let mut executor = AtomicExecutor::new();
        let ctx = executor.begin();

        executor.record_leg(
            ctx,
            LegOutcome::Success {
                chain: ChainKind::Evm(1),
                gas_used: 100_000,
                output_amount: 50_000,
            },
        );

        executor.record_leg(
            ctx,
            LegOutcome::Success {
                chain: ChainKind::Evm(137),
                gas_used: 80_000,
                output_amount: 45_000,
            },
        );

        let result = executor.finalize(ctx);
        assert!(result.is_ok());

        let finalized = result.unwrap();
        assert_eq!(finalized.total_gas, 180_000);
        assert_eq!(finalized.total_output, 95_000);
        assert_eq!(finalized.leg_count, 2);
    }

    #[test]
    fn test_one_failure_reverts_all() {
        let mut executor = AtomicExecutor::new();
        let ctx = executor.begin();

        executor.record_leg(
            ctx,
            LegOutcome::Success {
                chain: ChainKind::Evm(1),
                gas_used: 100_000,
                output_amount: 50_000,
            },
        );

        executor.record_leg(
            ctx,
            LegOutcome::Failure {
                chain: ChainKind::Svm,
                reason: "slippage exceeded".to_string(),
            },
        );

        executor.record_leg(
            ctx,
            LegOutcome::Success {
                chain: ChainKind::Evm(42161),
                gas_used: 70_000,
                output_amount: 30_000,
            },
        );

        let result = executor.finalize(ctx);
        assert!(result.is_err());

        match result.unwrap_err() {
            FlashloanError::AtomicRevert {
                context_id,
                failed_legs,
                total_legs,
            } => {
                assert_eq!(context_id, ctx);
                assert_eq!(failed_legs.len(), 1);
                // Only 2 legs recorded: after failure, context is Failed and
                // subsequent record_leg calls are no-ops.
                assert_eq!(total_legs, 2);
                assert_eq!(failed_legs[0].0, ChainKind::Svm);
            }
            other => panic!("expected AtomicRevert, got {:?}", other),
        }
    }

    #[test]
    fn test_double_finalize_rejected() {
        let mut executor = AtomicExecutor::new();
        let ctx = executor.begin();

        executor.record_leg(
            ctx,
            LegOutcome::Success {
                chain: ChainKind::Evm(1),
                gas_used: 50_000,
                output_amount: 10_000,
            },
        );

        assert!(executor.finalize(ctx).is_ok());
        assert!(matches!(
            executor.finalize(ctx),
            Err(FlashloanError::AlreadyFinalized(_))
        ));
    }

    #[test]
    fn test_context_state_tracking() {
        let mut executor = AtomicExecutor::new();
        let ctx = executor.begin();

        assert_eq!(executor.state(ctx), Some(ContextState::Active));

        executor.record_leg(
            ctx,
            LegOutcome::Failure {
                chain: ChainKind::Evm(1),
                reason: "revert".to_string(),
            },
        );

        assert_eq!(executor.state(ctx), Some(ContextState::Failed));
    }

    #[test]
    fn test_unknown_context() {
        let mut executor = AtomicExecutor::new();
        assert!(matches!(
            executor.finalize(999),
            Err(FlashloanError::UnknownContext(999))
        ));
    }
}
