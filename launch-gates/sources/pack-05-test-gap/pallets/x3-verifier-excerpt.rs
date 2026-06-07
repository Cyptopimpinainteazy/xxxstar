#![deny(unsafe_code)]
//! # X3 Verifier Pallet
//!
//! On-chain verification of X3 execution receipts from the swarm network.
//! This pallet validates off-chain computations and applies their state changes
//! to the canonical ledger after verification.
//!
//! ## Overview
//!
//! The X3 Verifier implements:
//! - **Receipt Submission**: Accept execution receipts from swarm nodes
//! - **Signature Verification**: Validate executor signatures
//! - **Merkle Proof Verification**: Verify state transition proofs
//! - **State Application**: Apply verified changes to chain state
//! - **Reward Distribution**: Pay executors for valid work
//! - **Slashing**: Penalize malicious or incorrect submissions
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      SWARM NETWORK                               │
//! │  ┌─────────┐   ┌─────────┐   ┌─────────┐                        │
//! │  │ Node 1  │   │ Node 2  │   │ Node N  │                        │
//! │  └────┬────┘   └────┬────┘   └────┬────┘                        │
//! │       │             │             │                              │
//! │       └─────────────┴─────────────┘                              │
//! │                     │                                            │
//! │                     ▼                                            │
//! │           ┌─────────────────┐                                    │
//! │           │ Execution       │                                    │
//! │           │ Receipt         │                                    │
//! │           └────────┬────────┘                                    │
//! └────────────────────│────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   X3 VERIFIER PALLET                             │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │  │  Signature  │  │   Merkle    │  │   State     │              │
//! │  │  Verify     │─▶│   Verify    │─▶│   Apply     │              │
//! │  └─────────────┘  └─────────────┘  └─────────────┘              │
//! │                                           │                      │
//! │                                           ▼                      │
//! │                                 ┌─────────────────┐              │
//! │                                 │ Canonical       │              │
//! │                                 │ Ledger Update   │              │
//! │                                 └─────────────────┘              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security
//!
//! - All receipts require valid executor signatures
//! - Merkle proofs verify state transitions
//! - Redundant verification for high-value operations
//! - Slashing for invalid submissions

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod runtime_api;
// Note: We don't re-export runtime_api::* to avoid conflicts with JobId defined in pallet

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_io::hashing::{blake2_256, keccak_256};
    use sp_runtime::traits::Saturating;
    use sp_std::vec::Vec;

    /// Current storage version
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ============================================================================
    // Types
    // ============================================================================

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Job identifier (32 bytes)
    pub type JobId = H256;

    /// State root (32 bytes)
    pub type StateRoot = H256;

    /// Execution receipt from swarm node
    #[allow(clippy::type_complexity)]
    #[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ExecutionReceipt<T: Config> {
        /// Unique job identifier
        pub job_id: JobId,
        /// Executor's public key (account)
        pub executor: T::AccountId,
        /// Hash of the input (bytecode + arguments)
        pub input_hash: H256,
        /// Hash of the output
        pub output_hash: H256,
        /// State root before execution
        pub state_root_before: StateRoot,
        /// State root after execution
        pub state_root_after: StateRoot,
        /// Gas consumed
        pub gas_used: u128,
        /// Execution timestamp
        pub timestamp: u64,
        /// Output data (bounded)
        pub output_data: BoundedVec<u8, T::MaxOutputSize>,
        /// State changes (key-value pairs)
        pub state_changes: BoundedVec<
            (
                BoundedVec<u8, T::MaxKeySize>,
                BoundedVec<u8, T::MaxValueSize>,
            ),
            T::MaxStateChanges,
        >,
        /// Merkle proof for state transition
        pub merkle_proof: BoundedVec<H256, T::MaxProofDepth>,
        /// Signature over receipt data
        pub signature: BoundedVec<u8, ConstU32<64>>,
    }

    impl<T: Config> Clone for ExecutionReceipt<T> {
        fn clone(&self) -> Self {
            Self {
                job_id: self.job_id,
                executor: self.executor.clone(),
                input_hash: self.input_hash,
                output_hash: self.output_hash,
                state_root_before: self.state_root_before,
                state_root_after: self.state_root_after,
                gas_used: self.gas_used,
                timestamp: self.timestamp,
                output_data: self.output_data.clone(),
                state_changes: self.state_changes.clone(),
                merkle_proof: self.merkle_proof.clone(),
                signature: self.signature.clone(),
            }
        }
    }

    impl<T: Config> core::fmt::Debug for ExecutionReceipt<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("ExecutionReceipt")
                .field("job_id", &self.job_id)
                .field("gas_used", &self.gas_used)
                .field("timestamp", &self.timestamp)
                .finish()
        }
    }

    impl<T: Config> PartialEq for ExecutionReceipt<T> {
        fn eq(&self, other: &Self) -> bool {
            self.job_id == other.job_id
                && self.executor == other.executor
                && self.input_hash == other.input_hash
                && self.output_hash == other.output_hash
                && self.gas_used == other.gas_used
        }
    }

    impl<T: Config> Eq for ExecutionReceipt<T> {}

    /// Job status
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
    pub enum JobStatus {
        #[default]
        Pending,
        Submitted,
        Verified,
        Applied,
        Failed,
        Disputed,
    }

    /// Job record
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct JobRecord<T: Config> {
        /// Job submitter
        pub submitter: T::AccountId,
        /// Bytecode hash
        pub bytecode_hash: H256,
        /// Input data hash
        pub input_hash: H256,
        /// Gas limit
        pub gas_limit: u128,
        /// Reward amount
        pub reward: BalanceOf<T>,
        /// Status
        pub status: JobStatus,
        /// Block submitted
        pub submitted_at: BlockNumberFor<T>,
        /// Executor (if assigned)
        pub executor: Option<T::AccountId>,
        /// Receipt (if submitted)
        pub receipt_hash: Option<H256>,
    }

    /// Registered executor
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
    #[scale_info(skip_type_params(T))]
    pub struct ExecutorRecord<T: Config> {
        /// Executor account
        pub account: T::AccountId,
        /// Stake amount
        pub stake: BalanceOf<T>,
        /// Jobs completed
        pub jobs_completed: u64,
        /// Jobs failed
        pub jobs_failed: u64,
        /// Total rewards earned
        pub total_rewards: BalanceOf<T>,
        /// Is active
        pub active: bool,
        /// Reputation score (0-100)
        pub reputation: u8,
    }

    // ============================================================================
    // Config
    // ============================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for rewards and staking
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Origin that can register executors
        type ExecutorRegistrar: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum stake to become executor
        #[pallet::constant]
        type MinExecutorStake: Get<BalanceOf<Self>>;

        /// Maximum output data size
        #[pallet::constant]
        type MaxOutputSize: Get<u32>;

        /// Maximum key size for state changes
        #[pallet::constant]
        type MaxKeySize: Get<u32>;

        /// Maximum value size for state changes
        #[pallet::constant]
        type MaxValueSize: Get<u32>;

        /// Maximum state changes per receipt
        #[pallet::constant]
        type MaxStateChanges: Get<u32>;

        /// Maximum Merkle proof depth
        #[pallet::constant]
        type MaxProofDepth: Get<u32>;

        /// Reward share for executor (percentage, e.g., 70)
        #[pallet::constant]
        type ExecutorRewardShare: Get<u32>;

        /// Protocol fee share (percentage, e.g., 15)
        #[pallet::constant]
        type ProtocolFeeShare: Get<u32>;

        /// Slash amount for invalid submissions
        #[pallet::constant]
        type SlashAmount: Get<BalanceOf<Self>>;

        /// Job timeout in blocks
        #[pallet::constant]
        type JobTimeout: Get<BlockNumberFor<Self>>;

        /// Weight information
        type WeightInfo: WeightInfo;
    }

    // ============================================================================
    // Pallet
    // ============================================================================

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ============================================================================
    // Storage
    // ============================================================================

    /// Registered executors
    #[pallet::storage]
    #[pallet::getter(fn executors)]
    pub type Executors<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ExecutorRecord<T>, OptionQuery>;

    /// Pending jobs
    #[pallet::storage]
    #[pallet::getter(fn jobs)]
    pub type Jobs<T: Config> = StorageMap<_, Blake2_128Concat, JobId, JobRecord<T>, OptionQuery>;

    /// Submitted receipts
    #[pallet::storage]
    #[pallet::getter(fn receipts)]
    pub type Receipts<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, ExecutionReceipt<T>, OptionQuery>;

    /// Verified state roots (job_id -> final state root)
    #[pallet::storage]
    pub type VerifiedStateRoots<T: Config> =
        StorageMap<_, Blake2_128Concat, JobId, StateRoot, OptionQuery>;

    /// Total jobs submitted
    #[pallet::storage]
    pub type TotalJobsSubmitted<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total jobs verified
    #[pallet::storage]
    pub type TotalJobsVerified<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Protocol treasury
    #[pallet::storage]
    pub type ProtocolTreasury<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Verification enabled
    #[pallet::storage]
    #[pallet::getter(fn verification_enabled)]
    pub type VerificationEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    // ============================================================================
    // Genesis
    // ============================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub verification_enabled: bool,
        pub initial_executors: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            VerificationEnabled::<T>::put(self.verification_enabled);
            for executor in &self.initial_executors {
                let record = ExecutorRecord {
                    account: executor.clone(),
                    stake: T::MinExecutorStake::get(),
                    jobs_completed: 0,
                    jobs_failed: 0,
                    total_rewards: 0u32.into(),
                    active: true,
                    reputation: 50,
                };
                Executors::<T>::insert(executor, record);
            }
        }
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New job submitted
        JobSubmitted {
            job_id: JobId,
            submitter: T::AccountId,
            reward: BalanceOf<T>,
        },
        /// Job assigned to executor
        JobAssigned {
            job_id: JobId,
            executor: T::AccountId,
        },
        /// Receipt submitted
        ReceiptSubmitted {
            job_id: JobId,
            executor: T::AccountId,
            receipt_hash: H256,
        },
        /// Receipt verified and state applied
        ReceiptVerified {
            job_id: JobId,
            state_root: StateRoot,
            gas_used: u128,
        },
        /// Receipt verification failed
        ReceiptFailed {
            job_id: JobId,
            reason: BoundedVec<u8, ConstU32<128>>,
        },
        /// Executor registered
        ExecutorRegistered {
            executor: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Executor slashed
        ExecutorSlashed {
            executor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// Reward distributed
        RewardDistributed {
            executor: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// State change applied
        StateChangeApplied {
            job_id: JobId,
            key: BoundedVec<u8, T::MaxKeySize>,
            value_hash: H256,
        },
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Verification is disabled
        VerificationDisabled,
        /// Job not found
        JobNotFound,
        /// Receipt not found
        ReceiptNotFound,
        /// Executor not registered
        ExecutorNotRegistered,
        /// Executor not active
        ExecutorNotActive,
        /// Invalid signature
        InvalidSignature,
        /// Invalid Merkle proof
        InvalidMerkleProof,
        /// State root mismatch
        StateRootMismatch,
        /// Job already completed
        JobAlreadyCompleted,
        /// Insufficient stake
        InsufficientStake,
        /// Job timeout
        JobTimeout,
        /// Not authorized
        NotAuthorized,
        /// Already registered
        AlreadyRegistered,
        /// Invalid receipt format
        InvalidReceiptFormat,
        /// Output too large
        OutputTooLarge,
    }

    // ============================================================================
    // Hooks
    // ============================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // Timeout expired jobs
            Self::timeout_expired_jobs(block);
        }
    }

    // ============================================================================
    // Calls
    // ============================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as an executor (stake required)
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_executor())]
        pub fn register_executor(origin: OriginFor<T>, stake: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !Executors::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
