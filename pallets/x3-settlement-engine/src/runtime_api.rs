//! Runtime API definitions for X3 Settlement Engine pallet
//!
//! These APIs allow external callers (RPC, off-chain workers, relayers) to query
//! settlement state and submitted proofs without submitting transactions.

use codec::{Codec, Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Transfer ID type for settlement operations
pub type TransferId = H256;

/// Settlement status enumeration
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum SettlementStatus {
    /// Transfer intent created, awaiting external execution
    Pending = 0,
    /// All legs complete and settled
    Completed = 1,
    /// Transfer refunded due to timeout or failure
    Refunded = 2,
    /// Dispute raised, awaiting resolution
    Disputed = 3,
}

/// Settlement transfer information (serializable version for RPC)
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SettlementResponse<AccountId, Balance, BlockNumber> {
    /// Transfer ID
    pub transfer_id: TransferId,
    /// Initiator account
    pub initiator: AccountId,
    /// Amount being transferred
    pub amount: Balance,
    /// Receiver account (if known)
    pub receiver: Option<AccountId>,
    /// Settlement status
    pub status: u8, // 0=Pending, 1=Completed, 2=Refunded, 3=Disputed
    /// Block when transfer was initiated
    pub initiated_at: BlockNumber,
    /// Timeout block (transfer auto-refunds if not completed by this block)
    pub timeout_at: BlockNumber,
    /// Number of legs in this settlement
    pub num_legs: u32,
    /// Number of legs that have completed
    pub legs_completed: u32,
}

/// Pending settlement with timeout information
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct PendingSettlement<AccountId> {
    /// Transfer ID
    pub transfer_id: TransferId,
    /// Amount at risk
    pub amount: u128,
    /// Timeout block
    pub timeout_block: u32,
    /// Receiver (if designated)
    pub receiver: Option<AccountId>,
}

/// Settlement engine status summary
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SettlementStatusResponse {
    /// Whether settlement engine is enabled
    pub engine_enabled: bool,
    /// Total pending transfers
    pub pending_transfers: u32,
    /// Total completed transfers
    pub completed_transfers: u64,
    /// Total refunded transfers
    pub refunded_transfers: u64,
    /// Total amount currently locked in settlements
    pub total_locked: u128,
}

sp_api::decl_runtime_apis! {
    /// Runtime API for querying X3 settlement engine state
    pub trait GovernanceSettlementApi<AccountId, Balance, BlockNumber>
    where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get settlement engine status
        fn settlement_status() -> SettlementStatusResponse;

        /// Get settlement information for a transfer
        fn get_settlement(transfer_id: TransferId) -> Option<SettlementResponse<AccountId, Balance, BlockNumber>>;

        /// Get all pending settlements for an account
        fn get_account_pending_settlements(account: AccountId) -> Vec<SettlementResponse<AccountId, Balance, BlockNumber>>;

        /// Get all pending transfers with timeouts expiring within N blocks
        fn get_pending_settlements_with_timeout(current_block: BlockNumber, window_blocks: BlockNumber) -> Vec<PendingSettlement<AccountId>>;

        /// Check if settlement engine is enabled
        fn is_settlement_enabled() -> bool;

        /// Get total amount currently locked in settlements
        fn get_total_locked_amount() -> Balance;
    }

    /// Runtime API for settlement finality verification (cross-chain proofs)
    pub trait SettlementFinalityApi<BlockNumber>
    where
        BlockNumber: Codec,
    {
        /// Confirm that a proof has reached settlement finality on target chain
        fn confirm_settlement_finality(proof_hash: H256, current_block: BlockNumber) -> bool;

        /// Get finality depth for a proof (how many blocks until settlement is final)
        fn get_finality_depth(proof_hash: H256, current_block: BlockNumber) -> u32;

        /// Check if a settlement operation has all required chain signatures
        fn verify_multi_chain_signatures(transfer_id: TransferId) -> bool;
    }
}
