//! Canonical intake and execution; the semantic implementation lives in x3-lang.
use crate::{X3IntegrationError, X3Result};
#[cfg(not(feature = "std"))]
use alloc::format;
pub use x3_lang_execution::{artifact::*, trading::*};

/// Legacy X3BC magic is disjoint from the canonical version/metadata prefix.
/// Unknown canonical versions are still routed to canonical validation and refused.
pub fn is_canonical_artifact(bytes: &[u8]) -> bool {
    !bytes.starts_with(b"X3BC")
        && matches!(
            bytes.get(1).copied(),
            Some(
                x3_lang_execution::spec::opcodes::META_NONCE
                    | x3_lang_execution::spec::opcodes::META_CHAIN_ID
                    | x3_lang_execution::spec::opcodes::META_VERSIONS
            )
        )
}

/// Developer hosts and validators execute through this same no-std engine.
/// A host with fixture capabilities cannot satisfy a production context.
pub fn execute_artifact(
    bytes: &[u8],
    gas_limit: u64,
    host: &mut dyn TradingHost,
    context: TradeExecutionContext,
) -> X3Result<TradeExecution> {
    let artifact = decode_artifact(bytes).map_err(X3IntegrationError::VerificationFailed)?;
    if artifact.gas_used > gas_limit {
        return Err(X3IntegrationError::GasExhausted {
            used: artifact.gas_used,
            limit: gas_limit,
        });
    }
    TradingVm::new()
        .execute_atomic(&artifact.operations, host, context)
        .map_err(|e| X3IntegrationError::ExecutionFailed(format!("{e}")))
}
