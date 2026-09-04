pub mod coordinator;
pub mod verifier;
pub mod signers;
pub mod ipc;
pub mod substrate_hook;
pub mod wallet_store;
pub mod x3_chain_service;

/// Wallet Core entrypoint.
/// Strict boundary: Only `verifier` talks to RPC. `signers` isolated.
pub struct ExecutionFirewall {}
