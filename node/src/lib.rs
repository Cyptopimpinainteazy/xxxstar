#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, rust_2018_idioms)]
#![allow(
    clippy::result_large_err,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

//! X3 Chain node library crate.
//!
//! This crate wires together the CLI, command routing, service factories, and
//! chain specification tooling for the X3 Chain layer-one blockchain node.
//! Consumers can use the re-exports provided here to bootstrap custom binaries,
//! integration tests, or benchmarking harnesses around the X3 Chain node
//! components.

/// CLI interface definitions for the X3 Chain node binary.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub mod cli;

/// Command dispatching and execution helpers for CLI invocations.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub mod command;

/// Phase 4: RPC Endpoints.
pub mod rpc;
pub mod rpc_frontier;

/// RPC rate limiting and security middleware.
pub mod rpc_middleware;

/// Phase 5: Network Bootstrapping.
pub mod network;

/// Free-space guard for authority nodes (disk-full protection).
pub mod disk_guard;

/// Phase 7: Telemetry/Monitoring.
pub mod metrics;

/// Per-runtime-call timing, so a slow import can be attributed to runtime
/// execution or to the client around it instead of being one opaque number.
pub mod timed_executor;

/// Chain specification constructors and utilities used to create X3 Chain
/// network configurations.
pub mod chain_spec;

/// Atomic gateway key handling and signed atomic-kernel extrinsic builders.
pub mod atomic_gateway;

/// Operator support for validator key rotation: derives session keys and
/// builds signed `session.set_keys` extrinsics against the on-chain custody
/// registry.
pub mod validator_rotation;

/// Node-side atomic gateway execution service.
pub mod atomic_service;

/// Certificates this node observed for finalized blocks (TICKET-107).
pub mod finality_certs;

/// Concrete signer for X3SettlementEngine live lock/claim/refund extrinsics.
///
/// The implementation lives in the `x3-runtime-signer` crate so the relayer can
/// sign the same runtime calls; this re-export keeps the path the node's own
/// tests and `atomic_gateway` have always used.
pub use x3_runtime_signer as x3vm_runtime_signer;

/// Flash Finality network bridge and gossip message handling.
pub mod flash_finality;
/// Service factory implementations, including node initialization, consensus
/// wiring, and RPC setup for the X3 Chain blockchain.
pub mod service;

mod logging;

/// Publicly re-export the CLI surface when it is available.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub use cli::{AtomicSwapCmd, AtomicSwapSubcommand, Cli, Commands};

/// Publicly re-export chain specification helpers.
pub use chain_spec::*;

/// Publicly re-export the service layer.
pub use service::*;

/// Run the X3 Chain node.
#[cfg(feature = "cli")]
pub fn run() -> Result<(), sc_cli::Error> {
    command::run()
}

/// Run the X3 Chain node (no-cli fallback).
#[cfg(not(feature = "cli"))]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    Err("CLI feature not enabled".into())
}
