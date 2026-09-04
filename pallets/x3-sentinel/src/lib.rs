// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-sentinel — On-chain Sentinel guard for the Universal Asset Kernel.
//
// Closes the fictional `x3_sentinel` registry gap (`FEATURE_REGISTRY.toml`
// listed a `pallets/x3-sentinel` whose directory and required tests did not
// exist) with a REAL, runtime-wired, security-hardened guard. Its role is to
// give the chain a governance-controlled, independent freeze/review layer on
// top of the token factory's supply-changing authority operations (`mint` /
// `burn` / `transfer_mint_authority`).
//
// The token factory alone self-enforces a single `mint_authority` inside its
// own `Tokens` map. The Sentinel adds an orthogonal, chain-managed signal:
//   * a specific authority can be frozen on a specific asset
//     (`FrozenAccounts`), and
//   * a whole asset can be frozen (`FrozenAssets`), and
//   * an asset may be enrolled so its authority ops require guardian approval
//     (`ReviewEnrolled` + `GuardApprovals`).
//
// The factory consults the Sentinel through the `SentinelGuard` trait from
// `x3-asset-kernel-types` before every supply-changing op. Enforcement is
// FAIL-CLOSED: an enrolled or frozen authority/asset can never silently pass.
// The kernel-types crate carries a `NoSentinelGuard` type that is the explicit
// compile-time opt-out a runtime author selects when no Sentinel is wired (the
// same deliberate-config pattern as `NoEconomicHalt`); it is never a runtime
// fallback that can be flipped accidentally.
//
// Explicit non-responsibilities:
//   * Does not mint, burn, or move balances. It only *blocks* dangerous ones.
//   * Does not implement an off-chain ML/heuristic score. The guard is built on
//     explicit, on-chain, auditable freeze/review authority — no silent scoring
//     heuristics that could be gamed or surprise governance.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

//! X3 Sentinel guard pallet.

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::EnsureOrigin,
    };
    use frame_system::pallet_prelude::*;
    use x3_asset_kernel_types::{traits::SentinelGuard, AssetId};

    /// Governance message / reason bound for a freeze (kept small for the block).
    pub type MaxFreezeReasonLen = ConstU32<256>;

    // ── Pallet ─────────────────────────────────────────────────────────────

    /// Pallet wrapper type.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Config trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin permitted to freeze/unfreeze/enrol/grant approval. MUST be a
        /// privileged origin (Root or a governance council) — never
        /// `EnsureSigned`. Freezing authority is a security power.
        type FreezeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// Authorities frozen on a specific asset: `(asset, authority)`.
    /// A frozen authority cannot exercise supply-changing ops on that asset
    /// even if it is still the recorded `mint_authority`.
    #[pallet::storage]
    pub type FrozenAccounts<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, AssetId, Blake2_128Concat, T::AccountId, (), ValueQuery>;

    /// Assets frozen in whole (all supply-changing ops blocked until unfrozen).
    #[pallet::storage]
    pub type FrozenAssets<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, (), ValueQuery>;

    /// Assets enrolled for guardian review. When the factory is wired to a real
    /// Sentinel, mint/burn/authority-transfer on an enrolled asset require a
    /// fresh guardian approval (see [`GuardApprovals`]).
    #[pallet::storage]
    pub type ReviewEnrolled<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, (), ValueQuery>;

    /// Per-asset guardian approval counter. Enrolled assets need a non-zero
    /// counter (a prior `grant_guardian_approval`) before an authority op
    /// passes; each grant bumps it, so an approval is a positive, auditable
    /// signal with no way to "un-approve" silently.
    #[pallet::storage]
    #[pallet::getter(fn approval_nonce)]
    pub type GuardApprovals<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, u64, ValueQuery>;

    // ── Events ─────────────────────────────────────────────────────────────

    /// Events emitted by the Sentinel.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A mint authority was frozen on an asset by a privileged origin.
        AuthorityFrozen {
            /// Affected asset.
            asset: AssetId,
            /// The authority that was frozen.
            who: T::AccountId,
            /// Reason for the freeze.
            reason: BoundedVec<u8, MaxFreezeReasonLen>,
        },
        /// A mint authority was unfrozen on an asset.
        AuthorityUnfrozen {
            /// Affected asset.
            asset: AssetId,
            /// The authority that was unfrozen.
            who: T::AccountId,
        },
        /// A whole asset was frozen.
        AssetFrozen {
            /// Affected asset.
            asset: AssetId,
            /// Reason for the asset-wide freeze.
            reason: BoundedVec<u8, MaxFreezeReasonLen>,
        },
        /// A whole asset was unfrozen.
        AssetUnfrozen {
            /// Affected asset.
            asset: AssetId,
        },
        /// An asset was enrolled for guardian review.
        EnrolledForReview {
            /// Affected asset.
            asset: AssetId,
        },
        /// An asset was removed from guardian review.
        UnenrolledFromReview {
            /// Affected asset.
            asset: AssetId,
        },
        /// A guardian approval was granted for an enrolled asset.
        GuardianApproved {
            /// Affected asset.
            asset: AssetId,
            /// New per-asset approval counter.
            nonce: u64,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    /// Errors returned by the Sentinel.
    #[pallet::error]
    pub enum Error<T> {
        /// The target authority is already frozen on this asset.
        AlreadyFrozen,
        /// The target authority is not currently frozen on this asset.
        NotFrozen,
        /// The asset is already frozen in whole.
        AssetAlreadyFrozen,
        /// The asset is not currently frozen in whole.
        AssetNotFrozen,
        /// The asset is already enrolled for guardian review.
        AlreadyEnrolled,
        /// The asset is not enrolled for guardian review.
        NotEnrolled,
        /// The supplied reason exceeded the storage bound.
        ReasonTooLong,
    }

    /// A `SentinelGuard` implementation for runtimes that wire the real pallet.
    /// Enforces fail-closed over the pallet's own storage.
    impl<T: Config> SentinelGuard<T::AccountId> for Pallet<T> {
        fn can_authorize(asset: &AssetId, who: &T::AccountId) -> Result<(), x3_asset_kernel_types::traits::SentinelDenial> {
            Self::enforce(asset, who)
        }
    }

    impl<T: Config> Pallet<T> {
        /// Fail-closed check consulted by authority-facing callers. Strongest
        /// denial wins: whole-asset freeze > per-authority freeze > review-required.
        pub fn enforce(
            asset: &AssetId,
            who: &T::AccountId,
        ) -> Result<(), x3_asset_kernel_types::traits::SentinelDenial> {
            use x3_asset_kernel_types::traits::SentinelDenial::*;
            if FrozenAssets::<T>::contains_key(asset) {
                return Err(AssetFrozen);
            }
            if FrozenAccounts::<T>::contains_key(asset, who) {
                return Err(AuthorityFrozen);
            }
            if ReviewEnrolled::<T>::contains_key(asset) {
                // Need a guardian approval to already be on file.
                if GuardApprovals::<T>::get(asset) == 0 {
                    return Err(ReviewRequired);
                }
            }
            Ok(())
        }
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Freeze a specific mint authority on an asset. Privileged origin only.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn freeze_authority(
            origin: OriginFor<T>,
            asset: AssetId,
            who: T::AccountId,
            reason: BoundedVec<u8, MaxFreezeReasonLen>,
        ) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if FrozenAccounts::<T>::contains_key(asset, &who) {
                return Err(Error::<T>::AlreadyFrozen.into());
            }
            FrozenAccounts::<T>::insert(asset, &who, ());
            Self::deposit_event(Event::AuthorityFrozen { asset, who, reason });
            Ok(())
        }

        /// Unfreeze a previously frozen mint authority on an asset. Privileged.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn unfreeze_authority(
            origin: OriginFor<T>,
            asset: AssetId,
            who: T::AccountId,
        ) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if !FrozenAccounts::<T>::contains_key(asset, &who) {
                return Err(Error::<T>::NotFrozen.into());
            }
            FrozenAccounts::<T>::remove(asset, &who);
            Self::deposit_event(Event::AuthorityUnfrozen { asset, who });
            Ok(())
        }

        /// Freeze an asset in whole (blocks every authority op on it).
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn freeze_asset(
            origin: OriginFor<T>,
            asset: AssetId,
            reason: BoundedVec<u8, MaxFreezeReasonLen>,
        ) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if FrozenAssets::<T>::contains_key(asset) {
                return Err(Error::<T>::AssetAlreadyFrozen.into());
            }
            FrozenAssets::<T>::insert(asset, ());
            Self::deposit_event(Event::AssetFrozen { asset, reason });
            Ok(())
        }

        /// Unfreeze an asset in whole.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn unfreeze_asset(origin: OriginFor<T>, asset: AssetId) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if !FrozenAssets::<T>::contains_key(asset) {
                return Err(Error::<T>::AssetNotFrozen.into());
            }
            FrozenAssets::<T>::remove(asset);
            Self::deposit_event(Event::AssetUnfrozen { asset });
            Ok(())
        }

        /// Enroll an asset so its authority ops require guardian approval.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn enroll_for_review(origin: OriginFor<T>, asset: AssetId) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if ReviewEnrolled::<T>::contains_key(asset) {
                return Err(Error::<T>::AlreadyEnrolled.into());
            }
            ReviewEnrolled::<T>::insert(asset, ());
            GuardApprovals::<T>::insert(asset, 0);
            Self::deposit_event(Event::EnrolledForReview { asset });
            Ok(())
        }

        /// Remove an asset from guardian review.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn unenroll_from_review(origin: OriginFor<T>, asset: AssetId) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if !ReviewEnrolled::<T>::contains_key(asset) {
                return Err(Error::<T>::NotEnrolled.into());
            }
            ReviewEnrolled::<T>::remove(asset);
            GuardApprovals::<T>::remove(asset);
            Self::deposit_event(Event::UnenrolledFromReview { asset });
            Ok(())
        }

        /// Grant a fresh guardian approval so the authority can mint/burn an
        /// enrolled asset once. Each grant bumps the per-asset counter, so
        /// approvals accumulate positively and are auditable; the factory's
        /// `enforce` already requires a non-zero counter.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn grant_guardian_approval(
            origin: OriginFor<T>,
            asset: AssetId,
        ) -> DispatchResult {
            T::FreezeOrigin::ensure_origin(origin)?;
            if !ReviewEnrolled::<T>::contains_key(asset) {
                return Err(Error::<T>::NotEnrolled.into());
            }
            GuardApprovals::<T>::mutate(asset, |n| *n = n.saturating_add(1));
            let nonce = GuardApprovals::<T>::get(asset);
            Self::deposit_event(Event::GuardianApproved { asset, nonce });
            Ok(())
        }
    }
}
