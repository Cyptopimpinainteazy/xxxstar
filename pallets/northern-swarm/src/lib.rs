//! # Northern Swarm Pallet (RC2)
//!
//! On-chain registry for the Northern Swarm off-chain executor network.
//!
//! This pallet supersedes the deprecated `pallet-swarm`.  Its scope is
//! intentionally minimal: it owns stake, hardware profiles, task assignment,
//! result-hash commits, and the slash/reward accounting.  All heavy computation
//! lives off-chain in `crates/northern-swarm`.
//!
//! ## Storage overview
//!
//! | Storage           | Type                                 | Description                        |
//! |-------------------|--------------------------------------|------------------------------------|
//! | `Executors`       | `Map<AccountId, ExecutorRecord>`     | Registered executor profiles       |
//! | `Tasks`           | `Map<TaskId, TaskRecord>`            | On-chain task registry             |
//! | `ResultCommits`   | `Map<(TaskId, AccountId), H256>`     | Result hash commits per executor   |
//! | `Config`          | `StorageValue<SwarmConfig>`          | Tunable parameters (via governance)|
//!
//! ## Extrinsics
//!
//! | Call                  | Who        | Description                              |
//! |-----------------------|------------|------------------------------------------|
//! | `register_executor`   | Any        | Lock stake + publish hardware profile    |
//! | `deregister_executor` | Self       | Unlock stake (cooldown enforced)         |
//! | `submit_heartbeat`    | Executor   | Prove liveness; resets slash timer       |
//! | `submit_task`         | Any        | Post a new task; locks task bond         |
//! | `claim_task`          | Executor   | Claim exclusive execution rights        |
//! | `submit_result`       | Executor   | Commit result hash for claimed task      |
//! | `slash_executor`      | Root/sudo  | Slash a misbehaving executor             |

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Hash, Zero};

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // -----------------------------------------------------------------------
    // Pallet config
    // -----------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for stake reservation and slash transfers.
        type Currency: ReservableCurrency<Self::AccountId>
            + LockableCurrency<Self::AccountId>
            + Currency<Self::AccountId>;

        /// Minimum stake required to register as an executor.
        #[pallet::constant]
        type MinExecutorStake: Get<BalanceOf<Self>>;

        /// Number of blocks an executor must wait after deregistering before
        /// their stake is released (prevents stake-withdraw-then-slash evasion).
        #[pallet::constant]
        type DeregistrationCooldown: Get<BlockNumberFor<Self>>;

        /// Maximum number of concurrent open tasks per executor.
        #[pallet::constant]
        type MaxClaimedTasksPerExecutor: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Registered executors: AccountId → ExecutorRecord.
    #[pallet::storage]
    #[pallet::getter(fn executors)]
    pub type Executors<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ExecutorRecord<BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// On-chain task registry: TaskId (H256) → TaskRecord.
    #[pallet::storage]
    #[pallet::getter(fn tasks)]
    pub type Tasks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        TaskRecord<T::AccountId, BalanceOf<T>, BlockNumberFor<T>, T::Hash>,
        OptionQuery,
    >;

    /// Result hash commits: (TaskId, ExecutorId) → result_hash (H256).
    ///
    /// Multiple executors commit to enable the RC3 quorum comparison.
    #[pallet::storage]
    #[pallet::getter(fn result_commits)]
    pub type ResultCommits<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,         // task_id
        Blake2_128Concat,
        T::AccountId,    // executor_id
        T::Hash,         // result_hash
        OptionQuery,
    >;

    /// Number of tasks claimed per executor (enforces MaxClaimedTasksPerExecutor).
    #[pallet::storage]
    pub type ClaimedTaskCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new executor registered with the given stake amount.
        ExecutorRegistered {
            executor: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// An executor initiated deregistration (stake still locked for cooldown).
        ExecutorDeregistering {
            executor: T::AccountId,
            unlock_at: BlockNumberFor<T>,
        },
        /// An executor's stake was unlocked after the cooldown expired.
        ExecutorStakeUnlocked { executor: T::AccountId },
        /// A new task was posted on-chain.
        TaskSubmitted {
            task_id: T::Hash,
            submitter: T::AccountId,
        },
        /// An executor claimed exclusive execution rights for a task.
        TaskClaimed {
            task_id: T::Hash,
            executor: T::AccountId,
        },
        /// An executor committed a result hash for a claimed task.
        ResultCommitted {
            task_id: T::Hash,
            executor: T::AccountId,
            result_hash: T::Hash,
        },
        /// A task was finalised with an accepted result hash.
        TaskFinalised {
            task_id: T::Hash,
            winning_hash: T::Hash,
        },
        /// An executor was slashed for misbehaviour.
        ExecutorSlashed {
            executor: T::AccountId,
            amount: BalanceOf<T>,
            reason: SlashReason,
        },
        /// An executor was rewarded for successful task completion.
        ExecutorRewarded {
            executor: T::AccountId,
            task_id: T::Hash,
            amount: BalanceOf<T>,
        },
        /// An executor submitted a heartbeat, resetting their liveness timer.
        HeartbeatReceived {
            executor: T::AccountId,
            block: BlockNumberFor<T>,
        },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// Executor is already registered.
        AlreadyRegistered,
        /// Executor is not registered.
        NotRegistered,
        /// Provided stake is below the minimum requirement.
        InsufficientStake,
        /// Task with this ID does not exist.
        TaskNotFound,
        /// Task is not in a claimable state.
        TaskNotClaimable,
        /// Task has already been claimed by another executor.
        TaskAlreadyClaimed,
        /// Caller did not claim this task.
        NotTaskExecutor,
        /// Executor has reached the maximum number of concurrent claimed tasks.
        TooManyClaimedTasks,
        /// A result for this task has already been committed by this executor.
        ResultAlreadyCommitted,
        /// Deregistration cooldown has not expired yet.
        CooldownNotExpired,
        /// Executor has active claimed tasks; deregister after releasing them.
        HasActiveTasks,
    }

    // -----------------------------------------------------------------------
    // Pallet struct
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -----------------------------------------------------------------------
    // Hooks
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            // RC3 hook placeholder: run quorum comparison and finalise tasks
            // whose all claimants have committed.
            let _ = block; // suppress unused warning until RC3
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as an executor by locking `stake` in reserve.
        ///
        /// Emits [`Event::ExecutorRegistered`].
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn register_executor(
            origin: OriginFor<T>,
            stake: BalanceOf<T>,
            hardware: HardwareProfile,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(!Executors::<T>::contains_key(&who), Error::<T>::AlreadyRegistered);
            ensure!(stake >= T::MinExecutorStake::get(), Error::<T>::InsufficientStake);

            T::Currency::reserve(&who, stake)?;

            let record = ExecutorRecord {
                stake,
                hardware,
                reputation: 0,
                status: ExecutorStatus::Active,
                last_heartbeat: frame_system::Pallet::<T>::block_number(),
                deregistering_at: None,
            };
            Executors::<T>::insert(&who, record);

            Self::deposit_event(Event::ExecutorRegistered {
                executor: who,
                stake,
            });
            Ok(())
        }

        /// Initiate deregistration.  Stake remains locked for
        /// [`Config::DeregistrationCooldown`] blocks.
        ///
        /// Fails if the executor has uncompleted claimed tasks.
        ///
        /// Emits [`Event::ExecutorDeregistering`].
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn deregister_executor(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut record =
                Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;

            ensure!(
                ClaimedTaskCount::<T>::get(&who) == 0,
                Error::<T>::HasActiveTasks,
            );

            let unlock_at = frame_system::Pallet::<T>::block_number()
                + T::DeregistrationCooldown::get();
            record.status = ExecutorStatus::Deregistering;
            record.deregistering_at = Some(unlock_at);
            Executors::<T>::insert(&who, &record);

            Self::deposit_event(Event::ExecutorDeregistering {
                executor: who,
                unlock_at,
            });
            Ok(())
        }

        /// Finalise deregistration and release reserved stake after cooldown.
        ///
        /// Emits [`Event::ExecutorStakeUnlocked`].
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn release_stake(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let record = Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;
            let unlock_at =
                record.deregistering_at.ok_or(Error::<T>::CooldownNotExpired)?;

            ensure!(
                frame_system::Pallet::<T>::block_number() >= unlock_at,
                Error::<T>::CooldownNotExpired,
            );

            T::Currency::unreserve(&who, record.stake);
            Executors::<T>::remove(&who);

            Self::deposit_event(Event::ExecutorStakeUnlocked { executor: who });
            Ok(())
        }

        /// Submit a heartbeat to prove liveness and reset the slash timer.
        ///
        /// Emits [`Event::HeartbeatReceived`].
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn submit_heartbeat(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut record =
                Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;

            let now = frame_system::Pallet::<T>::block_number();
            record.last_heartbeat = now;
            Executors::<T>::insert(&who, &record);

            Self::deposit_event(Event::HeartbeatReceived {
                executor: who,
                block: now,
            });
            Ok(())
        }

        /// Post a new task on-chain.  The `payload_uri` is a content-addressable
        /// reference (e.g. `ipfs://<CID>`) from which executors will fetch the
        /// job body.
        ///
        /// Emits [`Event::TaskSubmitted`].
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn submit_task(
            origin: OriginFor<T>,
            payload_uri: BoundedVec<u8, ConstU32<512>>,
            reward: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Derive deterministic task ID from (submitter, payload_uri, block).
            let block = frame_system::Pallet::<T>::block_number();
            let task_id = T::Hashing::hash_of(&(&who, &payload_uri, block));

            T::Currency::reserve(&who, reward)?;

            let record: TaskRecord<T::AccountId, BalanceOf<T>, BlockNumberFor<T>, T::Hash> =
                TaskRecord {
                    submitter: who.clone(),
                    payload_uri,
                    reward,
                    status: TaskStatus::Pending,
                    claimed_by: None,
                    submitted_at: block,
                    result_hash: None,
                };
            Tasks::<T>::insert(task_id, record);

            Self::deposit_event(Event::TaskSubmitted {
                task_id,
                submitter: who,
            });
            Ok(())
        }

        /// Claim exclusive execution rights for a pending task.
        ///
        /// Emits [`Event::TaskClaimed`].
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn claim_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(Executors::<T>::contains_key(&who), Error::<T>::NotRegistered);

            let count = ClaimedTaskCount::<T>::get(&who);
            ensure!(
                count < T::MaxClaimedTasksPerExecutor::get(),
                Error::<T>::TooManyClaimedTasks,
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(task.status == TaskStatus::Pending, Error::<T>::TaskNotClaimable);
                ensure!(task.claimed_by.is_none(), Error::<T>::TaskAlreadyClaimed);
                task.status = TaskStatus::Claimed;
                task.claimed_by = Some(who.clone());
                Ok(())
            })?;

            ClaimedTaskCount::<T>::mutate(&who, |c| *c += 1);

            Self::deposit_event(Event::TaskClaimed {
                task_id,
                executor: who,
            });
            Ok(())
        }

        /// Commit the result hash for a claimed task.
        ///
        /// The off-chain executor supplies a SHA-256 hash of its output.  In RC3
        /// this will be compared against other executors' commits to determine the
        /// quorum winner.
        ///
        /// Emits [`Event::ResultCommitted`].
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn submit_result(
            origin: OriginFor<T>,
            task_id: T::Hash,
            result_hash: T::Hash,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(Executors::<T>::contains_key(&who), Error::<T>::NotRegistered);

            ensure!(
                !ResultCommits::<T>::contains_key(task_id, &who),
                Error::<T>::ResultAlreadyCommitted,
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.claimed_by.as_ref() == Some(&who),
                    Error::<T>::NotTaskExecutor,
                );
                // RC1/RC2: single executor path — accept immediately.
                // RC3: store commit and defer to quorum finalisation in on_finalize.
                task.status = TaskStatus::ResultCommitted;
                task.result_hash = Some(result_hash);
                Ok(())
            })?;

            ResultCommits::<T>::insert(task_id, &who, result_hash);
            ClaimedTaskCount::<T>::mutate(&who, |c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ResultCommitted {
                task_id,
                executor: who,
                result_hash,
            });
            Ok(())
        }

        /// Slash a misbehaving executor.  Restricted to Root origin (governance).
        ///
        /// Emits [`Event::ExecutorSlashed`].
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn slash_executor(
            origin: OriginFor<T>,
            executor: T::AccountId,
            amount: BalanceOf<T>,
            reason: SlashReason,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut record =
                Executors::<T>::get(&executor).ok_or(Error::<T>::NotRegistered)?;

            let slash = amount.min(record.stake);
            let (slashed, _) = T::Currency::slash_reserved(&executor, slash);
            record.stake = record.stake.saturating_sub(slashed);

            if record.stake < T::MinExecutorStake::get() {
                record.status = ExecutorStatus::Suspended;
            }
            Executors::<T>::insert(&executor, &record);

            Self::deposit_event(Event::ExecutorSlashed {
                executor,
                amount: slashed,
                reason,
            });
            Ok(())
        }
    }
}
