//! Integration tests for `x3_integration::compiler_bridge`.
//!
//! The bridge contract is pinned here: until the real compiler handoff
//! is wired (gated on the workspace's `x3-compiler` crate building
//! end to end, plus `x3-lang/compiler` being reachable from this
//! workspace), `compile_source` MUST return a typed
//! `X3IntegrationError::CompilationFailed` and MUST NOT return
//! silently-fake bytes. This test exists so a future regression
//! (e.g. someone re-introducing `Ok(vec![])` as a placeholder) is
//! caught immediately.
//!
//! The test runs only under the `std` feature (which is the default
//! and which is what gateway/integration callers use). When the
//! upstream gate clears, the test is updated to assert the
//! `Ok(bytecode)` path.

#![cfg(feature = "std")]

use x3_integration::compiler_bridge::compile_source;
use x3_integration::X3IntegrationError;

#[test]
fn compile_source_returns_typed_error_until_wired() {
    // The bridge is gated on the workspace's x3-compiler pipeline
    // building end-to-end (it currently has a pre-existing
    // non-exhaustive match in its compiler pipeline) and on
    // x3-lang/compiler being reachable from this workspace (it
    // currently isn't, because of a different workspace.package
    // block in x3-lang/Cargo.toml). The bridge returns a typed
    // CompilationFailed error so production callers cannot mistake
    // an unimplemented bridge for a successful compilation.
    let result = compile_source("from ethereum.USDC amount 1");
    let err = result.expect_err("bridge must return Err until the real call is wired");
    match err {
        X3IntegrationError::CompilationFailed(msg) => {
            assert!(
                msg.contains("not yet wired") || msg.contains("pre-existing"),
                "error message must explain the gate, got {:?}",
                msg
            );
        }
        other => panic!("expected CompilationFailed, got {:?}", other),
    }
}

#[test]
fn compile_source_does_not_return_empty_bytes() {
    // A regression guard: the bridge must never return an empty
    // `Vec::new()` (which callers might mistake for valid
    // "no-op" bytecode). If this test ever starts passing the
    // bridge is *not* correctly failing closed.
    let result = compile_source("anything");
    if let Ok(bytes) = result {
        assert!(
            !bytes.is_empty(),
            "bridge must not return empty bytes (would be indistinguishable from a no-op)"
        );
    }
    // If `Err` is returned, the test trivially passes — the bridge
    // is correctly failing closed.
}
