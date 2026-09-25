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

#[cfg(test)]
mod tests {
    /// The fixture must be what the compiler emits *today*. If the compiler changes and this test
    /// fails, the fixture has to be regenerated rather than the assertion relaxed.
    #[test]
    fn the_embedded_program_compiles_to_the_embedded_bytes() {
        let compiled = x3_x3_integration::compiler_bridge::compile_source(super::X3_PROGRAM_SOURCE)
            .expect("the embedded source must compile");
        assert_eq!(
            compiled,
            super::X3_PROGRAM_FIXTURE,
            "the embedded artifact no longer matches its source — regenerate it from the compiler \
             rather than editing the bytes"
        );
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
