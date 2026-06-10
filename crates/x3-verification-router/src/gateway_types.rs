//! Gateway-facing types re-exported from x3-verification-router.
//!
//! These types are consumed by x3-crosschain-gateway, x3-gateway-indexer,
//! x3-external-route-registry, and x3-proof-envelope. They bridge the
//! lower-level VerificationRouter API (ChainKind, ProofEnvelope, etc.)
//! to the higher-level gateway abstractions.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::VerificationStrategy;

// ── Chain identity ──────────────────────────────────────────────────────────

/// External blockchain identifiers used by the gateway and route registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalChainId {
    EthereumSepolia,
    BaseSepolia,
    SolanaDevnet,
    EthereumMainnet,
    BaseMainnet,
    SolanaMainnet,
    // Generic fallback for integration tests or unknown chains.
    Other(u64),
}

/// Reference to an asset on an external chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalAssetRef {
    pub chain_id: ExternalChainId,
    pub token_address_or_mint: String,
    pub decimals: u8,
    pub symbol: String,
}

// ── Verification request / result types ─────────────────────────────────────

/// A verification request submitted by the gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequest {
    pub proof_id: [u8; 32],
    pub source_chain: ExternalChainId,
    pub source_block: u64,
    pub source_tx_hash: [u8; 32],
    pub external_asset: ExternalAssetRef,
    pub sender: String,
    pub recipient: String,
    pub amount: u128,
    pub nonce: u64,
    pub proof_payload: Vec<u8>,
    pub strategy: VerificationStrategy,
}

/// Outcome of a verification attempt, returned to the gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    pub proof_id: [u8; 32],
    pub verified: bool,
    pub failure_reason: Option<String>,
    pub chain: ExternalChainId,
    pub verified_at_block: Option<u64>,
    pub strategy: VerificationStrategy,
}
