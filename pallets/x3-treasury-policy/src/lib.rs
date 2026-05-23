#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # pallet-x3-treasury-policy
//!
//! **Module 6** of the X3 Phase 4.5 liquidity control plane.
//!
//! Manages allocation caps, vault funding controls, and the insurance reserve.
//!
//! ## Design invariants
//!
//! * Governance sets per-(chain, asset, lane-class) deployment caps via
//!   [`pallet::Call::set_allocation_cap`].
//! * Operator funding actions are pre-checked against the cap; large actions
//!   (above [`pallet::OperatorFundingThreshold`]) are deferred into
//!   [`pallet::PendingGovernanceActions`] and must be explicitly approved or
//!   rejected by the governance origin.
//! * The insurance reserve is a separately tracked balance capped by
//!   [`pallet::Config::MaxInsuranceReserve`]; only governance may move funds in
//!   or out.
//! * This pallet tracks its **own accounting ledger** — it does not call
//!   inventory helpers directly. Balance enforcement happens here before any
//!   caller acts.

pub use pallet::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pallet_x3_inventory::types::{AssetId, ChainId, LaneClass};
use scale_info::TypeInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Public type alias
// ---------------------------------------------------------------------------

/// Treasury accounting denomination.  Fixed as `u128` throughout this pallet
/// so that the insurance cap constant and all balance comparisons share a
/// single concrete type without additional generic parameters.
pub type Balance = u128;

// ---------------------------------------------------------------------------
// Composite storage key
// ---------------------------------------------------------------------------

/// Lookup key for a per-chain / per-asset / per-lane-class allocation cap.
///
/// Used as the composite map key in [`pallet::AllocationCaps`].
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct AllocationCapKey {
    pub chain_id: ChainId,
    pub asset_id: AssetId,
    pub lane_class: LaneClass,
}

// ---------------------------------------------------------------------------
// Public helper
// ---------------------------------------------------------------------------

/// Returns the remaining deployment capacity for the given
/// `(chain_id, asset_id, lane_class)` triple.
///
/// Returns `None` when no allocation cap has been configured for that key —
/// callers should treat absence of a cap as a hard block on all funding.
///
/// When the deployed amount already equals or exceeds the cap (which can
/// happen when caps are tightened after deployment), this returns `Some(0)`.
pub fn remaining_capacity<T: Config>(
    chain_id: ChainId,
    asset_id: AssetId,
    lane_class: LaneClass,
) -> Option<Balance> {
    let key = AllocationCapKey {
        chain_id,
        asset_id,
        lane_class: lane_class.clone(),
    };
    let cap = pallet::AllocationCaps::<T>::get(&key)?;
    let deployed = pallet::TreasuryDeployedByLaneClass::<T>::get(&lane_class);
    Some(cap.saturating_sub(deployed))
}

// ---------------------------------------------------------------------------
// Pallet
// ---------------------------------------------------------------------------

#[frame_support::pallet]
pub mod pallet {
    use super::{AllocationCapKey, Balance};
    use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
    use frame_system::pallet_prelude::*;
    use pallet_x3_inventory::{
        pallet::Vaults as InventoryVaults,
        types::{AssetId, ChainId, LaneClass, VaultId, VaultType},
    };

    // -----------------------------------------------------------------------
    // Pallet struct
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -----------------------------------------------------------------------
    // Config
    // -----------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_x3_inventory::pallet::Config {
        /// Governance origin required for cap expansions above the operator
        /// threshold and for all insurance reserve operations.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Operator origin (e.g. a sudo key or a small multi-sig committee)
        /// for routine vault funding and withdrawals.
        type OperatorOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Hard protocol safety cap on the insurance reserve balance.
        /// Depositing beyond this amount is always rejected regardless of
        /// governance approval.
        #[pallet::constant]
        type MaxInsuranceReserve: Get<u128>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Per-(chain, asset, lane-class) allocation caps set by governance.
    ///
    /// Absence of a cap is treated as a total block on funding for that triple.
    #[pallet::storage]
    pub type AllocationCaps<T: Config> =
        StorageMap<_, Blake2_128Concat, AllocationCapKey, Balance, OptionQuery>;

    /// Running total of capital deployed (committed) per lane class.
    ///
    /// Incremented by approved funding actions; decremented by withdrawals.
    /// Defaults to `0` for any lane class that has never had capital deployed.
    #[pallet::storage]
    pub type TreasuryDeployedByLaneClass<T: Config> =
        StorageMap<_, Blake2_128Concat, LaneClass, Balance, ValueQuery>;

    /// Total treasury balance committed to settlement float vaults across all
    /// lane classes combined.
    #[pallet::storage]
    pub type TotalDeployedSettlementFloat<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// Insurance and loss reserve balance.
    ///
    /// Tracked separately from the operational settlement float.  Only
    /// governance may move funds into or out of this reserve.
    #[pallet::storage]
    pub type InsuranceReserveBalance<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// Per-action threshold: if a single `fund_settlement_vault` call exceeds
    /// this amount, governance approval is required before funds are applied.
    ///
    /// Defaults to `0`, meaning all funding actions require governance until
    /// governance sets a non-zero threshold.
    #[pallet::storage]
    pub type OperatorFundingThreshold<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// Pending governance-gated funding actions.
    ///
    /// Key: `VaultId`.  Value: `(amount, lane_class, submitted_block_number)`.
    ///
    /// An entry is inserted by `fund_settlement_vault` when the amount exceeds
    /// `OperatorFundingThreshold`.  It is removed by either
    /// `approve_governance_action` (which also applies the balance update) or
    /// `reject_governance_action` (which discards it silently).
    #[pallet::storage]
    pub type PendingGovernanceActions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        VaultId,
        (Balance, LaneClass, BlockNumberFor<T>),
        OptionQuery,
    >;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An allocation cap was set or updated for a (chain, asset, lane-class) triple.
        AllocationCapSet { key: AllocationCapKey, cap: Balance },
        /// The operator-level per-action funding threshold was updated.
        OperatorThresholdSet { threshold: Balance },
        /// Capital was committed to a settlement float vault.
        VaultFunded {
            vault_id: VaultId,
            amount: Balance,
            lane_class: LaneClass,
        },
        /// Committed capital was recorded as withdrawn from a settlement float vault.
        VaultWithdrawn {
            vault_id: VaultId,
            amount: Balance,
            lane_class: LaneClass,
        },
        /// A funding request exceeded the operator threshold and was queued for
        /// governance review.  No balance changes were applied yet.
        GovernanceApprovalRequired {
            vault_id: VaultId,
            amount: Balance,
            lane_class: LaneClass,
        },
        /// Governance rejected a pending large funding action without applying it.
        GovernanceActionRejected { vault_id: VaultId },
        /// Funds were added to the insurance reserve.
        InsuranceReserveDeposited {
            amount: Balance,
            new_balance: Balance,
        },
        /// Funds were withdrawn from the insurance reserve.
        InsuranceReserveWithdrawn {
            amount: Balance,
            new_balance: Balance,
        },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// The proposed funding would push the lane-class deployment above its cap.
        AllocationCapExceeded,
        /// No allocation cap has been configured for this (chain, asset, lane-class) triple.
        /// Funding is blocked until governance sets a cap via `set_allocation_cap`.
        AllocationCapNotSet,
        /// Depositing would push the insurance reserve above `MaxInsuranceReserve`.
        InsuranceReserveAtMax,
        /// The requested withdrawal exceeds the current insurance reserve balance.
        InsuranceReserveInsufficient,
        /// Caller attempted to fund an `InsuranceLoss`-typed vault via
        /// `fund_settlement_vault`.  Use the insurance reserve extrinsics instead.
        InsuranceReserveUnreachable,
        /// No pending governance action exists for this vault.
        NoPendingAction,
        /// The referenced vault does not exist in the inventory.
        VaultNotFound,
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set or update the deployment cap for a (chain, asset, lane-class) triple.
        ///
        /// Any subsequent `fund_settlement_vault` call whose cumulative lane
        /// deployment would exceed this cap is rejected with
        /// [`Error::AllocationCapExceeded`].
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn set_allocation_cap(
            origin: OriginFor<T>,
            key: AllocationCapKey,
            cap: Balance,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            AllocationCaps::<T>::insert(&key, cap);
            Self::deposit_event(Event::AllocationCapSet { key, cap });
            Ok(())
        }

        /// Set the per-action threshold above which governance approval is required.
        ///
        /// Any `fund_settlement_vault` call with `amount > threshold` is queued
        /// in [`PendingGovernanceActions`] rather than applied immediately.
        ///
        /// A threshold of `0` means every non-zero funding action requires
        /// governance.
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(30_000_000, 0))]
        pub fn set_operator_funding_threshold(
            origin: OriginFor<T>,
            threshold: Balance,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            OperatorFundingThreshold::<T>::put(threshold);
            Self::deposit_event(Event::OperatorThresholdSet { threshold });
            Ok(())
        }

        /// Fund a settlement float vault.
        ///
        /// Validation steps (in order):
        ///
        /// 1. The vault must exist in the inventory; returns [`Error::VaultNotFound`]
        ///    otherwise.
        /// 2. The vault must **not** be of type `InsuranceLoss`; returns
        ///    [`Error::InsuranceReserveUnreachable`] otherwise.
        /// 3. An allocation cap must be configured for `(chain_id, asset_id,
        ///    lane_class)`; returns [`Error::AllocationCapNotSet`] otherwise.
        /// 4. `TreasuryDeployedByLaneClass[lane_class] + amount` must not exceed
        ///    the cap; returns [`Error::AllocationCapExceeded`] otherwise.
        /// 5. If `amount > OperatorFundingThreshold`, the action is stored in
        ///    [`PendingGovernanceActions`], [`Event::GovernanceApprovalRequired`]
        ///    is emitted, and `Ok(())` is returned without touching balances.
        ///    Otherwise, balances are updated immediately.
        ///
        /// Callable by: **Operator**.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(80_000_000, 0))]
        pub fn fund_settlement_vault(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: Balance,
            lane_class: LaneClass,
            chain_id: ChainId,
            asset_id: AssetId,
        ) -> DispatchResult {
            T::OperatorOrigin::ensure_origin(origin)?;

            // 1 + 2: vault existence and type guard.
            let vault = InventoryVaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            ensure!(
                !matches!(vault.vault_type, VaultType::InsuranceLoss),
                Error::<T>::InsuranceReserveUnreachable
            );

            // 3 + 4: allocation cap check.
            let key = AllocationCapKey {
                chain_id,
                asset_id,
                lane_class: lane_class.clone(),
            };
            let cap = AllocationCaps::<T>::get(&key).ok_or(Error::<T>::AllocationCapNotSet)?;
            let deployed = TreasuryDeployedByLaneClass::<T>::get(&lane_class);
            ensure!(
                deployed.saturating_add(amount) <= cap,
                Error::<T>::AllocationCapExceeded
            );

            // 5: operator threshold gate.
            let threshold = OperatorFundingThreshold::<T>::get();
            if amount > threshold {
                let block = <frame_system::Pallet<T>>::block_number();
                PendingGovernanceActions::<T>::insert(
                    vault_id,
                    (amount, lane_class.clone(), block),
                );
                Self::deposit_event(Event::GovernanceApprovalRequired {
                    vault_id,
                    amount,
                    lane_class,
                });
                return Ok(());
            }

            // Immediate apply: lane_class not moved in the deferred branch above,
            // so ownership is still valid here.
            Self::apply_funding(vault_id, amount, lane_class);
            Ok(())
        }

        /// Approve a pending governance-gated funding action.
        ///
        /// Removes the action from [`PendingGovernanceActions`], applies the
        /// balance updates to [`TreasuryDeployedByLaneClass`] and
        /// [`TotalDeployedSettlementFloat`], and emits [`Event::VaultFunded`].
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(60_000_000, 0))]
        pub fn approve_governance_action(
            origin: OriginFor<T>,
            vault_id: VaultId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            let (amount, lane_class, _submitted_at) =
                PendingGovernanceActions::<T>::take(vault_id).ok_or(Error::<T>::NoPendingAction)?;
            Self::apply_funding(vault_id, amount, lane_class);
            Ok(())
        }

        /// Reject a pending governance-gated funding action.
        ///
        /// Removes the action from [`PendingGovernanceActions`] without applying
        /// any balance changes.  Emits [`Event::GovernanceActionRejected`].
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(40_000_000, 0))]
        pub fn reject_governance_action(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            // `take` atomically removes and returns; None indicates no pending action.
            PendingGovernanceActions::<T>::take(vault_id).ok_or(Error::<T>::NoPendingAction)?;
            Self::deposit_event(Event::GovernanceActionRejected { vault_id });
            Ok(())
        }

        /// Record a withdrawal from a settlement float vault.
        ///
        /// Reduces [`TreasuryDeployedByLaneClass`] for `lane_class` and
        /// [`TotalDeployedSettlementFloat`] by `amount` (saturating to zero).
        /// Does **not** verify that the vault exists — withdrawals always succeed
        /// at the accounting layer to prevent accounting locks in edge cases.
        ///
        /// Callable by: **Operator**.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn withdraw_from_vault(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: Balance,
            lane_class: LaneClass,
        ) -> DispatchResult {
            T::OperatorOrigin::ensure_origin(origin)?;
            TreasuryDeployedByLaneClass::<T>::mutate(&lane_class, |b| {
                *b = b.saturating_sub(amount);
            });
            TotalDeployedSettlementFloat::<T>::mutate(|b| {
                *b = b.saturating_sub(amount);
            });
            Self::deposit_event(Event::VaultWithdrawn {
                vault_id,
                amount,
                lane_class,
            });
            Ok(())
        }

        /// Add funds to the insurance reserve.
        ///
        /// Fails with [`Error::InsuranceReserveAtMax`] if the new balance would
        /// exceed [`Config::MaxInsuranceReserve`] (including overflow).
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(40_000_000, 0))]
        pub fn deposit_insurance_reserve(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            let current = InsuranceReserveBalance::<T>::get();
            let max = T::MaxInsuranceReserve::get();
            // `checked_add` returns None on u128 overflow; `filter` rejects values
            // above the protocol cap.  Either case maps to `InsuranceReserveAtMax`.
            let new_balance = current
                .checked_add(amount)
                .filter(|&n| n <= max)
                .ok_or(Error::<T>::InsuranceReserveAtMax)?;
            InsuranceReserveBalance::<T>::put(new_balance);
            Self::deposit_event(Event::InsuranceReserveDeposited {
                amount,
                new_balance,
            });
            Ok(())
        }

        /// Withdraw funds from the insurance reserve.
        ///
        /// Fails with [`Error::InsuranceReserveInsufficient`] if `amount` exceeds
        /// the current reserve balance.
        ///
        /// Callable by: **Governance**.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(40_000_000, 0))]
        pub fn withdraw_insurance_reserve(origin: OriginFor<T>, amount: Balance) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            let current = InsuranceReserveBalance::<T>::get();
            ensure!(current >= amount, Error::<T>::InsuranceReserveInsufficient);
            let new_balance = current.saturating_sub(amount);
            InsuranceReserveBalance::<T>::put(new_balance);
            Self::deposit_event(Event::InsuranceReserveWithdrawn {
                amount,
                new_balance,
            });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Apply a confirmed (immediately approved or governance-approved) funding
        /// action to the deployment ledger and emit [`Event::VaultFunded`].
        ///
        /// Both counters are updated with saturating arithmetic.
        fn apply_funding(vault_id: VaultId, amount: Balance, lane_class: LaneClass) {
            TreasuryDeployedByLaneClass::<T>::mutate(&lane_class, |b| {
                *b = b.saturating_add(amount);
            });
            TotalDeployedSettlementFloat::<T>::mutate(|b| {
                *b = b.saturating_add(amount);
            });
            Self::deposit_event(Event::VaultFunded {
                vault_id,
                amount,
                lane_class,
            });
        }
    }
}
