#![deny(unsafe_code)]
// pallets/fraud-proofs/src/lib.rs
//
// Stub crate — the fraud-proof FRAME pallet is defined inline in the
// x3-chain-runtime crate at `runtime/src/fraud_proofs/pallet.rs` to avoid
// a circular dependency (runtime ← pallet ← runtime).
//
// If you need to use the pallet types from outside the runtime, extract the
// shared types into a new `crates/x3-fraud-proofs-types` library crate and
// have both this stub and the runtime import from that shared crate.
