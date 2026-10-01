#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::type_complexity)]
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
//! | `resolve_disputed_task` | Any      | Refund a disputed task's reserved reward |
//! | `slash_executor`      | Root/sudo  | Slash a misbehaving executor             |

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{BalanceStatus, Currency, LockableCurrency, ReservableCurrency},
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Hash, Saturating, Zero};
    use sp_std::vec::Vec;

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

        /// Number of matching independent result commits required to finalise.
        #[pallet::constant]
        type QuorumThreshold: Get<u32>;

        /// Maximum distinct executors that may claim one task.
        #[pallet::constant]
        type MaxExecutorsPerTask: Get<u32>;

        /// Runtime weight provider.
        type WeightInfo: WeightInfo;
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
        T::Hash, // task_id
        Blake2_128Concat,
        T::AccountId, // executor_id
        T::Hash,      // result_hash
        OptionQuery,
    >;

    /// Active task claims. Multiple executors may claim the same task so a
    /// deterministic M-of-N result quorum can form.
    #[pallet::storage]
    #[pallet::getter(fn task_claims)]
    pub type TaskClaims<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        (),
        OptionQuery,
    >;

    /// Number of distinct claim slots consumed for a task.
    #[pallet::storage]
    #[pallet::getter(fn task_claim_slots)]
    pub type TaskClaimSlots<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, u32, ValueQuery>;

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
        /// Every available result slot was consumed without a matching quorum.
        TaskDisputed { task_id: T::Hash },
        /// A disputed task's reserved reward was returned to its submitter.
        DisputedTaskRefunded {
            task_id: T::Hash,
            submitter: T::AccountId,
            amount: BalanceOf<T>,
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
        /// Executor exists but is not in Active status.
        ExecutorNotActive,
        /// Executor already holds a claim for this task.
        TaskAlreadyClaimedByExecutor,
        /// Task has reached its configured maximum number of executor claims.
        TaskClaimLimitReached,
        /// Runtime quorum configuration is internally inconsistent.
        InvalidQuorumConfig,
        /// Reserved reward could not be moved completely to a winning executor.
        RewardSettlementFailed,
        /// Task is not in a disputed state, so it has no reward to refund.
        TaskNotDisputed,
    }

    // -----------------------------------------------------------------------
    // Pallet struct
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as an executor by locking `stake` in reserve.
        ///
        /// Emits [`Event::ExecutorRegistered`].
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_executor())]
        pub fn register_executor(
            origin: OriginFor<T>,
            stake: BalanceOf<T>,
            hardware: HardwareProfile,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !Executors::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
            ensure!(
                stake >= T::MinExecutorStake::get(),
                Error::<T>::InsufficientStake
            );

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
        #[pallet::weight(T::WeightInfo::deregister_executor())]
        pub fn deregister_executor(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut record = Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;

            ensure!(
                ClaimedTaskCount::<T>::get(&who) == 0,
                Error::<T>::HasActiveTasks,
            );

            let unlock_at =
                frame_system::Pallet::<T>::block_number() + T::DeregistrationCooldown::get();
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
        #[pallet::weight(T::WeightInfo::release_stake())]
        pub fn release_stake(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let record = Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;
            let unlock_at = record
                .deregistering_at
                .ok_or(Error::<T>::CooldownNotExpired)?;

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
        #[pallet::weight(T::WeightInfo::submit_heartbeat())]
        pub fn submit_heartbeat(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut record = Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;

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
        #[pallet::weight(T::WeightInfo::submit_task())]
        pub fn submit_task(
            origin: OriginFor<T>,
            payload_uri: BoundedVec<u8, ConstU32<512>>,
            reward: BalanceOf<T>,
            kind: TaskKind,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let quorum = T::QuorumThreshold::get();
            let max_executors = T::MaxExecutorsPerTask::get();
            ensure!(
                quorum >= 2 && quorum <= max_executors,
                Error::<T>::InvalidQuorumConfig,
            );

            // Include workload kind in the ID so two otherwise-identical jobs
            // cannot alias while requiring different execution semantics.
            let block = frame_system::Pallet::<T>::block_number();
            let task_id = T::Hashing::hash_of(&(&who, &payload_uri, &kind, block));

            T::Currency::reserve(&who, reward)?;

            let record: TaskRecord<T::AccountId, BalanceOf<T>, BlockNumberFor<T>, T::Hash> =
                TaskRecord {
                    submitter: who.clone(),
                    payload_uri,
                    reward,
                    kind,
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
        #[pallet::weight(T::WeightInfo::claim_task())]
        pub fn claim_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let executor = Executors::<T>::get(&who).ok_or(Error::<T>::NotRegistered)?;
            ensure!(
                executor.status == ExecutorStatus::Active,
                Error::<T>::ExecutorNotActive,
            );
            ensure!(
                !TaskClaims::<T>::contains_key(task_id, &who),
                Error::<T>::TaskAlreadyClaimedByExecutor,
            );

            let count = ClaimedTaskCount::<T>::get(&who);
            ensure!(
                count < T::MaxClaimedTasksPerExecutor::get(),
                Error::<T>::TooManyClaimedTasks,
            );

            let slots = TaskClaimSlots::<T>::get(task_id);
            ensure!(
                slots < T::MaxExecutorsPerTask::get(),
                Error::<T>::TaskClaimLimitReached,
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    matches!(
                        task.status,
                        TaskStatus::Pending | TaskStatus::Claimed | TaskStatus::ResultCommitted
                    ),
                    Error::<T>::TaskNotClaimable,
                );
                task.status = TaskStatus::Claimed;
                if task.claimed_by.is_none() {
                    task.claimed_by = Some(who.clone());
                }
                Ok(())
            })?;

            TaskClaims::<T>::insert(task_id, &who, ());
            TaskClaimSlots::<T>::insert(task_id, slots.saturating_add(1));
            ClaimedTaskCount::<T>::mutate(&who, |c| *c = c.saturating_add(1));

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
        #[pallet::weight(T::WeightInfo::submit_result())]
        #[transactional]
        pub fn submit_result(
            origin: OriginFor<T>,
            task_id: T::Hash,
            result_hash: T::Hash,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                Executors::<T>::contains_key(&who),
                Error::<T>::NotRegistered
            );
            ensure!(
                TaskClaims::<T>::contains_key(task_id, &who),
                Error::<T>::NotTaskExecutor,
            );
            ensure!(
                !ResultCommits::<T>::contains_key(task_id, &who),
                Error::<T>::ResultAlreadyCommitted,
            );

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    matches!(
                        task.status,
                        TaskStatus::Claimed | TaskStatus::ResultCommitted
                    ),
                    Error::<T>::TaskNotClaimable,
                );
                task.status = TaskStatus::ResultCommitted;
                Ok(())
            })?;

            ResultCommits::<T>::insert(task_id, &who, result_hash);
            TaskClaims::<T>::remove(task_id, &who);
            ClaimedTaskCount::<T>::mutate(&who, |c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ResultCommitted {
                task_id,
                executor: who,
                result_hash,
            });

            let winners: Vec<T::AccountId> = ResultCommits::<T>::iter_prefix(task_id)
                .filter_map(|(executor, hash)| (hash == result_hash).then_some(executor))
                .collect();

            if (winners.len() as u32) >= T::QuorumThreshold::get() {
                Self::finalise_with_quorum(task_id, result_hash, winners)?;
                return Ok(());
            }

            let commit_count = ResultCommits::<T>::iter_prefix(task_id).count() as u32;
            if commit_count >= T::MaxExecutorsPerTask::get() {
                Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                    let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                    task.status = TaskStatus::Disputed;
                    task.result_hash = None;
                    Ok(())
                })?;
                Self::clear_remaining_claims(task_id);
                Self::deposit_event(Event::TaskDisputed { task_id });
            }

            Ok(())
        }

        /// Resolve a disputed task by returning the submitter's reserved reward.
        ///
        /// When a task fills every executor slot with non-matching result
        /// hashes, no witness can be shown wrong from hashes alone, so the
        /// executors' stakes stay intact — but the submitter's bond must not be
        /// stranded in reserve forever. This refunds the full reserved reward
        /// and moves the task to [`TaskStatus::Refunded`].
        ///
        /// Permissionless by design: the call can only ever move the
        /// submitter's own reserved funds back to the submitter, so gating it on
        /// governance would itself be a way to strand funds. A task can only be
        /// refunded once; the state transition to `Refunded` makes a second
        /// attempt fail with [`Error::TaskNotDisputed`].
        ///
        /// Emits [`Event::DisputedTaskRefunded`].
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::resolve_disputed_task())]
        pub fn resolve_disputed_task(origin: OriginFor<T>, task_id: T::Hash) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(
                task.status == TaskStatus::Disputed,
                Error::<T>::TaskNotDisputed,
            );

            // No quorum ever formed, so `task.reward` is exactly what is still
            // reserved for this task. Refund it in full and zero the record so a
            // replayed call cannot move anything even if the status guard is
            // ever weakened.
            let refund = task.reward;
            task.reward = Zero::zero();
            task.status = TaskStatus::Refunded;
            let submitter = task.submitter.clone();
            Tasks::<T>::insert(task_id, &task);

            if !refund.is_zero() {
                T::Currency::unreserve(&submitter, refund);
            }

            Self::deposit_event(Event::DisputedTaskRefunded {
                task_id,
                submitter,
                amount: refund,
            });
            Ok(())
        }

        /// Slash a misbehaving executor.  Restricted to Root origin (governance).
        ///
        /// Emits [`Event::ExecutorSlashed`].
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::slash_executor())]
        pub fn slash_executor(
            origin: OriginFor<T>,
            executor: T::AccountId,
            amount: BalanceOf<T>,
            reason: SlashReason,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut record = Executors::<T>::get(&executor).ok_or(Error::<T>::NotRegistered)?;

            let slash = amount.min(record.stake);
            let (_imbalance, remaining) = T::Currency::slash_reserved(&executor, slash);
            let slashed = slash.saturating_sub(remaining);
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

    impl<T: Config> Pallet<T> {
        fn finalise_with_quorum(
            task_id: T::Hash,
            winning_hash: T::Hash,
            winners: Vec<T::AccountId>,
        ) -> DispatchResult {
            let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            let winner_count = winners.len() as u32;
            ensure!(
                winner_count >= T::QuorumThreshold::get(),
                Error::<T>::InvalidQuorumConfig,
            );

            // Balance is an unsigned arithmetic type under Currency. Convert the
            // bounded winner count and split the total task bounty exactly once.
            let divisor: BalanceOf<T> = winner_count.into();
            let share = task.reward / divisor;
            let mut paid: BalanceOf<T> = Zero::zero();

            for executor in winners.iter() {
                let remaining = T::Currency::repatriate_reserved(
                    &task.submitter,
                    executor,
                    share,
                    BalanceStatus::Free,
                )?;
                ensure!(remaining.is_zero(), Error::<T>::RewardSettlementFailed);
                paid = paid.saturating_add(share);
                Self::deposit_event(Event::ExecutorRewarded {
                    executor: executor.clone(),
                    task_id,
                    amount: share,
                });
            }

            let refund = task.reward.saturating_sub(paid);
            if !refund.is_zero() {
                T::Currency::unreserve(&task.submitter, refund);
            }

            task.status = TaskStatus::Finalised;
            task.result_hash = Some(winning_hash);
            Tasks::<T>::insert(task_id, &task);
            Self::clear_remaining_claims(task_id);

            Self::deposit_event(Event::TaskFinalised {
                task_id,
                winning_hash,
            });
            Ok(())
        }

        fn clear_remaining_claims(task_id: T::Hash) {
            let claimants: Vec<T::AccountId> = TaskClaims::<T>::iter_prefix(task_id)
                .map(|(account, ())| account)
                .collect();
            for account in claimants {
                TaskClaims::<T>::remove(task_id, &account);
                ClaimedTaskCount::<T>::mutate(&account, |c| *c = c.saturating_sub(1));
            }
            TaskClaimSlots::<T>::remove(task_id);
        }
    }
}
