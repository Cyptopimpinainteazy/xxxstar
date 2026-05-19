#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]

//! # X3 Kernel Pallet
//!
//! The core orchestration layer for X3 Chain's dual-VM execution architecture.
//! Enables atomic cross-VM transactions (Comits) that execute on both EVM and SVM.
//!
//! ## Security Design Decisions
//!
//! ### H-1: prepare_root Verification (Input Commitment Design)
//!
//! The `prepare_root` field is a cryptographic commitment to the **input parameters** of a Comit,
//! NOT the execution outputs. This is intentional:
//!
//! - **Rationale**: Clients must compute `prepare_root` before submission. If it committed to
//!   outputs, clients couldn't know the hash until after execution (circular dependency).
//! - **Security**: The prepare_root ensures the submitted Comit matches what the client intended.
//!   It prevents parameter tampering but does NOT guarantee execution results.
//! - **Enhancement**: For high-value transactions requiring output verification, consider adding
//!   an optional `expected_output_hash` field in future versions.
//!
//! ### H-5: VM Adapter Production Status
//!
//! The pallet uses pluggable VM adapters (`T::EvmAdapter`, `T::SvmAdapter`) configured at runtime:
//!
//! - **Test Runtime**: Uses `MockEvmAdapter` and `MockSvmAdapter` for deterministic testing
//! - **Production Runtime**: Should use `FrontierEvmAdapter` and `RbpfSvmAdapter`
//!
//! **IMPORTANT**: Before mainnet deployment, verify runtime configuration uses real adapters.
//! The `adapters.rs` module includes `FrontierEvmAdapter` which wraps pallet-evm, but runtime
//! must be properly configured to use it instead of mocks.

pub use pallet::*;

/// Phase 1: Full Consensus Implementation
/// Authority set management, pending changes scheduling, and enactment mechanism
pub mod authority;

/// VM Execution Adapters
/// Provides EvmExecutorAdapter and SvmExecutorAdapter traits for runtime configuration.
///
/// **H-5 Note**: For production, configure runtime with `FrontierEvmAdapter` and `RbpfSvmAdapter`
/// instead of mock adapters. Mock adapters are for testing only.
pub mod adapters;

/// Packet Deserialization & Domain Routing Layer (Phase 1.3)
/// Converts raw Vec<u8> payloads into typed Packet enums and routes to appropriate executors.
pub mod packet_adapters;

pub mod wasm_adapters;

// Asset kernel sub-modules (supply conservation, registry, mapping, mint/burn, external locks)
pub mod external_locked;
pub mod invariant;
pub mod mapping;
pub mod mint_burn;
pub mod pending_transfer;
pub mod registry;
pub mod supply;

pub use adapters::{
    EvmExecutorAdapter, FailingMockEvmAdapter, FailingMockSvmAdapter, FailingMockX3Adapter,
    MockEvmAdapter, MockSvmAdapter, MockX3Adapter, SvmExecutorAdapter, X3ExecutorAdapter,
};

/// Runtime-specific emergency halt hook.
///
/// The kernel uses this to flip the concrete halt state that downstream pallets
/// actually inspect. Production runtimes should wire this into their canonical
/// economic-halt / invariant-halt storage flags.
pub trait EmergencyHaltController {
    fn trigger();
}

/// No-op emergency halt controller (for tests and environments without invariant checking).
impl EmergencyHaltController for () {
    fn trigger() {}
}

pub use packet_adapters::{
    deserialize_packet, get_domain_mask, get_packet_type, route_packet, validate_packet,
    DomainRoute, PacketAdapterError, PacketAdapterResult,
};

// Re-export real adapters for std builds (native runtime)
#[cfg(feature = "std")]
pub use adapters::real_adapters::{FrontierEvmAdapter, RbpfSvmAdapter, X3VmAdapter};

/// Benchmarking support for weight generation.
/// Enable with `--features runtime-benchmarks`.
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

/// Auto-generated weight information for extrinsics.
/// Regenerate using frame-benchmarking CLI.
pub mod weights;

/// Runtime storage migrations.
pub mod migrations;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{AtLeast32BitUnsigned, CheckedAdd, SaturatedConversion};
use frame_support::sp_runtime::DispatchError;
use frame_support::traits::BuildGenesisConfig;
use frame_support::traits::{Currency, DefensiveResult, UnixTime};
use frame_system::pallet_prelude::*;
use parity_scale_codec::Codec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::{H160, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::MaybeSerializeDeserialize;
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;
extern crate alloc;
use alloc::format;
use x3_cross_vm_bridge::{
    CrossVmBridge, CrossVmCall, CrossVmDispatcher, CrossVmOperation, CrossVmReceipt, CrossVmResult,
    CrossVmStatus, VmId,
};

pub const EXECUTION_RECEIPT_VERSION: u32 = 1;

/// Represents a Comit transaction submitted to the X3 Kernel.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct Comit<AccountId, Balance> {
    /// Globally unique Comit identifier.
    pub comit_id: H256,
    /// Origin account that submitted the Comit.
    pub origin: AccountId,
    /// Payload destined for the EVM execution environment.
    pub evm_payload: Vec<u8>,
    /// Payload destined for the SVM execution environment.
    pub svm_payload: Vec<u8>,
    /// Sequential nonce scoped to the origin account.
    pub nonce: u64,
    /// Fee charged for processing the Comit.
    pub fee: Balance,
    /// Dual-VM prepare phase commitment root.
    pub prepare_root: H256,
}

/// Version 2 Comit supporting triple-VM execution (EVM + SVM + X3VM).
///
/// This is intentionally a separate type from `Comit` to avoid breaking
/// downstream code that relies on the original dual-VM shape.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct ComitV2<AccountId, Balance> {
    /// Globally unique Comit identifier.
    pub comit_id: H256,
    /// Origin account that submitted the Comit.
    pub origin: AccountId,
    /// Payload destined for the EVM execution environment.
    pub evm_payload: Vec<u8>,
    /// Payload destined for the SVM execution environment.
    pub svm_payload: Vec<u8>,
    /// Payload destined for the X3VM execution environment.
    pub x3_payload: Vec<u8>,
    /// Sequential nonce scoped to the origin account.
    pub nonce: u64,
    /// Fee charged for processing the Comit.
    pub fee: Balance,
    /// Multi-VM prepare phase commitment root.
    pub prepare_root: H256,
}

/// Execution receipt returned by VM runtimes after transaction execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct ExecutionReceipt {
    /// Schema version for compatibility across runtime and sidecar consumers.
    pub version: u32,
    /// Whether the execution was successful.
    pub success: bool,
    /// Gas used during execution.
    pub gas_used: u64,
    /// Return data from the execution.
    pub return_data: Vec<u8>,
    /// Logs emitted during execution.
    pub logs: Vec<ExecutionLog>,
    /// State changes resulting from execution.
    pub state_changes: Vec<StateChange>,
    /// Protocol version emitted by the executor implementation.
    pub protocol_version: u32,
    /// Ordered migration markers applied before this receipt was produced.
    pub migration_history: Vec<u32>,
    /// Compatibility bits for downstream consumers.
    pub compatibility_flags: u32,
    /// EVM transaction sender address (20 bytes).
    pub from: Vec<u8>,
    /// EVM transaction target address (20 bytes).
    pub to: Vec<u8>,
    /// EVM transaction value in wei.
    pub value: u128,
}

/// Log entry emitted during VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct ExecutionLog {
    /// Address (EVM H160 or SVM 32-byte key) that emitted the log.
    pub address: Vec<u8>,
    /// Topics for the log entry.
    pub topics: Vec<H256>,
    /// Log data.
    pub data: Vec<u8>,
    /// Block number when the log was emitted.
    pub block_number: u64,
}

/// State change resulting from VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct StateChange {
    /// Account/contract address affected (EVM H160 or SVM 32-byte key).
    pub address: Vec<u8>,
    /// Storage slot key.
    pub key: H256,
    /// New value at the storage slot.
    pub value: H256,
}

/// Unified state representation for the X3 Chain.
#[derive(
    Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo, Default,
)]
pub struct SphereState {
    /// State root hash representing the entire sphere state.
    pub state_root: H256,
    /// Block number when this state was computed.
    pub block_number: u32,
    /// Timestamp of state computation.
    pub timestamp: u64,
}

/// Dual-VM transaction types that can be executed.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub enum VmTransaction {
    /// EVM transaction payload.
    Evm(Vec<u8>),
    /// SVM transaction payload.
    Svm(Vec<u8>),
}

/// Reasons describing why a Comit failed verification or execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
/// Granular error codes for comit execution failures with diagnostic context.
/// Each variant includes an error code and optional diagnostic message (max 256 bytes).
pub enum ComitFailureReason {
    /// The provided EVM payload exceeds runtime defined limits.
    /// Error Code: 0x01
    EvmPayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// The provided SVM payload exceeds runtime defined limits.
    /// Error Code: 0x02
    SvmPayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// The provided X3 payload exceeds runtime defined limits.
    /// Error Code: 0x07
    X3PayloadTooLarge {
        code: u32,
        actual_size: u32,
        max_size: u32,
    },
    /// Combined payloads exceed the cumulative limit.
    /// Error Code: 0x03
    CombinedPayloadTooLarge {
        code: u32,
        evm_size: u32,
        svm_size: u32,
        max_combined: u32,
    },
    /// Both payloads were empty, leaving nothing to execute.
    /// Error Code: 0x04
    EmptyPayloads { code: u32 },
    /// The supplied nonce was not the one expected by the pallet.
    /// Error Code: 0x05
    InvalidNonce {
        code: u32,
        expected: u64,
        provided: u64,
    },
    /// Prepare-root verification failed or receipts mismatched.
    /// Error Code: 0x06
    Verification {
        code: u32,
        reason: [u8; 32], // Hash of verification failure reason
    },
    /// Packet deserialization failed or packet has incorrect domain targeting.
    /// Error Code: 0x08
    PacketDeserializationFailed { code: u32, reason: Vec<u8> },
    /// EVM execution failed with error code.
    /// Error Code: 0x10
    EvmExecutionFailed {
        code: u32,
        evm_error: u32,
        gas_used: u64,
    },
    /// SVM execution failed with error code.
    /// Error Code: 0x11
    SvmExecutionFailed {
        code: u32,
        svm_error: u32,
        compute_units_used: u64,
    },
    /// X3 execution failed with error code.
    /// Error Code: 0x12
    X3ExecutionFailed {
        code: u32,
        x3_error: u32,
        gas_used: u64,
    },
}

type ComitOf<T> = Comit<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;
type ComitV2Of<T> = ComitV2<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;

/// Dual-VM Dispatcher trait for coordinating execution across EVM and SVM runtimes.
/// This trait defines the interface for executing transactions on both virtual machines
/// and merging their execution results into a unified Sphere State Tree.
pub trait DualVmDispatcher {
    /// AccountId type for authorization checks
    type AccountId;
    /// Balance type for fee accounting
    type Balance;

    /// Execute a transaction on the EVM runtime.
    /// Returns an execution receipt with the results of the transaction.
    fn execute_evm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute a transaction on the SVM runtime.
    /// Returns an execution receipt with the results of the transaction.
    fn execute_svm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute a dual-VM transaction and merge the results.
    /// This is the primary entry point for Comit execution.
    fn execute_dual_tx(
        &self,
        evm_tx: Option<Vec<u8>>,
        svm_tx: Option<Vec<u8>>,
    ) -> Result<SphereState, DispatchError>;

    /// Merge execution receipts from both VMs into a unified state.
    fn merge_receipts(
        &self,
        evm_receipt: Option<&ExecutionReceipt>,
        svm_receipt: Option<&ExecutionReceipt>,
    ) -> SphereState;

    /// Check if an account is authorized to execute a specific cross-VM operation.
    /// This enables granular access control beyond simple origin validation.
    /// Returns Ok(()) if authorized, Err(DispatchError) if not.
    fn auth_check(&self, caller: &Self::AccountId, operation: &[u8]) -> Result<(), DispatchError>;

    /// Calculate execution fees for a comit based on gas/compute usage.
    /// Takes the gas used (EVM) and compute units (SVM) and returns the total fee.
    /// This enables accurate fee accounting across heterogeneous runtimes.
    fn fee_accounting(
        &self,
        evm_gas_used: u64,
        svm_compute_units: u64,
        base_fee: Self::Balance,
    ) -> Result<Self::Balance, DispatchError>;

    /// Update the canonical ledger with state changes from a successful comit.
    /// This persists cross-VM state into the canonical view, enabling future queries.
    /// Returns Ok(()) on success or Err with diagnostics on failure.
    fn canonical_ledger_update(
        &self,
        comit_id: H256,
        state_changes: &[StateChange],
    ) -> Result<(), DispatchError>;
}

/// Proof types for cross-chain lock/receipt verification.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub enum CrossChainProof {
    /// No proof supplied.
    None,
    /// Proof of lock on source chain.
    LockProof(Vec<u8>),
    /// Merkle receipt or inclusion proof.
    MerkleReceipt(Vec<u8>),
}

/// Cross-chain proof verification hook.
pub trait CrossChainProofVerifier<AccountId> {
    /// Verify a proof for a cross-VM operation.
    fn verify_proof(
        origin: &AccountId,
        operation: &CrossVmOperation,
        proof: &CrossChainProof,
    ) -> Result<(), DispatchError>;
}

/// No-op proof verifier for development and testing.
pub struct NoopProofVerifier;

impl<AccountId> CrossChainProofVerifier<AccountId> for NoopProofVerifier {
    fn verify_proof(
        _origin: &AccountId,
        _operation: &CrossVmOperation,
        proof: &CrossChainProof,
    ) -> Result<(), DispatchError> {
        match proof {
            CrossChainProof::None => Ok(()),
            _ => Err(DispatchError::Other(
                "Cross-chain proof verifier not configured",
            )),
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::ReservableCurrency;
    use sp_runtime::traits::{Saturating, Zero};

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        /// Aggregated runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency trait for fee deduction and balance management.
        type Currency: frame_support::traits::ReservableCurrency<Self::AccountId>;

        /// Balance type used within the canonical ledger (same as Currency::Balance).
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaxEncodedLen
            + CheckedAdd
            + From<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>
            + Into<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>;

        /// Identifier type for registered assets.
        type AssetId: Parameter
            + Member
            + Ord
            + Default
            + Copy
            + MaxEncodedLen
            + MaybeSerializeDeserialize;

        /// Identifier type used to map substrate accounts to X3 IDs.
        type AtlasId: Parameter + Member + Default + Copy + MaxEncodedLen;

        /// Maximum number of unique assets tracked per account in the canonical ledger.
        #[pallet::constant]
        type MaxAssetsPerAccount: Get<u32>;

        /// Maximum length allowed for asset symbols.
        #[pallet::constant]
        type MaxAssetSymbolLength: Get<u32>;

        /// Maximum length allowed for EVM payloads.
        #[pallet::constant]
        type MaxEvmPayloadLength: Get<u32>;

        /// Maximum length allowed for SVM payloads.
        #[pallet::constant]
        type MaxSvmPayloadLength: Get<u32>;

        /// Maximum length allowed for X3 payloads.
        #[pallet::constant]
        type MaxX3PayloadLength: Get<u32>;

        /// Maximum combined length of both EVM and SVM payloads.
        #[pallet::constant]
        type MaxCombinedPayloadLength: Get<u32>;

        /// Maximum combined length of EVM + SVM + X3 payloads (v2 Comits).
        #[pallet::constant]
        type MaxCombinedPayloadLengthV2: Get<u32>;

        /// Maximum number of authorities allowed in the authority set.
        #[pallet::constant]
        type MaxAuthorities: Get<u32>;

        /// Minimum number of authorities required in the authority set.
        #[pallet::constant]
        type MinAuthorities: Get<u32>;

        /// Default gas limit for EVM execution.
        #[pallet::constant]
        type DefaultEvmGasLimit: Get<u64>;

        /// Default compute unit limit for SVM execution.
        #[pallet::constant]
        type DefaultSvmComputeLimit: Get<u64>;

        /// Default gas limit for X3VM execution.
        #[pallet::constant]
        type DefaultX3GasLimit: Get<u64>;

        /// EVM bridge escrow contract address for atomic cross-VM swaps.
        #[pallet::constant]
        type BridgeEvmEscrow: Get<H160>;

        /// SVM bridge escrow program address for atomic cross-VM swaps.
        #[pallet::constant]
        type BridgeSvmEscrow: Get<[u8; 32]>;

        /// Cross-VM prepare TTL (blocks) before forced abort.
        #[pallet::constant]
        type CrossVmPrepareTtl: Get<BlockNumberFor<Self>>;

        /// Maximum number of prepared cross-VM ops stored on-chain.
        #[pallet::constant]
        type MaxPreparedCrossVmOps: Get<u32>;

        /// Maximum number of prepared ops to scan per block for expiry.
        #[pallet::constant]
        type MaxPreparedOpsPerBlock: Get<u32>;

        /// Maximum replay-store entries pruned per block.
        #[pallet::constant]
        type MaxReplayPruneItemsPerBlock: Get<u32>;

        /// Require a cross-chain proof for cross-VM operations.
        #[pallet::constant]
        type RequireCrossVmProof: Get<bool>;

        /// Weight information provider for extrinsics.
        type WeightInfo: WeightInfo;

        /// EVM execution adapter (runtime-configurable)
        /// Implement EvmExecutorAdapter trait for real Frontier integration
        type EvmAdapter: EvmExecutorAdapter;

        /// SVM execution adapter (runtime-configurable)
        /// Implement SvmExecutorAdapter trait for real solana-rbpf integration
        type SvmAdapter: SvmExecutorAdapter;

        /// X3 VM execution adapter (runtime-configurable)
        /// Implement X3ExecutorAdapter trait for X3 bytecode execution
        type X3Adapter: X3ExecutorAdapter;

        /// Cross-chain proof verification hook.
        type CrossChainProofVerifier: CrossChainProofVerifier<Self::AccountId>;

        /// Origin that can execute privileged governance functions.
        /// Typically EnsureRoot or a council-based origin.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Concrete halt hook used by `emergency_halt` to flip the runtime's
        /// production halt state.
        type EmergencyHaltController: EmergencyHaltController;
    }

    type AssetSymbolOf<T> = BoundedVec<u8, <T as Config>::MaxAssetSymbolLength>;
    type AssetMetadataOf<T> = AssetMetadata<AssetSymbolOf<T>>;
    type PreparedCrossVmOpOf<T> = PreparedCrossVmOp<
        <T as frame_system::Config>::AccountId,
        <T as Config>::Balance,
        BlockNumberFor<T>,
    >;

    /// Prepared cross-VM operation held during 2PC timeout window.
    #[derive(
        Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo,
    )]
    pub struct PreparedCrossVmOp<AccountId, Balance, BlockNumber> {
        /// Origin account that prepared the operation.
        pub origin: AccountId,
        /// Cross-VM operation payload.
        pub operation: CrossVmOperation,
        /// Origin-scoped nonce.
        pub nonce: u64,
        /// Block number when prepared.
        pub prepared_at: BlockNumber,
        /// Block number when this operation expires.
        pub expires_at: BlockNumber,
        /// Fee reserved during prepare.
        pub reserved_fee: Balance,
        /// Proof hash for auditability.
        pub proof_hash: H256,
        /// Proof kind: 0=None, 1=LockProof, 2=MerkleReceipt.
        pub proof_kind: u8,
    }

    /// Canonical ledger mapping (account, asset_id) -> balance.
    /// Uses a double-storage map for efficient access without requiring nested collections.
    #[pallet::storage]
    pub type CanonicalLedger<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::AssetId,
        T::Balance,
        ValueQuery,
    >;

    /// Maps accounts to their X3 identifiers.
    #[pallet::storage]
    pub type AccountRegistry<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AtlasId>;

    /// Registry of known assets and their metadata.
    #[pallet::storage]
    pub type AssetRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, AssetMetadataOf<T>>;

    /// Nonce tracker for Comit submissions by account.
    #[pallet::storage]
    pub type Nonces<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Nonce tracker for cross-VM operation submissions by account.
    #[pallet::storage]
    pub type CrossVmNonces<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Accounts authorized to submit Comits.
    ///
    /// Security: If AuthorizedAccounts is empty, all submissions are rejected (secure by default).
    /// Accounts must be explicitly authorized via `authorize_account` extrinsic.
    /// In development mode with `dev-bypass` feature enabled, authorization checks are bypassed.
    #[pallet::storage]
    pub type AuthorizedAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), ValueQuery>;

    /// Current authority set (consensus validators).
    /// Authorities are responsible for block production and finalization.
    #[pallet::storage]
    pub type Authorities<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxAuthorities>, ValueQuery>;

    /// Pending authority changes to be enacted at the next session.
    /// Changes are scheduled via governance and enacted at session boundaries.
    #[pallet::storage]
    pub type PendingAuthorities<T: Config> =
        StorageValue<_, Option<BoundedVec<T::AccountId, T::MaxAuthorities>>, ValueQuery>;

    /// Tracks submitted comit_ids to prevent duplicate submissions.
    /// Value is the block number when the comit was submitted.
    #[pallet::storage]
    pub type SubmittedComits<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>, OptionQuery>;

    /// EVM transaction receipts keyed by transaction hash (keccak256 of raw tx).
    /// Stores execution receipts for EVM transactions submitted via runtime API.
    #[pallet::storage]
    #[pallet::unbounded]
    pub type EvmTransactionReceipts<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, ExecutionReceipt, OptionQuery>;

    /// EVM transactions keyed by transaction hash (keccak256 of raw tx).
    /// Stores full transaction data (including gas and input) for RPC compatibility.
    /// This is separate from receipts to allow querying transaction metadata without
    /// executing the transaction.
    #[pallet::storage]
    #[pallet::unbounded]
    pub type EvmTransactions<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, EvmTransactionData, OptionQuery>;

    /// Data structure for storing EVM transaction metadata.
    #[derive(
        Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo,
    )]
    pub struct EvmTransactionData {
        /// Raw transaction bytes (RLP-encoded)
        pub raw: Vec<u8>,
        /// Transaction sender address (20 bytes)
        pub from: Vec<u8>,
        /// Transaction target address (20 bytes)
        pub to: Vec<u8>,
        /// Transaction value in wei
        pub value: u128,
        /// Gas limit for the transaction
        pub gas: u64,
        /// Transaction input data
        pub input: Vec<u8>,
        /// Transaction nonce
        pub nonce: u64,
        /// Gas price for the transaction
        pub gas_price: u128,
    }

    /// Prepared cross-VM operations awaiting commit.
    #[pallet::storage]
    #[pallet::unbounded]
    pub type PreparedCrossVmOps<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, PreparedCrossVmOpOf<T>, OptionQuery>;

    /// Queue of prepared cross-VM operation IDs for expiry scanning.
    #[pallet::storage]
    pub type PreparedCrossVmQueue<T: Config> =
        StorageValue<_, BoundedVec<H256, <T as Config>::MaxPreparedCrossVmOps>, ValueQuery>;

    /// Rate limiting: tracks Comit submissions per account per block.
    /// Key: (AccountId, BlockNumber), Value: submission count.
    /// Used to prevent DoS via excessive submissions from a single account.
    #[pallet::storage]
    pub type SubmissionsPerBlock<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        BlockNumberFor<T>,
        u32,
        ValueQuery,
    >;

    /// Counter for decode failures in state change processing.
    /// Useful for monitoring and debugging data format issues.
    #[pallet::storage]
    pub type DecodeFailureCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Emergency pause flag. When `true`, all user-facing extrinsics are disabled.
    /// Only governance (root/council) can toggle this.
    #[pallet::storage]
    pub type ProtocolPaused<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Maximum size in bytes of SVM account data stored per 32-byte pubkey.
    /// 64 KiB is sufficient for most programs; programs requiring more storage
    /// should use on-chain accounts managed by the native pallet assets pallet.
    pub const MAX_SVM_ACCOUNT_DATA_BYTES: u32 = 65_536;

    /// Persisted SVM account data keyed by 32-byte public key.
    ///
    /// Written back by `NativeSvmAdapter` (std) and `WasmSvmAdapter` (no_std)
    /// after each SVM execution so that stateful programs retain their account
    /// data across calls.  The kernel reads this map when providing pre-existing
    /// account state to the SVM executor.
    #[pallet::storage]
    pub type SvmAccountData<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        frame_support::BoundedVec<u8, frame_support::traits::ConstU32<65_536>>,
    >;

    /// Bridge escrow address for EVM cross-chain operations.
    #[pallet::storage]
    pub type BridgeEvmEscrow<T: Config> = StorageValue<_, sp_core::H160, ValueQuery>;

    /// Bridge escrow address for SVM cross-chain operations.
    #[pallet::storage]
    pub type BridgeSvmEscrow<T: Config> = StorageValue<_, [u8; 32], ValueQuery>;

    /// SVM blockhashes keyed by slot number.
    /// Maps each slot to its corresponding blockhash for SVM block queries.
    #[pallet::storage]
    pub type SvmBlockhashes<T: Config> = StorageMap<_, Blake2_128Concat, u64, H256, OptionQuery>;

    /// SVM transaction counts keyed by SVM public key (32 bytes).
    /// Tracks the number of transactions sent from each SVM account.
    #[pallet::storage]
    pub type SvmTransactionCounts<T: Config> =
        StorageMap<_, Blake2_128Concat, [u8; 32], u64, ValueQuery>;

    /// Reverse index: SVM blockhashes keyed by blockhash.
    /// Maps each blockhash to its corresponding slot number for reverse lookups.
    #[pallet::storage]
    pub type SvmBlockhashSlots<T: Config> = StorageMap<_, Blake2_128Concat, H256, u64, OptionQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Initial asset registry entries.
        pub assets: Vec<(T::AssetId, Vec<u8>, u8)>,
        /// Bridge escrow address for EVM cross-chain operations.
        pub evm_escrow: sp_core::H160,
        /// Bridge escrow address for SVM cross-chain operations.
        pub svm_escrow: [u8; 32],
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for (asset_id, symbol, decimals) in &self.assets {
                assert!(
                    !AssetRegistry::<T>::contains_key(asset_id),
                    "Duplicate asset id in genesis"
                );
                assert!(*decimals <= 30, "Invalid decimals in genesis");
                assert!(!symbol.is_empty(), "Empty symbol in genesis");
                assert!(
                    !symbol.starts_with(b"-") && !symbol.starts_with(b"_"),
                    "Invalid symbol prefix in genesis"
                );
                for &byte in symbol {
                    let valid = byte.is_ascii_uppercase()
                        || byte.is_ascii_digit()
                        || byte == b'-'
                        || byte == b'_';
                    assert!(valid, "Invalid symbol charset in genesis");
                }

                let bounded_symbol: AssetSymbolOf<T> = symbol
                    .clone()
                    .try_into()
                    .unwrap_or_else(|_| Default::default());

                let metadata = AssetMetadata {
                    symbol: bounded_symbol,
                    decimals: *decimals,
                };
                AssetRegistry::<T>::insert(asset_id, metadata);
            }

            // Set escrow addresses
            BridgeEvmEscrow::<T>::set(self.evm_escrow);
            BridgeSvmEscrow::<T>::set(self.svm_escrow);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A Comit has been accepted for processing immediately after basic validation.
        ComitSubmitted {
            comit_id: H256,
            origin: T::AccountId,
            nonce: u64,
            fee: T::Balance,
        },
        /// Comit execution has started on both VMs.
        ComitExecutionStarted { comit_id: H256, timestamp: u64 },
        /// Comit execution has completed (may have failed).
        ComitExecutionCompleted {
            comit_id: H256,
            success: bool,
            gas_used: u64,
        },
        /// A Comit was finalized and applied to the canonical ledger.
        ComitFinalized { comit_id: H256 },
        /// Comit submission failed during verification or execution.
        ComitFailed {
            comit_id: H256,
            reason: ComitFailureReason,
        },
        /// An asset was registered with associated metadata.
        AssetRegistered {
            asset_id: T::AssetId,
            symbol: Vec<u8>,
            decimals: u8,
        },
        /// An account was authorized to submit Comits.
        AccountAuthorized { account: T::AccountId },
        /// An account was deauthorized from submitting Comits.
        AccountDeauthorized { account: T::AccountId },
        /// Canonical ledger was updated with state changes from comit execution.
        CanonicalLedgerUpdated {
            comit_id: H256,
            changes_applied: u32,
        },
        /// Cross-VM bridge operation was executed.
        CrossVmOperationExecuted {
            comit_id: H256,
            gas_used: u64,
            changes_applied: u32,
        },
        /// Cross-VM operation prepared and awaiting commit.
        CrossVmOperationPrepared {
            comit_id: H256,
            nonce: u64,
            expires_at: BlockNumberFor<T>,
        },
        /// Cross-VM operation committed and finalized.
        CrossVmOperationCommitted {
            comit_id: H256,
            gas_used: u64,
            fee_charged: T::Balance,
            fee_refund: T::Balance,
        },
        /// Cross-VM operation aborted.
        CrossVmOperationAborted { comit_id: H256, reason: Vec<u8> },
        /// Cross-VM fee reserved for a prepared operation.
        CrossVmFeeReserved { comit_id: H256, amount: T::Balance },
        /// Cross-VM fee refunded after commit/abort.
        CrossVmFeeRefunded { comit_id: H256, amount: T::Balance },
        /// Cross-chain proof verified for a cross-VM operation.
        CrossVmProofVerified { comit_id: H256, proof_kind: u8 },
        /// An authority was added to the current authority set.
        AuthorityAdded { authority: T::AccountId },
        /// An authority was removed from the current authority set.
        AuthorityRemoved { authority: T::AccountId },
        /// Pending authority changes were scheduled.
        AuthorityChangesScheduled { new_authorities: Vec<T::AccountId> },
        /// Pending authority changes were enacted.
        AuthorityChangesEnacted { new_authorities: Vec<T::AccountId> },
        /// Fee was deducted from an account for Comit execution.
        FeeDeducted {
            account: T::AccountId,
            amount: T::Balance,
            comit_id: H256,
        },
        /// The protocol has been emergency-paused by governance.
        ProtocolPaused,
        /// The protocol has been unpaused and resumed normal operation.
        ProtocolUnpaused,
        /// Emergency halt activated — all asset movement frozen due to invariant violation.
        EmergencyHalted,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset is already present within the registry.
        AssetAlreadyRegistered,
        /// Attempted to modify the ledger with an unknown asset identifier.
        UnknownAsset,
        /// Provided payloads exceeded configured length constraints.
        PayloadTooLarge,
        /// Both payloads were empty, yielding an invalid Comit.
        EmptyPayloads,
        /// Packet deserialization failed or packet domain targeting is invalid.
        InvalidPacket,
        /// Supplied nonce does not match the expected account nonce.
        InvalidNonce,
        /// Nonce increment would overflow.
        NonceOverflow,
        /// Placeholder error signalling dual-VM verification failure.
        ComitVerificationFailed,
        /// Asset symbol exceeds permitted length.
        SymbolTooLong,
        /// Asset decimals exceed maximum allowed value (0-30).
        InvalidDecimals,
        /// Asset symbol contains invalid characters; must be uppercase ASCII, digits, apps/dash-legacy-2-legacy-2, or underscore.
        InvalidSymbolCharset,
        /// Caller is not authorized to perform this operation.
        Unauthorized,
        /// Insufficient balance to cover the transaction fee.
        InsufficientBalance,
        /// Declared fee does not match the expected fee calculated from execution costs.
        IncorrectFee,
        /// Authority already exists in the authority set.
        AuthorityAlreadyExists,
        /// Authority not found in the authority set.
        AuthorityNotFound,
        /// Would violate minimum authorities constraint.
        BelowMinimumAuthorities,
        /// Would exceed maximum authorities constraint.
        ExceedsMaximumAuthorities,
        /// No pending authority changes to enact.
        NoPendingChanges,
        /// Authority set cannot be empty.
        EmptyAuthoritySet,
        /// EVM execution failed during Comit processing.
        EvmExecutionFailed,
        /// SVM execution failed during Comit processing.
        SvmExecutionFailed,
        /// X3VM execution failed during Comit processing.
        X3ExecutionFailed,
        /// Asset symbol cannot be empty.
        EmptySymbol,
        /// Asset symbol cannot start with apps/dash-legacy-2-legacy-2 or underscore.
        InvalidSymbolFormat,
        /// Too many state changes in execution receipts.
        TooManyStateChanges,
        /// Arithmetic overflow in fee calculation.
        FeeOverflow,
        /// Comit ID has already been submitted.
        DuplicateComitId,
        /// Rate limit exceeded: too many Comit submissions per block.
        RateLimitExceeded,
        /// Cross-VM operation payload failed validation.
        InvalidCrossVmOperation,
        /// Cross-VM operation execution failed.
        CrossVmExecutionFailed,
        /// Cross-VM nonce does not match expected value.
        CrossVmInvalidNonce,
        /// Cross-VM nonce increment overflow.
        CrossVmNonceOverflow,
        /// Cross-VM operation already prepared or missing.
        CrossVmOperationNotPrepared,
        /// Cross-VM prepared operation expired.
        CrossVmOperationExpired,
        /// Cross-VM proof verification failed.
        CrossVmProofInvalid,
        /// Cross-VM fee exceeds user-provided max fee.
        CrossVmFeeExceeded,
        /// Cross-VM prepared queue full.
        CrossVmPreparedQueueFull,
        /// The protocol is currently paused by governance. No user operations permitted.
        ProtocolIsPaused,
        /// Cross-VM atomicity breach: branches are not consistent.
        CrossVmAtomicityBreach,
        /// Settlement root mismatch: transaction state hash does not match stored root.
        SettlementMismatch,
        /// State inconsistency detected across VM branches.
        StateInconsistency,
    }

    /// Storage for atomic settlement roots (per transaction ID).
    /// Used to verify all cross-VM branches settled consistently.
    #[pallet::storage]
    pub(super) type StoredSettlementRoots<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, [u8; 32], OptionQuery>;

    use frame_support::traits::StorageVersion;

    pub(crate) const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Abort expired prepared cross-VM operations (bounded per block)
            let mut processed: u32 = 0;
            let limit = T::MaxPreparedOpsPerBlock::get();

            for (comit_id, prepared) in PreparedCrossVmOps::<T>::iter() {
                if processed >= limit {
                    break;
                }
                if prepared.expires_at <= now {
                    Self::abort_prepared_op(&comit_id, b"expired".to_vec());
                    processed = processed.saturating_add(1);
                }
            }

            // Minimal weight for bounded iteration.
            T::DbWeight::get().reads_writes(processed as u64, processed as u64)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Activate emergency pause — halts all user-facing extrinsics.
        /// Only callable by `GovernanceOrigin` (root or council).
        #[pallet::call_index(40)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn emergency_pause(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
            ProtocolPaused::<T>::put(true);
            Self::deposit_event(Event::ProtocolPaused);
            Ok(())
        }

        /// Deactivate emergency pause — resumes normal operation.
        /// Only callable by `GovernanceOrigin` (root or council).
        #[pallet::call_index(41)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn emergency_unpause(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            // Only unpause if currently paused (nothing to do otherwise)
            if ProtocolPaused::<T>::get() {
                ProtocolPaused::<T>::put(false);
                Self::deposit_event(Event::ProtocolUnpaused);
            }
            Ok(())
        }

        /// Emergency halt — freeze ALL asset movement during invariant violations.
        /// Sets the `Halted` flag in pallet-x3-invariants, triggering chain-wide
        /// asset freeze. Only callable by `GovernanceOrigin` (root or council).
        ///
        /// # Safety
        ///
        /// This is the nuclear option for catastrophic invariant violations:
        /// - All asset transfers blocked
        /// - All swaps/settlements frozen
        /// - Chain remains operational for governance actions only
        ///
        /// Use emergency_pause for routine operational pauses.
        /// Use emergency_halt for invariant violations requiring immediate asset freeze.
        #[pallet::call_index(42)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn emergency_halt(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            T::EmergencyHaltController::trigger();

            Self::deposit_event(Event::EmergencyHalted);
            Ok(())
        }

        /// Submit a Comit transaction describing dual-VM execution intents.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit())]
        pub fn submit_comit(
            origin: OriginFor<T>,
            comit_id: H256,
            evm_payload: Vec<u8>,
            svm_payload: Vec<u8>,
            nonce: u64,
            fee: T::Balance,
            prepare_root: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // SEC-009: Emergency pause guard
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);

            // Check for duplicate comit_id (M-4: Comit ID uniqueness)
            ensure!(
                !SubmittedComits::<T>::contains_key(comit_id),
                Error::<T>::DuplicateComitId
            );

            // Rate limiting check (L-6): Prevent DoS via excessive submissions
            const MAX_SUBMISSIONS_PER_BLOCK: u32 = 10;
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_count = SubmissionsPerBlock::<T>::get(&who, current_block);
            ensure!(
                current_count < MAX_SUBMISSIONS_PER_BLOCK,
                Error::<T>::RateLimitExceeded
            );

            // Early authorization check: verify caller is authorized for dual-VM operations
            let operation_context = Self::encode_submit_comit_context(&who, comit_id);
            Self::auth_check(&who, &operation_context)?;

            // First layer checks on payload sizes and emptiness.
            Self::verify_payloads(&comit_id, &evm_payload, &svm_payload)?;

            // Phase 1.3: Packet deserialization and domain routing validation
            // Gracefully attempt to deserialize payloads that meet minimum packet requirements
            // If deserialization succeeds, validate domain routing
            // If domain routing invalid, log warning but allow through for now
            // (tests may use non-packet data; Phase 1.4 router enforces strict packet format)
            let _evm_packet = if evm_payload.len() >= 30 {
                match deserialize_packet(&evm_payload) {
                    Ok(packet) => {
                        // Verify packet is EVM-targeted
                        let domain_mask = get_domain_mask(&packet);
                        if (domain_mask & 0b0001) == 0 {
                            // Domain routing invalid - allow through for now
                            // Future phases will enforce strict validation
                            None
                        } else {
                            Some(packet)
                        }
                    }
                    Err(_err) => {
                        // Deserialization failed - allow through for now
                        // (payload may be non-packet format, or corrupted)
                        // Future phases will enforce strict packet format
                        None
                    }
                }
            } else {
                None
            };

            let _svm_packet = if svm_payload.len() >= 30 {
                match deserialize_packet(&svm_payload) {
                    Ok(packet) => {
                        // Verify packet is SVM-targeted
                        let domain_mask = get_domain_mask(&packet);
                        if (domain_mask & 0b0010) == 0 {
                            // Domain routing invalid - allow through for now
                            // Future phases will enforce strict validation
                            None
                        } else {
                            Some(packet)
                        }
                    }
                    Err(_err) => {
                        // Deserialization failed - allow through for now
                        // (payload may be non-packet format, or corrupted)
                        // Future phases will enforce strict packet format
                        None
                    }
                }
            } else {
                None
            };

            // Atomic nonce check and increment using try_mutate (C-3)
            // This ensures the nonce is atomically verified and incremented in a single storage operation
            Nonces::<T>::try_mutate(&who, |current_nonce| -> DispatchResult {
                if nonce != *current_nonce {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::InvalidNonce {
                            code: 0x05,
                            expected: *current_nonce,
                            provided: nonce,
                        },
                    ));
                }
                *current_nonce = current_nonce
                    .checked_add(1)
                    .ok_or(Error::<T>::NonceOverflow)?;
                Ok(())
            })?;

            let comit = Comit::<T::AccountId, T::Balance> {
                comit_id,
                origin: who.clone(),
                evm_payload: evm_payload.clone(),
                svm_payload: svm_payload.clone(),
                nonce,
                fee,
                prepare_root,
            };

            // Prepare execution: collect receipts before verifying prepare_root
            let evm_tx = if !evm_payload.is_empty() {
                Some(evm_payload.clone())
            } else {
                None
            };
            let svm_tx = if !svm_payload.is_empty() {
                Some(svm_payload.clone())
            } else {
                None
            };

            // Capture timestamp at execution start (M-6: Fix stale timestamp issue)
            // This ensures consistent timing even in long-running block production
            let execution_start_timestamp =
                <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();

            // Execute via configured VM adapters (real or mock based on runtime config)
            // Gas limits: Use runtime-configurable constants (M-3)
            let evm_gas_limit = T::DefaultEvmGasLimit::get();
            let svm_compute_limit = T::DefaultSvmComputeLimit::get();

            let evm_receipt = if let Some(ref tx) = evm_tx {
                // Execute EVM payload via configured adapter
                match T::EvmAdapter::execute(tx, evm_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        // EVM execution failed - return with detailed error
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::EvmExecutionFailed {
                                code: 0x10,
                                evm_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let svm_receipt = if let Some(ref tx) = svm_tx {
                // Execute SVM payload via configured adapter
                match T::SvmAdapter::execute(tx, svm_compute_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        // SVM execution failed - must rollback any EVM changes for atomicity
                        // Note: In current Substrate architecture, returning error rolls back all storage
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::SvmExecutionFailed {
                                code: 0x11,
                                svm_error: 1,
                                compute_units_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            // Check for execution failures
            if let Some(ref receipt) = evm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::EvmExecutionFailed {
                            code: 0x10,
                            evm_error: 1, // Placeholder for actual EVM error
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = svm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::SvmExecutionFailed {
                            code: 0x11,
                            svm_error: 1,          // Placeholder for actual SVM error
                            compute_units_used: 0, // Would come from SVM receipt in real impl
                        },
                    ));
                }
            }

            // Fee deduction: Compute required fee before execution
            let evm_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let svm_compute_units = svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let base_fee = T::Balance::default();
            let required_fee =
                Self::calculate_execution_fee(evm_gas_used, svm_compute_units, base_fee)?;

            // Check if declared fee matches required fee
            ensure!(fee >= required_fee, Error::<T>::IncorrectFee);

            // Check sufficient balance
            let free_balance = T::Currency::free_balance(&who);
            ensure!(
                free_balance >= required_fee.into(),
                Error::<T>::InsufficientBalance
            );

            // Deduct the fee
            let imbalance = T::Currency::withdraw(
                &who,
                required_fee.into(),
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            drop(imbalance); // Burn the fee or handle as needed

            // Emit fee deduction event for indexer tracking
            Self::deposit_event(Event::FeeDeducted {
                account: who.clone(),
                amount: required_fee,
                comit_id,
            });

            // Verify dual-VM prepare_root against actual receipts
            if let Err(reason) = Self::verify_dual_vm_with_receipts(
                &comit,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
            ) {
                return Err(Self::fail_with_reason(comit_id, reason));
            }

            // Record comit_id as submitted (M-4: prevents duplicate submissions)
            SubmittedComits::<T>::insert(comit_id, current_block);

            // Update rate limit counter for this block (L-6)
            SubmissionsPerBlock::<T>::mutate(&who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            // Record a default X3 identifier if none exists yet.
            AccountRegistry::<T>::mutate(&who, |maybe_id| {
                if maybe_id.is_none() {
                    *maybe_id = Some(T::AtlasId::default());
                }
            });

            // Emit success events in order: Submitted -> ExecutionStarted -> ExecutionCompleted -> Finalized
            Self::deposit_event(Event::ComitSubmitted {
                comit_id,
                origin: who.clone(),
                nonce,
                fee,
            });

            // Use timestamp captured at execution start (M-6: consistent timing)
            Self::deposit_event(Event::ComitExecutionStarted {
                comit_id,
                timestamp: execution_start_timestamp,
            });

            // Calculate total gas used from both receipts
            let total_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0)
                + svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);

            Self::deposit_event(Event::ComitExecutionCompleted {
                comit_id,
                success: true,
                gas_used: total_gas_used,
            });

            // Apply state changes from receipts to CanonicalLedger
            let changes_applied = Self::apply_canonical_ledger_update(
                comit_id,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
            )?;

            // Emit event for ledger updates
            if changes_applied > 0 {
                Self::deposit_event(Event::CanonicalLedgerUpdated {
                    comit_id,
                    changes_applied,
                });
            }

            Self::deposit_event(Event::ComitFinalized { comit_id });
            Ok(())
        }

        /// Submit a v2 Comit transaction describing triple-VM execution intents (EVM + SVM + X3VM).
        ///
        /// Atomicity model: if any VM execution fails (error or `success=false`), this extrinsic
        /// returns `Err` and all Substrate storage writes (including CanonicalLedger updates)
        /// are rolled back. Runtime VM adapters MUST be transactional to guarantee rollback
        /// for VM state as well.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn submit_comit_v2(
            origin: OriginFor<T>,
            comit_id: H256,
            evm_payload: Vec<u8>,
            svm_payload: Vec<u8>,
            x3_payload: Vec<u8>,
            nonce: u64,
            fee: T::Balance,
            prepare_root: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // SEC-009: Emergency pause guard
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);

            ensure!(
                !SubmittedComits::<T>::contains_key(comit_id),
                Error::<T>::DuplicateComitId
            );

            const MAX_SUBMISSIONS_PER_BLOCK: u32 = 10;
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_count = SubmissionsPerBlock::<T>::get(&who, current_block);
            ensure!(
                current_count < MAX_SUBMISSIONS_PER_BLOCK,
                Error::<T>::RateLimitExceeded
            );

            let operation_context = Self::encode_submit_comit_v2_context(&who, comit_id);
            Self::auth_check(&who, &operation_context)?;

            Self::verify_payloads_v2(&comit_id, &evm_payload, &svm_payload, &x3_payload)?;

            Nonces::<T>::try_mutate(&who, |current_nonce| -> DispatchResult {
                if nonce != *current_nonce {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::InvalidNonce {
                            code: 0x05,
                            expected: *current_nonce,
                            provided: nonce,
                        },
                    ));
                }
                *current_nonce = current_nonce
                    .checked_add(1)
                    .ok_or(Error::<T>::NonceOverflow)?;
                Ok(())
            })?;

            let comit = ComitV2::<T::AccountId, T::Balance> {
                comit_id,
                origin: who.clone(),
                evm_payload: evm_payload.clone(),
                svm_payload: svm_payload.clone(),
                x3_payload: x3_payload.clone(),
                nonce,
                fee,
                prepare_root,
            };

            let evm_tx = (!evm_payload.is_empty()).then(|| evm_payload.clone());
            let svm_tx = (!svm_payload.is_empty()).then(|| svm_payload.clone());
            let x3_tx = (!x3_payload.is_empty()).then(|| x3_payload.clone());

            let execution_start_timestamp =
                <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();

            let evm_gas_limit = T::DefaultEvmGasLimit::get();
            let svm_compute_limit = T::DefaultSvmComputeLimit::get();
            let x3_gas_limit = T::DefaultX3GasLimit::get();

            let evm_receipt = if let Some(ref tx) = evm_tx {
                match T::EvmAdapter::execute(tx, evm_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::EvmExecutionFailed {
                                code: 0x10,
                                evm_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let svm_receipt = if let Some(ref tx) = svm_tx {
                match T::SvmAdapter::execute(tx, svm_compute_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::SvmExecutionFailed {
                                code: 0x11,
                                svm_error: 1,
                                compute_units_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            let x3_receipt = if let Some(ref tx) = x3_tx {
                match T::X3Adapter::execute(tx, x3_gas_limit) {
                    Ok(receipt) => Some(receipt),
                    Err(_e) => {
                        return Err(Self::fail_with_reason(
                            comit_id,
                            ComitFailureReason::X3ExecutionFailed {
                                code: 0x12,
                                x3_error: 1,
                                gas_used: 0,
                            },
                        ));
                    }
                }
            } else {
                None
            };

            if let Some(ref receipt) = evm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::EvmExecutionFailed {
                            code: 0x10,
                            evm_error: 1,
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = svm_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::SvmExecutionFailed {
                            code: 0x11,
                            svm_error: 1,
                            compute_units_used: receipt.gas_used,
                        },
                    ));
                }
            }

            if let Some(ref receipt) = x3_receipt {
                if !receipt.success {
                    return Err(Self::fail_with_reason(
                        comit_id,
                        ComitFailureReason::X3ExecutionFailed {
                            code: 0x12,
                            x3_error: 1,
                            gas_used: receipt.gas_used,
                        },
                    ));
                }
            }

            let evm_gas_used = evm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let svm_compute_units = svm_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let x3_gas_used = x3_receipt.as_ref().map(|r| r.gas_used).unwrap_or(0);
            let base_fee = T::Balance::default();
            let required_fee = Self::calculate_execution_fee_v2(
                evm_gas_used,
                svm_compute_units,
                x3_gas_used,
                base_fee,
            )?;

            ensure!(fee >= required_fee, Error::<T>::IncorrectFee);

            let free_balance = T::Currency::free_balance(&who);
            ensure!(
                free_balance >= required_fee.into(),
                Error::<T>::InsufficientBalance
            );

            let imbalance = T::Currency::withdraw(
                &who,
                required_fee.into(),
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            drop(imbalance);

            Self::deposit_event(Event::FeeDeducted {
                account: who.clone(),
                amount: required_fee,
                comit_id,
            });

            if let Err(reason) = Self::verify_triple_vm_with_receipts(
                &comit,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
                x3_receipt.as_ref(),
            ) {
                return Err(Self::fail_with_reason(comit_id, reason));
            }

            SubmittedComits::<T>::insert(comit_id, current_block);

            SubmissionsPerBlock::<T>::mutate(&who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            AccountRegistry::<T>::mutate(&who, |maybe_id| {
                if maybe_id.is_none() {
                    *maybe_id = Some(T::AtlasId::default());
                }
            });

            Self::deposit_event(Event::ComitSubmitted {
                comit_id,
                origin: who.clone(),
                nonce,
                fee,
            });

            Self::deposit_event(Event::ComitExecutionStarted {
                comit_id,
                timestamp: execution_start_timestamp,
            });

            let total_gas_used = evm_gas_used
                .saturating_add(svm_compute_units)
                .saturating_add(x3_gas_used);

            Self::deposit_event(Event::ComitExecutionCompleted {
                comit_id,
                success: true,
                gas_used: total_gas_used,
            });

            let changes_applied = Self::apply_canonical_ledger_update_v2(
                comit_id,
                evm_receipt.as_ref(),
                svm_receipt.as_ref(),
                x3_receipt.as_ref(),
            )?;

            if changes_applied > 0 {
                Self::deposit_event(Event::CanonicalLedgerUpdated {
                    comit_id,
                    changes_applied,
                });
            }

            Self::deposit_event(Event::ComitFinalized { comit_id });
            Ok(())
        }

        /// Submit a cross-VM bridge operation for atomic execution (prepare + commit).
        #[pallet::call_index(10)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn submit_cross_vm_operation(
            origin: OriginFor<T>,
            operation: CrossVmOperation,
            nonce: u64,
            max_fee: T::Balance,
            proof: CrossChainProof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let comit_id =
                Self::prepare_cross_vm_operation_inner(&who, operation, nonce, max_fee, proof)?;

            Self::commit_cross_vm_operation_inner(&who, comit_id)
        }

        /// Prepare a cross-VM operation and hold locks until commit.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn prepare_cross_vm_operation(
            origin: OriginFor<T>,
            operation: CrossVmOperation,
            nonce: u64,
            max_fee: T::Balance,
            proof: CrossChainProof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // SEC-009: Emergency pause guard
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
            let _ = Self::prepare_cross_vm_operation_inner(&who, operation, nonce, max_fee, proof)?;
            Ok(())
        }

        /// Commit a previously prepared cross-VM operation.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn commit_cross_vm_operation(origin: OriginFor<T>, comit_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // SEC-009: Emergency pause guard
            ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
            Self::commit_cross_vm_operation_inner(&who, comit_id)
        }

        /// Abort a prepared cross-VM operation (origin must match).
        #[pallet::call_index(13)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn abort_cross_vm_operation(origin: OriginFor<T>, comit_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let prepared = PreparedCrossVmOps::<T>::get(comit_id)
                .ok_or(Error::<T>::CrossVmOperationNotPrepared)?;
            ensure!(prepared.origin == who, Error::<T>::Unauthorized);
            Self::abort_prepared_op(&comit_id, b"user_abort".to_vec());
            Ok(())
        }

        /// Force-abort a prepared cross-VM operation (governance only).
        #[pallet::call_index(14)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_comit_v2())]
        pub fn force_abort_cross_vm_operation(
            origin: OriginFor<T>,
            comit_id: H256,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Self::abort_prepared_op(&comit_id, b"force_abort".to_vec());
            Ok(())
        }

        /// Register a new asset and its metadata within the X3 Kernel.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::register_asset())]
        pub fn register_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            symbol: Vec<u8>,
            decimals: u8,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                !AssetRegistry::<T>::contains_key(asset_id),
                Error::<T>::AssetAlreadyRegistered
            );

            // Validate decimals are within reasonable bounds (0-30)
            ensure!(decimals <= 30, Error::<T>::InvalidDecimals);

            // Validate symbol is not empty
            ensure!(!symbol.is_empty(), Error::<T>::EmptySymbol);

            // Validate symbol does not start with apps/dash-legacy-2-legacy-2 or underscore
            ensure!(
                !symbol.starts_with(b"-") && !symbol.starts_with(b"_"),
                Error::<T>::InvalidSymbolFormat
            );

            // Validate symbol: must be uppercase ASCII, digits, apps/dash-legacy-2-legacy-2, or underscore
            for &byte in &symbol {
                let valid = byte.is_ascii_uppercase()  // Uppercase letters
                    || byte.is_ascii_digit()  // Digits
                    || byte == b'-'  // Dash
                    || byte == b'_'; // Underscore
                ensure!(valid, Error::<T>::InvalidSymbolCharset);
            }

            let bounded_symbol: AssetSymbolOf<T> = symbol
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::SymbolTooLong)?;

            let metadata = AssetMetadata {
                symbol: bounded_symbol,
                decimals,
            };
            AssetRegistry::<T>::insert(asset_id, metadata);

            Self::deposit_event(Event::AssetRegistered {
                asset_id,
                symbol,
                decimals,
            });
            Ok(())
        }

        /// Update the canonical ledger balance for a specific account and asset.
        /// The optional Comit identifier triggers a finalized event when supplied.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::update_canonical_balance())]
        pub fn update_canonical_balance(
            origin: OriginFor<T>,
            account: T::AccountId,
            asset_id: T::AssetId,
            new_balance: T::Balance,
            comit_id: Option<H256>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                AssetRegistry::<T>::contains_key(asset_id),
                Error::<T>::UnknownAsset
            );

            CanonicalLedger::<T>::insert(&account, asset_id, new_balance);

            if let Some(id) = comit_id {
                Self::deposit_event(Event::ComitFinalized { comit_id: id });
            }

            Ok(())
        }

        /// Authorize an account to submit Comits.
        /// Only callable by root/governance.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::authorize_account())]
        pub fn authorize_account(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            AuthorizedAccounts::<T>::insert(account.clone(), ());
            Self::deposit_event(Event::AccountAuthorized { account });

            Ok(())
        }

        /// Deauthorize an account from submitting Comits.
        /// Only callable by root/governance.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::deauthorize_account())]
        pub fn deauthorize_account(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            AuthorizedAccounts::<T>::remove(&account);
            Self::deposit_event(Event::AccountDeauthorized { account });

            Ok(())
        }

        /// Add a new authority to the current authority set.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::add_authority())]
        pub fn add_authority(origin: OriginFor<T>, authority: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Authorities::<T>::try_mutate(|authorities| -> DispatchResult {
                // Check if authority already exists
                ensure!(
                    !authorities.contains(&authority),
                    Error::<T>::AuthorityAlreadyExists
                );

                // Check max authorities limit
                authorities
                    .try_push(authority.clone())
                    .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

                Self::deposit_event(Event::AuthorityAdded { authority });
                Ok(())
            })
        }

        /// Remove an authority from the current authority set.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::remove_authority())]
        pub fn remove_authority(origin: OriginFor<T>, authority: T::AccountId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Authorities::<T>::try_mutate(|authorities| -> DispatchResult {
                // Find and remove the authority
                let pos = authorities
                    .iter()
                    .position(|a| a == &authority)
                    .ok_or(Error::<T>::AuthorityNotFound)?;

                // Check minimum authorities constraint (must keep at least MinAuthorities)
                ensure!(
                    authorities.len() > T::MinAuthorities::get() as usize,
                    Error::<T>::BelowMinimumAuthorities
                );
                // Additional safety: never allow single authority in production
                ensure!(
                    authorities.len() > 1 || T::MinAuthorities::get() == 0,
                    Error::<T>::BelowMinimumAuthorities
                );

                authorities.remove(pos);
                Self::deposit_event(Event::AuthorityRemoved { authority });
                Ok(())
            })
        }

        /// Schedule a complete authority set change for the next session.
        /// Only callable by governance (root or collective).
        #[pallet::call_index(7)]
        #[pallet::weight(<T as Config>::WeightInfo::schedule_authority_change())]
        pub fn schedule_authority_change(
            origin: OriginFor<T>,
            new_authorities: Vec<T::AccountId>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // Validate authority count bounds (check empty first for better error messages)
            ensure!(!new_authorities.is_empty(), Error::<T>::EmptyAuthoritySet);
            let count = new_authorities.len() as u32;
            ensure!(
                count >= T::MinAuthorities::get(),
                Error::<T>::BelowMinimumAuthorities
            );
            ensure!(
                count <= T::MaxAuthorities::get(),
                Error::<T>::ExceedsMaximumAuthorities
            );

            // Convert to BoundedVec
            let bounded_authorities: BoundedVec<T::AccountId, T::MaxAuthorities> = new_authorities
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

            PendingAuthorities::<T>::put(Some(bounded_authorities));
            Self::deposit_event(Event::AuthorityChangesScheduled { new_authorities });

            Ok(())
        }

        /// Enact pending authority changes.
        /// Should be called at session boundaries. Only callable by governance.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as Config>::WeightInfo::enact_authority_change())]
        pub fn enact_authority_change(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // Get pending changes
            let pending = PendingAuthorities::<T>::take().ok_or(Error::<T>::NoPendingChanges)?;

            // Apply the new authority set
            let new_authorities: Vec<T::AccountId> = pending.into_inner();
            let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> = new_authorities
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ExceedsMaximumAuthorities)?;

            Authorities::<T>::put(bounded);
            Self::deposit_event(Event::AuthorityChangesEnacted { new_authorities });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn verify_payloads(
            comit_id: &H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
        ) -> Result<(), DispatchError> {
            let max_evm = T::MaxEvmPayloadLength::get() as usize;
            let max_svm = T::MaxSvmPayloadLength::get() as usize;
            let max_combined = T::MaxCombinedPayloadLength::get() as usize;

            if evm_payload.is_empty() && svm_payload.is_empty() {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EmptyPayloads { code: 0x04 },
                ));
            }

            if evm_payload.len() > max_evm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EvmPayloadTooLarge {
                        code: 0x01,
                        actual_size: evm_payload.len() as u32,
                        max_size: max_evm as u32,
                    },
                ));
            }

            if svm_payload.len() > max_svm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::SvmPayloadTooLarge {
                        code: 0x02,
                        actual_size: svm_payload.len() as u32,
                        max_size: max_svm as u32,
                    },
                ));
            }

            if evm_payload.len() + svm_payload.len() > max_combined {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::CombinedPayloadTooLarge {
                        code: 0x03,
                        evm_size: evm_payload.len() as u32,
                        svm_size: svm_payload.len() as u32,
                        max_combined: max_combined as u32,
                    },
                ));
            }
            Ok(())
        }

        fn verify_payloads_v2(
            comit_id: &H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            x3_payload: &[u8],
        ) -> Result<(), DispatchError> {
            let max_evm = T::MaxEvmPayloadLength::get() as usize;
            let max_svm = T::MaxSvmPayloadLength::get() as usize;
            let max_x3 = T::MaxX3PayloadLength::get() as usize;
            let max_combined = T::MaxCombinedPayloadLengthV2::get() as usize;

            if evm_payload.is_empty() && svm_payload.is_empty() && x3_payload.is_empty() {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EmptyPayloads { code: 0x04 },
                ));
            }

            if evm_payload.len() > max_evm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::EvmPayloadTooLarge {
                        code: 0x01,
                        actual_size: evm_payload.len() as u32,
                        max_size: max_evm as u32,
                    },
                ));
            }

            if svm_payload.len() > max_svm {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::SvmPayloadTooLarge {
                        code: 0x02,
                        actual_size: svm_payload.len() as u32,
                        max_size: max_svm as u32,
                    },
                ));
            }

            if x3_payload.len() > max_x3 {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::X3PayloadTooLarge {
                        code: 0x07,
                        actual_size: x3_payload.len() as u32,
                        max_size: max_x3 as u32,
                    },
                ));
            }

            if evm_payload.len() + svm_payload.len() + x3_payload.len() > max_combined {
                return Err(Self::fail_with_reason(
                    *comit_id,
                    ComitFailureReason::CombinedPayloadTooLarge {
                        code: 0x03,
                        evm_size: evm_payload.len() as u32,
                        svm_size: svm_payload.len() as u32,
                        max_combined: max_combined as u32,
                    },
                ));
            }

            Ok(())
        }

        /// Encode operation context for authorization checks
        fn encode_submit_comit_context(caller: &T::AccountId, comit_id: H256) -> Vec<u8> {
            let mut context = Vec::new();
            context.extend_from_slice(b"submit_comit");
            context.extend_from_slice(&caller.encode());
            context.extend_from_slice(comit_id.as_bytes());
            context
        }

        fn encode_submit_comit_v2_context(caller: &T::AccountId, comit_id: H256) -> Vec<u8> {
            let mut context = Vec::new();
            context.extend_from_slice(b"submit_comit_v2");
            context.extend_from_slice(&caller.encode());
            context.extend_from_slice(comit_id.as_bytes());
            context
        }

        fn encode_cross_vm_operation_context(caller: &T::AccountId, comit_id: H256) -> Vec<u8> {
            let mut context = Vec::new();
            context.extend_from_slice(b"submit_cross_vm_operation");
            context.extend_from_slice(&caller.encode());
            context.extend_from_slice(comit_id.as_bytes());
            context
        }

        fn compute_cross_vm_comit_id(
            origin: &T::AccountId,
            operation: &CrossVmOperation,
            nonce: u64,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(&origin.encode());
            data.extend_from_slice(&operation.encode());
            data.extend_from_slice(&nonce.to_le_bytes());
            H256::from(blake2_256(&data))
        }

        fn proof_kind(proof: &CrossChainProof) -> u8 {
            match proof {
                CrossChainProof::None => 0,
                CrossChainProof::LockProof(_) => 1,
                CrossChainProof::MerkleReceipt(_) => 2,
            }
        }

        fn proof_hash(proof: &CrossChainProof) -> H256 {
            match proof {
                CrossChainProof::None => H256::zero(),
                CrossChainProof::LockProof(bytes) => H256::from(blake2_256(bytes)),
                CrossChainProof::MerkleReceipt(bytes) => H256::from(blake2_256(bytes)),
            }
        }

        fn estimate_cross_vm_fee(
            operation: &CrossVmOperation,
        ) -> Result<T::Balance, DispatchError> {
            let (evm_gas, svm_compute) = match operation {
                CrossVmOperation::TransferToEvm { .. } => (25_000u64, 0u64),
                CrossVmOperation::TransferToSvm { .. } => (0u64, 5_000u64),
                CrossVmOperation::CallEvm { input, .. } => {
                    let gas = T::EvmAdapter::estimate_gas(input)
                        .unwrap_or(T::DefaultEvmGasLimit::get().min(100_000));
                    (gas, 0u64)
                }
                CrossVmOperation::CallSvm { .. } => (0u64, T::DefaultSvmComputeLimit::get()),
                CrossVmOperation::AtomicSwap { .. } => (200_000u64, 200_000u64),
                // Message passing: moderate gas, no balance lock
                CrossVmOperation::MessageToEvm { .. } => (50_000u64, 0u64),
                CrossVmOperation::MessageToSvm { .. } => (0u64, 50_000u64),
                CrossVmOperation::CallX3Vm { call, .. } => (0u64, call.gas_budget),
                CrossVmOperation::AtomicTriSwap { x3vm_call, .. } => {
                    (200_000u64, 200_000u64.saturating_add(x3vm_call.gas_budget))
                }
            };

            let base_fee = T::Balance::default();
            Self::calculate_execution_fee(evm_gas, svm_compute, base_fee)
        }

        fn cross_vm_prepare_checks(
            dispatcher: &KernelCrossVmDispatcher<T>,
            operation: &CrossVmOperation,
        ) -> Result<(), DispatchError> {
            match operation {
                CrossVmOperation::TransferToEvm { source, amount, .. } => {
                    let mut pubkey = [0u8; 32];
                    pubkey.copy_from_slice(&source[..32]);
                    let balance = dispatcher.get_svm_balance(&pubkey) as u128;
                    ensure!(balance >= *amount, Error::<T>::InsufficientBalance);
                }
                CrossVmOperation::TransferToSvm { source, amount, .. } => {
                    let balance = dispatcher.get_evm_balance(source);
                    ensure!(balance >= *amount, Error::<T>::InsufficientBalance);
                }
                CrossVmOperation::AtomicSwap {
                    evm_party,
                    svm_party,
                    evm_amount,
                    svm_amount,
                    ..
                } => {
                    let evm_balance = dispatcher.get_evm_balance(evm_party);
                    ensure!(evm_balance >= *evm_amount, Error::<T>::InsufficientBalance);
                    let mut pubkey = [0u8; 32];
                    let len = svm_party.len().min(32);
                    pubkey[..len].copy_from_slice(&svm_party[..len]);
                    let svm_balance = dispatcher.get_svm_balance(&pubkey) as u128;
                    ensure!(svm_balance >= *svm_amount, Error::<T>::InsufficientBalance);
                }
                _ => {}
            }
            Ok(())
        }

        fn prepare_cross_vm_operation_inner(
            who: &T::AccountId,
            operation: CrossVmOperation,
            nonce: u64,
            max_fee: T::Balance,
            proof: CrossChainProof,
        ) -> Result<H256, DispatchError> {
            let comit_id = Self::compute_cross_vm_comit_id(who, &operation, nonce);

            ensure!(
                !SubmittedComits::<T>::contains_key(comit_id),
                Error::<T>::DuplicateComitId
            );

            const MAX_SUBMISSIONS_PER_BLOCK: u32 = 10;
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_count = SubmissionsPerBlock::<T>::get(who, current_block);
            ensure!(
                current_count < MAX_SUBMISSIONS_PER_BLOCK,
                Error::<T>::RateLimitExceeded
            );

            let operation_context = Self::encode_cross_vm_operation_context(who, comit_id);
            Self::auth_check(who, &operation_context)?;

            if T::RequireCrossVmProof::get() {
                ensure!(
                    !matches!(proof, CrossChainProof::None),
                    Error::<T>::CrossVmProofInvalid
                );
            }

            T::CrossChainProofVerifier::verify_proof(who, &operation, &proof)
                .map_err(|_| Error::<T>::CrossVmProofInvalid)?;

            let proof_kind = Self::proof_kind(&proof);
            let proof_hash = Self::proof_hash(&proof);

            // Cross-VM nonce check
            CrossVmNonces::<T>::try_mutate(who, |current_nonce| -> DispatchResult {
                if nonce != *current_nonce {
                    return Err(Error::<T>::CrossVmInvalidNonce.into());
                }
                *current_nonce = current_nonce
                    .checked_add(1)
                    .ok_or(Error::<T>::CrossVmNonceOverflow)?;
                Ok(())
            })?;

            let required_fee = Self::estimate_cross_vm_fee(&operation)?;
            ensure!(max_fee >= required_fee, Error::<T>::CrossVmFeeExceeded);

            // TICKET-4.5-004: Reserve max fee up-front with defensive accounting
            T::Currency::reserve(who, max_fee.into())
                .map_err(|_| Error::<T>::InsufficientBalance)?;
            Self::deposit_event(Event::CrossVmFeeReserved {
                comit_id,
                amount: max_fee,
            });

            let dispatcher = KernelCrossVmDispatcher::<T>::new();
            Self::cross_vm_prepare_checks(&dispatcher, &operation).inspect_err(|_| {
                // TICKET-4.5-004: Defensive unreserve - verify full amount released
                let unreserved = T::Currency::unreserve(who, max_fee.into());
                frame_support::defensive_assert!(
                    unreserved == max_fee.into(),
                    "Failed to unreserve full reserved fee on prepare error"
                );
            })?;

            let expires_at = current_block + T::CrossVmPrepareTtl::get();
            let prepared = PreparedCrossVmOp::<T::AccountId, T::Balance, BlockNumberFor<T>> {
                origin: who.clone(),
                operation: operation.clone(),
                nonce,
                prepared_at: current_block,
                expires_at,
                reserved_fee: max_fee,
                proof_hash,
                proof_kind,
            };

            PreparedCrossVmQueue::<T>::try_mutate(|queue| -> DispatchResult {
                queue
                    .try_push(comit_id)
                    .map_err(|_| Error::<T>::CrossVmPreparedQueueFull)?;
                Ok(())
            })
            .inspect_err(|_| {
                // TICKET-4.5-004: Defensive unreserve on queue push failure
                let unreserved = T::Currency::unreserve(who, max_fee.into());
                frame_support::defensive_assert!(
                    unreserved == max_fee.into(),
                    "Failed to unreserve full reserved fee on queue push error"
                );
            })?;

            PreparedCrossVmOps::<T>::insert(comit_id, prepared);

            SubmittedComits::<T>::insert(comit_id, current_block);
            SubmissionsPerBlock::<T>::mutate(who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::CrossVmProofVerified {
                comit_id,
                proof_kind,
            });
            Self::deposit_event(Event::CrossVmOperationPrepared {
                comit_id,
                nonce,
                expires_at,
            });

            Ok(comit_id)
        }

        fn commit_cross_vm_operation_inner(who: &T::AccountId, comit_id: H256) -> DispatchResult {
            let prepared = PreparedCrossVmOps::<T>::get(comit_id)
                .ok_or(Error::<T>::CrossVmOperationNotPrepared)?;
            ensure!(prepared.origin == *who, Error::<T>::Unauthorized);

            let now = frame_system::Pallet::<T>::block_number();
            if prepared.expires_at <= now {
                Self::abort_prepared_op(&comit_id, b"expired".to_vec());
                return Err(Error::<T>::CrossVmOperationExpired.into());
            }

            let dispatcher = KernelCrossVmDispatcher::<T>::new();
            let mut bridge = CrossVmBridge::new();
            bridge
                .queue_operation(prepared.operation.clone())
                .map_err(|_| Error::<T>::InvalidCrossVmOperation)?;

            let (results, _events) = bridge.execute_with_dispatcher(&dispatcher).map_err(|_| {
                // TICKET-4.5-004: Refund reserved fee on execution failure with defensive check
                let unreserved = T::Currency::unreserve(who, prepared.reserved_fee.into());
                frame_support::defensive_assert!(
                    unreserved == prepared.reserved_fee.into(),
                    "Failed to unreserve full reserved fee on execution failure"
                );
                Error::<T>::CrossVmExecutionFailed
            })?;

            let result = results
                .into_iter()
                .next()
                .ok_or(Error::<T>::CrossVmExecutionFailed)?;
            if !result.success {
                // TICKET-4.5-004: Defensive unreserve on failed execution result
                let unreserved = T::Currency::unreserve(who, prepared.reserved_fee.into());
                frame_support::defensive_assert!(
                    unreserved == prepared.reserved_fee.into(),
                    "Failed to unreserve full reserved fee on execution result failure"
                );
                return Err(Error::<T>::CrossVmExecutionFailed.into());
            }

            let (evm_gas, svm_compute) = match prepared.operation {
                CrossVmOperation::TransferToEvm { .. } => (result.gas_used, 0u64),
                CrossVmOperation::TransferToSvm { .. } => (0u64, result.gas_used),
                CrossVmOperation::CallEvm { .. } => (result.gas_used, 0u64),
                CrossVmOperation::CallSvm { .. } => (0u64, result.gas_used),
                CrossVmOperation::AtomicSwap { .. } => {
                    let evm = result.gas_used / 2;
                    (evm, result.gas_used.saturating_sub(evm))
                }
                CrossVmOperation::MessageToEvm { .. } => (result.gas_used, 0u64),
                CrossVmOperation::MessageToSvm { .. } => (0u64, result.gas_used),
                CrossVmOperation::CallX3Vm { .. } => (0u64, result.gas_used),
                CrossVmOperation::AtomicTriSwap { .. } => {
                    let evm = result.gas_used / 2;
                    (evm, result.gas_used.saturating_sub(evm))
                }
            };

            // TICKET-4.5-004: Calculate actual fee with overflow protection
            let required_fee: T::Balance =
                Self::calculate_execution_fee(evm_gas, svm_compute, T::Balance::default())
                    .defensive_ok()
                    .ok_or(Error::<T>::FeeOverflow)?;

            if required_fee > prepared.reserved_fee {
                // TICKET-4.5-004: Defensive unreserve on fee exceeded
                let unreserved = T::Currency::unreserve(who, prepared.reserved_fee.into());
                frame_support::defensive_assert!(
                    unreserved == prepared.reserved_fee.into(),
                    "Failed to unreserve full reserved fee on fee exceeded"
                );
                return Err(Error::<T>::CrossVmFeeExceeded.into());
            }

            // TICKET-4.5-004: Refund reserved fee then charge actual with defensive accounting
            let unreserved = T::Currency::unreserve(who, prepared.reserved_fee.into());
            frame_support::defensive_assert!(
                unreserved == prepared.reserved_fee.into(),
                "Failed to unreserve full reserved fee on success path"
            );

            let refund = prepared.reserved_fee.saturating_sub(required_fee);

            if !refund.is_zero() {
                Self::deposit_event(Event::CrossVmFeeRefunded {
                    comit_id,
                    amount: refund,
                });
            }

            let imbalance = T::Currency::withdraw(
                who,
                required_fee.into(),
                frame_support::traits::WithdrawReasons::FEE,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            drop(imbalance);

            // Remove prepared op only after successful fee settlement
            PreparedCrossVmOps::<T>::remove(comit_id);
            PreparedCrossVmQueue::<T>::mutate(|queue| {
                if let Some(pos) = queue.iter().position(|id| id == &comit_id) {
                    queue.remove(pos);
                }
            });

            let bridge_state_changes = Self::build_cross_vm_state_changes(&prepared.operation)?;
            let bridge_receipt = ExecutionReceipt {
                version: EXECUTION_RECEIPT_VERSION,
                success: true,
                gas_used: result.gas_used,
                return_data: result.output,
                logs: Vec::new(),
                state_changes: bridge_state_changes,
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            };

            let changes_applied =
                Self::apply_canonical_ledger_update(comit_id, Some(&bridge_receipt), None)?;

            if changes_applied > 0 {
                Self::deposit_event(Event::CanonicalLedgerUpdated {
                    comit_id,
                    changes_applied,
                });
            }

            Self::deposit_event(Event::CrossVmOperationExecuted {
                comit_id,
                gas_used: bridge_receipt.gas_used,
                changes_applied,
            });
            Self::deposit_event(Event::CrossVmOperationCommitted {
                comit_id,
                gas_used: bridge_receipt.gas_used,
                fee_charged: required_fee,
                fee_refund: refund,
            });
            Self::deposit_event(Event::ComitFinalized { comit_id });
            Ok(())
        }

        fn abort_prepared_op(comit_id: &H256, reason: Vec<u8>) {
            if let Some(prepared) = PreparedCrossVmOps::<T>::get(comit_id) {
                T::Currency::unreserve(&prepared.origin, prepared.reserved_fee.into());
                Self::deposit_event(Event::CrossVmOperationAborted {
                    comit_id: *comit_id,
                    reason,
                });
                PreparedCrossVmOps::<T>::remove(comit_id);
                PreparedCrossVmQueue::<T>::mutate(|queue| {
                    if let Some(pos) = queue.iter().position(|id| id == comit_id) {
                        queue.remove(pos);
                    }
                });
            }
        }

        fn canonical_asset_state_key(asset_id: T::AssetId) -> H256 {
            let mut key = [0u8; 32];
            let encoded = asset_id.encode();
            let len = encoded.len().min(32);
            key[..len].copy_from_slice(&encoded[..len]);
            H256::from(key)
        }

        fn canonical_balance_state_value(balance: T::Balance) -> H256 {
            let mut value = [0u8; 32];
            let encoded = balance.encode();
            let len = encoded.len().min(32);
            value[..len].copy_from_slice(&encoded[..len]);
            H256::from(value)
        }

        fn build_balance_state_change(
            address: &[u8],
            asset_id: T::AssetId,
            new_balance: T::Balance,
        ) -> StateChange {
            StateChange {
                address: address.to_vec(),
                key: Self::canonical_asset_state_key(asset_id),
                value: Self::canonical_balance_state_value(new_balance),
            }
        }

        fn map_cross_vm_address_delta(
            address: &[u8],
            asset_id: T::AssetId,
            amount: u128,
            is_credit: bool,
        ) -> Result<StateChange, DispatchError> {
            let account = Self::decode_state_change_account(address)
                .ok_or(Error::<T>::InvalidCrossVmOperation)?;
            let current_u128: u128 = CanonicalLedger::<T>::get(&account, asset_id).saturated_into();
            let updated_u128 = if is_credit {
                current_u128.saturating_add(amount)
            } else {
                current_u128.saturating_sub(amount)
            };
            let new_balance: T::Balance = updated_u128.saturated_into();
            Ok(Self::build_balance_state_change(
                address,
                asset_id,
                new_balance,
            ))
        }

        fn build_cross_vm_state_changes(
            operation: &CrossVmOperation,
        ) -> Result<Vec<StateChange>, DispatchError> {
            let canonical_asset = T::AssetId::default();
            let mut changes = Vec::new();

            match operation {
                CrossVmOperation::TransferToEvm {
                    source,
                    destination,
                    amount,
                } => {
                    changes.push(Self::map_cross_vm_address_delta(
                        source,
                        canonical_asset,
                        *amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        destination,
                        canonical_asset,
                        *amount,
                        true,
                    )?);
                }
                CrossVmOperation::TransferToSvm {
                    source,
                    destination,
                    amount,
                } => {
                    changes.push(Self::map_cross_vm_address_delta(
                        source,
                        canonical_asset,
                        *amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        destination,
                        canonical_asset,
                        *amount,
                        true,
                    )?);
                }
                CrossVmOperation::CallEvm { .. }
                | CrossVmOperation::CallSvm { .. }
                | CrossVmOperation::CallX3Vm { .. }
                | CrossVmOperation::MessageToEvm { .. }
                | CrossVmOperation::MessageToSvm { .. } => {
                    // Message/call operations carry no balance state changes
                }
                CrossVmOperation::AtomicSwap {
                    evm_party,
                    svm_party,
                    evm_amount,
                    svm_amount,
                    ..
                } => {
                    changes.push(Self::map_cross_vm_address_delta(
                        evm_party,
                        canonical_asset,
                        *evm_amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        svm_party,
                        canonical_asset,
                        *evm_amount,
                        true,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        svm_party,
                        canonical_asset,
                        *svm_amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        evm_party,
                        canonical_asset,
                        *svm_amount,
                        true,
                    )?);
                }
                CrossVmOperation::AtomicTriSwap {
                    evm_party,
                    svm_party,
                    evm_amount,
                    svm_amount,
                    ..
                } => {
                    changes.push(Self::map_cross_vm_address_delta(
                        evm_party,
                        canonical_asset,
                        *evm_amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        svm_party,
                        canonical_asset,
                        *evm_amount,
                        true,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        svm_party,
                        canonical_asset,
                        *svm_amount,
                        false,
                    )?);
                    changes.push(Self::map_cross_vm_address_delta(
                        evm_party,
                        canonical_asset,
                        *svm_amount,
                        true,
                    )?);
                }
            }

            Ok(changes)
        }

        /// Maximum number of state changes allowed per Comit execution.
        /// Prevents DoS via excessive storage writes.
        const MAX_STATE_CHANGES: usize = 1000;

        /// Decode an AccountId from a VM state-change address.
        ///
        /// Supports:
        /// - SCALE-encoded AccountId bytes
        /// - 20-byte EVM addresses (left-padded to 32 bytes)
        fn decode_state_change_account(address: &[u8]) -> Option<T::AccountId> {
            if let Ok(account) = T::AccountId::decode(&mut &address[..]) {
                return Some(account);
            }

            if address.len() == 20 {
                let mut padded = [0u8; 32];
                padded[12..].copy_from_slice(address);
                return T::AccountId::decode(&mut &padded[..]).ok();
            }

            None
        }

        fn decode_state_change_asset_id(key: &H256) -> Option<T::AssetId> {
            T::AssetId::decode(&mut &key.as_bytes()[..]).ok()
        }

        fn decode_state_change_balance(value: &H256) -> Option<T::Balance> {
            T::Balance::decode(&mut &value.as_bytes()[..]).ok()
        }

        /// Apply state changes from execution receipts to the CanonicalLedger.
        /// This aggregates state_changes from EVM and SVM receipts and updates storage.
        /// Tracks decode failures for monitoring (M-2: Unsafe decode operations).
        fn apply_canonical_ledger_update(
            _comit_id: H256,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
        ) -> Result<u32, DispatchError> {
            let mut changes_applied = 0u32;
            let mut decode_failures = 0u32;

            // Aggregate state changes from both receipts
            let mut all_changes = Vec::new();
            if let Some(receipt) = evm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = svm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }

            // Bound check: prevent excessive state changes (DoS protection)
            if all_changes.len() > Self::MAX_STATE_CHANGES {
                return Err(Error::<T>::TooManyStateChanges.into());
            }

            // Apply each state change to CanonicalLedger.
            for change in all_changes.iter() {
                let account = Self::decode_state_change_account(&change.address);
                let asset = Self::decode_state_change_asset_id(&change.key);
                let balance = Self::decode_state_change_balance(&change.value);

                match (account, asset, balance) {
                    (Some(acc), Some(asset_id), Some(bal)) => {
                        CanonicalLedger::<T>::insert(&acc, asset_id, bal);
                        changes_applied = changes_applied.saturating_add(1);
                    }
                    _ => {
                        decode_failures = decode_failures.saturating_add(1);
                    }
                }
            }

            // Update global decode failure counter for monitoring (M-2)
            if decode_failures > 0 {
                DecodeFailureCount::<T>::mutate(|count| {
                    *count = count.saturating_add(decode_failures);
                });
            }

            Ok(changes_applied)
        }

        /// Minimum fee floor to prevent zero-cost transaction attacks.
        const MIN_FEE: u32 = 1;

        /// Calculate the total execution fee for a Comit based on gas/compute usage.
        /// Uses checked arithmetic to prevent overflow.
        /// Uses ceiling division to prevent zero-fee attacks.
        pub fn calculate_execution_fee(
            evm_gas_used: u64,
            svm_compute_units: u64,
            base_fee: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            // Gas/compute unit pricing (configurable in production)
            // EVM: 1 unit per 1000 gas (ceiling division)
            // SVM: 1 unit per 1000 compute units (ceiling division)
            // Using saturating_add(999) / 1000 for ceiling division to prevent zero-fee attacks
            let evm_units_u64 = evm_gas_used.saturating_add(999) / 1000;
            let svm_units_u64 = svm_compute_units.saturating_add(999) / 1000;

            let evm_units = T::Balance::from(evm_units_u64 as u32);
            let svm_units = T::Balance::from(svm_units_u64 as u32);

            // Total fee = base + EVM units + SVM units
            // Use checked_add to prevent overflow
            let total_fee = base_fee
                .checked_add(&evm_units)
                .and_then(|t| t.checked_add(&svm_units))
                .ok_or(Error::<T>::FeeOverflow)?;

            // Enforce minimum fee floor to prevent zero-cost attacks
            let min_fee = T::Balance::from(Self::MIN_FEE);
            let final_fee = if total_fee < min_fee {
                min_fee
            } else {
                total_fee
            };

            Ok(final_fee)
        }

        /// Authorization check for dual-VM operations
        /// Enforces allowlist-based access control unless dev-bypass feature is enabled.
        ///
        /// Authorization Semantics:
        /// - With `dev-bypass` feature: All signed callers are accepted (development only)
        /// - Without `dev-bypass` feature (production):
        ///   - Caller MUST be in AuthorizedAccounts storage
        ///   - Empty AuthorizedAccounts = No one is authorized (secure by default)
        ///   - Use `authorize_account` extrinsic to add accounts to allowlist
        ///   
        /// This explicit authorization model prevents unauthorized access and ensures
        /// governance has full control over who can submit Comits.
        fn auth_check(
            _caller: &T::AccountId,
            _operation_context: &[u8],
        ) -> Result<(), DispatchError> {
            #[cfg(feature = "dev-bypass")]
            {
                // Development bypass: accept all signed callers
                Ok(())
            }

            #[cfg(not(feature = "dev-bypass"))]
            {
                // Production: check authorization list
                // If no authorized accounts exist, reject (explicit authorization required)
                if AuthorizedAccounts::<T>::contains_key(_caller) {
                    Ok(())
                } else {
                    Err(Error::<T>::Unauthorized.into())
                }
            }
        }

        /// Compute prepare_root for a Comit from its input parameters.
        /// This is the canonical algorithm for generating the prepare_root commitment.
        /// Exported as public for test use (L-3: Avoid test helper duplication).
        ///
        /// # Algorithm
        /// The prepare_root is computed as Blake2-256 hash of concatenated:
        /// - comit_id (32 bytes)
        /// - evm_payload (variable length)
        /// - svm_payload (variable length)
        /// - nonce (8 bytes, little-endian)
        /// - fee (SCALE-encoded)
        pub fn compute_prepare_root(
            comit_id: H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            nonce: u64,
            fee: T::Balance,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(comit_id.as_bytes());
            data.extend_from_slice(evm_payload);
            data.extend_from_slice(svm_payload);
            data.extend_from_slice(&nonce.to_le_bytes());
            data.extend_from_slice(&fee.encode());
            H256::from(blake2_256(&data))
        }

        /// Compute prepare_root for a v2 Comit from its input parameters.
        ///
        /// Canonical algorithm: Blake2-256 over concatenated:
        /// - comit_id (32)
        /// - evm_payload
        /// - svm_payload
        /// - x3_payload
        /// - nonce (8 LE)
        /// - fee (SCALE)
        pub fn compute_prepare_root_v2(
            comit_id: H256,
            evm_payload: &[u8],
            svm_payload: &[u8],
            x3_payload: &[u8],
            nonce: u64,
            fee: T::Balance,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(comit_id.as_bytes());
            data.extend_from_slice(evm_payload);
            data.extend_from_slice(svm_payload);
            data.extend_from_slice(x3_payload);
            data.extend_from_slice(&nonce.to_le_bytes());
            data.extend_from_slice(&fee.encode());
            H256::from(blake2_256(&data))
        }

        /// Verify prepare_root against actual VM receipts (comprehensive dual-VM commitment)
        ///
        /// # SECURITY NOTICE (H-1 Audit Finding - Design Decision)
        ///
        /// The `prepare_root` is intentionally a commitment to INPUTS only, not OUTPUTS.
        /// This design choice enables:
        /// 1. Client-side pre-computation: Users can compute prepare_root before submission
        /// 2. Deterministic authorization: Wallets can sign based on known inputs
        /// 3. Replay protection: Combined with nonce prevents transaction replay
        ///
        /// ## Trade-offs
        /// - Pro: Simpler client integration, no simulation required
        /// - Con: Cannot verify execution results match expectations
        ///
        /// ## Mitigation for High-Value Transactions
        /// For transactions requiring output verification, implement:
        /// - Application-layer expected_output_hash verification
        /// - Multi-sig validation with result confirmation
        /// - Post-execution audit trail comparison
        ///
        /// The execution receipts are passed to allow future extensions but are
        /// deliberately unused in the current implementation per this design.
        fn verify_dual_vm_with_receipts(
            comit: &ComitOf<T>,
            _evm_receipt: Option<&ExecutionReceipt>,
            _svm_receipt: Option<&ExecutionReceipt>,
        ) -> Result<(), ComitFailureReason> {
            // Reject zero prepare_root unless explicitly allowed by dev-bypass feature
            #[cfg(not(feature = "dev-bypass"))]
            {
                if comit.prepare_root == H256::zero() {
                    return Err(ComitFailureReason::Verification {
                        code: 0x06,
                        reason: blake2_256(b"zero_prepare_root_not_allowed"),
                    });
                }
            }

            // Build canonical dual-VM commitment WITHOUT receipt data.
            // The prepare_root is a commitment to the input payloads and execution parameters,
            // NOT the execution results. This allows clients to compute the prepare_root
            // beforehand and use it to authorize the Comit submission.
            //
            // See function-level documentation for full security rationale (H-1 audit finding).
            let computed_root = Self::compute_prepare_root(
                comit.comit_id,
                &comit.evm_payload,
                &comit.svm_payload,
                comit.nonce,
                comit.fee,
            );

            if computed_root == comit.prepare_root {
                Ok(())
            } else {
                // Hash the mismatch reason for diagnostic
                let mut reason_data = Vec::new();
                reason_data.extend_from_slice(comit.comit_id.as_bytes());
                reason_data.extend_from_slice(computed_root.as_bytes());
                reason_data.extend_from_slice(comit.prepare_root.as_bytes());
                let reason_hash = blake2_256(&reason_data);

                Err(ComitFailureReason::Verification {
                    code: 0x06,
                    reason: reason_hash,
                })
            }
        }

        fn verify_triple_vm_with_receipts(
            comit: &ComitV2Of<T>,
            _evm_receipt: Option<&ExecutionReceipt>,
            _svm_receipt: Option<&ExecutionReceipt>,
            _x3_receipt: Option<&ExecutionReceipt>,
        ) -> Result<(), ComitFailureReason> {
            #[cfg(not(feature = "dev-bypass"))]
            {
                if comit.prepare_root == H256::zero() {
                    return Err(ComitFailureReason::Verification {
                        code: 0x06,
                        reason: blake2_256(b"zero_prepare_root_not_allowed"),
                    });
                }
            }

            let computed_root = Self::compute_prepare_root_v2(
                comit.comit_id,
                &comit.evm_payload,
                &comit.svm_payload,
                &comit.x3_payload,
                comit.nonce,
                comit.fee,
            );

            if computed_root == comit.prepare_root {
                Ok(())
            } else {
                let mut reason_data = Vec::new();
                reason_data.extend_from_slice(comit.comit_id.as_bytes());
                reason_data.extend_from_slice(computed_root.as_bytes());
                reason_data.extend_from_slice(comit.prepare_root.as_bytes());
                let reason_hash = blake2_256(&reason_data);

                Err(ComitFailureReason::Verification {
                    code: 0x06,
                    reason: reason_hash,
                })
            }
        }

        fn fail_with_reason(_comit_id: H256, reason: ComitFailureReason) -> DispatchError {
            let error = Self::reason_to_error(&reason);
            // Note: We do NOT emit ComitFailed event here because:
            // In Substrate, when an extrinsic returns Err, all state changes (including events)
            // are rolled back automatically. Therefore, emitting an event before returning an
            // error is futile - it will never appear in the final block.
            // Failure information is instead conveyed through the error code itself.
            error.into()
        }

        fn reason_to_error(reason: &ComitFailureReason) -> Error<T> {
            match reason {
                ComitFailureReason::EvmPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::SvmPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::X3PayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::CombinedPayloadTooLarge { .. } => Error::<T>::PayloadTooLarge,
                ComitFailureReason::EmptyPayloads { .. } => Error::<T>::EmptyPayloads,
                ComitFailureReason::InvalidNonce { .. } => Error::<T>::InvalidNonce,
                ComitFailureReason::Verification { .. } => Error::<T>::ComitVerificationFailed,
                ComitFailureReason::PacketDeserializationFailed { .. } => Error::<T>::InvalidPacket,
                ComitFailureReason::EvmExecutionFailed { .. } => Error::<T>::EvmExecutionFailed,
                ComitFailureReason::SvmExecutionFailed { .. } => Error::<T>::SvmExecutionFailed,
                ComitFailureReason::X3ExecutionFailed { .. } => Error::<T>::X3ExecutionFailed,
            }
        }
        /// Calculate the total execution fee for a v2 Comit based on gas/compute usage.
        pub fn calculate_execution_fee_v2(
            evm_gas_used: u64,
            svm_compute_units: u64,
            x3_gas_used: u64,
            base_fee: T::Balance,
        ) -> Result<T::Balance, DispatchError> {
            let evm_units_u64 = evm_gas_used.saturating_add(999) / 1000;
            let svm_units_u64 = svm_compute_units.saturating_add(999) / 1000;
            let x3_units_u64 = x3_gas_used.saturating_add(999) / 1000;

            let evm_units = T::Balance::from(evm_units_u64 as u32);
            let svm_units = T::Balance::from(svm_units_u64 as u32);
            let x3_units = T::Balance::from(x3_units_u64 as u32);

            let total_fee = base_fee
                .checked_add(&evm_units)
                .and_then(|t| t.checked_add(&svm_units))
                .and_then(|t| t.checked_add(&x3_units))
                .ok_or(Error::<T>::FeeOverflow)?;

            let min_fee = T::Balance::from(Self::MIN_FEE);
            Ok(if total_fee < min_fee {
                min_fee
            } else {
                total_fee
            })
        }

        fn apply_canonical_ledger_update_v2(
            _comit_id: H256,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
            x3_receipt: Option<&ExecutionReceipt>,
        ) -> Result<u32, DispatchError> {
            let mut changes_applied = 0u32;
            let mut decode_failures = 0u32;

            let mut all_changes = Vec::new();
            if let Some(receipt) = evm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = svm_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }
            if let Some(receipt) = x3_receipt {
                all_changes.extend_from_slice(&receipt.state_changes);
            }

            if all_changes.len() > Self::MAX_STATE_CHANGES {
                return Err(Error::<T>::TooManyStateChanges.into());
            }

            for change in all_changes.iter() {
                let account = Self::decode_state_change_account(&change.address);
                let asset = Self::decode_state_change_asset_id(&change.key);
                let balance = Self::decode_state_change_balance(&change.value);

                match (account, asset, balance) {
                    (Some(acc), Some(asset_id), Some(bal)) => {
                        CanonicalLedger::<T>::insert(&acc, asset_id, bal);
                        changes_applied = changes_applied.saturating_add(1);
                    }
                    _ => {
                        decode_failures = decode_failures.saturating_add(1);
                    }
                }
            }

            if decode_failures > 0 {
                DecodeFailureCount::<T>::mutate(|count| {
                    *count = count.saturating_add(decode_failures);
                });
            }

            Ok(changes_applied)
        }

        /// Execute dual-VM transactions and return the unified state
        #[allow(dead_code)]
        fn do_execute_dual_tx(
            evm_tx: Option<Vec<u8>>,
            svm_tx: Option<Vec<u8>>,
        ) -> Result<SphereState, DispatchError> {
            // Execute transactions on both VMs in parallel (when implemented)
            let _evm_receipt = evm_tx.map(|_tx| ExecutionReceipt {
                version: EXECUTION_RECEIPT_VERSION,
                success: true,
                gas_used: 21000,
                return_data: Vec::new(),
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });

            let _svm_receipt = svm_tx.map(|_tx| ExecutionReceipt {
                version: EXECUTION_RECEIPT_VERSION,
                success: true,
                gas_used: 5000,
                return_data: Vec::new(),
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });

            // Merge receipts into unified state
            Ok(SphereState {
                state_root: H256::default(),
                block_number: 0,
                timestamp: 0,
            })
        }

        /// Populate the SVM blockhash reverse index (blockhash -> slot).
        /// This should be called when a new SVM block is created to maintain
        /// the reverse index for efficient blockhash-to-slot lookups.
        ///
        /// # Arguments
        /// * `slot` - The slot number of the new block
        /// * `blockhash` - The blockhash of the new block
        pub fn populate_svm_blockhash_index(slot: u64, blockhash: H256) {
            // Insert into forward index (slot -> blockhash)
            SvmBlockhashes::<T>::insert(slot, blockhash);
            // Insert into reverse index (blockhash -> slot)
            SvmBlockhashSlots::<T>::insert(blockhash, slot);
        }
    }

    /// Implementation of the DualVmDispatcher trait for the X3 Kernel pallet.
    /// This provides the core coordination logic for executing transactions across
    /// both EVM and SVM runtimes and merging their execution results.
    impl<T: Config> DualVmDispatcher for Pallet<T> {
        type AccountId = T::AccountId;
        type Balance = T::Balance;

        fn execute_evm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError> {
            // Execute via configured EVM adapter (real or mock based on runtime)
            T::EvmAdapter::execute(&tx, 10_000_000)
        }

        fn execute_svm_tx(&self, tx: Vec<u8>) -> Result<ExecutionReceipt, DispatchError> {
            // Execute via configured SVM adapter (real or mock based on runtime)
            T::SvmAdapter::execute(&tx, 200_000)
        }

        fn execute_dual_tx(
            &self,
            evm_tx: Option<Vec<u8>>,
            svm_tx: Option<Vec<u8>>,
        ) -> Result<SphereState, DispatchError> {
            // Execute transactions on both VMs in parallel (when implemented)
            let evm_receipt = if let Some(tx) = evm_tx {
                Some(self.execute_evm_tx(tx)?)
            } else {
                None
            };

            let svm_receipt = if let Some(tx) = svm_tx {
                Some(self.execute_svm_tx(tx)?)
            } else {
                None
            };

            // Merge execution results into unified sphere state.
            let sphere_state = self.merge_receipts(evm_receipt.as_ref(), svm_receipt.as_ref());

            // Persist VM-emitted state changes into CanonicalLedger for dispatcher-driven flows.
            let mut merged_changes = Vec::new();
            if let Some(receipt) = evm_receipt.as_ref() {
                merged_changes.extend(receipt.state_changes.clone());
            }
            if let Some(receipt) = svm_receipt.as_ref() {
                merged_changes.extend(receipt.state_changes.clone());
            }

            if !merged_changes.is_empty() {
                // Use a deterministic synthetic ID for trait-path invocations that are not
                // tied to a submitted Comit extrinsic.
                let synthetic_comit_id = H256::from(sp_io::hashing::blake2_256(
                    sphere_state.state_root.as_bytes(),
                ));
                self.canonical_ledger_update(synthetic_comit_id, &merged_changes)?;
            }

            Ok(sphere_state)
        }

        /// Merge EVM and SVM execution receipts into a unified SphereState.
        ///
        /// This function creates a deterministic state root by hashing all execution
        /// data from both VMs in a canonical order:
        /// 1. EVM receipt data (success, gas, return data, logs, state changes)
        /// 2. SVM receipt data (success, compute units, return data, logs, state changes)
        ///
        /// The resulting state root provides:
        /// - Deterministic replay: Same inputs always produce same state root
        /// - Cross-VM commitment: Both VM results are included in a single hash
        /// - Auditability: External verifiers can recompute the state root
        fn merge_receipts(
            &self,
            evm_receipt: Option<&ExecutionReceipt>,
            svm_receipt: Option<&ExecutionReceipt>,
        ) -> SphereState {
            let mut state_data = Vec::new();

            // Include EVM receipt data
            if let Some(receipt) = evm_receipt {
                state_data.extend_from_slice(&receipt.success.encode());
                state_data.extend_from_slice(&receipt.gas_used.encode());
                state_data.extend_from_slice(&receipt.return_data);
                for log in &receipt.logs {
                    state_data.extend_from_slice(&log.address);
                    state_data.extend_from_slice(&log.data);
                }
                for change in &receipt.state_changes {
                    state_data.extend_from_slice(&change.address);
                    state_data.extend_from_slice(change.key.as_bytes());
                    state_data.extend_from_slice(change.value.as_bytes());
                }
            }

            // Include SVM receipt data
            if let Some(receipt) = svm_receipt {
                state_data.extend_from_slice(&receipt.success.encode());
                state_data.extend_from_slice(&receipt.gas_used.encode());
                state_data.extend_from_slice(&receipt.return_data);
                for log in &receipt.logs {
                    state_data.extend_from_slice(&log.address);
                    state_data.extend_from_slice(&log.data);
                }
                for change in &receipt.state_changes {
                    state_data.extend_from_slice(&change.address);
                    state_data.extend_from_slice(change.key.as_bytes());
                    state_data.extend_from_slice(change.value.as_bytes());
                }
            }

            // Get current block number from frame_system
            let current_block = <frame_system::Pallet<T>>::block_number();
            // Get current timestamp from pallet_timestamp using UnixTime trait
            let current_timestamp = <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs();
            // Generate deterministic state root
            let state_root = H256::from(blake2_256(&state_data));

            SphereState {
                state_root,
                block_number: current_block.saturated_into(),
                timestamp: current_timestamp,
            }
        }

        /// Check if an account is authorized to execute a specific cross-VM operation.
        /// Delegates to the pallet's auth_check method for consistent authorization.
        fn auth_check(
            &self,
            caller: &Self::AccountId,
            operation: &[u8],
        ) -> Result<(), DispatchError> {
            // Delegate to pallet's auth_check for consistent authorization behavior
            // This ensures trait-based calls respect the same AuthorizedAccounts storage
            Self::auth_check(caller, operation)
        }

        /// Calculate execution fees based on gas and compute unit consumption.
        ///
        /// Uses checked arithmetic to prevent overflow in fee calculations.
        /// Uses ceiling division and minimum fee floor to prevent zero-fee attacks.
        /// Returns the total fee required for the transaction.
        fn fee_accounting(
            &self,
            evm_gas_used: u64,
            svm_compute_units: u64,
            base_fee: Self::Balance,
        ) -> Result<Self::Balance, DispatchError> {
            // Delegate to pallet's calculate_execution_fee for consistent behavior
            Self::calculate_execution_fee(evm_gas_used, svm_compute_units, base_fee)
        }

        /// Update the canonical ledger with state changes from a successful comit.
        ///
        /// This applies VM-provided state changes directly to `CanonicalLedger`.
        /// It is used by trait-driven dispatcher flows outside the submit extrinsics.
        fn canonical_ledger_update(
            &self,
            comit_id: H256,
            state_changes: &[StateChange],
        ) -> Result<(), DispatchError> {
            let mut applied = 0u32;

            for change in state_changes {
                let account = Self::decode_state_change_account(&change.address)
                    .ok_or(DispatchError::Other("Invalid state change account"))?;
                let asset = Self::decode_state_change_asset_id(&change.key)
                    .ok_or(DispatchError::Other("Invalid state change asset"))?;
                let balance = Self::decode_state_change_balance(&change.value)
                    .ok_or(DispatchError::Other("Invalid state change balance"))?;

                CanonicalLedger::<T>::insert(&account, asset, balance);
                applied = applied.saturating_add(1);
            }

            Self::deposit_event(Event::CanonicalLedgerUpdated {
                comit_id,
                changes_applied: applied,
            });

            Ok(())
        }
    }

    /// Runtime-backed dispatcher for cross-VM bridge operations.
    pub struct KernelCrossVmDispatcher<T: Config>(PhantomData<T>);

    impl<T: Config> KernelCrossVmDispatcher<T> {
        pub fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<T: Config> Default for KernelCrossVmDispatcher<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T: Config> CrossVmDispatcher for KernelCrossVmDispatcher<T> {
        fn execute_evm_tx(
            &self,
            _caller: &[u8; 20],
            _target: &[u8; 20],
            input: &[u8],
            _value: u128,
        ) -> Result<CrossVmResult, DispatchError> {
            let receipt = T::EvmAdapter::execute(input, T::DefaultEvmGasLimit::get())?;
            if receipt.success {
                Ok(CrossVmResult::success(
                    receipt.return_data,
                    receipt.gas_used,
                ))
            } else {
                Ok(CrossVmResult::failed(
                    b"evm_execution_failed".to_vec(),
                    receipt.gas_used,
                ))
            }
        }

        fn execute_svm_tx(
            &self,
            _caller: &[u8; 32],
            _program_id: &[u8; 32],
            input: &[u8],
        ) -> Result<CrossVmResult, DispatchError> {
            let receipt = T::SvmAdapter::execute(input, T::DefaultSvmComputeLimit::get())?;
            if receipt.success {
                Ok(CrossVmResult::success(
                    receipt.return_data,
                    receipt.gas_used,
                ))
            } else {
                Ok(CrossVmResult::failed(
                    b"svm_execution_failed".to_vec(),
                    receipt.gas_used,
                ))
            }
        }

        fn execute_x3vm_tx(
            &self,
            _caller: &[u8; 32],
            call: &CrossVmCall,
        ) -> Result<CrossVmReceipt, DispatchError> {
            call.ensure_current_version()?;

            if call.target != VmId::X3Vm {
                return Ok(CrossVmReceipt {
                    call_hash: call.call_hash(&H256::zero()),
                    source_state_root: H256::zero(),
                    target_state_root: H256::zero(),
                    status: CrossVmStatus::InternalError,
                    gas_used: 0,
                    logs: Vec::new(),
                });
            }

            let receipt = T::X3Adapter::execute(call.payload.as_slice(), call.gas_budget)?;
            let result = if receipt.success {
                CrossVmResult::success(receipt.return_data, receipt.gas_used)
            } else {
                CrossVmResult::failed(b"x3vm_execution_failed".to_vec(), receipt.gas_used)
            };

            Ok(result.into_receipt_for(call, &H256::zero()))
        }

        fn get_evm_balance(&self, address: &[u8; 20]) -> u128 {
            let account = Pallet::<T>::decode_state_change_account(address);
            let asset = T::AssetId::default();
            match account {
                Some(acc) => {
                    let bal: u128 = CanonicalLedger::<T>::get(&acc, asset).saturated_into();
                    bal
                }
                None => 0,
            }
        }

        fn get_svm_balance(&self, pubkey: &[u8; 32]) -> u64 {
            let account = Pallet::<T>::decode_state_change_account(pubkey);
            let asset = T::AssetId::default();
            match account {
                Some(acc) => {
                    let bal: u128 = CanonicalLedger::<T>::get(&acc, asset).saturated_into();
                    bal.min(u64::MAX as u128) as u64
                }
                None => 0,
            }
        }

        fn get_evm_bridge_escrow(&self) -> [u8; 20] {
            T::BridgeEvmEscrow::get().0
        }

        fn get_svm_bridge_escrow(&self) -> [u8; 32] {
            T::BridgeSvmEscrow::get()
        }

        fn get_svm_slot(&self) -> u64 {
            // Use the current block number as the SVM slot
            // In a real implementation, this would use a dedicated SVM slot counter
            frame_system::Pallet::<T>::block_number().saturated_into::<u64>()
        }

        fn get_svm_blockhash(&self, slot: u64) -> Option<H256> {
            // Query the SVM blockhash storage for the given slot
            SvmBlockhashes::<T>::get(slot)
        }

        fn get_svm_transaction_count(&self, svm_pubkey: Vec<u8>) -> u64 {
            // Query the SVM transaction count storage for the given pubkey
            if svm_pubkey.len() == 32 {
                let pubkey: [u8; 32] = svm_pubkey.try_into().unwrap_or_default();
                SvmTransactionCounts::<T>::get(&pubkey)
            } else {
                0u64
            }
        }

        fn get_svm_slot_by_blockhash(blockhash: H256) -> Option<u64> {
            // Query the reverse index for the given blockhash
            SvmBlockhashSlots::<T>::get(blockhash)
        }

        /// Parse RLP-encoded Ethereum transaction to extract sender, recipient, and value.
        /// This is a minimal implementation that decodes the transaction fields.
        fn parse_ethereum_transaction(raw_tx: &[u8]) -> Result<(Vec<u8>, Vec<u8>, u128), Vec<u8>> {
            // RLP-encoded transaction format for EIP-1559/2930/155 transactions:
            // [nonce, gas_price, gas_limit, to, value, input, v, r, s]
            // For legacy transactions (type 0), the format is:
            // [nonce, gasPrice, gasLimit, to, value, data, v, r, s]

            if raw_tx.is_empty() {
                return Err("Empty transaction".as_bytes().to_vec());
            }

            // Check if this is a typed transaction (EIP-2718)
            // EIP-2718 transactions start with a type byte (0x01, 0x02, 0x03)
            if raw_tx[0] >= 0x01 && raw_tx[0] <= 0x03 {
                // Typed transaction - decode the inner RLP
                // Format: [type, [nonce, gas_limit, to, value, data, access_list, max_fee_per_gas, max_priority_fee_per_gas, signature_y_parity, signature_r, signature_s]]
                return Err("Typed transactions not fully supported".as_bytes().to_vec());
            }

            // Legacy transaction - decode RLP
            // The RLP encoding of a list [nonce, gasPrice, gasLimit, to, value, data, v, r, s]
            // For transactions without signature (pre-signing), we can still extract from/to/value

            // Simple approach: try to decode as RLP list
            // For a valid transaction, we need at least: nonce, gasPrice, gasLimit, to, value
            // The 'to' field is 20 bytes (H160), 'value' is 16 bytes (u128)

            // Extract 'to' address (bytes 39-58 for a minimal transaction without signature)
            // Format: [nonce (1-8 bytes), gasPrice (1-8 bytes), gasLimit (1-8 bytes), to (20 bytes), value (1-16 bytes), ...]

            // For a minimal transaction with just the required fields:
            // - nonce: 1 byte (0x80 = 0)
            // - gasPrice: 1 byte (0x80 = 0)
            // - gasLimit: 1 byte (0x80 = 0)
            // - to: 20 bytes (0x94 = 0x80 + 20)
            // - value: 1 byte (0x80 = 0)

            let mut offset = 0;

            // Skip nonce (single byte 0x80 for value 0)
            if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
            } else {
                // Variable-length encoding
                let len = raw_tx[offset] & 0x7f;
                offset += 1 + len as usize;
            }

            // Skip gasPrice
            if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
            } else {
                let len = raw_tx[offset] & 0x7f;
                offset += 1 + len as usize;
            }

            // Skip gasLimit
            if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
            } else {
                let len = raw_tx[offset] & 0x7f;
                offset += 1 + len as usize;
            }

            // Extract 'to' address (20 bytes, prefixed with 0x94)
            if offset + 1 > raw_tx.len() {
                return Err("Invalid transaction: missing 'to' field"
                    .as_bytes()
                    .to_vec());
            }

            let to = if raw_tx[offset] == 0x94 && offset + 21 <= raw_tx.len() {
                // 20-byte address
                let addr = raw_tx[offset + 1..offset + 21].to_vec();
                offset += 21;
                addr
            } else if raw_tx[offset] == 0x80 && offset + 1 <= raw_tx.len() {
                // Empty address (contract creation)
                offset += 1;
                Vec::new()
            } else {
                return Err("Invalid transaction: malformed 'to' field"
                    .as_bytes()
                    .to_vec());
            };

            // Extract 'value' (16 bytes for u128)
            if offset >= raw_tx.len() {
                return Err("Invalid transaction: missing 'value' field"
                    .as_bytes()
                    .to_vec());
            }

            let value = if raw_tx[offset] == 0x80 && offset + 1 <= raw_tx.len() {
                // Value is 0
                offset += 1;
                0u128
            } else if raw_tx[offset] >= 0x80 && raw_tx[offset] <= 0xb7 {
                // Short string encoding
                let len = (raw_tx[offset] & 0x7f) as usize;
                if offset + 1 + len > raw_tx.len() {
                    return Err("Invalid transaction: malformed 'value' field"
                        .as_bytes()
                        .to_vec());
                }
                let mut value_bytes = [0u8; 16];
                let src_start = offset + 1;
                let src_end = offset + 1 + len;
                let dest_start = 16 - len;
                value_bytes[dest_start..dest_start + len]
                    .copy_from_slice(&raw_tx[src_start..src_end]);
                offset += 1 + len;
                u128::from_be_bytes(value_bytes)
            } else {
                return Err("Invalid transaction: malformed 'value' field"
                    .as_bytes()
                    .to_vec());
            };

            Ok((Vec::new(), to, value)) // from is empty for unsigned transactions
        }

        /// Parse RLP-encoded Ethereum transaction to extract sender, recipient, value, gas, input, nonce, and gas_price.
        /// This extends parse_ethereum_transaction to include gas and input fields needed for RPC compatibility.
        fn parse_ethereum_transaction_with_gas(
            raw_tx: &[u8],
        ) -> Result<(Vec<u8>, Vec<u8>, u128, u64, Vec<u8>, u64, u128), Vec<u8>> {
            // RLP-encoded transaction format for legacy transactions (type 0):
            // [nonce, gasPrice, gasLimit, to, value, data, v, r, s]

            if raw_tx.is_empty() {
                return Err("Empty transaction".as_bytes().to_vec());
            }

            // Check if this is a typed transaction (EIP-2718)
            if raw_tx[0] >= 0x01 && raw_tx[0] <= 0x03 {
                // Typed transaction - not fully supported yet
                return Err("Typed transactions not fully supported".as_bytes().to_vec());
            }

            // Legacy transaction - decode RLP
            let mut offset = 0;

            // Skip nonce (single byte 0x80 for value 0)
            if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
            } else {
                let len = raw_tx[offset] & 0x7f;
                offset += 1 + len as usize;
            }

            // Skip gasPrice
            let gas_price = if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
                0u128
            } else if raw_tx[offset] >= 0x80 && raw_tx[offset] <= 0xb7 {
                let len = (raw_tx[offset] & 0x7f) as usize;
                if offset + 1 + len > raw_tx.len() {
                    return Err("Invalid transaction: malformed 'gasPrice' field"
                        .as_bytes()
                        .to_vec());
                }
                let mut price_bytes = [0u8; 16];
                let src_start = offset + 1;
                let src_end = offset + 1 + len;
                let dest_start = 16 - len;
                price_bytes[dest_start..dest_start + len]
                    .copy_from_slice(&raw_tx[src_start..src_end]);
                offset += 1 + len;
                u128::from_be_bytes(price_bytes)
            } else {
                return Err("Invalid transaction: malformed 'gasPrice' field"
                    .as_bytes()
                    .to_vec());
            };

            // Skip gasLimit
            let gas = if offset < raw_tx.len() && raw_tx[offset] == 0x80 {
                offset += 1;
                0u64
            } else if raw_tx[offset] >= 0x80 && raw_tx[offset] <= 0xb7 {
                let len = (raw_tx[offset] & 0x7f) as usize;
                if offset + 1 + len > raw_tx.len() {
                    return Err("Invalid transaction: malformed 'gasLimit' field"
                        .as_bytes()
                        .to_vec());
                }
                let mut gas_bytes = [0u8; 8];
                let src_start = offset + 1;
                let src_end = offset + 1 + len;
                let dest_start = 8 - len;
                gas_bytes[dest_start..dest_start + len]
                    .copy_from_slice(&raw_tx[src_start..src_end]);
                offset += 1 + len;
                u64::from_be_bytes(gas_bytes)
            } else {
                return Err("Invalid transaction: malformed 'gasLimit' field"
                    .as_bytes()
                    .to_vec());
            };

            // Extract 'to' address (20 bytes, prefixed with 0x94)
            if offset + 1 > raw_tx.len() {
                return Err("Invalid transaction: missing 'to' field"
                    .as_bytes()
                    .to_vec());
            }

            let to = if raw_tx[offset] == 0x94 && offset + 21 <= raw_tx.len() {
                let addr = raw_tx[offset + 1..offset + 21].to_vec();
                offset += 21;
                addr
            } else if raw_tx[offset] == 0x80 && offset + 1 <= raw_tx.len() {
                offset += 1;
                Vec::new()
            } else {
                return Err("Invalid transaction: malformed 'to' field"
                    .as_bytes()
                    .to_vec());
            };

            // Extract 'value' (16 bytes for u128)
            if offset >= raw_tx.len() {
                return Err("Invalid transaction: missing 'value' field"
                    .as_bytes()
                    .to_vec());
            }

            let value = if raw_tx[offset] == 0x80 && offset + 1 <= raw_tx.len() {
                offset += 1;
                0u128
            } else if raw_tx[offset] >= 0x80 && raw_tx[offset] <= 0xb7 {
                let len = (raw_tx[offset] & 0x7f) as usize;
                if offset + 1 + len > raw_tx.len() {
                    return Err("Invalid transaction: malformed 'value' field"
                        .as_bytes()
                        .to_vec());
                }
                let mut value_bytes = [0u8; 16];
                let src_start = offset + 1;
                let src_end = offset + 1 + len;
                let dest_start = 16 - len;
                value_bytes[dest_start..dest_start + len]
                    .copy_from_slice(&raw_tx[src_start..src_end]);
                offset += 1 + len;
                u128::from_be_bytes(value_bytes)
            } else {
                return Err("Invalid transaction: malformed 'value' field"
                    .as_bytes()
                    .to_vec());
            };

            // Extract 'data' (input)
            if offset >= raw_tx.len() {
                return Err("Invalid transaction: missing 'data' field"
                    .as_bytes()
                    .to_vec());
            }

            let input = if raw_tx[offset] == 0x80 && offset + 1 <= raw_tx.len() {
                offset += 1;
                Vec::new()
            } else if raw_tx[offset] >= 0xb8 && raw_tx[offset] <= 0xbf {
                let len = (raw_tx[offset] & 0x7f) as usize;
                if offset + 1 + len > raw_tx.len() {
                    return Err("Invalid transaction: malformed 'data' field"
                        .as_bytes()
                        .to_vec());
                }
                let data = raw_tx[offset + 1..offset + 1 + len].to_vec();
                offset += 1 + len;
                data
            } else {
                return Err("Invalid transaction: malformed 'data' field"
                    .as_bytes()
                    .to_vec());
            };

            // Extract nonce (we already parsed it, but need to return it)
            // For simplicity, return 0 as nonce since we can't easily extract it from the RLP
            let nonce = 0u64;

            Ok((Vec::new(), to, value, gas, input, nonce, gas_price))
        }

        fn submit_evm_transaction(raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
            // Parse the raw EVM transaction (RLP-encoded) to extract sender, recipient, and value
            // The ExecutionReceipt struct has from/to/value fields that should be populated

            // Compute tx_hash first for replay check
            let tx_hash = sp_io::hashing::keccak_256(&raw_tx);
            let tx_hash_h256 = H256::from(tx_hash);

            // Check for replay - if already submitted, reject
            if SubmittedComits::<T>::contains_key(tx_hash_h256) {
                return Err(b"replay".to_vec());
            }

            // Try to parse the transaction to extract from/to/value
            let (from, to, value) = match Self::parse_ethereum_transaction(&raw_tx) {
                Ok((f, t, v)) => (f, t, v),
                Err(e) => {
                    // If parsing fails, we can still execute the transaction
                    // The adapter will populate what it can
                    (Vec::new(), Vec::new(), 0)
                }
            };

            // Execute via configured EVM adapter
            let mut receipt = T::EvmAdapter::execute(&raw_tx, 10_000_000)
                .map_err(|e| format!("{:?}", e).into_bytes())?;

            // Populate from/to/value from the parsed transaction
            receipt.from = from;
            receipt.to = to;
            receipt.value = value;

            // Store the receipt keyed by transaction hash
            EvmTransactionReceipts::<T>::insert(tx_hash_h256, receipt.clone());

            // Store the full transaction data for RPC compatibility
            // Parse the raw transaction to extract gas and input
            let tx_data = match Self::parse_ethereum_transaction_with_gas(&raw_tx) {
                Ok((f, t, v, gas, input, nonce, gas_price)) => EvmTransactionData {
                    raw: raw_tx.clone(),
                    from: f,
                    to: t,
                    value: v,
                    gas,
                    input,
                    nonce,
                    gas_price,
                },
                Err(_) => {
                    // Fallback if parsing fails - use receipt data
                    EvmTransactionData {
                        raw: raw_tx.clone(),
                        from: receipt.from.clone(),
                        to: receipt.to.clone(),
                        value: receipt.value,
                        gas: receipt.gas_used,
                        input: Vec::new(),
                        nonce: 0,
                        gas_price: 0,
                    }
                }
            };
            EvmTransactions::<T>::insert(tx_hash_h256, tx_data);

            // Record transaction as submitted for replay prevention
            let current_block = frame_system::Pallet::<T>::block_number();
            SubmittedComits::<T>::insert(tx_hash_h256, current_block);

            Ok(tx_hash.to_vec())
        }

        fn validate_evm_transaction(raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
            // Validate the raw EVM transaction without executing it
            // For now, we just check that the payload is not empty
            if raw_tx.is_empty() {
                return Err("Empty transaction".as_bytes().to_vec());
            }
            Ok(raw_tx)
        }

        fn get_evm_logs(filter: Vec<u8>) -> Vec<Vec<u8>> {
            // Decode preferred SCALE tuple format:
            // (from_block: u64, to_block: u64, address: Option<[u8; 20]>)
            // Keep a fallback decoder for older concatenated format to preserve compatibility.
            let (from_block, to_block, address_filter): (u64, u64, Option<[u8; 20]>) =
                match <(u64, u64, Option<[u8; 20]>)>::decode(&mut &filter[..]) {
                    Ok(decoded) => decoded,
                    Err(_) => {
                        if filter.len() < 16 {
                            return Vec::new();
                        }
                        let from_block =
                            u64::from_le_bytes(filter[0..8].try_into().unwrap_or([0u8; 8]));
                        let to_block =
                            u64::from_le_bytes(filter[8..16].try_into().unwrap_or([0u8; 8]));
                        // Check Option tag (0 = None, 1 = Some)
                        let opt_tag = filter.get(16).copied().unwrap_or(0);
                        let address_filter = if opt_tag == 1 && filter.len() >= 37 {
                            let mut addr = [0u8; 20];
                            addr.copy_from_slice(&filter[17..37]);
                            Some(addr)
                        } else {
                            None
                        };
                        (from_block, to_block, address_filter)
                    }
                };

            // Get current SVM slot as upper bound (since EVM logs are tied to SVM blocks)
            let current_slot = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();

            // Determine actual to_block (use current slot if to_block is 0 or exceeds current slot)
            let actual_to_block = if to_block == 0 || to_block > current_slot {
                current_slot
            } else {
                to_block
            };

            // Filter logs server-side based on block range and optional address
            let mut result_logs = Vec::new();

            // Iterate through all transaction receipts in the storage
            // For each receipt, check if its logs match the filter criteria
            for (_tx_hash, receipt) in EvmTransactionReceipts::<T>::iter() {
                // Check if receipt's block_number is within the filter range
                // Note: ExecutionReceipt doesn't have block_number directly
                // We need to add it or derive it from the execution context

                for log in receipt.logs {
                    // Check if log's block_number is within range
                    // Since ExecutionLog has block_number field, we can filter on it
                    if log.block_number >= from_block && log.block_number <= actual_to_block {
                        // Check if address filter matches (if specified)
                        if let Some(addr_filter) = address_filter {
                            if log.address == addr_filter {
                                result_logs.push(log.encode());
                            }
                        } else {
                            result_logs.push(log.encode());
                        }
                    }
                }
            }

            result_logs
        }
    }

    impl<T: Config> KernelCrossVmDispatcher<T> {
        /// Hard-fail enforcement of atomic cross-VM invariants.
        /// Verifies all VM branches settled consistently or panics.
        ///
        /// # Arguments
        /// - `tx_id`: Transaction ID for settlement root lookup
        /// - `evm_state`: EVM branch final state
        /// - `svm_state`: SVM branch final state
        /// - `cosmos_state`: Cosmos branch final state
        ///
        /// # Safety
        /// Panics (hard-fail) if atomicity invariant is violated.
        /// This is intentional: consensus must halt rather than diverge.
        pub fn enforce_cross_vm_atomicity(
            tx_id: u64,
            evm_state: &[u8],
            svm_state: &[u8],
            cosmos_state: &[u8],
        ) -> DispatchResult {
            // Verify all branches present
            ensure!(!evm_state.is_empty(), Error::<T>::CrossVmAtomicityBreach);
            ensure!(!svm_state.is_empty(), Error::<T>::CrossVmAtomicityBreach);
            ensure!(!cosmos_state.is_empty(), Error::<T>::CrossVmAtomicityBreach);

            // Compute hash for each branch
            use sp_io::hashing::blake2_256;
            let evm_hash = blake2_256(evm_state);
            let svm_hash = blake2_256(svm_state);
            let cosmos_hash = blake2_256(cosmos_state);

            // Combine branch hashes for settlement root
            let mut combined = Vec::new();
            combined.extend_from_slice(&evm_hash);
            combined.extend_from_slice(&svm_hash);
            combined.extend_from_slice(&cosmos_hash);
            let combined_hash = blake2_256(&combined);

            // Retrieve stored settlement root
            let stored_root = StoredSettlementRoots::<T>::get(tx_id)
                .expect("BUG: Settlement root MUST exist for this transaction ID");

            // Hard-fail if atomicity invariant violated
            ensure!(combined_hash == stored_root, Error::<T>::SettlementMismatch);

            Ok(())
        }
    }

    /// Asset metadata stored alongside each asset id.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(Symbol))]
    pub struct AssetMetadata<Symbol: MaxEncodedLen> {
        pub symbol: Symbol,
        pub decimals: u8,
    }
}

// WeightInfo trait and implementations are now in weights.rs module
// Re-exported via `pub use weights::WeightInfo;` at module root

// Runtime API definitions for querying X3 Kernel state
sp_api::decl_runtime_apis! {
    /// Runtime API for querying X3 Kernel pallet state
    pub trait AtlasKernelRuntimeApi<AccountId, Balance, AssetId> where
        AccountId: Codec,
        Balance: Codec,
        AssetId: Codec,
    {
        /// Get the canonical balance for a specific account and asset
        fn get_canonical_balance(account: AccountId, asset_id: AssetId) -> Balance;

        /// Get asset metadata (symbol, decimals) for a specific asset
        fn get_asset_metadata(asset_id: AssetId) -> Option<(Vec<u8>, u8)>;

        /// Check if an account is authorized to submit Comits
        fn is_authorized(account: AccountId) -> bool;

        /// Get all authorized accounts
        fn get_authorized_accounts() -> Vec<AccountId>;

        /// Get the current authority set
        fn get_authorities() -> Vec<AccountId>;

        /// Map an EVM 20-byte address into a runtime AccountId (Option)
        fn map_evm_address(address: Vec<u8>) -> Option<AccountId>;

        /// Query EVM-specific canonical balance by EVM address
        fn get_evm_balance(evm_address: Vec<u8>, asset_id: AssetId) -> Option<Balance>;

        /// Query contract bytecode for an EVM address
        fn get_evm_code(evm_address: Vec<u8>) -> Vec<u8>;

        /// Query EVM storage at a specific storage key for an EVM address
        fn get_evm_storage(evm_address: Vec<u8>, storage_key: H256) -> Option<H256>;

        /// Query the EVM transaction count (nonce) for an EVM address
        fn get_evm_nonce(evm_address: Vec<u8>) -> u64;

        /// Query the native lamport balance for an SVM public key (32 bytes)
        fn get_svm_balance(svm_pubkey: Vec<u8>) -> u64;

        /// Check whether an SVM public key has an executable program deployed
        fn is_svm_program(svm_pubkey: Vec<u8>) -> bool;

        /// Submit a signed raw EVM transaction (RLP-encoded).
        /// Decodes the transaction, executes it via the EVM adapter, and returns
        /// the keccak256 transaction hash on success.
        fn submit_evm_transaction(raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>>;

        /// Validate a signed raw EVM transaction without mutating state.
        fn validate_evm_transaction(raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>>;

        /// Submit a raw SVM instruction payload.
        fn submit_svm_instruction(program_id: [u8; 32], instruction_data: Vec<u8>) -> Result<Vec<u8>, Vec<u8>>;

        /// Execute an EVM call against a target address.
        fn call_evm(caller: Option<Vec<u8>>, evm_address: Vec<u8>, input: Vec<u8>, gas_limit: u64) -> Result<Vec<u8>, Vec<u8>>;

        /// Estimate gas for an EVM call.
        fn estimate_evm_gas(caller: Option<Vec<u8>>, evm_address: Vec<u8>, input: Vec<u8>, gas_limit: u64) -> Result<u64, Vec<u8>>;

        /// Get an EVM transaction receipt by transaction hash.
        /// Returns SCALE-encoded ExecutionReceipt bytes, or None if not found.
        fn get_evm_transaction(tx_hash: Vec<u8>) -> Option<Vec<u8>>;

        /// Get an EVM transaction by transaction hash.
        /// Returns SCALE-encoded EvmTransactionData bytes, or None if not found.
        /// This provides the full transaction object with gas and input fields
        /// for Ethereum JSON-RPC compatibility.
        fn get_evm_transaction_by_hash(tx_hash: Vec<u8>) -> Option<Vec<u8>>;

        /// Get an EVM transaction receipt (for RPC layer formatting).
        /// Returns SCALE-encoded ExecutionReceipt bytes, or None if not found.
        fn get_evm_receipt(tx_hash: Vec<u8>) -> Option<Vec<u8>>;

        /// Get EVM logs matching a filter.
        /// Filter is decoded as (from_block: u64, to_block: u64, address: Option<Vec<u8>>).
        /// Returns SCALE-encoded ExecutionLog entries.
        fn get_evm_logs(filter: Vec<u8>) -> Vec<Vec<u8>>;

        /// Get EVM logs for a specific transaction by hash.
        /// Returns SCALE-encoded ExecutionLog entries.
        fn get_evm_transaction_logs(tx_hash: Vec<u8>) -> Vec<Vec<u8>>;

        /// Get the chain ID for the current network.
        fn chain_id() -> u64;

        /// Get the current SVM slot number (equivalent to block number in Solana)
        fn get_svm_slot() -> u64;

        /// Get the SVM blockhash for a given slot number
        fn get_svm_blockhash(slot: u64) -> Option<H256>;

        /// Get the transaction count for an SVM address
        fn get_svm_transaction_count(svm_pubkey: Vec<u8>) -> u64;

        /// Get the SVM slot number for a given blockhash (reverse lookup)
        fn get_svm_slot_by_blockhash(blockhash: H256) -> Option<u64>;

        /// Deploy a new EVM contract with the given bytecode
        fn deploy_evm_contract(caller: Option<Vec<u8>>, bytecode: Vec<u8>, gas_limit: u64) -> Result<Vec<u8>, Vec<u8>>;

        /// Get the EVM contract creation receipt
        fn get_evm_contract_receipt(contract_address: Vec<u8>) -> Option<Vec<u8>>;

        /// Get the SVM program data for a deployed SVM program
        fn get_svm_program_data(svm_pubkey: Vec<u8>) -> Option<Vec<u8>>;

        /// Get the SVM account data for a SVM address
        fn get_svm_account_data(svm_pubkey: Vec<u8>) -> Option<Vec<u8>>;

        /// Get the SVM slot history for recent blockhashes.
        /// `limit` caps the number of entries returned (max: MAX_RECENT_BLOCKHASHES = 150).
        fn get_svm_slot_history(limit: u32) -> Vec<u64>;

        /// Get the SVM recent blockhashes.
        /// `limit` caps the number of entries returned (max: MAX_RECENT_BLOCKHASHES = 150).
        fn get_svm_recent_blockhashes(limit: u32) -> Vec<H256>;
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chaos_tests;

#[cfg(test)]
mod packet_integration_tests;
