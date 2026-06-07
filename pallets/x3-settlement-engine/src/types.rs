//! Core types for the X3 Settlement Engine

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Maximum size for escrow address data
pub const MAX_ESCROW_ADDRESS_SIZE: u32 = 64;
/// Maximum merkle proof depth
pub const MAX_MERKLE_PROOF_DEPTH: u32 = 32;
/// Maximum receipt data size
pub const MAX_RECEIPT_DATA_SIZE: u32 = 1024;

// ============================================================================
// Intent Types
// ============================================================================

/// Settlement intent: the source of truth for an atomic swap
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
#[scale_info(skip_type_params(AccountId))]
pub struct SettlementIntent<AccountId> {
    /// Unique intent identifier
    pub intent_id: H256,
    /// Maker (initiator) of the swap
    pub maker: AccountId,
    /// Taker (counterparty) of the swap
    pub taker: AccountId,
    /// Asset A (maker sends to taker)
    pub asset_a: AssetSpec,
    /// Asset B (taker sends to maker)
    pub asset_b: AssetSpec,
    /// Hash of the secret (for HTLC)
    pub secret_hash: H256,
    /// Unix timestamp when settlement expires
    pub timeout: u64,
    /// Unix timestamp when intent was created
    pub created_at: u64,
    /// Total number of settlement legs
    pub legs_total: u32,
    /// Number of legs with locked escrow
    pub legs_locked: u32,
    /// Number of legs that have been claimed
    pub legs_claimed: u32,
}

/// Asset specification (chain + token + amount)
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
pub struct AssetSpec {
    /// Chain where asset resides
    pub chain: ExternalChainId,
    /// Token identifier (contract address, mint address, or "native")
    pub token: TokenId,
    /// Amount in smallest units
    pub amount: u128,
}

/// Token identifier (chain-agnostic)
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
pub enum TokenId {
    /// Native currency (ETH, SOL, BTC)
    Native,
    /// ERC20/SPL token by address (20 or 32 bytes)
    Contract([u8; 32]),
    /// X3 asset ID
    X3Asset(u32),
}

impl Default for TokenId {
    fn default() -> Self {
        Self::Native
    }
}

/// Intent state machine states
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
    Default,
)]
pub enum IntentState {
    /// Intent created, waiting for escrows
    #[default]
    Created,
    /// Some escrows locked, waiting for more
    FundingInProgress,
    /// All escrows locked, ready for execution
    FullyFunded,
    /// External execution in progress
    ExecutingExternal,
    /// Claim in progress (secret revealed)
    Claiming,
    /// Successfully finalized
    Finalized,
    /// Refunded due to timeout or failure
    Refunded,
    /// Halted due to invariant violation
    Halted,
}

/// Settlement transfer: tracks individual cross-chain settlement operations
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct SettlementTransfer<AccountId, Balance> {
    /// Unique transfer identifier (H256)
    pub transfer_id: H256,
    /// Initiator account (who started the settlement)
    pub initiator: AccountId,
    /// Receiver account (who will receive settled assets)
    pub receiver: Option<AccountId>,
    /// Amount being settled
    pub amount: Balance,
    /// Settlement status (0=Pending, 1=Completed, 2=Refunded, 3=Disputed)
    pub status: u8,
    /// Block when transfer was initiated
    pub initiated_at: u32,
    /// Timeout block (auto-refund if not completed by this block)
    pub timeout_at: u32,
    /// Number of settlement legs
    pub num_legs: u32,
    /// Number of completed legs
    pub legs_completed: u32,
}

// ============================================================================
// Escrow Types
// ============================================================================

/// Escrow leg for a settlement intent
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
#[scale_info(skip_type_params(AccountId))]
pub struct EscrowLeg<AccountId> {
    /// Parent intent ID
    pub intent_id: H256,
    /// Leg index within intent
    pub leg_index: u32,
    /// Account that deposited
    pub depositor: AccountId,
    /// Chain where escrow lives
    pub chain: ExternalChainId,
    /// Amount locked
    pub amount: u128,
    /// Escrow address/script (chain-specific)
    pub escrow_address: BoundedVec<u8, ConstU32<MAX_ESCROW_ADDRESS_SIZE>>,
    /// Current state
    pub state: EscrowLegState,
    /// When escrow was locked
    pub locked_at: u64,
    /// Proof of external execution (if any)
    pub proof: Option<SettlementProof>,
}

/// Escrow leg state
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
    Default,
)]
pub enum EscrowLegState {
    /// Waiting for deposit
    #[default]
    Pending,
    /// Assets locked in escrow
    Locked,
    /// Released to recipient
    Released,
    /// Refunded to depositor
    Refunded,
}

// ============================================================================
// BTC Types
// ============================================================================

/// BTC UTXO state tracked by X3
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen)]
pub struct BtcUtxoState {
    /// Transaction ID
    pub txid: H256,
    /// Output index
    pub vout: u32,
    /// Amount in satoshis
    pub amount_sats: u64,
    /// Associated intent (if any)
    pub intent_id: Option<H256>,
    /// Number of confirmations
    pub confirmations: u32,
    /// Whether UTXO has been spent
    pub spent: bool,
    /// Block hash containing this UTXO
    pub block_hash: H256,
}

/// BTC block header (80 bytes, compact)
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
pub struct BtcBlockHeader {
    /// Block version
    pub version: u32,
    /// Previous block hash
    pub prev_block_hash: H256,
    /// Merkle root of transactions
    pub merkle_root: H256,
    /// Block timestamp
    pub timestamp: u32,
    /// Difficulty target (nBits)
    pub bits: u32,
    /// Nonce for PoW
    pub nonce: u32,
    /// Block height (not in actual header, but needed)
    pub height: u64,
}

// ============================================================================
// Proof Types
// ============================================================================

/// Settlement proof for external chain verification
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
pub struct SettlementProof {
    /// Type of proof
    pub proof_type: ProofType,
    /// Transaction hash on external chain
    pub tx_hash: H256,
    /// Block hash containing the transaction
    pub block_hash: H256,
    /// Number of confirmations
    pub confirmations: u32,
    /// Merkle proof (for MPT or SPV)
    pub merkle_proof: BoundedVec<H256, ConstU32<MAX_MERKLE_PROOF_DEPTH>>,
    /// Receipt/witness data (chain-specific)
    pub receipt_data: BoundedVec<u8, ConstU32<MAX_RECEIPT_DATA_SIZE>>,
}

/// Proof type used for verification
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
    Default,
)]
pub enum ProofType {
    /// Merkle Patricia Trie proof (EVM)
    #[default]
    MerkleTrie,
    /// Bitcoin SPV proof
    BitcoinSpv,
    /// Solana proof
    SolanaProof,
    /// ZK proof (for L2s)
    ZkProof,
    /// Light client state proof
    LightClient,
    /// Optimistic (fraud proof window)
    Optimistic,
}

// ============================================================================
// Chain Types
// ============================================================================

/// External chain identifier
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
    Hash,
    Default,
)]
pub enum ExternalChainId {
    /// Native X3 chain (EVM/SVM/X3VM internal)
    #[default]
    X3Native,
    /// Bitcoin mainnet
    Bitcoin,
    /// Bitcoin testnet
    BitcoinTestnet,
    /// Ethereum mainnet
    Ethereum,
    /// Arbitrum One
    Arbitrum,
    /// Base
    Base,
    /// Polygon PoS
    Polygon,
    /// Optimism
    Optimism,
    /// Avalanche C-Chain
    Avalanche,
    /// BNB Smart Chain
    Bnb,
    /// Solana mainnet
    Solana,
    /// Solana devnet
    SolanaDevnet,
    /// Generic EVM chain by chain ID
    EvmChain(u64),
}

/// Finality configuration for a chain
#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
)]
pub struct FinalityConfig {
    /// Chain ID
    pub chain: ExternalChainId,
    /// Required confirmations for finality
    pub confirmations_required: u32,
    /// Average block time in milliseconds
    pub block_time_ms: u64,
    /// Proof type to use
    pub proof_type: ProofType,
    /// Challenge period (for optimistic proofs)
    pub challenge_period_seconds: u64,
    /// Maximum reorg depth observed
    pub max_reorg_depth: u32,
}

// ============================================================================
// Refund/Error Types
// ============================================================================

/// Reason for settlement refund
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
pub enum RefundReason {
    /// Settlement timeout expired
    Timeout,
    /// External execution failed
    ExternalFailure,
    /// Invariant violation detected
    InvariantViolation,
    /// User requested cancellation (before fully funded)
    UserCancelled,
    /// Proof verification failed
    ProofRejected,
    /// BTC confirmation timeout
    BtcConfirmationTimeout,
}

/// Types of invariant violations (CRITICAL)
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
pub enum InvariantViolationType {
    /// Settlement finalized with incomplete legs
    PartialExecution,
    /// Cross-VM reentrancy detected
    CrossVmReentrancy,
    /// BTC released without X3 confirmation
    BtcReleaseWithoutConfirmation,
    /// Timeout rules bypassed
    TimeoutBypass,
}

// ============================================================================
// Settlement Events (Canonical Schemas)
// ============================================================================

/// Canonical event: Trade matched
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct TradeMatchedEvent<AccountId> {
    pub match_id: H256,
    pub maker: AccountId,
    pub taker: AccountId,
    pub price: u128,
    pub amount: u128,
    pub timestamp: u64,
}

/// Canonical event: Settlement intent created on X3
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct X3IntentCreatedEvent<AccountId> {
    pub intent_id: H256,
    pub maker: AccountId,
    pub taker: AccountId,
    pub asset_a: AssetSpec,
    pub asset_b: AssetSpec,
    pub secret_hash: H256,
    pub timeout: u64,
}

/// Canonical event: Assets locked in X3 escrow
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct X3AssetsLockedEvent {
    pub intent_id: H256,
    pub leg_index: u32,
    pub chain: ExternalChainId,
    pub amount: u128,
    pub escrow_address: Vec<u8>,
}

/// Canonical event: External proof submitted
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct ExternalProofSubmittedEvent {
    pub intent_id: H256,
    pub chain: ExternalChainId,
    pub proof_type: ProofType,
    pub tx_hash: H256,
    pub confirmations: u32,
}

/// Canonical event: Settlement finalized on X3
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct X3FinalizedEvent {
    pub intent_id: H256,
    pub maker_received: u128,
    pub taker_received: u128,
    pub settlement_time_ms: u64,
}

/// Canonical event: Settlement refunded
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct X3RefundedEvent {
    pub intent_id: H256,
    pub reason: RefundReason,
    pub maker_returned: u128,
    pub taker_returned: u128,
}
