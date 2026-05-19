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
pub mod wasm_adapters;
pub use adapters::{
    EvmExecutorAdapter, FailingMockEvmAdapter, FailingMockSvmAdapter, FailingMockX3Adapter,
    MockEvmAdapter, MockSvmAdapter, MockX3Adapter, SvmExecutorAdapter, X3ExecutorAdapter,
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
use frame_support::traits::{Currency, UnixTime};
use frame_system::pallet_prelude::*;
use parity_scale_codec::Codec;
use sp_core::{H160, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::MaybeSerializeDeserialize;
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;
use x3_cross_vm_bridge::{
    CrossVmBridge, CrossVmCall, CrossVmDispatcher, CrossVmOperation, CrossVmReceipt, CrossVmResult,
    CrossVmStatus, VmId,
};

pub const EXECUTION_RECEIPT_VERSION: u32 = 1;

/// Represents a Comit transaction submitted to the X3 Kernel.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
}

/// Log entry emitted during VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ExecutionLog {
    /// Address (EVM H160 or SVM 32-byte key) that emitted the log.
    pub address: Vec<u8>,
    /// Topics for the log entry.
    pub topics: Vec<H256>,
    /// Log data.
    pub data: Vec<u8>,
}

/// State change resulting from VM execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StateChange {
    /// Account/contract address affected (EVM H160 or SVM 32-byte key).
    pub address: Vec<u8>,
    /// Storage slot key.
    pub key: H256,
    /// New value at the storage slot.
    pub value: H256,
}

/// Unified state representation for the X3 Chain.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct SphereState {
    /// State root hash representing the entire sphere state.
    pub state_root: H256,
    /// Block number when this state was computed.
    pub block_number: u32,
    /// Timestamp of state computation.
    pub timestamp: u64,
}

/// Dual-VM transaction types that can be executed.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum VmTransaction {
    /// EVM transaction payload.
    Evm(Vec<u8>),
    /// SVM transaction payload.
    Svm(Vec<u8>),
}

/// Reasons describing why a Comit failed verification or execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
