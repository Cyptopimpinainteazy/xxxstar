#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Automation Pallet
//!
//! Keeper network and automated task execution for X3 Chain.
//! Enables conditional execution of tasks based on on-chain conditions.

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
    use parity_scale_codec::Encode;
    use sp_core::H256;
    use sp_runtime::traits::{SaturatedConversion, Saturating};
    use x3_automation::{Action, Condition, ExecutionResult, Task, TaskId, TaskStatus};
    // Note: Would integrate with oracle pallet for price data

    /// Balance type alias
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Maximum number of tasks per account
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Active automated tasks
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new automated task was registered
        TaskRegistered {
            task_id: TaskId,
            owner: T::AccountId,
            condition: Condition,
        },
        /// A task was executed
        TaskExecuted {
            task_id: TaskId,
            success: bool,
            fee_charged: BalanceOf<T>,
        },
        /// A task was cancelled
        TaskCancelled {
            task_id: TaskId,
            owner: T::AccountId,
        },
        /// A task expired
        TaskExpired { task_id: TaskId },
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
        /// Task execution failed
        TaskExecutionFailed,
        /// Task condition not met
        ConditionNotMet,
        /// Invalid task parameters
        InvalidTask,
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

            // Check account task limit
            let account_tasks = AccountTasks::<T>::get(&who);
            ensure!(
                account_tasks.len() < T::MaxTasksPerAccount::get() as usize,
                Error::<T>::TooManyTasks
            );

            // Validate condition and action
            Self::validate_task_parameters(&condition, &action)?;

            // Calculate expiry
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

            // Check balance for registration fee
            let registration_fee = T::BaseRegistrationFee::get();
            ensure!(
                T::Currency::free_balance(&who) >= registration_fee,
                Error::<T>::InsufficientBalance
            );

            // Generate task ID
            let task_counter = TaskCounter::<T>::get();
            let task_id = Self::generate_task_id(task_counter, &who);
            TaskCounter::<T>::put(task_counter.saturating_add(1));

            // Reserve registration fee
            T::Currency::reserve(&who, registration_fee)?;

            // Create task
            let task = Task {
                id: task_id,
                owner: who.clone(),
                condition,
                action,
                max_fee,
                expiry_block,
                status: TaskStatus::Active,
            };

            // Store task
            Tasks::<T>::insert(task_id, task.clone());
            AccountTasks::<T>::mutate(&who, |tasks| {
                tasks.try_push(task_id).ok();
            });

            Self::deposit_event(Event::TaskRegistered {
                task_id,
                owner: who,
                condition: task.condition,
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
            ensure!(task.status == TaskStatus::Active, Error::<T>::InvalidTask);

            // Remove task
            Tasks::<T>::remove(task_id);
            AccountTasks::<T>::mutate(&who, |tasks| {
                if let Some(pos) = tasks.iter().position(|&id| id == task_id) {
                    tasks.swap_remove(pos);
                }
            });

            // Refund registration fee
            let registration_fee = T::BaseRegistrationFee::get();
            T::Currency::unreserve(&who, registration_fee);

            Self::deposit_event(Event::TaskCancelled {
                task_id,
                owner: who,
            });

            Ok(())
        }

        /// Execute a task (called by keepers/off-chain workers)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::execute_task())]
        pub fn execute_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let keeper = ensure_signed(origin)?;

            let mut task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(task.status == TaskStatus::Active, Error::<T>::InvalidTask);

            // Check condition
            ensure!(Self::check_condition(&task), Error::<T>::ConditionNotMet);

            // Check expiry
            let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
            if current_block >= task.expiry_block {
                task.status = TaskStatus::Expired;
                Tasks::<T>::insert(task_id, task);
                Self::deposit_event(Event::TaskExpired { task_id });
                return Err(Error::<T>::InvalidTask.into());
            }

            // Execute action
            let execution_result = Self::execute_action(&task)?;

            // Update task status
            task.status = if execution_result.success {
                TaskStatus::Executed
            } else {
                TaskStatus::Failed
            };
            Tasks::<T>::insert(task_id, task.clone());

            // Remove from account's tasks
            AccountTasks::<T>::mutate(&task.owner, |tasks| {
                if let Some(pos) = tasks.iter().position(|&id| id == task_id) {
                    tasks.swap_remove(pos);
                }
            });

            // Pay execution fee to keeper
            let execution_fee = T::ExecutionFee::get().min(task.max_fee);
            T::Currency::transfer(
                &task.owner,
                &keeper,
                execution_fee,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Unreserve registration fee
            let registration_fee = T::BaseRegistrationFee::get();
            T::Currency::unreserve(&task.owner, registration_fee);

            Self::deposit_event(Event::TaskExecuted {
                task_id,
                success: execution_result.success,
                fee_charged: execution_fee,
            });

            Ok(())
        }
    }
}

use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{Currency, Get, ReservableCurrency},
};
use pallet::BalanceOf;
use parity_scale_codec::Encode;
use sp_core::H256;
use sp_runtime::{traits::SaturatedConversion, DispatchError};
use sp_std::vec;
use sp_std::vec::Vec;
use x3_automation::{Action, Condition, ExecutionResult, Task, TaskId};

impl<T: pallet::Config> pallet::Pallet<T> {
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
            Condition::PriceThreshold { asset_id, .. } => {
                // Validate that asset exists (simplified check)
                ensure!(*asset_id > 0, Error::<T>::InvalidTask);
            }
            Condition::Custom(data) => {
                ensure!(!data.is_empty(), Error::<T>::InvalidTask);
            }
        }

        match action {
            Action::Extrinsic {
                pallet_index,
                call_index,
                ..
            } => {
                // Basic validation - in production would validate pallet/call exists
                ensure!(
                    *pallet_index > 0 || *call_index > 0,
                    Error::<T>::InvalidTask
                );
            }
            Action::Custom(data) => {
                ensure!(!data.is_empty(), Error::<T>::InvalidTask);
            }
        }

        Ok(())
    }

    /// Check if task condition is met
    fn check_condition(task: &Task<T::AccountId, BalanceOf<T>>) -> bool {
        match &task.condition {
            Condition::BlockNumber(target_block) => {
                let current_block =
                    frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
                current_block >= *target_block
            }
            Condition::PriceThreshold {
                asset_id: _,
                threshold: _,
                above: _,
            } => {
                // TODO: Integrate with oracle pallet for price conditions
                false
            }
            Condition::Custom(_) => {
                // Custom conditions would need custom logic
                false
            }
        }
    }

    /// Execute task action
    fn execute_action(
        task: &Task<T::AccountId, BalanceOf<T>>,
    ) -> Result<ExecutionResult, DispatchError> {
        // Simplified execution - in production would dispatch to actual pallets
        match &task.action {
            Action::Extrinsic { .. } => {
                // Would dispatch the extrinsic here
                Ok(ExecutionResult {
                    task_id: task.id,
                    success: true,
                    gas_used: 21000,
                    fee_charged: T::ExecutionFee::get().saturated_into::<u128>(),
                    output: vec![],
                })
            }
            Action::Custom(_) => {
                // Custom action execution
                Ok(ExecutionResult {
                    task_id: task.id,
                    success: true,
                    gas_used: 10000,
                    fee_charged: T::ExecutionFee::get().saturated_into::<u128>(),
                    output: vec![],
                })
            }
        }
    }

    /// Clean up expired tasks (called by off-chain worker or governance)
    pub fn cleanup_expired_tasks() -> u32 {
        let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u64>();
        let mut cleaned = 0u32;

        // Simplified cleanup - in production would iterate all tasks
        // This is just a placeholder implementation
        cleaned
    }
}
