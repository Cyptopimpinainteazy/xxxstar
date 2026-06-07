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
///
/// This module is only compiled when the `cli` feature flag is active, which is
/// the default for native builds of the node binary.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub mod cli;

/// Command dispatching and execution helpers for CLI invocations.
///
/// This module is conditionally compiled with the `cli` feature flag to ensure
/// the node can still build for Wasm hosts or other environments where a native
/// CLI is unnecessary.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub mod command;

/// Phase 4: RPC Endpoints
/// Custom JSON-RPC methods for authority, EVM, and bridge queries
pub mod rpc;
pub mod rpc_frontier;

/// RPC rate limiting and security middleware
pub mod rpc_middleware;

/// Phase 5: Network Bootstrapping
/// Bootstrap configuration, peer discovery, and protocol settings
pub mod network;

/// Phase 6: Validator Setup
/// Validator registration, session key derivation, and key rotation
pub mod authority;

/// Phase 7: Telemetry/Monitoring
/// Prometheus metrics, health checks, and observability
pub mod metrics;

/// Chain specification constructors and utilities used to create X3 Chain
/// network configurations.
pub mod chain_spec;

/// Flash Finality network bridge and gossip message handling.
pub mod flash_finality;
/// Service factory implementations, including node initialization, consensus
/// wiring, and RPC setup for the X3 Chain blockchain.
pub mod service;

// Local helper for colorful CLI logging
mod logging;

/// Publicly re-export the CLI surface when it is available.
///
/// Downstream crates can bring [`Cli`](cli::Cli) and related types into scope
/// without depending on the internal module layout.
#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub use cli::{AtomicSwapCmd, AtomicSwapSubcommand, Cli, Commands};

// Command module internals are accessed via the `run()` function below.

/// Publicly re-export chain specification helpers for consumers that need to
/// programmatically construct custom networks.
pub use chain_spec::*;

/// Publicly re-export the service layer so that external tools can spin up
/// X3 Chain nodes (full or light) without reaching into module internals.
pub use service::*;

/// Run the X3 Chain node
#[cfg(feature = "cli")]
pub fn run() -> Result<(), sc_cli::Error> {
    command::run()
}

/// Run the X3 Chain node (no-cli fallback)
#[cfg(not(feature = "cli"))]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    Err("CLI feature not enabled".into())
}
