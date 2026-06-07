//! Benchmark stubs for x3-invariants.
//!
//! This file exists to satisfy the `runtime-benchmarks` feature gate.

#[cfg(not(feature = "runtime-benchmarks"))]
compile_error!(
    "This module should only be compiled with the `runtime-benchmarks` feature enabled."
);

// Add benchmarks here when runtime benchmarking is required.
