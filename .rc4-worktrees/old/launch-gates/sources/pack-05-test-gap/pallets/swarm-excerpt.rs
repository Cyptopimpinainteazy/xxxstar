#![deny(unsafe_code)]
//! # X3 Chain GPU Swarm Pallet
//!
//! **DEPRECATED**: The swarm pallet is superseded by the Inferstructor service.
//! It is retained for reference and historical finality only and will be removed
//! in a future release. Do not add new production dependencies on `pallet-swarm`.
//! See `docs/openspec/changes/refactor-swarm-legacy/` for migration guidance.
//!
//! On-chain contributor registry, task lifecycle, commit-reveal jury verification,
//! and reward distribution for the distributed GPU compute swarm.
//!
//! ## Overview
//!
//! This pallet provides on-chain primitives for the X3 Chain GPU swarm:
//!
//! - **Contributor Registry**: GPU node operators register with a stake, advertise
//!   capabilities, and maintain liveness via heartbeats.
//! - **Task Lifecycle**: Users submit compute tasks on-chain (payload stored off-chain),
//!   contributors claim and execute them, then submit result hashes.
//! - **Jury Verification**: A commit-reveal voting scheme validates execution results
//!   before rewards are distributed.
//! - **Reward Distribution**: Rewards flow from task submitters to contributors and
//!   the protocol treasury upon successful verification.
//! - **Slashing**: Dishonest or unresponsive contributors are slashed.
//!
//! ## Architecture
//!
//! ```text
//!  Submitter ─► submit_task() ─► Pending
//!                                   │
//!  Contributor ◄─ claim_task() ◄────┘
//!       │
//!       ├─► submit_result() ─► Verifying
//!       │                         │
//!  Jury ├─► commit_vote()   ◄─────┘
//!       ├─► reveal_vote()
//!       │
//!       └─► finalize_session() ─► Completed (reward paid)
//!                                 or Failed (slash applied)
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]

pub use pallet::*;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons},
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{BlakeTwo256, Hash, Saturating, Zero};
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ========================================================================
    // Config
    // ========================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for staking, rewards, and slashing.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Origin that can update swarm configuration.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can slash contributors.
        type SlashOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Minimum stake to register as a contributor.
        #[pallet::constant]
        type MinContributorStake: Get<BalanceOf<Self>>;

        /// Heartbeat interval in blocks. Contributors must heartbeat within this window.
        #[pallet::constant]
        type HeartbeatInterval: Get<BlockNumberFor<Self>>;

        /// Unstaking cooldown period in blocks.
        #[pallet::constant]
        type UnstakeCooldown: Get<BlockNumberFor<Self>>;

        /// Default task timeout in blocks.
        #[pallet::constant]
        type DefaultTaskTimeout: Get<BlockNumberFor<Self>>;

        /// Duration of jury commit phase in blocks.
        #[pallet::constant]
        type CommitPhaseDuration: Get<BlockNumberFor<Self>>;

        /// Duration of jury reveal phase in blocks.
        #[pallet::constant]
        type RevealPhaseDuration: Get<BlockNumberFor<Self>>;

        /// Contributor reward percentage (0-100).
        #[pallet::constant]
        type ContributorRewardPct: Get<u8>;

        /// Protocol fee percentage (0-100).
        #[pallet::constant]
        type ProtocolFeePct: Get<u8>;

        /// Slash amount for misbehavior.
        #[pallet::constant]
        type SlashAmount: Get<BalanceOf<Self>>;

        /// Maximum concurrent tasks per contributor.
        #[pallet::constant]
        type MaxTasksPerContributor: Get<u32>;

        /// Maximum jury voters per session.
        #[pallet::constant]
        type MaxJuryVoters: Get<u32>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage
    // ========================================================================

    /// Next contributor ID counter.
    #[pallet::storage]
    #[pallet::getter(fn next_contributor_id)]
    pub type NextContributorId<T> = StorageValue<_, ContributorId, ValueQuery>;

    /// All registered contributors.
    #[pallet::storage]
    #[pallet::getter(fn contributors)]
    pub type Contributors<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ContributorId,
        Contributor<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Account to contributor ID mapping.
    #[pallet::storage]
    #[pallet::getter(fn account_contributor)]
    pub type AccountContributor<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ContributorId, OptionQuery>;

    /// All submitted tasks.
    #[pallet::storage]
    #[pallet::getter(fn tasks)]
    pub type Tasks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TaskId,
        SwarmTask<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Task results.
    #[pallet::storage]
    #[pallet::getter(fn task_results)]
    pub type TaskResults<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TaskId,
        TaskResult<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Active jury sessions.
    #[pallet::storage]
    #[pallet::getter(fn jury_sessions)]
    pub type JurySessions<T: Config> =
        StorageMap<_, Blake2_128Concat, SessionId, JurySession<BlockNumberFor<T>>, OptionQuery>;

    /// Active jury session per task (prevents duplicate reward/slash finalization paths).
    #[pallet::storage]
    #[pallet::getter(fn active_session_by_task)]
    pub type ActiveSessionByTask<T: Config> =
        StorageMap<_, Blake2_128Concat, TaskId, SessionId, OptionQuery>;

    /// Jury vote commitments: (session_id, voter) -> commitment.
    #[pallet::storage]
    #[pallet::getter(fn vote_commitments)]
    pub type VoteCommitments<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SessionId,
        Blake2_128Concat,
        T::AccountId,
        VoteCommitment<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Jury vote reveals: (session_id, voter) -> reveal.
    #[pallet::storage]
    #[pallet::getter(fn vote_reveals)]
    pub type VoteReveals<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        SessionId,
        Blake2_128Concat,
        T::AccountId,
        VoteReveal<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Number of active tasks assigned to each contributor.
    #[pallet::storage]
    #[pallet::getter(fn contributor_active_tasks)]
    pub type ContributorActiveTasks<T: Config> =
        StorageMap<_, Blake2_128Concat, ContributorId, u32, ValueQuery>;

    // ----- Global Counters -----

    #[pallet::storage]
    #[pallet::getter(fn total_contributors)]
    pub type TotalContributors<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_contributors)]
    pub type ActiveContributors<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_tasks_submitted)]
    pub type TotalTasksSubmitted<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_tasks_completed)]
    pub type TotalTasksCompleted<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_tasks_failed)]
    pub type TotalTasksFailed<T> = StorageValue<_, u64, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new contributor registered.
        ContributorRegistered {
            contributor_id: ContributorId,
            account: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Contributor deregistration requested.
        ContributorDeregistering {
            contributor_id: ContributorId,
            cooldown_until: BlockNumberFor<T>,
        },
        /// Contributor fully deregistered and stake returned.
        ContributorDeregistered {
            contributor_id: ContributorId,
            account: T::AccountId,
        },
        /// Contributor sent a heartbeat.
        Heartbeat {
            contributor_id: ContributorId,
            block: BlockNumberFor<T>,
        },
        /// A compute task was submitted.
        TaskSubmitted {
            task_id: TaskId,
            submitter: T::AccountId,
            workload_type: WorkloadType,
            reward: BalanceOf<T>,
        },
        /// A task was claimed by a contributor.
        TaskClaimed {
            task_id: TaskId,
            contributor_id: ContributorId,
        },
        /// A task result was submitted.
        ResultSubmitted {
            task_id: TaskId,
            contributor_id: ContributorId,
            result_hash: H256,
        },
        /// A jury session was started for a task result.
        JurySessionStarted {
            session_id: SessionId,
            task_id: TaskId,
        },
        /// A jury vote commitment was received.
        VoteCommitted {
            session_id: SessionId,
            voter: T::AccountId,
        },
        /// A jury vote was revealed.
        VoteRevealed {
            session_id: SessionId,
            voter: T::AccountId,
            vote: bool,
        },
        /// A jury session was finalized.
        JurySessionFinalized {
            session_id: SessionId,
            task_id: TaskId,
            verdict_valid: bool,
            yes_votes: u32,
            no_votes: u32,
        },
        /// Reward distributed to contributor.
        RewardDistributed {
            task_id: TaskId,
            contributor_id: ContributorId,
            amount: BalanceOf<T>,
        },
        /// Protocol fee charged from task reward.
        ProtocolFeeCharged {
            task_id: TaskId,
            amount: BalanceOf<T>,
        },
        /// Contributor was slashed.
        ContributorSlashed {
            contributor_id: ContributorId,
            amount: BalanceOf<T>,
            reason: BoundedVec<u8, ConstU32<128>>,
        },
        /// Task was cancelled.
        TaskCancelled {
            task_id: TaskId,
            submitter: T::AccountId,
        },
        /// Task timed out.
        TaskTimedOut { task_id: TaskId },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Contributor not found.
        ContributorNotFound,
        /// Account is already a contributor.
        AlreadyRegistered,
        /// Insufficient stake amount.
        InsufficientStake,
        /// Contributor is not active.
        ContributorNotActive,
        /// Contributor already deregistering.
        AlreadyDeregistering,
        /// Cooldown not yet expired.
        CooldownNotExpired,
        /// Task not found.
        TaskNotFound,
        /// Task is not in the expected status.
        InvalidTaskStatus,
        /// Contributor does not meet task requirements.
        RequirementsNotMet,
        /// Contributor has too many active tasks.
        TooManyActiveTasks,
        /// Not the task submitter.
        NotTaskSubmitter,
        /// Not the assigned contributor.
        NotAssignedContributor,
        /// Result already submitted.
        ResultAlreadySubmitted,
        /// Jury session not found.
        SessionNotFound,
        /// Jury session not in expected phase.
        InvalidSessionPhase,
        /// Already voted in this session.
        AlreadyVoted,
        /// No commitment found for reveal.
        NoCommitmentFound,
        /// Commitment mismatch during reveal.
        CommitmentMismatch,
        /// Session has not reached deadline.
        SessionNotReady,
        /// Task deadline has passed.
        TaskDeadlinePassed,
        /// Task already has an active jury session.
        JurySessionAlreadyExists,
        /// Task not yet timed out.
        TaskNotTimedOut,
        /// Contributor not deregistering.
        NotDeregistering,
        /// Maximum jury voters reached.
        MaxJuryVotersReached,
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Lightweight: actual timeout cleanup done via explicit extrinsics
            // or off-chain workers to avoid unbounded iteration
            Weight::zero()
        }
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ----------------------------------------------------------------
        // Contributor Management
        // ----------------------------------------------------------------

        /// Register as a GPU contributor with a stake.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_contributor())]
        pub fn register_contributor(
            origin: OriginFor<T>,
            stake: BalanceOf<T>,
            name: BoundedVec<u8, ConstU32<64>>,
            capabilities: GpuCapabilities,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure not already registered
            ensure!(
                !AccountContributor::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );

            // Check minimum stake
            ensure!(
                stake >= T::MinContributorStake::get(),
                Error::<T>::InsufficientStake
            );

            // Reserve the stake
            T::Currency::reserve(&who, stake)?;

            let contributor_id = NextContributorId::<T>::get();
            let current_block = frame_system::Pallet::<T>::block_number();

            let contributor = Contributor {
                id: contributor_id,
                account: who.clone(),
                stake,
                status: ContributorStatus::Active,
                capabilities,
                name,
                reputation: 100, // Neutral starting reputation
                tasks_completed: 0,
                tasks_failed: 0,
                registered_at: current_block,
                last_heartbeat: current_block,
                deregister_at: Zero::zero(),
            };

            Contributors::<T>::insert(contributor_id, contributor);
            AccountContributor::<T>::insert(&who, contributor_id);

            NextContributorId::<T>::put(contributor_id.saturating_add(1));
            TotalContributors::<T>::mutate(|n| *n = n.saturating_add(1));
            ActiveContributors::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::ContributorRegistered {
                contributor_id,
                account: who,
                stake,
            });

            Ok(())
        }

        /// Begin deregistration (starts unstaking cooldown).
        #[pallet::call_index(1)]
