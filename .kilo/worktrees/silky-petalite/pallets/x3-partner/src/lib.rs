#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # pallet-x3-partner
//!
//! Partner Capacity Manager (Module 5) for the X3 Phase 4.5 liquidity control plane.
//!
//! ## Responsibilities
//!
//! - Register and lifecycle-manage liquidity partners (active / suspended / terminated).
//! - Maintain a composite health score (0–10 000 bps) derived from five weighted metrics.
//! - Enforce per-partner capital exposure caps; reject reservation additions that would breach them.
//! - Maintain a bidirectional partner ↔ lane approval index.
//! - Expose `is_partner_eligible` for the reservation engine to gate route selection.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic)]
// Allow module-level re-exports and large function signatures common in FRAME pallets.
#![allow(clippy::module_name_repetitions, clippy::too_many_arguments)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;
    use pallet_x3_inventory::types::{LaneId, LaneStatus, PartnerId, PartnerStatus};
    use sp_runtime::traits::Saturating;

    // -----------------------------------------------------------------------
    // Constants
    // -----------------------------------------------------------------------

    /// Partners with a health score below this threshold (50 %) are blocked from new routes
    /// and cannot be reinstated without first raising the score above it.
    pub const MIN_PARTNER_HEALTH_BPS: u32 = 5_000;

    // -----------------------------------------------------------------------
    // PartnerState
    // -----------------------------------------------------------------------

    /// Full state record for a single partner in the capacity manager.
    ///
    /// `Balance` is the same associated type exposed by `pallet_x3_inventory::Config`.
    /// `MaxLanes` is a runtime constant that bounds the `approved_lanes` vector.
    #[derive(
        Clone,
        Debug,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
    )]
    #[scale_info(skip_type_params(MaxLanes))]
    pub struct PartnerState<
        Balance: Encode + Decode + MaxEncodedLen + TypeInfo + Clone + core::fmt::Debug + PartialEq + Eq,
        MaxLanes: Get<u32>,
    > {
        /// Unique partner identifier.
        pub partner_id: PartnerId,
        /// Lifecycle status.
        pub status: PartnerStatus,
        /// Composite health score in basis points (0–10 000, where 10 000 = perfect).
        pub health_score_bps: u32,
        /// Maximum total capital exposure this partner may carry at any one time.
        pub exposure_limit: Balance,
        /// Current outstanding exposure (sum of active reservations attributed to this partner).
        pub current_exposure: Balance,
        /// Quote response time p95 in milliseconds (raw metric; lower is better).
        pub quote_response_time_ms_p95: u32,
        /// Fill reliability in basis points (filled / requested * 10 000; higher is better).
        pub fill_reliability_bps: u32,
        /// Rejected reservation rate in basis points (lower is better).
        pub rejected_reservation_bps: u32,
        /// Stale quote rate in basis points (lower is better).
        pub stale_quote_bps: u32,
        /// Rolling 30-day dispute count (lower is better; each unit costs 500 bps).
        pub dispute_count: u32,
        /// Lane IDs this partner is approved to serve.
        pub approved_lanes: BoundedVec<LaneId, MaxLanes>,
    }

    // -----------------------------------------------------------------------
    // Pallet definition
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -----------------------------------------------------------------------
    // Config
    // -----------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_x3_inventory::pallet::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of lanes a single partner may be approved on.
        #[pallet::constant]
        type MaxApprovedLanesPerPartner: Get<u32>;

        /// Maximum number of partners that may be approved on a single lane.
        #[pallet::constant]
        type MaxPartnersPerLane: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Type aliases for readability
    // -----------------------------------------------------------------------

    type BalanceOf<T> = <T as pallet_x3_inventory::pallet::Config>::Balance;

    type PartnerStateOf<T> = PartnerState<BalanceOf<T>, <T as Config>::MaxApprovedLanesPerPartner>;

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// All registered partners, keyed by `PartnerId`.
    #[pallet::storage]
    #[pallet::getter(fn partners)]
    pub type Partners<T: Config> =
        StorageMap<_, Blake2_128Concat, PartnerId, PartnerStateOf<T>, OptionQuery>;

    /// Reverse index: lane → list of approved partner IDs.
    ///
    /// Used by the reservation engine to enumerate eligible partners for a given lane
    /// without scanning the full `Partners` map.
    #[pallet::storage]
    #[pallet::getter(fn lane_partners)]
    pub type LanePartners<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        LaneId,
        BoundedVec<PartnerId, T::MaxPartnersPerLane>,
        ValueQuery,
    >;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new partner was registered with `exposure_limit` capital allowance.
        PartnerRegistered {
            partner_id: PartnerId,
            exposure_limit: BalanceOf<T>,
        },
        /// Partner health metrics were refreshed; `health_score_bps` is the new composite score.
        PartnerHealthUpdated {
            partner_id: PartnerId,
            health_score_bps: u32,
        },
        /// Health score dropped below [`MIN_PARTNER_HEALTH_BPS`]; partner is blocked.
        HealthBelowThreshold {
            partner_id: PartnerId,
            health_score_bps: u32,
        },
        /// Partner was approved to serve the given lane.
        LaneApproved {
            partner_id: PartnerId,
            lane_id: LaneId,
        },
        /// Partner's approval for the given lane was revoked.
        LaneRevoked {
            partner_id: PartnerId,
            lane_id: LaneId,
        },
        /// Partner's current_exposure was updated (add or subtract).
        ExposureRecorded {
            partner_id: PartnerId,
            current_exposure: BalanceOf<T>,
        },
        /// Partner was suspended; all routes through this partner are blocked.
        PartnerSuspended { partner_id: PartnerId },
        /// Previously-suspended partner was reinstated to active status.
        PartnerReinstated { partner_id: PartnerId },
        /// Partner was permanently terminated and cannot be reinstated.
        PartnerTerminated { partner_id: PartnerId },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// No partner exists with the given `PartnerId`.
        PartnerNotFound,
        /// A partner with that `PartnerId` is already registered.
        PartnerAlreadyExists,
        /// Operation requires the partner to be in `Active` status.
        PartnerNotActive,
        /// Partner health score is below [`MIN_PARTNER_HEALTH_BPS`]; reinstate or improve metrics.
        PartnerUnhealthy,
        /// Adding this exposure amount would exceed the partner's configured `exposure_limit`.
        ExposureCapExceeded,
        /// The given `LaneId` does not exist in `pallet-x3-inventory`.
        LaneNotFound,
        /// Partner is already approved for this lane.
        LaneAlreadyApproved,
        /// Partner is not approved for this lane (cannot revoke a non-existent approval).
        LaneNotApproved,
        /// `approved_lanes` BoundedVec is full; remove a lane before adding another.
        TooManyLanes,
        /// `LanePartners` BoundedVec is full; cannot add another partner to this lane.
        TooManyPartnersOnLane,
        /// A terminated partner cannot be reinstated under any circumstances.
        CannotReinstateTerminated,
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new partner in the capacity manager.
        ///
        /// Creates the partner record with `PartnerStatus::Active`, a perfect health score of
        /// 10 000 bps, and zero current exposure.  Only root may call this.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(60_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn register_partner(
            origin: OriginFor<T>,
            partner_id: PartnerId,
            exposure_limit: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                !Partners::<T>::contains_key(partner_id),
                Error::<T>::PartnerAlreadyExists
            );

            let state = PartnerStateOf::<T> {
                partner_id,
                status: PartnerStatus::Active,
                health_score_bps: 10_000,
                exposure_limit: exposure_limit.clone(),
                current_exposure: <BalanceOf<T> as Default>::default(),
                quote_response_time_ms_p95: 0,
                fill_reliability_bps: 10_000,
                rejected_reservation_bps: 0,
                stale_quote_bps: 0,
                dispute_count: 0,
                approved_lanes: BoundedVec::default(),
            };

            Partners::<T>::insert(partner_id, state);
            Self::deposit_event(Event::PartnerRegistered {
                partner_id,
                exposure_limit,
            });
            Ok(())
        }

        /// Approve a partner to serve a specific lane.
        ///
        /// Validates that the lane exists in `pallet-x3-inventory` before recording the
        /// bidirectional mapping.  Only root may call this.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(70_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2)))]
        pub fn add_approved_lane(
            origin: OriginFor<T>,
            partner_id: PartnerId,
            lane_id: LaneId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Verify the lane is registered in the inventory pallet.
            ensure!(
                pallet_x3_inventory::pallet::Lanes::<T>::contains_key(lane_id),
                Error::<T>::LaneNotFound
            );

            Partners::<T>::try_mutate(partner_id, |maybe_partner| -> DispatchResult {
                let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;

                ensure!(
                    !partner.approved_lanes.contains(&lane_id),
                    Error::<T>::LaneAlreadyApproved
                );

                partner
                    .approved_lanes
                    .try_push(lane_id)
                    .map_err(|_| Error::<T>::TooManyLanes)?;

                Ok(())
            })?;

            LanePartners::<T>::try_mutate(lane_id, |partners_vec| -> DispatchResult {
                ensure!(
                    !partners_vec.contains(&partner_id),
                    Error::<T>::LaneAlreadyApproved
                );
                partners_vec
                    .try_push(partner_id)
                    .map_err(|_| Error::<T>::TooManyPartnersOnLane)?;
                Ok(())
            })?;

            Self::deposit_event(Event::LaneApproved {
                partner_id,
                lane_id,
            });
            Ok(())
        }

        /// Revoke a partner's approval for a lane.
        ///
        /// Removes the lane from the partner's `approved_lanes` and removes the partner from the
        /// `LanePartners` reverse index.  Only root may call this.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(65_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2)))]
        pub fn remove_approved_lane(
            origin: OriginFor<T>,
            partner_id: PartnerId,
            lane_id: LaneId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            Partners::<T>::try_mutate(partner_id, |maybe_partner| -> DispatchResult {
                let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;

                let pos = partner
                    .approved_lanes
                    .iter()
                    .position(|id| id == &lane_id)
                    .ok_or(Error::<T>::LaneNotApproved)?;

                partner.approved_lanes.remove(pos);
                Ok(())
            })?;

            LanePartners::<T>::mutate(lane_id, |partners_vec| {
                if let Some(pos) = partners_vec.iter().position(|id| id == &partner_id) {
                    partners_vec.remove(pos);
                }
            });

            Self::deposit_event(Event::LaneRevoked {
                partner_id,
                lane_id,
            });
            Ok(())
        }

        /// Update raw performance metrics for a partner and recompute the composite health score.
        ///
        /// Health formula (all components 0–10 000, weighted average with denominator 9):
        ///
        /// ```text
        /// fill_score        = fill_reliability_bps
        /// rejection_score   = 10_000 - min(rejected_reservation_bps, 10_000)
        /// stale_score       = 10_000 - min(stale_quote_bps, 10_000)
        /// dispute_score     = max(0, 10_000 - dispute_count * 500)
        /// response_score    = max(0, 10_000 - quote_response_time_ms_p95 * 10)
        /// health = (fill_score * 3 + rejection_score * 2 + stale_score + dispute_score * 2 + response_score) / 9
        /// ```
        ///
        /// If the resulting score is below [`MIN_PARTNER_HEALTH_BPS`], a
        /// `HealthBelowThreshold` event is additionally emitted.  Only root may call this.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(55_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn update_health_metrics(
            origin: OriginFor<T>,
            partner_id: PartnerId,
            quote_response_time_ms_p95: u32,
            fill_reliability_bps: u32,
            rejected_reservation_bps: u32,
            stale_quote_bps: u32,
            dispute_count: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let health_score_bps = Partners::<T>::try_mutate(
                partner_id,
                |maybe_partner| -> Result<u32, DispatchError> {
                    let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;

                    partner.quote_response_time_ms_p95 = quote_response_time_ms_p95;
                    partner.fill_reliability_bps = fill_reliability_bps;
                    partner.rejected_reservation_bps = rejected_reservation_bps;
                    partner.stale_quote_bps = stale_quote_bps;
                    partner.dispute_count = dispute_count;

                    let score = Self::compute_health_score(
                        quote_response_time_ms_p95,
                        fill_reliability_bps,
                        rejected_reservation_bps,
                        stale_quote_bps,
                        dispute_count,
                    );
                    partner.health_score_bps = score;
                    Ok(score)
                },
            )?;

            Self::deposit_event(Event::PartnerHealthUpdated {
                partner_id,
                health_score_bps,
            });

            if health_score_bps < MIN_PARTNER_HEALTH_BPS {
                Self::deposit_event(Event::HealthBelowThreshold {
                    partner_id,
                    health_score_bps,
                });
            }

            Ok(())
        }

        /// Record a change in the partner's current capital exposure.
        ///
        /// `is_add = true` increases `current_exposure`; `is_add = false` decreases it
        /// (saturating at zero).  Returns `ExposureCapExceeded` if adding `amount` would push
        /// `current_exposure` above `exposure_limit`.
        ///
        /// Any signed origin is accepted so that the reservation engine (or other runtime
        /// modules) can call this without needing root privileges.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn record_exposure(
            origin: OriginFor<T>,
            partner_id: PartnerId,
            amount: BalanceOf<T>,
            is_add: bool,
        ) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            let new_exposure = Partners::<T>::try_mutate(
                partner_id,
                |maybe_partner| -> Result<BalanceOf<T>, DispatchError> {
                    let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;

                    if is_add {
                        let new_val = partner.current_exposure.saturating_add(amount);
                        ensure!(
                            new_val <= partner.exposure_limit,
                            Error::<T>::ExposureCapExceeded
                        );
                        partner.current_exposure = new_val;
                    } else {
                        partner.current_exposure = partner.current_exposure.saturating_sub(amount);
                    }

                    Ok(partner.current_exposure.clone())
                },
            )?;

            Self::deposit_event(Event::ExposureRecorded {
                partner_id,
                current_exposure: new_exposure,
            });
            Ok(())
        }

        /// Suspend a partner, blocking all route selection through this partner.
        ///
        /// A suspended partner can be reinstated via [`reinstate_partner`] if its health score
        /// is above [`MIN_PARTNER_HEALTH_BPS`].  Only root may call this.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(45_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn suspend_partner(origin: OriginFor<T>, partner_id: PartnerId) -> DispatchResult {
            ensure_root(origin)?;

            Partners::<T>::try_mutate(partner_id, |maybe_partner| -> DispatchResult {
                let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;
                partner.status = PartnerStatus::Suspended;
                Ok(())
            })?;

            Self::deposit_event(Event::PartnerSuspended { partner_id });
            Ok(())
        }

        /// Reinstate a previously-suspended partner to `Active` status.
        ///
        /// Fails if:
        /// - The partner does not exist.
        /// - The partner is `Terminated` (permanent; cannot be reinstated).
        /// - The partner's current health score is below [`MIN_PARTNER_HEALTH_BPS`].
        ///
        /// Only root may call this.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn reinstate_partner(origin: OriginFor<T>, partner_id: PartnerId) -> DispatchResult {
            ensure_root(origin)?;

            Partners::<T>::try_mutate(partner_id, |maybe_partner| -> DispatchResult {
                let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;

                ensure!(
                    partner.status != PartnerStatus::Terminated,
                    Error::<T>::CannotReinstateTerminated
                );
                ensure!(
                    partner.health_score_bps >= MIN_PARTNER_HEALTH_BPS,
                    Error::<T>::PartnerUnhealthy
                );

                partner.status = PartnerStatus::Active;
                Ok(())
            })?;

            Self::deposit_event(Event::PartnerReinstated { partner_id });
            Ok(())
        }

        /// Permanently terminate a partner.
        ///
        /// Termination is irreversible: subsequent calls to [`reinstate_partner`] will return
        /// [`Error::CannotReinstateTerminated`].  Only root may call this.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(45_000_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1)))]
        pub fn terminate_partner(origin: OriginFor<T>, partner_id: PartnerId) -> DispatchResult {
            ensure_root(origin)?;

            Partners::<T>::try_mutate(partner_id, |maybe_partner| -> DispatchResult {
                let partner = maybe_partner.as_mut().ok_or(Error::<T>::PartnerNotFound)?;
                partner.status = PartnerStatus::Terminated;
                Ok(())
            })?;

            Self::deposit_event(Event::PartnerTerminated { partner_id });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Compute the composite health score from the five raw performance metrics.
        ///
        /// All intermediate component scores are in the range `[0, 10_000]` (basis points).
        /// The weighted average uses denominator 9 to keep integer arithmetic simple.
        ///
        /// | Component        | Weight | Direction |
        /// |-----------------|--------|-----------|
        /// | fill_score      |   3    | higher → better |
        /// | rejection_score |   2    | lower rejected bps → higher score |
        /// | stale_score     |   1    | lower stale bps → higher score |
        /// | dispute_score   |   2    | fewer disputes → higher score |
        /// | response_score  |   1    | lower latency → higher score |
        fn compute_health_score(
            quote_response_time_ms_p95: u32,
            fill_reliability_bps: u32,
            rejected_reservation_bps: u32,
            stale_quote_bps: u32,
            dispute_count: u32,
        ) -> u32 {
            let fill_score: u32 = fill_reliability_bps.min(10_000);

            let rejection_score: u32 = 10_000u32.saturating_sub(rejected_reservation_bps.min(10_000));

            let stale_score: u32 = 10_000u32.saturating_sub(stale_quote_bps.min(10_000));

            // Each dispute costs 500 bps; saturate at 0 so 20+ disputes do not underflow.
            let dispute_penalty: u32 = dispute_count.saturating_mul(500).min(10_000);
            let dispute_score: u32 = 10_000u32.saturating_sub(dispute_penalty);

            // Each millisecond of p95 latency costs 10 bps; saturate at 0 above 1 000 ms.
            let response_penalty: u32 = quote_response_time_ms_p95.saturating_mul(10).min(10_000);
            let response_score: u32 = 10_000u32.saturating_sub(response_penalty);

            // Weighted sum: weights = [3, 2, 1, 2, 1], denominator = 9.
            let weighted_sum: u32 = fill_score
                .saturating_mul(3)
                .saturating_add(rejection_score.saturating_mul(2))
                .saturating_add(stale_score)
                .saturating_add(dispute_score.saturating_mul(2))
                .saturating_add(response_score);

            weighted_sum / 9
        }
    }

    // -----------------------------------------------------------------------
    // Public non-dispatchable helper
    // -----------------------------------------------------------------------

    /// Returns `true` if the partner is eligible to fill a route on the specified lane.
    ///
    /// Eligibility requires all three of:
    /// 1. The partner exists and has `PartnerStatus::Active`.
    /// 2. The partner's health score is at or above [`MIN_PARTNER_HEALTH_BPS`].
    /// 3. The lane is present in the partner's `approved_lanes` list.
    ///
    /// Additionally, if the lane's status in the inventory pallet is `Frozen`, this function
    /// returns `false` regardless of partner state.
    ///
    /// This function performs only read-only storage accesses and never panics.
    pub fn is_partner_eligible<T: Config>(partner_id: PartnerId, lane_id: LaneId) -> bool {
        // Lane must exist and not be frozen.
        let lane_ok = pallet_x3_inventory::pallet::Lanes::<T>::get(lane_id)
            .map(|lane| lane.status != LaneStatus::Frozen)
            .unwrap_or(false);

        if !lane_ok {
            return false;
        }

        // Partner must be active, healthy, and approved for the lane.
        Partners::<T>::get(partner_id)
            .map(|p| {
                p.status == PartnerStatus::Active
                    && p.health_score_bps >= MIN_PARTNER_HEALTH_BPS
                    && p.approved_lanes.contains(&lane_id)
            })
            .unwrap_or(false)
    }
}
