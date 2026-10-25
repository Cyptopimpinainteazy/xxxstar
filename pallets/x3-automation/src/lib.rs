#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::if_not_else)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::empty_line_after_outer_attr)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::shadow_unrelated)]

//! # X3 Automation Pallet
//!
//! Keeper network and automated task execution for X3 Chain.
//!
//! A task is a `(condition, action, budget)` triple. The pallet's rules:
//!
//! * **A condition that cannot be evaluated is not met.** A missing oracle price
//!   returns `Error::PriceUnavailable`; it is never read as zero, which would
//!   silently satisfy every "below threshold" trigger.
//! * **An action is dispatched or it is not.** `Config::Dispatcher` performs the
//!   effect. The pallet records `Executed` only when the dispatcher returns
//!   `Ok`; there is no path that reports success without a dispatch.
//! * **A keeper is paid only for work that happened.** A successful dispatch
//!   pays the execution fee. A refused dispatch pays the attempt fee, capped by
//!   `max_fee` and by what the owner can actually cover. An undecidable
//!   condition pays nothing, because nothing ran.
//! * **Every successful attempt writes a receipt.** `ExecutionReceipt` is
//!   inserted in the same transaction as the effect it describes, keyed by
//!   `(task_id, attempt)`, so a receipt cannot outlive or precede its effect.
//!   A refused dispatch writes no receipt: the trail records effects, not
//!   intentions.

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use frame_support::{
        ensure,
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{SaturatedConversion, Saturating, Zero};
    use sp_std::vec::Vec;

    use x3_automation::{
        Action, ActionOutcome, Condition, ExecutionReceipt, Task, TaskId, TaskStatus,
    };

    /// Trait for querying asset prices from an oracle.
    ///
    /// `None` means "I cannot answer right now" and is always a refusal, never a
    /// zero price.
    pub trait OracleProvider {
        /// Returns the current price of `asset_id` scaled to a fixed-point
        /// representation (e.g. 1e18), or `None` if unavailable.
        fn get_price(asset_id: &[u8]) -> Option<u128>;
    }

    /// Trait for evaluating custom conditions defined by keepers or governance.
    pub trait CustomConditionEvaluator {
        /// Return true if the condition data holds at the current chain state.
        fn evaluate(data: &[u8; 64]) -> bool;
    }

    /// Performs a task's action for the task owner.
    ///
    /// Returning `Ok` is the only thing that makes the pallet mark a task
    /// `Executed` and pay the execution fee. Implementations must not report
    /// success for an effect they did not perform.
    pub trait ActionDispatcher<AccountId> {
        /// Run `action` on behalf of `owner`.
        ///
        /// Returns what the action did, the weight it actually consumed, and a
        /// 64-byte result the receipt carries.
        fn dispatch(
            owner: &AccountId,
            action: &Action,
        ) -> Result<(ActionOutcome, Weight, [u8; 64]), DispatchError>;
    }

    /// Default evaluator that refuses every custom condition.
    pub struct RefuseCustomEvaluator;
    impl CustomConditionEvaluator for RefuseCustomEvaluator {
        fn evaluate(_data: &[u8; 64]) -> bool {
            false
        }
    }

    /// Unavailable oracle that always returns `None` — safe default when no oracle
    /// is configured. Policy conditions depending on price will never trigger.
    pub struct UnavailableOracle;
    impl OracleProvider for UnavailableOracle {
        fn get_price(_asset_id: &[u8]) -> Option<u128> {
            None
        }
    }

    /// Dispatcher for a runtime that has wired no action backend.
    ///
    /// Refuses every action, which is the honest answer: a chain that has not
    /// decided how a task may move funds must not pretend the move happened.
    pub struct RefuseAllActions;
    impl<AccountId> ActionDispatcher<AccountId> for RefuseAllActions {
        fn dispatch(
            _owner: &AccountId,
            _action: &Action,
        ) -> Result<(ActionOutcome, Weight, [u8; 64]), DispatchError> {
            Err(DispatchError::Other("AutomationActionBackendUnavailable"))
        }
    }

    /// Balance type alias
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Currency type for fees
        type Currency: Currency<Self::AccountId>
            + frame_support::traits::ReservableCurrency<Self::AccountId>;

        /// Maximum tasks per account
        #[pallet::constant]
        type MaxTasksPerAccount: Get<u32>;

        /// Base fee for task registration
        #[pallet::constant]
        type BaseRegistrationFee: Get<BalanceOf<Self>>;

        /// Fee per task execution (paid to keepers)
        #[pallet::constant]
        type ExecutionFee: Get<BalanceOf<Self>>;

        /// Maximum task expiry blocks from now
        #[pallet::constant]
        type MaxTaskExpiryBlocks: Get<u32>;

        /// Attempts allowed before a failing task becomes terminal (`Failed`).
        ///
        /// `1` means a single try: the first dispatch error finishes the task.
        #[pallet::constant]
        type MaxAttempts: Get<u32>;

        /// Wall-clock time in milliseconds, supplied by the runtime.
        ///
        /// Injected rather than reading `pallet_timestamp` so this pallet does
        /// not constrain the runtime's timestamp configuration.
        type Now: Get<u64>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;

        /// Oracle provider for price-feed conditions.
        type Oracle: OracleProvider;

        /// Custom condition evaluator (default: RefuseCustomEvaluator).
        type CustomEvaluator: CustomConditionEvaluator;

        /// Executes task actions. Every effect a task can have goes through here.
        type Dispatcher: ActionDispatcher<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Automated tasks by id.
    #[pallet::storage]
    #[pallet::getter(fn tasks)]
    pub type Tasks<T: Config> =
        StorageMap<_, Blake2_128Concat, TaskId, Task<T::AccountId, BalanceOf<T>>, OptionQuery>;

    /// Task IDs per account
    #[pallet::storage]
    #[pallet::getter(fn account_tasks)]
    pub type AccountTasks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<TaskId, <T as Config>::MaxTasksPerAccount>,
        ValueQuery,
    >;

    /// Task execution counter
    #[pallet::storage]
    pub type TaskCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// One receipt per `(task_id, attempt)`.
    ///
    /// The key is the idempotency unit: an attempt number is consumed exactly
    /// once, so a replayed `execute_task` cannot double-write or double-pay.
    #[pallet::storage]
    #[pallet::getter(fn receipt)]
    pub type Receipts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        TaskId,
        Twox64Concat,
        u32,
        ExecutionReceipt,
        OptionQuery,
    >;

    /// Most recent attempt number per task, used to allocate the next one.
    #[pallet::storage]
    #[pallet::getter(fn last_attempt)]
    pub type LastAttempt<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new automated task was registered
        TaskRegistered {
            task_id: TaskId,
            owner: T::AccountId,
            condition: Condition,
        },
        /// A task's action ran and succeeded; the keeper was paid the execution fee.
        TaskExecuted {
            task_id: TaskId,
            attempt: u32,
            keeper: T::AccountId,
            fee_charged: BalanceOf<T>,
        },
        /// A task's action was refused or failed. The keeper is paid only the
        /// attempt fee, never the execution fee.
        TaskExecutionFailed {
            task_id: TaskId,
            attempt: u32,
            keeper: T::AccountId,
            error: DispatchError,
        },
        /// The task's condition could not be evaluated, so nothing ran and the
        /// keeper was paid nothing.
        TaskConditionUnavailable {
            task_id: TaskId,
            keeper: T::AccountId,
        },
        /// A task was cancelled
        TaskCancelled {
            task_id: TaskId,
            owner: T::AccountId,
        },
        /// A task expired
        TaskExpired { task_id: TaskId },
        /// A task reached `MaxAttempts` failures and is now terminal.
        TaskExhausted { task_id: TaskId },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Task ID already exists
        TaskAlreadyExists,
        /// Task not found
        TaskNotFound,
        /// Caller is not the task owner
        NotTaskOwner,
        /// Too many tasks for this account
        TooManyTasks,
        /// Task expiry too far in the future
        ExpiryTooFar,
        /// Insufficient balance for registration fee
        InsufficientBalance,
        /// The action was dispatched and refused, or the dispatch failed.
        TaskExecutionFailed,
        /// Task condition not met
        ConditionNotMet,
        /// Invalid task parameters
        InvalidTask,
        /// The oracle has no price for the asset, so the condition cannot be
        /// decided. Refused rather than read as zero.
        PriceUnavailable,
        /// The task already ran to a terminal state.
        TaskNotActive,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new automated task
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_task())]
        pub fn register_task(
            origin: OriginFor<T>,
            condition: Condition,
            action: Action,
            max_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let account_tasks = AccountTasks::<T>::get(&who);
            ensure!(
                account_tasks.len() < T::MaxTasksPerAccount::get() as usize,
                Error::<T>::TooManyTasks
            );

            Self::validate_task_parameters(&condition, &action)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            let max_expiry = current_block.saturating_add(T::MaxTaskExpiryBlocks::get().into());
            let expiry_block = match condition {
                Condition::BlockNumber(block) => block.min(max_expiry.saturated_into::<u64>()),
                _ => max_expiry.saturated_into::<u64>(),
            };
            ensure!(
                expiry_block > current_block.saturated_into::<u64>(),
                Error::<T>::ExpiryTooFar
            );

            let registration_fee = T::BaseRegistrationFee::get();
            ensure!(
                T::Currency::free_balance(&who) >= registration_fee,
                Error::<T>::InsufficientBalance
            );

            let task_counter = TaskCounter::<T>::get();
            let task_id = Self::generate_task_id(task_counter, &who);
            TaskCounter::<T>::put(task_counter.saturating_add(1));

            T::Currency::reserve(&who, registration_fee)?;

            let task = Task {
                id: task_id,
                owner: who.clone(),
                condition: condition.clone(),
                action,
                max_fee,
                expiry_block,
                status: TaskStatus::Active,
                attempts: 0,
            };

            Tasks::<T>::insert(task_id, task);
            AccountTasks::<T>::mutate(&who, |tasks| {
                tasks.try_push(task_id).ok();
            });

            Self::deposit_event(Event::TaskRegistered {
                task_id,
                owner: who,
                condition,
            });

            Ok(())
        }

        /// Cancel a task
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::cancel_task())]
        pub fn cancel_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(task.owner == who, Error::<T>::NotTaskOwner);
            ensure!(task.status == TaskStatus::Active, Error::<T>::TaskNotActive);

            Tasks::<T>::remove(task_id);
            Self::release_reservation(&task);

            Self::deposit_event(Event::TaskCancelled {
                task_id,
                owner: who,
            });

            Ok(())
        }

        /// Execute a task (called by keepers or the off-chain scheduler)
        ///
        /// Fails, changing nothing, when the condition is false or cannot be
        /// evaluated: nothing ran, so there is nothing to record.
        ///
        /// When the dispatcher refuses the action the call **succeeds** and
        /// records a failed attempt in `TaskExecutionFailed` plus the attempt
        /// counter. Returning an error there would revert the very record that
        /// proves the attempt was made. The keeper is paid the attempt fee only,
        /// never the execution fee, and `TaskExecuted` is emitted only for a
        /// dispatch that happened.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::execute_task())]
        pub fn execute_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let keeper = ensure_signed(origin)?;

            let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(task.status == TaskStatus::Active, Error::<T>::TaskNotActive);

            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
            if current_block >= task.expiry_block {
                Self::finish(&task, TaskStatus::Expired);
                Self::deposit_event(Event::TaskExpired { task_id });
                return Ok(());
            }

            // A condition we cannot evaluate is not a condition we satisfy.
            match Self::evaluate_condition(&task) {
                Ok(true) => {}
                Ok(false) => return Err(Error::<T>::ConditionNotMet.into()),
                Err(error) => {
                    Self::deposit_event(Event::TaskConditionUnavailable { task_id, keeper });
                    return Err(error);
                }
            }

            let attempt = LastAttempt::<T>::get(task_id).saturating_add(1);
            LastAttempt::<T>::insert(task_id, attempt);
            task.attempts = attempt;

            match T::Dispatcher::dispatch(&task.owner, &task.action) {
                Ok((outcome, weight, output)) => {
                    Receipts::<T>::insert(
                        task_id,
                        attempt,
                        ExecutionReceipt {
                            task_id,
                            attempt,
                            keeper_hash: sp_io::hashing::blake2_256(&keeper.encode()),
                            block: current_block,
                            outcome,
                            weight_ref_time: weight.ref_time(),
                            weight_proof_size: weight.proof_size(),
                            output,
                        },
                    );

                    Self::finish(&task, TaskStatus::Executed);

                    // Paid only now, and only for a dispatch that happened.
                    let fee = Self::pay_success_fee(&task, &keeper)?;
                    Self::deposit_event(Event::TaskExecuted {
                        task_id,
                        attempt,
                        keeper,
                        fee_charged: fee,
                    });

                    Ok(())
                }
                Err(error) => {
                    // The attempt really happened: the condition was evaluated
                    // and the dispatcher was called. This branch therefore
                    // returns `Ok` and records the failure, because returning
                    // `Err` would revert the attempt counter, the fee payment
                    // and the event along with it. A dispatch that did not
                    // happen must not be indistinguishable from one that was
                    // never recorded.
                    let exhausted = attempt >= T::MaxAttempts::get();
                    if exhausted {
                        Self::finish(&task, TaskStatus::Failed);
                    } else {
                        Tasks::<T>::insert(task_id, task.clone());
                    }

                    // The keeper did work, so it is paid the attempt fee —
                    // capped by `max_fee` and by what the owner holds — and
                    // never the execution fee.
                    let _paid = Self::pay_attempt_fee(&task, &keeper);

                    Self::deposit_event(Event::TaskExecutionFailed {
                        task_id,
                        attempt,
                        keeper,
                        error,
                    });
                    if exhausted {
                        Self::deposit_event(Event::TaskExhausted { task_id });
                    }

                    Ok(())
                }
            }
        }
    }

    impl<T: Config> Pallet<T> {
        /// Move a task to a terminal status and release the owner's reservation.
        fn finish(task: &Task<T::AccountId, BalanceOf<T>>, status: TaskStatus) {
            let mut finished = task.clone();
            finished.status = status;
            Tasks::<T>::insert(finished.id, finished);
            Self::release_reservation(task);
        }

        fn release_reservation(task: &Task<T::AccountId, BalanceOf<T>>) {
            AccountTasks::<T>::mutate(&task.owner, |tasks| {
                if let Some(pos) = tasks.iter().position(|&id| id == task.id) {
                    tasks.swap_remove(pos);
                }
            });
            let registration_fee = T::BaseRegistrationFee::get();
            T::Currency::unreserve(&task.owner, registration_fee);
        }

        /// Fee for a successful dispatch, bounded by the task's budget.
        fn pay_success_fee(
            task: &Task<T::AccountId, BalanceOf<T>>,
            keeper: &T::AccountId,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let fee = T::ExecutionFee::get().min(task.max_fee);
            T::Currency::transfer(
                &task.owner,
                keeper,
                fee,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            Ok(fee)
        }

        /// Fee for a refused dispatch: paid, but never more than the owner holds
        /// or the task budgeted. Returns whether anything was paid.
        fn pay_attempt_fee(task: &Task<T::AccountId, BalanceOf<T>>, keeper: &T::AccountId) -> bool {
            let fee = T::ExecutionFee::get().min(task.max_fee);
            if fee.is_zero() || T::Currency::free_balance(&task.owner) < fee {
                return false;
            }
            T::Currency::transfer(
                &task.owner,
                keeper,
                fee,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )
            .is_ok()
        }

        /// Generate a unique task ID
        fn generate_task_id(counter: u64, account: &T::AccountId) -> TaskId {
            let mut data = Vec::new();
            data.extend_from_slice(&counter.to_le_bytes());
            data.extend_from_slice(&account.encode());
            H256::from(sp_io::hashing::blake2_256(&data))
        }

        /// Validate task parameters
        fn validate_task_parameters(condition: &Condition, action: &Action) -> DispatchResult {
            match condition {
                Condition::BlockNumber(block) => {
                    let current_block =
                        frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
                    ensure!(*block > current_block, Error::<T>::InvalidTask);
                }
                Condition::Timestamp(at) => {
                    ensure!(*at > 0, Error::<T>::InvalidTask);
                }
                Condition::PriceThreshold { asset_id, .. } => {
                    ensure!(*asset_id > 0, Error::<T>::InvalidTask);
                }
                Condition::PriceBand {
                    asset_id,
                    lower,
                    upper,
                } => {
                    ensure!(*asset_id > 0, Error::<T>::InvalidTask);
                    // An inverted band can never fire; refuse it up front rather
                    // than storing a task that is dead on arrival.
                    ensure!(lower <= upper, Error::<T>::InvalidTask);
                }
                // Matching an unbounded System::Events list in consensus execution
                // can exhaust PoV. No bounded, indexed event source exists yet.
                // Reject registration rather than charge for a task that cannot fire.
                Condition::Event { .. } => return Err(Error::<T>::InvalidTask.into()),
                Condition::Manual => {}
                Condition::Custom(data) => {
                    ensure!(!data.is_empty(), Error::<T>::InvalidTask);
                }
            }

            match action {
                Action::TransferNative { amount, .. }
                | Action::TransferLedgerAsset { amount, .. } => {
                    ensure!(*amount > 0, Error::<T>::InvalidTask);
                }
                Action::NoChange => {}
                Action::Custom(data) => {
                    ensure!(!data.is_empty(), Error::<T>::InvalidTask);
                }
            }

            Ok(())
        }

        /// Evaluate a task's condition against current chain state.
        ///
        /// `Err(PriceUnavailable)` is returned when a price trigger cannot be
        /// decided. It is never collapsed into `Ok(false)`, because the caller
        /// must distinguish "not yet" from "we do not know".
        fn evaluate_condition(
            task: &Task<T::AccountId, BalanceOf<T>>,
        ) -> Result<bool, DispatchError> {
            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
            Ok(match &task.condition {
                Condition::BlockNumber(target_block) => current_block >= *target_block,
                Condition::Timestamp(at) => T::Now::get() >= *at,
                Condition::PriceThreshold {
                    asset_id,
                    threshold,
                    above,
                } => {
                    let price = Self::oracle_price(*asset_id)?;
                    x3_automation::conditions::check_price_condition(
                        *asset_id,
                        *threshold,
                        *above,
                        Some(price),
                    )
                }
                Condition::PriceBand {
                    asset_id,
                    lower,
                    upper,
                } => {
                    let price = Self::oracle_price(*asset_id)?;
                    x3_automation::conditions::check_price_band(*lower, *upper, price)
                }
                // Legacy event tasks must also fail closed at execution, never
                // scan an unbounded block event list or pretend the event fired.
                Condition::Event { .. } => {
                    return Err(DispatchError::Other("EventConditionUnsupported"));
                },
                Condition::Manual => true,
                Condition::Custom(data) => T::CustomEvaluator::evaluate(data),
            })
        }

        /// Price for `asset_id`, or `PriceUnavailable`.
        ///
        /// The `unwrap_or(0)` this replaces made every "below threshold" trigger
        /// fire on an outage; failing closed is the only safe reading.
        fn oracle_price(asset_id: u32) -> Result<u64, DispatchError> {
            let key = asset_id.to_le_bytes();
            let price = T::Oracle::get_price(&key).ok_or(Error::<T>::PriceUnavailable)?;
            u64::try_from(price).map_err(|_| DispatchError::from(Error::<T>::PriceUnavailable))
        }

        /// Clean up expired tasks (called by governance or a keeper).
        ///
        /// Not run in `on_initialize`: it iterates every task, and paying that
        /// cost on every block would make the pallet's base weight unbounded.
        pub fn cleanup_expired_tasks() -> u32 {
            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
            let mut cleaned = 0u32;

            let expired: Vec<TaskId> = Tasks::<T>::iter()
                .filter(|(_id, task)| {
                    task.expiry_block <= current_block && task.status == TaskStatus::Active
                })
                .map(|(task_id, _)| task_id)
                .collect();

            for task_id in expired {
                if let Some(task) = Tasks::<T>::get(task_id) {
                    if task.status == TaskStatus::Active {
                        Self::finish(&task, TaskStatus::Expired);
                        Self::deposit_event(Event::TaskExpired { task_id });
                        cleaned = cleaned.saturating_add(1);
                    }
                }
            }

            cleaned
        }

        /// Active tasks whose time-based trigger may already be met, capped at
        /// `max_batch`.
        ///
        /// This is a bounded *scan*, not an index: a production keeper should
        /// follow `TaskRegistered` events and keep its own schedule, because the
        /// cost here is O(tasks). It exists so a keeper can bootstrap without an
        /// indexer, and so tests can assert the due set directly.
        pub fn due_tasks(block: u64, max_batch: u32) -> Vec<TaskId> {
            Tasks::<T>::iter()
                .filter(|(_id, task)| task.status == TaskStatus::Active)
                .filter(|(_id, task)| match &task.condition {
                    Condition::BlockNumber(target) => block >= *target,
                    Condition::Manual => true,
                    _ => false,
                })
                .map(|(task_id, _)| task_id)
                .take(max_batch as usize)
                .collect()
        }
    }
}
