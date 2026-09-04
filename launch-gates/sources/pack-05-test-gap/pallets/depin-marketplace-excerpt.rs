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
