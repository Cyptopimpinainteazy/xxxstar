//! Northern Swarm — off-chain execution layer for the X3 network.
//!
//! # RC Roadmap
//!
//! | RC  | Crate / Location                 | Description                                      | Status   |
//! |-----|----------------------------------|--------------------------------------------------|----------|
//! | RC1 | `crates/northern-swarm`          | Off-chain executor + JSON-RPC chain watcher      | scaffold |
//! | RC2 | `pallets/northern-swarm`         | On-chain registry v2 (stake, assign, slash)      | scaffold |
//! | RC3 | `crates/northern-swarm` (module) | 3-executor quorum — matching hash wins           | planned  |
//! | RC4 | `crates/x3-compiler` integration | X3 Lang job compiler → deterministic bytecode    | planned  |
//! | RC5 | cross-VM integration             | Agents submit intents/proofs to x3-cross-vm-*    | planned  |
//!
//! ## Legacy
//! `pallets/swarm` is retained as a **historical reference only**.
//! Do not add new production dependencies to `pallets/swarm`.

pub mod chain_watcher;
pub mod executor;
pub mod result_submitter;
pub mod types;

pub use chain_watcher::ChainWatcher;
pub use executor::TaskExecutor;
pub use result_submitter::ResultSubmitter;
pub use types::*;
