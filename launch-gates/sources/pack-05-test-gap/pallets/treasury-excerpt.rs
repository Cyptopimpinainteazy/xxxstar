#![deny(unsafe_code)]
//! # X3Chain Treasury Pallet
//!
//! A comprehensive treasury management system supporting:
//! - Multi-sig spending approvals
//! - Spending tracks with different approval thresholds
//! - Recurring payments with configurable intervals
//! - Yield strategies that can be delegated to AI agents
//! - Emergency pause system
//!
//! ## Overview
//!
//! The treasury pallet manages protocol funds with multiple security layers.
//! Spending proposals require multi-sig approval based on the spending track.
//! AI agents can be delegated to execute yield strategies within defined limits.
//!
//! ## Spending Tracks
//!
//! - Small: Low threshold, fast approval (e.g., bug bounties)
//! - Medium: Standard threshold (e.g., grants)
//! - Large: High threshold, longer voting (e.g., major initiatives)
//! - Critical: Maximum security, council-only (e.g., protocol upgrades)

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

pub mod migrations;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
        Blake2_128Concat, PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, Saturating, Zero},
        Percent,
    };
    use sp_std::prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for treasury operations.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Origin that can approve small spending proposals.
        type SmallSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve medium spending proposals.
        type MediumSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve large spending proposals.
        type LargeSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can approve critical spending (council).
        type CriticalSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can pause treasury operations.
        type PauseOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that can configure yield strategies.
        type YieldConfigOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The treasury's pallet ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Maximum signers for multi-sig.
        #[pallet::constant]
        type MaxSigners: Get<u32>;

        /// Maximum pending proposals.
        #[pallet::constant]
        type MaxProposals: Get<u32>;

        /// Maximum recurring payments.
        #[pallet::constant]
        type MaxRecurringPayments: Get<u32>;

        /// Maximum yield strategies.
        #[pallet::constant]
        type MaxYieldStrategies: Get<u32>;

        /// Small spend threshold.
        #[pallet::constant]
        type SmallSpendLimit: Get<BalanceOf<Self>>;

        /// Medium spend threshold.
        #[pallet::constant]
        type MediumSpendLimit: Get<BalanceOf<Self>>;

        /// Large spend threshold.
        #[pallet::constant]
        type LargeSpendLimit: Get<BalanceOf<Self>>;

        /// Proposal bond percentage.
        #[pallet::constant]
        type ProposalBond: Get<Percent>;

        /// Minimum proposal bond.
        #[pallet::constant]
        type ProposalBondMinimum: Get<BalanceOf<Self>>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    // ========================================================================
    // Storage Items
    // ========================================================================

    /// Counter for spending proposal IDs.
    #[pallet::storage]
    #[pallet::getter(fn proposal_count)]
    pub type ProposalCount<T> = StorageValue<_, u32, ValueQuery>;

    /// All spending proposals.
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        SpendingProposal<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Multi-sig signers for spending.
    #[pallet::storage]
    #[pallet::getter(fn signers)]
    pub type Signers<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxSigners>, ValueQuery>;

    /// Approvals for each proposal.
    #[pallet::storage]
    #[pallet::getter(fn approvals)]
    pub type Approvals<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BoundedVec<T::AccountId, T::MaxSigners>, ValueQuery>;

    /// Recurring payment schedules.
    #[pallet::storage]
    #[pallet::getter(fn recurring_payments)]
    pub type RecurringPayments<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        RecurringPayment<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Counter for recurring payment IDs.
    #[pallet::storage]
    #[pallet::getter(fn recurring_payment_count)]
    pub type RecurringPaymentCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Active yield strategies.
    #[pallet::storage]
    #[pallet::getter(fn yield_strategies)]
    pub type YieldStrategies<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        YieldStrategy<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    /// Counter for yield strategy IDs.
    #[pallet::storage]
    #[pallet::getter(fn yield_strategy_count)]
    pub type YieldStrategyCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Whether treasury is paused.
    #[pallet::storage]
    #[pallet::getter(fn is_paused)]
    pub type IsPaused<T> = StorageValue<_, bool, ValueQuery>;

    /// Emergency pause info.
    #[pallet::storage]
    #[pallet::getter(fn pause_info)]
    pub type PauseInfo<T: Config> =
        StorageValue<_, EmergencyPause<T::AccountId, BlockNumberFor<T>>, OptionQuery>;

    /// Treasury statistics.
    #[pallet::storage]
    #[pallet::getter(fn stats)]
    pub type Stats<T: Config> = StorageValue<_, TreasuryStats<BalanceOf<T>>, ValueQuery>;

    // ========================================================================
    // Events
    // ========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A spending proposal was submitted.
        ProposalSubmitted {
            proposal_id: u32,
            proposer: T::AccountId,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            track: SpendingTrack,
        },
        /// A proposal was approved by a signer.
        ProposalApproved {
            proposal_id: u32,
            signer: T::AccountId,
            approvals: u32,
            threshold: u32,
        },
        /// A proposal was executed.
        ProposalExecuted {
            proposal_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// A proposal was rejected.
        ProposalRejected { proposal_id: u32 },
        /// A recurring payment was created.
        RecurringPaymentCreated {
            payment_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            interval: BlockNumberFor<T>,
        },
        /// A recurring payment was executed.
        RecurringPaymentExecuted {
            payment_id: u32,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// A recurring payment was cancelled.
        RecurringPaymentCancelled { payment_id: u32 },
        /// A yield strategy was registered.
        YieldStrategyRegistered {
            strategy_id: u32,
            agent: T::AccountId,
            max_allocation: BalanceOf<T>,
        },
        /// A yield strategy was executed.
        YieldStrategyExecuted {
            strategy_id: u32,
            amount: BalanceOf<T>,
            profit: BalanceOf<T>,
        },
        /// A yield strategy was deactivated.
        YieldStrategyDeactivated { strategy_id: u32 },
        /// Treasury was paused.
        TreasuryPaused {
            by: T::AccountId,
            reason: BoundedVec<u8, ConstU32<256>>,
        },
        /// Treasury was unpaused.
        TreasuryUnpaused { by: T::AccountId },
        /// Signers were updated.
        SignersUpdated { signers: Vec<T::AccountId> },
        /// Funds deposited to treasury.
        Deposited {
            from: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    // ========================================================================
    // Errors
    // ========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Proposal not found.
        ProposalNotFound,
        /// Too many proposals.
        TooManyProposals,
        /// Insufficient treasury balance.
        InsufficientBalance,
        /// Not a valid signer.
        NotSigner,
        /// Already approved this proposal.
        AlreadyApproved,
        /// Treasury is paused.
        TreasuryPaused,
        /// Treasury is not paused.
        TreasuryNotPaused,
        /// Invalid spending track.
        InvalidTrack,
        /// Amount exceeds track limit.
        AmountExceedsLimit,
        /// Recurring payment not found.
        RecurringPaymentNotFound,
        /// Too many recurring payments.
        TooManyRecurringPayments,
        /// Payment not due yet.
        PaymentNotDue,
        /// Yield strategy not found.
        YieldStrategyNotFound,
        /// Too many yield strategies.
        TooManyYieldStrategies,
        /// Strategy allocation exceeded.
        AllocationExceeded,
        /// Invalid yield return.
        InvalidYieldReturn,
        /// Not the strategy agent.
        NotStrategyAgent,
        /// Strategy is not active.
        StrategyNotActive,
        /// Too many signers.
        TooManySigners,
        /// Threshold not reached.
        ThresholdNotReached,
        /// Zero amount not allowed.
        ZeroAmount,
        /// Proposal already executed.
        AlreadyExecuted,
        /// Arithmetic overflow.
        ArithmeticOverflow,
    }

    // ========================================================================
    // Genesis Config
    // ========================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_signers: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let signers: BoundedVec<T::AccountId, T::MaxSigners> =
                self.initial_signers.clone().try_into().unwrap_or_else(|_| {
                    // This should never happen in practice as initial_signers is validated
                    // during chain spec creation, but we provide a safe fallback
                    BoundedVec::truncate_from(vec![])
                });
            Signers::<T>::put(signers);
        }
    }

    // ========================================================================
    // Hooks
    // ========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();

            if !IsPaused::<T>::get() {
                // Process recurring payments
                weight = weight.saturating_add(Self::process_recurring_payments(n));
            }

            weight
        }
    }

    // ========================================================================
    // Extrinsics
    // ========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a spending proposal.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_proposal())]
        pub fn submit_proposal(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            amount: BalanceOf<T>,
            description: BoundedVec<u8, ConstU32<1024>>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

            // Determine spending track
            let track = Self::determine_track(amount);

            // Reserve proposal bond
            let bond = Self::calculate_bond(amount);
            T::Currency::reserve(&proposer, bond)?;

            let proposal_id = ProposalCount::<T>::get();
            ensure!(
                proposal_id < T::MaxProposals::get(),
                Error::<T>::TooManyProposals
            );

            let current_block = frame_system::Pallet::<T>::block_number();

            let proposal = SpendingProposal {
                id: proposal_id,
                proposer: proposer.clone(),
                beneficiary: beneficiary.clone(),
                amount,
                bond,
                description,
                track,
                status: ProposalStatus::Pending,
                submitted_at: current_block,
                executed_at: None,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            ProposalCount::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::ProposalSubmitted {
                proposal_id,
                proposer,
                beneficiary,
                amount,
                track,
            });

            Ok(())
        }

        /// Approve a spending proposal (multi-sig).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::approve_proposal())]
        pub fn approve_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            ensure!(!IsPaused::<T>::get(), Error::<T>::TreasuryPaused);

            // Verify signer
            let signers = Signers::<T>::get();
            ensure!(signers.contains(&signer), Error::<T>::NotSigner);

            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Pending,
                Error::<T>::AlreadyExecuted
            );

            // Check not already approved
            let mut approvals = Approvals::<T>::get(proposal_id);
            ensure!(!approvals.contains(&signer), Error::<T>::AlreadyApproved);

            // Add approval
            approvals
                .try_push(signer.clone())
                .map_err(|_| Error::<T>::TooManySigners)?;

            let approval_count = approvals.len() as u32;
            let threshold = Self::get_threshold(&proposal.track);

            Approvals::<T>::insert(proposal_id, approvals);

            Self::deposit_event(Event::ProposalApproved {
                proposal_id,
                signer,
                approvals: approval_count,
                threshold,
            });

            // Auto-execute if threshold reached
            if approval_count >= threshold {
                Self::do_execute_proposal(proposal_id)?;
            }

            Ok(())
        }

        /// Execute an approved proposal.
