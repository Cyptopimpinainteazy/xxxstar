//! A compiled X3BC artifact for the kernel's benchmarks, and the tests that keep it honest.
//!
//! `submit_comit_v2`'s X3 payload is the *compiled program*, and the adapter that runs it validates
//! the envelope (magic, format version, minimum loader version, body checksum). A benchmark cannot
//! compile a program inside the wasm runtime, so the bytes are embedded here — and the source they
//! came from is embedded beside them, with a test that recompiles it and requires the two to match.
//!
//! Without that test this would be a hand-assembled fixture, which is the shape that hid a reader
//! which never checked a checksum (TICKET-108): a fixture that is not what the compiler emits cannot
//! tell "the runtime accepted my program" from "the runtime accepted my bytes".

/// The `.x3` source the fixture is compiled from. Kept here so the bytes are reviewable as a program
/// rather than as an opaque blob.
pub const X3_PROGRAM_SOURCE: &str = "fn main() -> i64 {\n    return 42;\n}\n";

/// `compile_source(X3_PROGRAM_SOURCE)` — 63 bytes, produced 2026-09-25 with
/// `crates/x3-integration`'s compiler bridge, which is the compiler this chain runs.
pub const X3_PROGRAM_FIXTURE: &[u8] = &[
    88, 51, 66, 67, 0, 0, 1, 0, 0, 0, 0, 0, 181, 77, 89, 13, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    0, 0, 0, 4, 0, 102, 110, 95, 48, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 5, 0, 0, 0, 24, 0,
    42, 5, 0, 0, 0,
];

/// A program that only ends by running out of gas: it counts up while the count is non-negative,
/// and the count cannot wrap within any gas limit the chain allows. `x3_execute` runs it with a gas
/// limit `g`, so the work it measures is exactly `g` instructions of the chain's engine.
pub const X3_LOOP_SOURCE: &str =
    "fn main() -> i64 {\n    let mut i = 0;\n    while i >= 0 {\n        i = i + 1;\n    }\n    return i;\n}\n";

/// `compile_source(X3_LOOP_SOURCE)` — 113 bytes, produced 2026-09-26 with the compiler bridge.
pub const X3_LOOP_FIXTURE: &[u8] = &[
    88, 51, 66, 67, 0, 0, 1, 0, 0, 0, 0, 0, 51, 88, 196, 77, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    0, 0, 0, 4, 0, 102, 110, 95, 48, 0, 0, 0, 0, 0, 10, 0, 10, 0, 1, 0, 0, 0, 0, 55, 0, 0, 0, 24,
    0, 0, 17, 1, 0, 24, 2, 0, 24, 3, 1, 1, 17, 0, 0, 0, 17, 4, 1, 69, 5, 4, 2, 2, 5, 35, 0, 0, 0,
    1, 50, 0, 0, 0, 17, 6, 1, 32, 7, 6, 3, 17, 1, 7, 1, 17, 0, 0, 0, 17, 9, 1, 5, 9, 0, 0,
];

#[cfg(test)]
mod tests {
    /// The fixture must be what the compiler emits *today*. If the compiler changes and this test
    /// fails, the fixture has to be regenerated rather than the assertion relaxed.
    #[test]
    fn the_embedded_program_compiles_to_the_embedded_bytes() {
        let compiled =
            x3_x3_integration::compiler_bridge::compile_legacy_source(super::X3_PROGRAM_SOURCE)
                .expect("the embedded source must compile");
        assert_eq!(
            compiled,
            super::X3_PROGRAM_FIXTURE,
            "the embedded artifact no longer matches its source — regenerate it from the compiler \
             rather than editing the bytes"
        );
    }

    /// The loop fixture is what the compiler emits for its source, for the same reason.
    #[test]
    fn the_embedded_loop_compiles_to_the_embedded_bytes() {
        let compiled =
            x3_x3_integration::compiler_bridge::compile_legacy_source(super::X3_LOOP_SOURCE)
                .expect("the embedded loop must compile");
        assert_eq!(
            compiled,
            super::X3_LOOP_FIXTURE,
            "the embedded loop no longer matches its source — regenerate it from the compiler"
        );
    }

    /// And it spends exactly the gas it is given, on the engine the chain runs: the property
    /// `x3_execute`'s linear model rests on.
    #[test]
    fn the_embedded_loop_runs_to_exactly_its_gas_limit() {
        use crate::X3ExecutorAdapter;
        for limit in [1_000u64, 25_000, 400_000] {
            let receipt =
                crate::wasm_adapters::WasmX3Adapter::execute(super::X3_LOOP_FIXTURE, limit)
                    .expect("the loop executes");
            assert!(!receipt.success, "the loop only ends by exhausting its gas");
            assert_eq!(receipt.gas_used, limit);
        }
    }

    /// And it must be an artifact the production adapter accepts, which is the property the
    /// benchmark depends on: the runtime's `X3VmAdapter` validates before it executes.
    #[test]
    fn the_embedded_program_passes_the_production_adapters_validation() {
        use crate::X3ExecutorAdapter;
        crate::X3VmAdapter::validate(super::X3_PROGRAM_FIXTURE)
            .expect("the adapter the chain configures must accept the benchmark fixture");
    }
}
