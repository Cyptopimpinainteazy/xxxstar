#![deny(unsafe_code)]
//! # DePIN GPU Marketplace Pallet
//!
//! Proposal: DEPIN-GPU-001
//!
//! Turns validator GPU idle capacity into a revenue-generating compute marketplace.
//! Validators expose sandboxed GPU workers for paid off-chain workloads (AI inference,
//! ZK proving, video transcoding, etc.) while the base chain retains absolute priority
//! over block-building work.
//!
//! ## Revenue Split
//!
//! - 55% → Validator (provider)
//! - 25% → Burned (deflationary pressure)
//! - 20% → Redistributed to token stakers
//!
//! ## Key Invariants
//!
//! - DEPIN-MARKET-001: Block-building preempts marketplace jobs within 2ms
//! - DEPIN-MARKET-002: Revenue split conserves total (validator + burn + staker = 100%)
//! - DEPIN-MARKET-003: Escrow locked before assignment, released on completion/timeout
//! - DEPIN-MARKET-004: Sandbox memory isolation enforced
//! - DEPIN-MARKET-005: Provider stake slashed on verified failure

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, OnUnbalanced, ReservableCurrency},
        Blake2_128Concat, PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, SaturatedConversion, Saturating},
        Perbill,
    };
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ──────────────────────────────────────────────────────────────
    // Config
    // ──────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency for escrow, staking, and revenue distribution.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Handler for burned tokens.
        type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Origin that can configure marketplace parameters.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The marketplace pallet's ID (for escrow account).
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Revenue share to the validator provider (basis points out of 10_000).
        #[pallet::constant]
        type ValidatorShareBps: Get<u16>;

        /// Revenue share to burn (basis points out of 10_000).
        #[pallet::constant]
        type BurnShareBps: Get<u16>;

        /// Revenue share to stakers (basis points out of 10_000).
        #[pallet::constant]
        type StakerShareBps: Get<u16>;

        /// Minimum stake required to register as a provider.
        #[pallet::constant]
        type MinProviderStake: Get<BalanceOf<Self>>;

        /// Maximum number of active jobs per provider.
        #[pallet::constant]
        type MaxJobsPerProvider: Get<u32>;

        /// Maximum duration of a job in blocks.
        #[pallet::constant]
        type MaxJobDuration: Get<BlockNumberFor<Self>>;

        /// Maximum number of pending orders in the order book.
        #[pallet::constant]
        type MaxPendingOrders: Get<u32>;

        /// Slash fraction applied on verified job failure.
        #[pallet::constant]
        type SlashFraction: Get<Perbill>;

        /// Weight info for benchmarking.
        type WeightInfo: WeightInfo;
    }

    // ──────────────────────────────────────────────────────────────
    // Storage
    // ──────────────────────────────────────────────────────────────

    /// Registered GPU compute providers (validator nodes).
    #[pallet::storage]
    #[pallet::getter(fn providers)]
    pub type Providers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ProviderInfo<T>, OptionQuery>;

    /// Active jobs being executed.
    #[pallet::storage]
    #[pallet::getter(fn active_jobs)]
    pub type ActiveJobs<T: Config> =
        StorageMap<_, Blake2_128Concat, JobId, MarketplaceJob<T>, OptionQuery>;

    /// Pending order book — customers waiting for providers.
    #[pallet::storage]
    #[pallet::getter(fn pending_orders)]
    pub type PendingOrders<T: Config> =
        StorageValue<_, BoundedVec<Order<T>, T::MaxPendingOrders>, ValueQuery>;

    /// Total number of completed jobs across the marketplace.
    #[pallet::storage]
    #[pallet::getter(fn total_jobs_completed)]
    pub type TotalJobsCompleted<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total revenue distributed through the marketplace.
    #[pallet::storage]
    #[pallet::getter(fn total_revenue)]
    pub type TotalRevenue<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Total tokens burned through marketplace fees.
    #[pallet::storage]
    #[pallet::getter(fn total_burned)]
    pub type TotalBurned<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Number of active jobs per provider.
    #[pallet::storage]
    #[pallet::getter(fn provider_job_count)]
    pub type ProviderJobCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    /// Whether the marketplace is paused.
    #[pallet::storage]
    #[pallet::getter(fn is_paused)]
    pub type Paused<T: Config> = StorageValue<_, bool, ValueQuery>;

    // ──────────────────────────────────────────────────────────────
    // Events
    // ──────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new GPU provider registered. [provider, stake]
        ProviderRegistered {
            provider: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// A provider deregistered. [provider]
        ProviderDeregistered { provider: T::AccountId },
        /// A provider was paused (self or admin). [provider]
        ProviderPaused { provider: T::AccountId },
        /// A provider resumed. [provider]
        ProviderResumed { provider: T::AccountId },
        /// A new compute job was submitted to the order book. [job_id, customer, max_price]
        OrderSubmitted {
            job_id: JobId,
            customer: T::AccountId,
            max_price: BalanceOf<T>,
        },
        /// A job was assigned to a provider. [job_id, provider]
        JobAssigned {
            job_id: JobId,
            provider: T::AccountId,
        },
        /// A job completed successfully. [job_id, provider, compute_units, revenue]
        JobCompleted {
            job_id: JobId,
            provider: T::AccountId,
            compute_units: u64,
            revenue: BalanceOf<T>,
        },
        /// A job failed. [job_id, provider, reason]
        JobFailed {
            job_id: JobId,
            provider: T::AccountId,
            reason: JobFailureReason,
        },
        /// Revenue was distributed. [job_id, validator_share, burned, staker_share]
        RevenueDistributed {
            job_id: JobId,
            validator_share: BalanceOf<T>,
            burned: BalanceOf<T>,
            staker_share: BalanceOf<T>,
        },
        /// A provider was slashed for job failure. [provider, amount]
        ProviderSlashed {
            provider: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// An order was cancelled by the customer. [job_id, customer]
        OrderCancelled {
            job_id: JobId,
            customer: T::AccountId,
        },
        /// Marketplace paused by admin.
        MarketplacePaused,
        /// Marketplace resumed by admin.
        MarketplaceResumed,
    }

    // ──────────────────────────────────────────────────────────────
    // Errors
    // ──────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Provider is already registered.
        ProviderAlreadyRegistered,
        /// Provider not found.
        ProviderNotFound,
        /// Insufficient stake to register.
        InsufficientStake,
        /// Job not found.
        JobNotFound,
        /// Order not found in the pending queue.
        OrderNotFound,
        /// Maximum jobs per provider reached.
        MaxJobsReached,
        /// Order book is full.
        OrderBookFull,
        /// Job duration exceeds maximum.
        JobDurationExceeded,
        /// Provider is not active.
        ProviderNotActive,
        /// Only the job customer can cancel.
        NotJobCustomer,
        /// Job is already assigned and cannot be cancelled.
        JobAlreadyAssigned,
        /// Revenue split does not sum to 10_000 bps.
        InvalidRevenueSplit,
        /// Marketplace is paused.
        MarketplacePaused,
        /// Cannot deregister with active jobs.
        ActiveJobsExist,
        /// Insufficient balance for escrow.
        InsufficientBalance,
        /// Only assigned provider can complete/fail a job.
        NotAssignedProvider,
        /// Arithmetic overflow in fee calculation.
        ArithmeticOverflow,
    }

    // ──────────────────────────────────────────────────────────────
    // Genesis
    // ──────────────────────────────────────────────────────────────

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub paused: bool,
        #[serde(skip)]
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Paused::<T>::put(self.paused);
            // Validate revenue split sums to 10_000
            let total = T::ValidatorShareBps::get() as u32
                + T::BurnShareBps::get() as u32
                + T::StakerShareBps::get() as u32;
            assert_eq!(total, 10_000, "Revenue split must sum to 10_000 bps");
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Hooks
    // ──────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Expire timed-out jobs
            let mut weight = Weight::zero();
            let mut expired_jobs = Vec::new();

            for (job_id, job) in ActiveJobs::<T>::iter() {
                weight = weight.saturating_add(T::DbWeight::get().reads(1));
                if now > job.deadline {
                    expired_jobs.push((job_id, job));
                }
            }

            for (job_id, job) in expired_jobs {
                if let Some(provider) = &job.assigned_provider {
                    // Slash provider for timeout
                    Self::slash_provider(provider.clone(), &job_id);
                }
                // Refund customer escrow
                let escrow_account = Self::account_id();
                let _ = T::Currency::transfer(
                    &escrow_account,
                    &job.customer,
                    job.escrow.saturated_into(),
                    ExistenceRequirement::AllowDeath,
                );
                ActiveJobs::<T>::remove(job_id);
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                Self::deposit_event(Event::JobFailed {
                    job_id,
                    provider: job.assigned_provider.unwrap_or_else(Self::account_id),
                    reason: JobFailureReason::Timeout,
                });
            }

            weight
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Dispatchables
    // ──────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register as a GPU compute provider by staking the minimum amount.
        ///
        /// # Invariant: DEPIN-MARKET-003
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn register_provider(
            origin: OriginFor<T>,
            gpu_specs: GpuSpecification,
            price_per_compute_unit: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Paused::<T>::get(), Error::<T>::MarketplacePaused);
            ensure!(
                !Providers::<T>::contains_key(&who),
                Error::<T>::ProviderAlreadyRegistered
            );

            let min_stake = T::MinProviderStake::get();
            ensure!(
                T::Currency::free_balance(&who) >= min_stake,
                Error::<T>::InsufficientStake
            );

            // Reserve the stake
            T::Currency::reserve(&who, min_stake)?;

            let now = <frame_system::Pallet<T>>::block_number();

            let provider = ProviderInfo {
                account: who.clone(),
                stake: min_stake.saturated_into(),
                gpu_specs,
                price_per_compute_unit: price_per_compute_unit.saturated_into(),
                reputation: 5_000, // Start at 50%
                total_jobs_completed: 0,
                total_revenue: 0u128,
                status: ProviderStatus::Active,
                registered_at: now,
            };

            Providers::<T>::insert(&who, provider);

            Self::deposit_event(Event::ProviderRegistered {
                provider: who,
                stake: min_stake,
            });

            Ok(())
        }

        /// Deregister as a provider and reclaim staked tokens.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn deregister_provider(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let provider = Providers::<T>::get(&who).ok_or(Error::<T>::ProviderNotFound)?;
            ensure!(
                ProviderJobCount::<T>::get(&who) == 0,
                Error::<T>::ActiveJobsExist
            );

            // Unreserve stake
            T::Currency::unreserve(&who, provider.stake.saturated_into());

            Providers::<T>::remove(&who);
            ProviderJobCount::<T>::remove(&who);

            Self::deposit_event(Event::ProviderDeregistered { provider: who });

            Ok(())
        }

        /// Pause a provider (self-service).
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn pause_provider(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Providers::<T>::try_mutate(&who, |maybe_provider| -> DispatchResult {
                let provider = maybe_provider
                    .as_mut()
                    .ok_or(Error::<T>::ProviderNotFound)?;
                provider.status = ProviderStatus::Paused;
                Ok(())
            })?;

            Self::deposit_event(Event::ProviderPaused { provider: who });
            Ok(())
        }

        /// Resume a paused provider.
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn resume_provider(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Providers::<T>::try_mutate(&who, |maybe_provider| -> DispatchResult {
                let provider = maybe_provider
                    .as_mut()
                    .ok_or(Error::<T>::ProviderNotFound)?;
                provider.status = ProviderStatus::Active;
                Ok(())
            })?;

            Self::deposit_event(Event::ProviderResumed { provider: who });
            Ok(())
        }

        /// Submit a compute job order to the order book with escrowed payment.
        ///
        /// # Invariant: DEPIN-MARKET-003
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn submit_order(
            origin: OriginFor<T>,
            job_type: DePinJobType,
            gpu_requirements: GpuRequirements,
            max_price: BalanceOf<T>,
            duration_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Paused::<T>::get(), Error::<T>::MarketplacePaused);
            ensure!(
                duration_blocks <= T::MaxJobDuration::get(),
                Error::<T>::JobDurationExceeded
            );

            // Lock escrow
            ensure!(
                T::Currency::free_balance(&who) >= max_price,
                Error::<T>::InsufficientBalance
            );

            let escrow_account = Self::account_id();
            T::Currency::transfer(
                &who,
                &escrow_account,
                max_price,
                ExistenceRequirement::KeepAlive,
            )?;

            let now = <frame_system::Pallet<T>>::block_number();
            let job_id = Self::next_job_id();

            let order = Order {
                job_id,
                customer: who.clone(),
                job_type,
                gpu_requirements,
                max_price: max_price.saturated_into(),
                duration_blocks,
                submitted_at: now,
            };

            PendingOrders::<T>::try_mutate(|orders| -> DispatchResult {
                orders
                    .try_push(order)
                    .map_err(|_| Error::<T>::OrderBookFull)?;
                Ok(())
            })?;

            Self::deposit_event(Event::OrderSubmitted {
                job_id,
                customer: who,
                max_price,
            });

            Ok(())
        }

        /// Accept an order from the order book and begin execution.
        ///
        /// # Invariant: DEPIN-MARKET-003
        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn accept_order(origin: OriginFor<T>, job_id: JobId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Paused::<T>::get(), Error::<T>::MarketplacePaused);

            let provider = Providers::<T>::get(&who).ok_or(Error::<T>::ProviderNotFound)?;
            ensure!(
                provider.status == ProviderStatus::Active,
                Error::<T>::ProviderNotActive
            );
            ensure!(
                ProviderJobCount::<T>::get(&who) < T::MaxJobsPerProvider::get(),
                Error::<T>::MaxJobsReached
            );

            // Find and remove the order
            let order =
                PendingOrders::<T>::try_mutate(|orders| -> Result<Order<T>, DispatchError> {
                    let idx = orders
                        .iter()
                        .position(|o| o.job_id == job_id)
                        .ok_or(Error::<T>::OrderNotFound)?;
                    Ok(orders.remove(idx))
                })?;

            let now = <frame_system::Pallet<T>>::block_number();
            let deadline = now.saturating_add(order.duration_blocks);

            let job = MarketplaceJob {
                id: job_id,
                customer: order.customer,
                job_type: order.job_type,
                gpu_requirements: order.gpu_requirements,
                escrow: order.max_price,
                assigned_provider: Some(who.clone()),
                status: JobStatus::Executing,
                submitted_at: order.submitted_at,
                assigned_at: Some(now),
                deadline,
                result_hash: None,
                compute_units_used: 0,
            };

            ActiveJobs::<T>::insert(job_id, job);
            ProviderJobCount::<T>::mutate(&who, |count| *count = count.saturating_add(1));

            Self::deposit_event(Event::JobAssigned {
                job_id,
                provider: who,
            });

            Ok(())
        }

        /// Report job completion and trigger revenue distribution.
        ///
        /// # Invariant: DEPIN-MARKET-002, DEPIN-MARKET-003
        #[pallet::call_index(6)]
        #[pallet::weight(T::DbWeight::get().reads_writes(4, 5))]
        pub fn complete_job(
            origin: OriginFor<T>,
            job_id: JobId,
            _result_hash: sp_core::H256,
            compute_units: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let job = ActiveJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;
            ensure!(
                job.assigned_provider.as_ref() == Some(&who),
                Error::<T>::NotAssignedProvider
            );

            // Distribute revenue from escrow
            let revenue: BalanceOf<T> = job.escrow.saturated_into();
            Self::distribute_revenue(job_id, &who, revenue)?;

            // Update provider stats
            Providers::<T>::try_mutate(&who, |maybe_provider| -> DispatchResult {
                if let Some(provider) = maybe_provider.as_mut() {
                    provider.total_jobs_completed = provider.total_jobs_completed.saturating_add(1);
                    provider.total_revenue = provider.total_revenue.saturating_add(job.escrow);
                    provider.reputation = provider.reputation.saturating_add(100).min(10_000);
                }
                Ok(())
            })?;

            // Clean up
            ActiveJobs::<T>::remove(job_id);
            ProviderJobCount::<T>::mutate(&who, |count| *count = count.saturating_sub(1));
            TotalJobsCompleted::<T>::mutate(|total| *total = total.saturating_add(1));

            Self::deposit_event(Event::JobCompleted {
                job_id,
                provider: who,
                compute_units,
                revenue,
            });

            Ok(())
        }

        /// Report a job failure. Provider is slashed.
        ///
        /// # Invariant: DEPIN-MARKET-005
        #[pallet::call_index(7)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 4))]
        pub fn report_job_failure(
            origin: OriginFor<T>,
            job_id: JobId,
            reason: JobFailureReason,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let job = ActiveJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;
            ensure!(
                job.assigned_provider.as_ref() == Some(&who),
                Error::<T>::NotAssignedProvider
            );

            // Slash provider
            Self::slash_provider(who.clone(), &job_id);

            // Refund customer
            let escrow_account = Self::account_id();
            let _ = T::Currency::transfer(
                &escrow_account,
                &job.customer,
                job.escrow.saturated_into(),
                ExistenceRequirement::AllowDeath,
            );

            // Clean up
            ActiveJobs::<T>::remove(job_id);
            ProviderJobCount::<T>::mutate(&who, |count| *count = count.saturating_sub(1));

            Self::deposit_event(Event::JobFailed {
                job_id,
                provider: who,
                reason,
            });

            Ok(())
        }

        /// Cancel a pending order (customer only). Refunds escrow.
        #[pallet::call_index(8)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn cancel_order(origin: OriginFor<T>, job_id: JobId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let order =
                PendingOrders::<T>::try_mutate(|orders| -> Result<Order<T>, DispatchError> {
                    let idx = orders
                        .iter()
                        .position(|o| o.job_id == job_id && o.customer == who)
                        .ok_or(Error::<T>::OrderNotFound)?;
                    Ok(orders.remove(idx))
                })?;

            // Refund escrow
            let escrow_account = Self::account_id();
            let _ = T::Currency::transfer(
                &escrow_account,
                &who,
                order.max_price.saturated_into(),
                ExistenceRequirement::AllowDeath,
            );

            Self::deposit_event(Event::OrderCancelled {
                job_id,
                customer: who,
            });

            Ok(())
        }

        /// Pause the entire marketplace (admin only).
        #[pallet::call_index(9)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn pause_marketplace(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Paused::<T>::put(true);
            Self::deposit_event(Event::MarketplacePaused);
            Ok(())
        }

        /// Resume the marketplace (admin only).
        #[pallet::call_index(10)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn resume_marketplace(origin: OriginFor<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Paused::<T>::put(false);
            Self::deposit_event(Event::MarketplaceResumed);
            Ok(())
        }
    }

    // ──────────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Get the escrow account for this pallet.
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Generate a unique job ID from block number and extrinsic index.
        fn next_job_id() -> JobId {
            let block = <frame_system::Pallet<T>>::block_number();
            let ext_index = <frame_system::Pallet<T>>::extrinsic_index().unwrap_or(0);
            let mut raw = [0u8; 16];
            raw[..8].copy_from_slice(&block.using_encoded(|e| {
                let mut buf = [0u8; 8];
                let len = e.len().min(8);
                buf[..len].copy_from_slice(&e[..len]);
                buf
            }));
            raw[8..12].copy_from_slice(&ext_index.to_le_bytes());
            // Mix with total jobs for uniqueness
            let total = TotalJobsCompleted::<T>::get();
            raw[12..16].copy_from_slice(&(total as u32).to_le_bytes());
            JobId(raw)
        }

        /// Distribute revenue according to the configured split.
        ///
        /// # Invariant: DEPIN-MARKET-002
        fn distribute_revenue(
            job_id: JobId,
            provider: &T::AccountId,
            total: BalanceOf<T>,
        ) -> DispatchResult {
            let validator_bps = T::ValidatorShareBps::get() as u32;
            let burn_bps = T::BurnShareBps::get() as u32;
            let staker_bps = T::StakerShareBps::get() as u32;

            // Safety: validated at genesis
            debug_assert_eq!(validator_bps + burn_bps + staker_bps, 10_000);

            let validator_share = Perbill::from_parts(validator_bps * 100_000) * total;
            let burn_share = Perbill::from_parts(burn_bps * 100_000) * total;
            let staker_share = total
                .saturating_sub(validator_share)
                .saturating_sub(burn_share);

            let escrow_account = Self::account_id();

            // Pay validator
            T::Currency::transfer(
                &escrow_account,
                provider,
                validator_share,
                ExistenceRequirement::AllowDeath,
            )?;

            // Burn
            let imbalance = T::Currency::slash(&escrow_account, burn_share).0;
            T::BurnDestination::on_unbalanced(imbalance);
            TotalBurned::<T>::mutate(|b| *b = b.saturating_add(burn_share));

            // Staker share stays in escrow account for distribution by staking pallet
            // (or transferred to treasury)
            TotalRevenue::<T>::mutate(|r| *r = r.saturating_add(total));

            Self::deposit_event(Event::RevenueDistributed {
                job_id,
                validator_share,
                burned: burn_share,
                staker_share,
            });

            Ok(())
        }

        /// Slash a provider for job failure or timeout.
        ///
        /// # Invariant: DEPIN-MARKET-005
        fn slash_provider(provider: T::AccountId, _job_id: &JobId) {
            if let Some(mut info) = Providers::<T>::get(&provider) {
                let slash_amount: BalanceOf<T> =
                    (T::SlashFraction::get() * info.stake).saturated_into();
                let (imbalance, _remaining) = T::Currency::slash_reserved(&provider, slash_amount);
                T::BurnDestination::on_unbalanced(imbalance);

                info.stake = info.stake.saturating_sub(slash_amount.saturated_into());
                info.reputation = info.reputation.saturating_sub(500);

                if info.reputation == 0 {
                    info.status = ProviderStatus::Slashed;
                }

                Providers::<T>::insert(&provider, info);

                Self::deposit_event(Event::ProviderSlashed {
                    provider,
                    amount: slash_amount,
                });
            }
        }
    }
}
