#![deny(unsafe_code)]
//! # X3 dApp Hub Pallet — Phase 8
//!
//! On-chain dApp registry with governance-controlled approval lifecycle,
//! pluggable revenue-split policies, and per-developer earnings accounting.
//!
//! ## Invariants
//!
//! - DAPP-001: Only a registered, policy-backed dApp may receive revenue.
//! - DAPP-002: Revenue is recorded only for dApps whose `ApprovalStatus` is
//!   `Approved`.
//! - DAPP-003: A developer may not hold more than `MaxDAppsPerDeveloper` dApps.
//! - DAPP-004: Total live dApps may not exceed `MaxActiveDApps`.
//! - DAPP-005: A revenue policy is stored only when its entries sum to 10 000 bps.
//! - DAPP-006: `withdraw_earnings` reduces balance only by at most the accrued amount.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
pub use x3_revenue_sharing::{
    validate_split, ApprovalStatus, PlacementTier, RevenueDestination, RevenueSplitPolicy,
};

// ── Public types ───────────────────────────────────────────────────────────────

/// Auto-incremented integer identifier for a registered dApp.
pub type DAppId = u64;

/// Complete on-chain record for a registered dApp.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct DAppState<AccountId, BlockNumber> {
    /// Unique identifier, assigned at registration.
    pub dapp_id: DAppId,
    /// Account that registered the dApp.
    pub developer: AccountId,
    /// Opaque category tag chosen by the registrant.
    pub category_id: u32,
    /// Key into `RevenuePolicies` storage.
    pub revenue_policy_id: u32,
    /// Current marketplace placement tier.
    pub placement: PlacementTier,
    /// Governance approval state.
    pub approval_status: ApprovalStatus,
    /// Block at which the dApp was registered.
    pub registered_at: BlockNumber,
    /// Cumulative gross revenue ever recorded for this dApp.
    pub total_revenue_collected: u128,
    /// Cumulative amount that has been credited to the developer's earnings.
    pub total_developer_paid: u128,
}

// ── Pallet ─────────────────────────────────────────────────────────────────────

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ── Config ────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that may perform all governance actions.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum dApps a single developer account may register simultaneously.
        #[pallet::constant]
        type MaxDAppsPerDeveloper: Get<u32>;

        /// Maximum total live dApps across all developers.
        #[pallet::constant]
        type MaxActiveDApps: Get<u32>;

        /// Native token amount deducted (tracked only) on registration.
        #[pallet::constant]
        type RegistrationDepositAmount: Get<u128>;

        /// Native token fee (tracked only) when upgrading to Featured placement.
        #[pallet::constant]
        type FeaturedPlacementFeeAmount: Get<u128>;

        /// Native token fee (tracked only) when upgrading to Premium placement.
        #[pallet::constant]
        type PremiumPlacementFeeAmount: Get<u128>;
    }

    // ── Storage ───────────────────────────────────────────────────────────────

    /// Primary dApp registry: DAppId → DAppState.
    #[pallet::storage]
    #[pallet::getter(fn dapps)]
    pub type DApps<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        DAppId,
        DAppState<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Per-developer dApp list for limit enforcement.
    #[pallet::storage]
    #[pallet::getter(fn developer_dapps)]
    pub type DeveloperDApps<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<DAppId, T::MaxDAppsPerDeveloper>,
        ValueQuery,
    >;

    /// Revenue-split policies keyed by policy_id.
    #[pallet::storage]
    #[pallet::getter(fn revenue_policies)]
    pub type RevenuePolicies<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, RevenueSplitPolicy, OptionQuery>;

    /// Total number of registered dApps (across all statuses).
    #[pallet::storage]
    #[pallet::getter(fn total_dapps)]
    pub type TotalDApps<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Monotonically increasing counter used to assign fresh `DAppId` values.
    #[pallet::storage]
    #[pallet::getter(fn next_dapp_id)]
    pub type NextDAppId<T: Config> = StorageValue<_, DAppId, ValueQuery>;

    /// Accumulated earnings available for withdrawal per developer.
    #[pallet::storage]
    #[pallet::getter(fn developer_earnings)]
    pub type DeveloperEarnings<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

    // ── Genesis ───────────────────────────────────────────────────────────────

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[allow(unused)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {}
    }

    // ── Hooks ─────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new dApp was registered and is awaiting approval.
        DAppRegistered {
            dapp_id: DAppId,
            developer: T::AccountId,
        },
        /// A pending dApp was approved by governance.
        DAppApproved { dapp_id: DAppId },
        /// A dApp was rejected by governance.
        DAppRejected { dapp_id: DAppId },
        /// An approved dApp was suspended by governance.
        DAppSuspended { dapp_id: DAppId },
        /// A dApp's placement tier was updated.
        PlacementUpdated {
            dapp_id: DAppId,
            tier: PlacementTier,
        },
        /// Revenue was recorded and split according to the policy.
        RevenueRecorded {
            dapp_id: DAppId,
            gross_amount: u128,
            developer_share: u128,
            protocol_share: u128,
        },
        /// A revenue-split policy was stored or updated.
        RevenuePolicySet { policy_id: u32 },
        /// A developer withdrew accrued earnings.
        EarningsWithdrawn {
            developer: T::AccountId,
            amount: u128,
        },
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No dApp with this ID exists.
        DAppNotFound,
        /// Revenue can only be recorded for an `Approved` dApp.
        DAppNotApproved,
        /// The referenced revenue policy does not exist.
        PolicyNotFound,
        /// The submitted policy's entries do not sum to 10 000 bps.
        InvalidSplitPolicy,
        /// This developer already holds `MaxDAppsPerDeveloper` dApps.
        MaxDAppsReached,
        /// The developer has insufficient accrued earnings for this withdrawal.
        InsufficientEarnings,
        /// The global dApp registry is full (`MaxActiveDApps` reached).
        MaxActiveDAppsReached,
        /// The caller is not the developer that registered this dApp.
        NotDeveloper,
    }

    // ── Extrinsics ────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new dApp.
        ///
        /// Any signed origin.  The dApp is placed in `Pending` status until
        /// governance calls [`approve_dapp`].
        ///
        /// Preconditions:
        /// - The referenced `revenue_policy_id` must already exist.
        /// - The developer must hold fewer than `MaxDAppsPerDeveloper` dApps.
        /// - Global `TotalDApps` must be below `MaxActiveDApps`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn register_dapp(
            origin: OriginFor<T>,
            category_id: u32,
            revenue_policy_id: u32,
        ) -> DispatchResult {
            let developer = ensure_signed(origin)?;

            ensure!(
                RevenuePolicies::<T>::contains_key(revenue_policy_id),
                Error::<T>::PolicyNotFound
            );

            let total = TotalDApps::<T>::get();
            ensure!(
                total < T::MaxActiveDApps::get(),
                Error::<T>::MaxActiveDAppsReached
            );

            let dev_dapps = DeveloperDApps::<T>::get(&developer);
            ensure!(
                (dev_dapps.len() as u32) < T::MaxDAppsPerDeveloper::get(),
                Error::<T>::MaxDAppsReached
            );

            let dapp_id = NextDAppId::<T>::get();
            NextDAppId::<T>::put(dapp_id.saturating_add(1));

            let now = frame_system::Pallet::<T>::block_number();

            let dapp = DAppState {
                dapp_id,
                developer: developer.clone(),
                category_id,
                revenue_policy_id,
                placement: PlacementTier::Standard,
                approval_status: ApprovalStatus::Pending,
                registered_at: now,
                total_revenue_collected: 0u128,
                total_developer_paid: 0u128,
            };

            DApps::<T>::insert(dapp_id, &dapp);

            DeveloperDApps::<T>::try_mutate(&developer, |list| {
                list.try_push(dapp_id)
                    .map_err(|_| Error::<T>::MaxDAppsReached)
            })?;

            TotalDApps::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::DAppRegistered { dapp_id, developer });
            Ok(())
        }

        /// Approve a pending dApp.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn approve_dapp(origin: OriginFor<T>, dapp_id: DAppId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            DApps::<T>::try_mutate(dapp_id, |opt| -> Result<(), DispatchError> {
                let dapp = opt.as_mut().ok_or(Error::<T>::DAppNotFound)?;
                dapp.approval_status = ApprovalStatus::Approved;
                Ok(())
            })?;

            Self::deposit_event(Event::DAppApproved { dapp_id });
            Ok(())
        }

        /// Reject a dApp.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn reject_dapp(origin: OriginFor<T>, dapp_id: DAppId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            DApps::<T>::try_mutate(dapp_id, |opt| -> Result<(), DispatchError> {
                let dapp = opt.as_mut().ok_or(Error::<T>::DAppNotFound)?;
                dapp.approval_status = ApprovalStatus::Rejected;
                Ok(())
            })?;

            Self::deposit_event(Event::DAppRejected { dapp_id });
            Ok(())
        }

        /// Suspend an approved dApp.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn suspend_dapp(origin: OriginFor<T>, dapp_id: DAppId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            DApps::<T>::try_mutate(dapp_id, |opt| -> Result<(), DispatchError> {
                let dapp = opt.as_mut().ok_or(Error::<T>::DAppNotFound)?;
                dapp.approval_status = ApprovalStatus::Suspended;
                Ok(())
            })?;

            Self::deposit_event(Event::DAppSuspended { dapp_id });
            Ok(())
        }

        /// Update the marketplace placement tier of a dApp.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn set_placement(
            origin: OriginFor<T>,
            dapp_id: DAppId,
            tier: PlacementTier,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            DApps::<T>::try_mutate(dapp_id, |opt| -> Result<(), DispatchError> {
                let dapp = opt.as_mut().ok_or(Error::<T>::DAppNotFound)?;
                dapp.placement = tier.clone();
                Ok(())
            })?;

            Self::deposit_event(Event::PlacementUpdated { dapp_id, tier });
            Ok(())
        }

        /// Record gross revenue for a dApp and distribute according to its policy.
        ///
        /// `GovernanceOrigin` only (called by an off-chain payment processor bridge).
        ///
        /// The developer's share (as defined by the `DeveloperAccount` entry in the
        /// policy) is credited to `DeveloperEarnings` for later withdrawal.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn record_revenue(
            origin: OriginFor<T>,
            dapp_id: DAppId,
            gross_amount: u128,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // Read dapp once, verify approval.
            let dapp = DApps::<T>::get(dapp_id).ok_or(Error::<T>::DAppNotFound)?;
            ensure!(
                dapp.approval_status == ApprovalStatus::Approved,
                Error::<T>::DAppNotApproved
            );

            // Fetch the split policy.
            let policy = RevenuePolicies::<T>::get(dapp.revenue_policy_id)
                .ok_or(Error::<T>::PolicyNotFound)?;

            // Find the developer's share in basis points.
            let developer_bps: u32 = policy.entries[..policy.entries_len as usize]
                .iter()
                .find(|e| e.destination == RevenueDestination::DeveloperAccount)
                .map(|e| e.share_bps)
                .unwrap_or(0);

            let developer_share = gross_amount
                .saturating_mul(developer_bps as u128)
                .checked_div(10_000)
                .unwrap_or(0);
            let protocol_share = gross_amount.saturating_sub(developer_share);

            let developer = dapp.developer.clone();

            // Credit developer earnings.
            DeveloperEarnings::<T>::mutate(&developer, |e| {
                *e = e.saturating_add(developer_share);
            });

            // Update dapp totals.
            DApps::<T>::mutate(dapp_id, |opt| {
                if let Some(d) = opt {
                    d.total_revenue_collected =
                        d.total_revenue_collected.saturating_add(gross_amount);
                    d.total_developer_paid = d.total_developer_paid.saturating_add(developer_share);
                }
            });

            Self::deposit_event(Event::RevenueRecorded {
                dapp_id,
                gross_amount,
                developer_share,
                protocol_share,
            });
            Ok(())
        }

        /// Store or replace a revenue-split policy.
        ///
        /// `GovernanceOrigin` only.  The policy is rejected unless its active
        /// entries sum to exactly 10 000 basis points.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn set_revenue_policy(
            origin: OriginFor<T>,
            policy_id: u32,
            policy: RevenueSplitPolicy,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            ensure!(validate_split(&policy), Error::<T>::InvalidSplitPolicy);

            RevenuePolicies::<T>::insert(policy_id, &policy);

            Self::deposit_event(Event::RevenuePolicySet { policy_id });
            Ok(())
        }

        /// Withdraw accrued earnings.
        ///
        /// Any signed origin (a developer withdrawing their own balance).
        ///
        /// Note: this pallet tracks earnings in storage only; the integrator
        /// must hook an actual fund transfer here for production use.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn withdraw_earnings(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            let developer = ensure_signed(origin)?;

            DeveloperEarnings::<T>::try_mutate(
                &developer,
                |earnings| -> Result<(), DispatchError> {
                    ensure!(*earnings >= amount, Error::<T>::InsufficientEarnings);
                    *earnings = earnings.saturating_sub(amount);
                    Ok(())
                },
            )?;

            Self::deposit_event(Event::EarningsWithdrawn { developer, amount });
            Ok(())
        }
    }
}
