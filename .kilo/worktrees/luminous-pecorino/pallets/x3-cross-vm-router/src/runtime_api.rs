//! Runtime API definitions for X3 Cross-VM Router (Bridge) pallet
//!
//! These APIs allow external callers (RPC, off-chain workers, relayers) to query
//! bridge routing state and external chain verification status without submitting transactions.

use codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Chain ID for external chains (EVM networks, Solana, Bitcoin, etc.)
pub type ExternalChainId = u32;

/// Bridge root state information
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BridgeRootState {
    /// Merkle root hash for external chain state
    pub root_hash: H256,
    /// Block number on external chain
    pub block_number: u32,
    /// Timestamp when root was registered
    pub registered_at: u64,
    /// Number of validator signatures confirming this root
    pub signature_count: u32,
}

/// Cross-chain transfer status
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum CrossChainStatus {
    /// Transfer initiated on X3, awaiting external execution
    PendingExecution = 0,
    /// Transfer executed on external chain, awaiting finality
    ExecutedAwaitingFinality = 1,
    /// Transfer finalized and confirmed on external chain
    Finalized = 2,
    /// Transfer failed or timed out
    Failed = 3,
}

/// Bridge route information
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BridgeRoute {
    /// Source chain ID (X3 = 0, Ethereum = 1, Solana = 2, Bitcoin = 3, etc.)
    pub source_chain: ExternalChainId,
    /// Destination chain ID
    pub destination_chain: ExternalChainId,
    /// Is this route currently enabled
    pub enabled: bool,
    /// Minimum confirmation depth required for finality
    pub min_confirmations: u32,
}

/// Proof verification information
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProofVerificationState {
    /// Hash of the proof submitted
    pub proof_hash: H256,
    /// Status of proof verification
    pub verified: bool,
    /// Chain ID this proof applies to
    pub chain_id: ExternalChainId,
    /// Block number on target chain
    pub chain_block_number: u32,
}

sp_api::decl_runtime_apis! {
    /// Bridge router query API for cross-chain validation
    /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
    pub trait BridgeRouterApi {
        /// Get list of supported external chains
        /// Returns vector of chain IDs (Ethereum = 1, Solana = 2, Bitcoin = 3, etc.)
        fn supported_chains() -> Vec<ExternalChainId>;

        /// Get current verified root for a specific external chain
        /// Used for SPV verification and light client state queries
        fn current_root(chain_id: ExternalChainId) -> Option<BridgeRootState>;

        /// Get bridge pause status for a chain
        /// Returns true if bridge is paused (emergency shutdown)
        fn is_bridge_paused(chain_id: ExternalChainId) -> bool;

        /// Check if a specific proof hash has been verified and registered
        fn is_proof_registered(chain_id: ExternalChainId, proof_hash: H256) -> bool;

        /// Get cross-chain transfer status by transfer ID
        fn query_cross_chain_status(transfer_id: H256) -> Option<u8>;

        /// Get all active bridge routes
        fn get_bridge_routes() -> Vec<BridgeRoute>;

        /// Check if route between two chains is enabled
        fn is_route_enabled(source: ExternalChainId, destination: ExternalChainId) -> bool;

        /// Verify cross-chain signatures for a transfer
        /// Returns number of valid signatures collected
        fn verify_cross_chain_signatures(transfer_id: H256, chain_id: ExternalChainId) -> u32;
    }

    /// Bridge settlement finality API
    /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
    pub trait BridgeSettlementApi {
        /// Confirm settlement finality on external chain
        fn confirm_settlement_finality(transfer_id: H256, confirmations: u32) -> bool;

        /// Get finality depth for external chain (reorg risk assessment)
        fn get_finality_depth(chain_id: ExternalChainId) -> u32;

        /// Get proof verification state
        fn get_proof_state(proof_hash: H256) -> Option<ProofVerificationState>;
    }
}
