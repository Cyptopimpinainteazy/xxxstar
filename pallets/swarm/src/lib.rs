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

pub mod determinism;
pub use determinism::{verify_deterministic_output, DeterminismTier, TaskDeterminismSpec};

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
    use pallet_x3_invariants;
    use sp_core::H256;
    use sp_runtime::traits::{BlakeTwo256, Hash, SaturatedConversion, Saturating, Zero};
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
    pub trait Config:
        frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_x3_invariants::Config
    {
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

    #[pallet::storage]
    #[pallet::getter(fn agent_capabilities)]
    pub(super) type AgentCapabilities<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        CapabilityEnvelope<BalanceOf<T>>,
        OptionQuery,
    >;

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
        /// Agent budget was successfully deducted.
        CapabilityBudgetDeducted {
            agent: T::AccountId,
            amount: BalanceOf<T>,
            remaining: BalanceOf<T>,
        },
        /// Capability budget validation failed.
        CapabilityBudgetExceeded {
            agent: T::AccountId,
            amount: BalanceOf<T>,
            available: BalanceOf<T>,
        },
        /// Kill-switch prevented capability execution.
        CapabilityKillSwitchHit { agent: T::AccountId },
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
        /// Capability budget exceeded.
        BudgetExceeded,
        /// Agent capability revoked or not found.
        CapabilityRevoked,
        /// Kill-switch activated for this agent.
        KillSwitchActive,
    }

    /// Agent capability envelope with budget tracking.
    #[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq, Eq)]
    pub struct CapabilityEnvelope<Balance> {
        /// Maximum budget for this agent
        pub budget: Balance,
        /// Amount already spent
        pub spent: Balance,
        /// Agent role/tier
        pub role: AgentRole,
        /// Is the kill-switch activated?
        pub killed: bool,
    }

    /// Agent role tiers
    #[derive(Clone, Copy, Encode, Decode, Debug, TypeInfo, PartialEq, Eq)]
    pub enum AgentRole {
        /// Full validator with all capabilities
        Validator,
        /// Gossip relay only
        Gossiper,
        /// Proof generation (restricted compute)
        Prover,
        /// Monitoring and observation only
        Monitor,
        /// Emergency or quarantined state (no operations)
        Quarantined,
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
        #[pallet::weight(<T as Config>::WeightInfo::register_contributor())]
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
        #[pallet::weight(<T as Config>::WeightInfo::deregister_contributor())]
        pub fn request_deregister(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&who).ok_or(Error::<T>::ContributorNotFound)?;

            Contributors::<T>::try_mutate(contributor_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::ContributorNotFound)?;
                ensure!(
                    c.status == ContributorStatus::Active || c.status == ContributorStatus::Idle,
                    Error::<T>::AlreadyDeregistering
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                c.status = ContributorStatus::Deregistering;
                c.deregister_at = current_block;

                ActiveContributors::<T>::mutate(|n| *n = n.saturating_sub(1));

                let cooldown_until = current_block.saturating_add(T::UnstakeCooldown::get());
                Self::deposit_event(Event::ContributorDeregistering {
                    contributor_id,
                    cooldown_until,
                });

                Ok(())
            })
        }

        /// Complete deregistration after cooldown and return stake.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::deregister_contributor())]
        pub fn complete_deregister(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&who).ok_or(Error::<T>::ContributorNotFound)?;

            let contributor =
                Contributors::<T>::get(contributor_id).ok_or(Error::<T>::ContributorNotFound)?;

            ensure!(
                contributor.status == ContributorStatus::Deregistering,
                Error::<T>::NotDeregistering
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let cooldown_end = contributor
                .deregister_at
                .saturating_add(T::UnstakeCooldown::get());
            ensure!(
                current_block >= cooldown_end,
                Error::<T>::CooldownNotExpired
            );

            // Return stake
            T::Currency::unreserve(&who, contributor.stake);

            // Clean up storage
            Contributors::<T>::remove(contributor_id);
            AccountContributor::<T>::remove(&who);
            ContributorActiveTasks::<T>::remove(contributor_id);

            TotalContributors::<T>::mutate(|n| *n = n.saturating_sub(1));

            Self::deposit_event(Event::ContributorDeregistered {
                contributor_id,
                account: who,
            });

            Ok(())
        }

        /// Submit a liveness heartbeat.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::heartbeat())]
        pub fn heartbeat(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&who).ok_or(Error::<T>::ContributorNotFound)?;

            Contributors::<T>::try_mutate(contributor_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::ContributorNotFound)?;
                ensure!(
                    c.status == ContributorStatus::Active,
                    Error::<T>::ContributorNotActive
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                c.last_heartbeat = current_block;

                Self::deposit_event(Event::Heartbeat {
                    contributor_id,
                    block: current_block,
                });

                Ok(())
            })
        }

        // ----------------------------------------------------------------
        // Task Lifecycle
        // ----------------------------------------------------------------

        /// Submit a compute task. Reward is reserved from submitter's balance.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_task())]
        pub fn submit_task(
            origin: OriginFor<T>,
            workload_type: WorkloadType,
            payload_hash: H256,
            reward: BalanceOf<T>,
            priority: TaskPriority,
            min_vram_mb: u32,
            min_compute_score: u32,
            verification_count: u8,
            deadline_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;

            // Reserve the reward
            T::Currency::reserve(&submitter, reward)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let deadline = if deadline_blocks > Zero::zero() {
                current_block.saturating_add(deadline_blocks)
            } else {
                current_block.saturating_add(T::DefaultTaskTimeout::get())
            };

            // Generate task ID from submission data
            let task_id: TaskId =
                BlakeTwo256::hash_of(&(&submitter, &workload_type, &payload_hash, &current_block));

            let task = SwarmTask {
                id: task_id,
                submitter: submitter.clone(),
                workload_type,
                payload_hash,
                priority,
                reward,
                status: TaskStatus::Pending,
                min_vram_mb,
                min_compute_score,
                verification_count: verification_count.max(1),
                submitted_at: current_block,
                deadline,
                assigned_to: None,
                assigned_at: None,
            };

            Tasks::<T>::insert(task_id, task);
            TotalTasksSubmitted::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::TaskSubmitted {
                task_id,
                submitter,
                workload_type,
                reward,
            });

            Ok(())
        }

        /// Contributor claims a pending task.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::claim_task())]
        pub fn claim_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&who).ok_or(Error::<T>::ContributorNotFound)?;

            let contributor =
                Contributors::<T>::get(contributor_id).ok_or(Error::<T>::ContributorNotFound)?;
            ensure!(
                contributor.status == ContributorStatus::Active,
                Error::<T>::ContributorNotActive
            );

            // Check active task limit
            let active = ContributorActiveTasks::<T>::get(contributor_id);
            if active >= T::MaxTasksPerContributor::get() {
                pallet_x3_invariants::Pallet::<T>::report_custom_invariant(
                    frame_system::Pallet::<T>::block_number(),
                    pallet_x3_invariants::InvariantKind::MaxTasksPerContributor,
                    active as u128,
                    T::MaxTasksPerContributor::get() as u128,
                );
            }
            ensure!(
                active < T::MaxTasksPerContributor::get(),
                Error::<T>::TooManyActiveTasks
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.status == TaskStatus::Pending,
                    Error::<T>::InvalidTaskStatus
                );

                // Check deadline
                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(
                    current_block < task.deadline,
                    Error::<T>::TaskDeadlinePassed
                );

                // Check contributor meets requirements
                ensure!(
                    contributor.capabilities.vram_mb >= task.min_vram_mb,
                    Error::<T>::RequirementsNotMet
                );
                ensure!(
                    contributor.capabilities.compute_score >= task.min_compute_score,
                    Error::<T>::RequirementsNotMet
                );

                task.status = TaskStatus::Assigned;
                task.assigned_to = Some(contributor_id);
                task.assigned_at = Some(current_block);

                ContributorActiveTasks::<T>::mutate(contributor_id, |n| *n = n.saturating_add(1));

                Self::deposit_event(Event::TaskClaimed {
                    task_id,
                    contributor_id,
                });

                Ok(())
            })
        }

        /// Contributor submits task execution result.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_result())]
        pub fn submit_result(
            origin: OriginFor<T>,
            task_id: TaskId,
            result_hash: H256,
            compute_units_used: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&who).ok_or(Error::<T>::ContributorNotFound)?;

            // Ensure no result already submitted
            ensure!(
                !TaskResults::<T>::contains_key(task_id),
                Error::<T>::ResultAlreadySubmitted
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.status == TaskStatus::Assigned,
                    Error::<T>::InvalidTaskStatus
                );
                ensure!(
                    task.assigned_to == Some(contributor_id),
                    Error::<T>::NotAssignedContributor
                );

                let current_block = frame_system::Pallet::<T>::block_number();

                // Store result
                let result = TaskResult {
                    task_id,
                    contributor_id,
                    executor: who.clone(),
                    result_hash,
                    compute_units_used,
                    submitted_at: current_block,
                };
                TaskResults::<T>::insert(task_id, result);

                // Move task to Verifying if verification is required, else complete directly
                if task.verification_count > 0 {
                    task.status = TaskStatus::Verifying;
                } else {
                    // No verification required - complete immediately
                    task.status = TaskStatus::Completed;
                    Self::do_distribute_reward(task, contributor_id)?;
                }

                Self::deposit_event(Event::ResultSubmitted {
                    task_id,
                    contributor_id,
                    result_hash,
                });

                Ok(())
            })
        }

        // ----------------------------------------------------------------
        // Jury Verification
        // ----------------------------------------------------------------

        /// Start a jury verification session for a task result.
        /// Can be called by anyone once a result is in Verifying status.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as Config>::WeightInfo::start_jury_session())]
        pub fn start_jury_session(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            ensure_signed(origin)?;

            let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(
                task.status == TaskStatus::Verifying,
                Error::<T>::InvalidTaskStatus
            );
            ensure!(
                !ActiveSessionByTask::<T>::contains_key(task_id),
                Error::<T>::JurySessionAlreadyExists
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let session_id: SessionId = BlakeTwo256::hash_of(&(&task_id, &current_block));

            let session = JurySession {
                id: session_id,
                task_id,
                phase: JuryPhase::Commit,
                commit_count: 0,
                reveal_count: 0,
                yes_votes: 0,
                no_votes: 0,
                started_at: current_block,
                commit_deadline: current_block.saturating_add(T::CommitPhaseDuration::get()),
                reveal_deadline: current_block
                    .saturating_add(T::CommitPhaseDuration::get())
                    .saturating_add(T::RevealPhaseDuration::get()),
            };

            JurySessions::<T>::insert(session_id, session);
            ActiveSessionByTask::<T>::insert(task_id, session_id);

            Self::deposit_event(Event::JurySessionStarted {
                session_id,
                task_id,
            });

            Ok(())
        }

        /// Submit a vote commitment: H(vote || nonce).
        #[pallet::call_index(8)]
        #[pallet::weight(<T as Config>::WeightInfo::commit_vote())]
        pub fn commit_vote(
            origin: OriginFor<T>,
            session_id: SessionId,
            commitment: H256,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            let contributor_id =
                AccountContributor::<T>::get(&voter).ok_or(Error::<T>::ContributorNotFound)?;
            let contributor =
                Contributors::<T>::get(contributor_id).ok_or(Error::<T>::ContributorNotFound)?;
            ensure!(
                contributor.status == ContributorStatus::Active,
                Error::<T>::ContributorNotActive
            );

            JurySessions::<T>::try_mutate(session_id, |maybe_session| -> DispatchResult {
                let session = maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                ensure!(
                    session.phase == JuryPhase::Commit,
                    Error::<T>::InvalidSessionPhase
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(
                    current_block <= session.commit_deadline,
                    Error::<T>::InvalidSessionPhase
                );

                // Check not already voted
                ensure!(
                    !VoteCommitments::<T>::contains_key(session_id, &voter),
                    Error::<T>::AlreadyVoted
                );

                // Check max voters
                ensure!(
                    session.commit_count < T::MaxJuryVoters::get(),
                    Error::<T>::MaxJuryVotersReached
                );

                let vote_commitment = VoteCommitment {
                    voter: voter.clone(),
                    commitment,
                    committed_at: current_block,
                };

                VoteCommitments::<T>::insert(session_id, &voter, vote_commitment);
                session.commit_count = session.commit_count.saturating_add(1);

                Self::deposit_event(Event::VoteCommitted { session_id, voter });

                Ok(())
            })
        }

        /// Advance session to reveal phase (anyone can call after commit deadline).
        #[pallet::call_index(9)]
        #[pallet::weight(<T as Config>::WeightInfo::commit_vote())]
        pub fn advance_to_reveal(origin: OriginFor<T>, session_id: SessionId) -> DispatchResult {
            ensure_signed(origin)?;

            JurySessions::<T>::try_mutate(session_id, |maybe_session| -> DispatchResult {
                let session = maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                ensure!(
                    session.phase == JuryPhase::Commit,
                    Error::<T>::InvalidSessionPhase
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(
                    current_block > session.commit_deadline,
                    Error::<T>::SessionNotReady
                );

                session.phase = JuryPhase::Reveal;

                Ok(())
            })
        }

        /// Reveal a vote with the original vote and nonce.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as Config>::WeightInfo::reveal_vote())]
        pub fn reveal_vote(
            origin: OriginFor<T>,
            session_id: SessionId,
            vote: bool,
            nonce: H256,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            JurySessions::<T>::try_mutate(session_id, |maybe_session| -> DispatchResult {
                let session = maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                ensure!(
                    session.phase == JuryPhase::Reveal,
                    Error::<T>::InvalidSessionPhase
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(
                    current_block <= session.reveal_deadline,
                    Error::<T>::InvalidSessionPhase
                );

                // Get the commitment
                let commitment = VoteCommitments::<T>::get(session_id, &voter)
                    .ok_or(Error::<T>::NoCommitmentFound)?;

                // Verify: H(vote_byte || nonce) == commitment
                let vote_byte: u8 = if vote { 1 } else { 0 };
                let mut preimage = [0u8; 33];
                preimage[0] = vote_byte;
                preimage[1..33].copy_from_slice(nonce.as_bytes());
                let computed: H256 = BlakeTwo256::hash(&preimage);
                ensure!(
                    computed == commitment.commitment,
                    Error::<T>::CommitmentMismatch
                );

                // Check not already revealed
                ensure!(
                    !VoteReveals::<T>::contains_key(session_id, &voter),
                    Error::<T>::AlreadyVoted
                );

                let reveal = VoteReveal {
                    voter: voter.clone(),
                    vote,
                    nonce,
                    revealed_at: current_block,
                };

                VoteReveals::<T>::insert(session_id, &voter, reveal);
                session.reveal_count = session.reveal_count.saturating_add(1);

                if vote {
                    session.yes_votes = session.yes_votes.saturating_add(1);
                } else {
                    session.no_votes = session.no_votes.saturating_add(1);
                }

                Self::deposit_event(Event::VoteRevealed {
                    session_id,
                    voter,
                    vote,
                });

                Ok(())
            })
        }

        /// Finalize a jury session after the reveal deadline.
        /// Distributes rewards or slashes based on the verdict.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as Config>::WeightInfo::finalize_session())]
        pub fn finalize_session(origin: OriginFor<T>, session_id: SessionId) -> DispatchResult {
            ensure_signed(origin)?;

            let session = JurySessions::<T>::get(session_id).ok_or(Error::<T>::SessionNotFound)?;

            ensure!(
                session.phase == JuryPhase::Reveal,
                Error::<T>::InvalidSessionPhase
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                current_block > session.reveal_deadline,
                Error::<T>::SessionNotReady
            );

            let task_id = session.task_id;
            let verdict_valid = session.yes_votes > session.no_votes && session.reveal_count > 0;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.status == TaskStatus::Verifying,
                    Error::<T>::InvalidTaskStatus
                );

                if verdict_valid {
                    // Task verified successfully
                    task.status = TaskStatus::Completed;

                    if let Some(contributor_id) = task.assigned_to {
                        Self::do_distribute_reward(task, contributor_id)?;

                        // Update contributor stats
                        Contributors::<T>::mutate(contributor_id, |maybe_c| {
                            if let Some(c) = maybe_c {
                                c.tasks_completed = c.tasks_completed.saturating_add(1);
                                c.reputation = c.reputation.saturating_add(1).min(200);
                            }
                        });

                        ContributorActiveTasks::<T>::mutate(contributor_id, |n| {
                            *n = n.saturating_sub(1)
                        });
                    }

                    TotalTasksCompleted::<T>::mutate(|n| *n = n.saturating_add(1));
                } else {
                    // Task failed verification
                    task.status = TaskStatus::Failed;

                    if let Some(contributor_id) = task.assigned_to {
                        // Slash contributor for invalid result
                        Self::do_slash(contributor_id, b"Invalid execution result")?;

                        Contributors::<T>::mutate(contributor_id, |maybe_c| {
                            if let Some(c) = maybe_c {
                                c.tasks_failed = c.tasks_failed.saturating_add(1);
                                c.reputation = c.reputation.saturating_sub(5);
                            }
                        });

                        ContributorActiveTasks::<T>::mutate(contributor_id, |n| {
                            *n = n.saturating_sub(1)
                        });
                    }

                    // Return reward to submitter
                    T::Currency::unreserve(&task.submitter, task.reward);

                    TotalTasksFailed::<T>::mutate(|n| *n = n.saturating_add(1));
                }

                Ok(())
            })?;

            // Close session
            JurySessions::<T>::mutate(session_id, |maybe_session| {
                if let Some(s) = maybe_session {
                    s.phase = JuryPhase::Closed;
                }
            });
            ActiveSessionByTask::<T>::remove(task_id);

            Self::deposit_event(Event::JurySessionFinalized {
                session_id,
                task_id,
                verdict_valid,
                yes_votes: session.yes_votes,
                no_votes: session.no_votes,
            });

            Ok(())
        }

        // ----------------------------------------------------------------
        // Task Cancellation / Timeout
        // ----------------------------------------------------------------

        /// Cancel a pending task. Only the submitter can cancel.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as Config>::WeightInfo::cancel_task())]
        pub fn cancel_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(task.submitter == who, Error::<T>::NotTaskSubmitter);
                ensure!(
                    task.status == TaskStatus::Pending,
                    Error::<T>::InvalidTaskStatus
                );

                task.status = TaskStatus::Cancelled;

                // Return reserved reward
                T::Currency::unreserve(&who, task.reward);

                Self::deposit_event(Event::TaskCancelled {
                    task_id,
                    submitter: who.clone(),
                });

                Ok(())
            })
        }

        /// Mark a task as timed out. Anyone can call after the deadline.
        #[pallet::call_index(13)]
        #[pallet::weight(<T as Config>::WeightInfo::cancel_task())]
        pub fn timeout_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.status == TaskStatus::Pending || task.status == TaskStatus::Assigned,
                    Error::<T>::InvalidTaskStatus
                );

                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(current_block > task.deadline, Error::<T>::TaskNotTimedOut);

                task.status = TaskStatus::TimedOut;

                // Return reward to submitter
                T::Currency::unreserve(&task.submitter, task.reward);

                // If assigned, decrement contributor's active tasks
                if let Some(contributor_id) = task.assigned_to {
                    ContributorActiveTasks::<T>::mutate(contributor_id, |n| {
                        *n = n.saturating_sub(1)
                    });
                }

                Self::deposit_event(Event::TaskTimedOut { task_id });

                Ok(())
            })
        }

        // ----------------------------------------------------------------
        // Admin
        // ----------------------------------------------------------------

        /// Slash a contributor (governance action).
        #[pallet::call_index(14)]
        #[pallet::weight(<T as Config>::WeightInfo::slash_contributor())]
        pub fn slash_contributor(
            origin: OriginFor<T>,
            contributor_id: ContributorId,
            reason: BoundedVec<u8, ConstU32<128>>,
        ) -> DispatchResult {
            T::SlashOrigin::ensure_origin(origin)?;
            Self::do_slash(contributor_id, &reason)?;
            Ok(())
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    impl<T: Config> Pallet<T> {
        /// Distribute reward to the contributor and protocol.
        fn do_distribute_reward(
            task: &SwarmTask<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
            contributor_id: ContributorId,
        ) -> DispatchResult {
            let contributor =
                Contributors::<T>::get(contributor_id).ok_or(Error::<T>::ContributorNotFound)?;

            let reward = task.reward;
            let contributor_pct = T::ContributorRewardPct::get() as u128;
            let protocol_pct = T::ProtocolFeePct::get() as u128;

            // Calculate contributor share
            // reward * contributor_pct / 100
            let reward_u128: u128 = reward.try_into().unwrap_or(0u128);
            let contributor_share_u128 = reward_u128
                .saturating_mul(contributor_pct)
                .checked_div(100)
                .unwrap_or(0);
            let protocol_fee_u128 = reward_u128
                .saturating_mul(protocol_pct)
                .checked_div(100)
                .unwrap_or(0);

            // Unreserve the full reward from submitter
            T::Currency::unreserve(&task.submitter, reward);

            // Transfer contributor share
            let contributor_share: BalanceOf<T> =
                contributor_share_u128.try_into().unwrap_or(Zero::zero());

            T::Currency::transfer(
                &task.submitter,
                &contributor.account,
                contributor_share,
                ExistenceRequirement::KeepAlive,
            )?;

            // Charge protocol fee from the reward budget.
            // Until a dedicated fee sink account is configured, burn the fee.
            let protocol_fee: BalanceOf<T> = protocol_fee_u128.try_into().unwrap_or(Zero::zero());
            if protocol_fee > Zero::zero() {
                let imbalance = T::Currency::withdraw(
                    &task.submitter,
                    protocol_fee,
                    WithdrawReasons::FEE,
                    ExistenceRequirement::KeepAlive,
                )?;
                drop(imbalance);
                Self::deposit_event(Event::ProtocolFeeCharged {
                    task_id: task.id,
                    amount: protocol_fee,
                });
            }

            Self::deposit_event(Event::RewardDistributed {
                task_id: task.id,
                contributor_id,
                amount: contributor_share,
            });

            Ok(())
        }

        /// Slash a contributor's stake.
        fn do_slash(contributor_id: ContributorId, reason: &[u8]) -> DispatchResult {
            Contributors::<T>::try_mutate(contributor_id, |maybe_c| -> DispatchResult {
                let c = maybe_c.as_mut().ok_or(Error::<T>::ContributorNotFound)?;

                let slash = T::SlashAmount::get().min(c.stake);

                // Slash from reserved balance (burned)
                let (_, _remaining) = T::Currency::slash_reserved(&c.account, slash);

                c.stake = c.stake.saturating_sub(slash);
                c.status = ContributorStatus::Slashed;
                c.reputation = c.reputation.saturating_sub(20);

                let bounded_reason: BoundedVec<u8, ConstU32<128>> =
                    reason.to_vec().try_into().unwrap_or_default();

                ActiveContributors::<T>::mutate(|n| *n = n.saturating_sub(1));

                Self::deposit_event(Event::ContributorSlashed {
                    contributor_id,
                    amount: slash,
                    reason: bounded_reason,
                });

                Ok(())
            })
        }

        /// Get swarm statistics.
        pub fn get_stats() -> SwarmStats {
            SwarmStats {
                total_contributors: TotalContributors::<T>::get(),
                active_contributors: ActiveContributors::<T>::get(),
                total_tasks_submitted: TotalTasksSubmitted::<T>::get(),
                total_tasks_completed: TotalTasksCompleted::<T>::get(),
                total_tasks_failed: TotalTasksFailed::<T>::get(),
                pending_tasks: 0, // Would need iteration to count
                active_jury_sessions: 0,
            }
        }

        /// Check agent capability budget and deduct amount if permitted.
        /// Returns BudgetExceeded if budget insufficient, or KillSwitchActive if agent is killed.
        pub fn check_and_deduct_capability(
            agent: &T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            AgentCapabilities::<T>::try_mutate(agent, |maybe_envelope| -> DispatchResult {
                let envelope = maybe_envelope
                    .as_mut()
                    .ok_or(Error::<T>::CapabilityRevoked)?;

                // Check kill-switch
                if envelope.killed {
                    Self::deposit_event(Event::CapabilityKillSwitchHit {
                        agent: agent.clone(),
                    });
                    Self::report_capability_invariant(
                        frame_system::Pallet::<T>::block_number(),
                        pallet_x3_invariants::InvariantKind::CapabilityKillSwitch,
                        1,
                        1,
                    );
                    ensure!(!envelope.killed, Error::<T>::KillSwitchActive);
                }

                // Check budget availability
                let available = envelope.budget.saturating_sub(envelope.spent);
                if available < amount {
                    Self::deposit_event(Event::CapabilityBudgetExceeded {
                        agent: agent.clone(),
                        amount,
                        available,
                    });
                    Self::report_capability_invariant(
                        frame_system::Pallet::<T>::block_number(),
                        pallet_x3_invariants::InvariantKind::CapabilityBudget,
                        amount.saturated_into::<u128>(),
                        available.saturated_into::<u128>(),
                    );
                    ensure!(available >= amount, Error::<T>::BudgetExceeded);
                }

                // Deduct from budget
                envelope.spent = envelope.spent.saturating_add(amount);
                let remaining = envelope.budget.saturating_sub(envelope.spent);
                Self::deposit_event(Event::CapabilityBudgetDeducted {
                    agent: agent.clone(),
                    amount,
                    remaining,
                });

                Ok(())
            })
        }

        fn report_capability_invariant(
            block: BlockNumberFor<T>,
            invariant: pallet_x3_invariants::InvariantKind,
            observed: u128,
            bound: u128,
        ) {
            pallet_x3_invariants::Pallet::<T>::report_custom_invariant(
                block, invariant, observed, bound,
            );
        }
    }
}
