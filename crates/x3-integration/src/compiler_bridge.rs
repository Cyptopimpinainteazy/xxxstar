//! Compiler bridge: hand a real `.x3` source string to the
//! workspace's production compiler pipeline and return its bytecode.
//!
//! The integration crate is a thin adapter — it owns no parsing,
//! lowering, or codegen. The compiler of record on the
//! chain-integration path is `crates/x3-compiler` (the
//! `x3-compiler` package); the canonical language-level compiler is
//! `x3-lang/compiler` (a separate workspace) used by the CLI sandbox
//! and the x3-lang examples.
//!
//! As of 2026-06-06 the bridge returns an explicit
//! `X3IntegrationError::CompilationFailed` rather than minting fake
//! bytecode. Wiring the real call is gated on two preconditions:
//!   1. The workspace's `x3-compiler` crate must compile end to end.
//!      It currently has a non-exhaustive `match` on
//!      `parser::Expr::Assign` in its own compiler pipeline
//!      (pre-existing breakage, not introduced by this bridge).
//!   2. `x3-lang-compiler` lives in a different `Cargo.toml`
//!      workspace and inherits a different `workspace.package` block,
//!      so it is not reachable as a workspace-internal dep.
//!
//! The `compile_source` function below is structured to fail closed
//! with a precise error message naming the gate, instead of
//! returning silently-fake bytes. Callers and the test in
//! `tests/compiler_bridge.rs` pin this contract: the bridge MUST
//! return an error, never an empty `Vec::new()`, so no production
//! code path can mistake an unimplemented bridge for a real
//! compilation.

use crate::{X3IntegrationError, X3Result};

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

/// Compile X3 source into X3 bytecode.
///
/// Returns `X3IntegrationError::CompilationFailed` with a precise
/// reason. Once the gates above are cleared this will hand the
/// source to `x3_compiler::Compiler::compile` (or
/// `x3_lang_compiler::compile_source` for the language-level path)
/// and return the serialized bytecode. The test
/// `tests/compiler_bridge.rs` pins the contract that the result is
/// always a typed error until the real call is wired.
pub fn compile_source(_source: &str) -> X3Result<Vec<u8>> {
    Err(X3IntegrationError::CompilationFailed(String::from(
        "x3-integration::compiler_bridge::compile_source is not yet wired to a real \
             compiler; the workspace's x3-compiler crate has a pre-existing non-exhaustive \
             match on parser::Expr::Assign, and x3-lang/compiler lives in a different \
             workspace. Bridge fails closed with a typed error so production code cannot \
             mistake an unimplemented bridge for a successful compilation.",
    )))
}
