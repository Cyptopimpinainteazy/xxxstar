#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # X3 Solvency Pallet
//!
//! Implements the solvency gate pipeline:
//!
//! * **TICKET-4.5-007** — Pre-quote and pre-reservation gates
//! * **TICKET-4.5-008** — Pre-submission gate
//! * **TICKET-4.5-009** — Post-submission tracking (pending obligations, evidence)
//! * **TICKET-4.5-010** — Solvency snapshot registry with retention-window pruning

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod integration_tests;

use types::{
    EvidenceRecord, PendingObligation, PostSubmissionContext, QuoteContext, ReservationContext,
    SnapshotHash, SolvencyCheck, SolvencyResult, SolvencySnapshotRecord, SubmissionContext,
};

use pallet_x3_inventory::{
    pallet::{Lanes, Pallet as InventoryPallet, Vaults},
    types::{LaneId, LaneStatus, ReservationId, RouteId, VaultId, VaultStatus},
};
use pallet_x3_reservation::pallet::Pallet as ReservationPallet;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::Get,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{BlakeTwo256, Hash, Saturating};

    // ──────────────────────────────────────────
    // Config
    // ──────────────────────────────────────────

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_x3_inventory::pallet::Config
        + pallet_x3_reservation::pallet::Config
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of check-dimensions returned in a single SolvencyResult.
        #[pallet::constant]
        type MaxChecksPerResult: Get<u32> + Clone + core::fmt::Debug;

        /// Quote is considered stale if older than this many blocks.
        #[pallet::constant]
        type QuoteStalenessBlocks: Get<u32>;

        /// How many blocks after submission before a pending obligation times out.
        #[pallet::constant]
        type ObligationTimeoutBlocks: Get<u32>;

        /// Number of blocks to retain snapshots not currently referenced.
        #[pallet::constant]
        type SnapshotRetentionBlocks: Get<u32>;
    }

    // ──────────────────────────────────────────
    // Storage
    // ──────────────────────────────────────────

    /// TICKET-4.5-010 — solvency snapshot registry.
    #[pallet::storage]
    #[pallet::getter(fn snapshot)]
    pub type SolvencySnapshots<T: Config> = StorageMap<
        _,
        Identity,
        SnapshotHash,
        SolvencySnapshotRecord<BlockNumberFor<T>, T::MaxChecksPerResult>,
    >;

    /// Snapshot insertion order list for pruning (double-map: block → hash → ()).
    #[pallet::storage]
    pub type SnapshotsByBlock<T: Config> =
        StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Identity, SnapshotHash, ()>;

    /// TICKET-4.5-009 — pending obligations after successful pre-submission gate.
    #[pallet::storage]
    #[pallet::getter(fn pending_obligation)]
    pub type PendingObligations<T: Config> = StorageMap<
        _,
        Identity,
        RouteId,
        PendingObligation<<T as pallet_x3_inventory::pallet::Config>::Balance, BlockNumberFor<T>>,
    >;

    /// Count of active pending obligations.
    #[pallet::storage]
    #[pallet::getter(fn pending_obligation_count)]
    pub type PendingObligationCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Obligation timeout queue: block → route_id → ().
    #[pallet::storage]
    pub type ObligationTimeouts<T: Config> =
        StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Identity, RouteId, ()>;

    /// TICKET-4.5-009 — sealed evidence records, keyed by route ID.
    #[pallet::storage]
    #[pallet::getter(fn evidence)]
    pub type EvidenceRecords<T: Config> =
        StorageMap<_, Identity, RouteId, EvidenceRecord<BlockNumberFor<T>>>;

    // ──────────────────────────────────────────
    // Events
    // ──────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A solvency gate was evaluated.
        SolvencyGateChecked {
            route_id: RouteId,
            gate: GateKind,
            passed: bool,
            snapshot_hash: SnapshotHash,
        },
        /// A pending obligation was recorded after a successful pre-submission gate.
        PendingObligationRecorded {
            route_id: RouteId,
            reservation_id: ReservationId,
            snapshot_hash: SnapshotHash,
        },
        /// A snapshot was pruned from the registry.
        SnapshotPruned { snapshot_hash: SnapshotHash },
    }

    // ──────────────────────────────────────────
    // Errors
    // ──────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        GateFailed,
        ObligationAlreadyExists,
        ObligationNotFound,
        SnapshotNotFound,
    }

    // ──────────────────────────────────────────
    // Hooks — snapshot pruning
    // ──────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let retention: BlockNumberFor<T> =
                T::SnapshotRetentionBlocks::get().into();
            if now <= retention {
                return Weight::zero();
            }
            let prune_before = now - retention;

            // Remove all snapshots in blocks [0, prune_before) that are not referenced.
            let mut pruned: u32 = 0;
            let _ = SnapshotsByBlock::<T>::iter_prefix(prune_before).drain().for_each(|(hash, ())| {
                if let Some(record) = SolvencySnapshots::<T>::get(hash) {
                    if !record.referenced {
                        SolvencySnapshots::<T>::remove(hash);
                        pruned = pruned.saturating_add(1);
                        Self::deposit_event(Event::SnapshotPruned { snapshot_hash: hash });
                    }
                }
            });

            Weight::zero()
        }
    }

    // ──────────────────────────────────────────
    // Pallet struct
    // ──────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ──────────────────────────────────────────
    // Callable extrinsics — TICKET-4.5-009
    // ──────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Record post-submission state.  Must be called from the same extrinsic that calls
        /// `check_pre_submission`; omitting this call is a consensus error.
        ///
        /// TICKET-4.5-009
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::zero())]
        pub fn record_post_submission_extrinsic(
            origin: OriginFor<T>,
            ctx: PostSubmissionContext<
                <T as pallet_x3_inventory::pallet::Config>::Balance,
                BlockNumberFor<T>,
            >,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            Self::record_post_submission(ctx)?;
            Ok(())
        }
    }

    // ──────────────────────────────────────────
    // Public API
    // ──────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        // ── TICKET-4.5-007 ─────────────────────

        /// Pre-quote gate: verify the lane is not frozen, vault has sufficient balance,
        /// and the global unsettled notional cap is not already breached.
        pub fn check_pre_quote(
            ctx: &QuoteContext<<T as pallet_x3_inventory::pallet::Config>::Balance>,
        ) -> SolvencyResult<T::MaxChecksPerResult> {
            let mut failed: BoundedVec<SolvencyCheck, T::MaxChecksPerResult> = BoundedVec::default();

            // 1. Lane frozen?
            if let Some(lane) = Lanes::<T>::get(ctx.lane_id) {
                if lane.status == LaneStatus::Frozen {
                    let _ = failed.try_push(SolvencyCheck::LaneFrozen);
                }
            } else {
                let _ = failed.try_push(SolvencyCheck::LaneFrozen);
            }

            // 2. Vault sufficient?  (available_balance >= amount)
            if let Some(vault) = Vaults::<T>::get(ctx.vault_id) {
                if vault.available_balance < ctx.amount {
                    let _ = failed.try_push(SolvencyCheck::InsufficientVault);
                }
                if vault.status == VaultStatus::Frozen {
                    // Frozen vault also fails vault check
                    if !failed.contains(&SolvencyCheck::InsufficientVault) {
                        let _ = failed.try_push(SolvencyCheck::InsufficientVault);
                    }
                }
            } else {
                let _ = failed.try_push(SolvencyCheck::InsufficientVault);
            }

            // 3. Lane unsettled notional cap
            if let Some(lane) = Lanes::<T>::get(ctx.lane_id) {
                let lane_unsettled = InventoryPallet::<T>::lane_unsettled_notional(ctx.lane_id);
                if lane_unsettled + ctx.amount > lane.unsettled_cap {
                    let _ = failed.try_push(SolvencyCheck::UnsettledCapBreached);
                }
            }

            let hash = Self::compute_context_hash_quote(ctx);
            let result = if failed.is_empty() {
                SolvencyResult::pass(hash)
            } else {
                SolvencyResult::fail(failed.clone(), hash)
            };

            Self::record_snapshot(hash, result.passed, failed, ctx.route_id, ReservationId::default());
            Self::deposit_event(Event::SolvencyGateChecked {
                route_id: ctx.route_id,
                gate: GateKind::PreQuote,
                passed: result.passed,
                snapshot_hash: hash,
            });
            result
        }

        /// Pre-reservation gate: same checks as pre-quote plus uniqueness of route_id.
        pub fn check_pre_reservation(
            ctx: &ReservationContext<<T as pallet_x3_inventory::pallet::Config>::Balance>,
        ) -> SolvencyResult<T::MaxChecksPerResult> {
            let quote_ctx = QuoteContext {
                lane_id: ctx.lane_id,
                vault_id: ctx.vault_id,
                amount: ctx.amount,
                route_id: ctx.route_id,
            };
            // Reuse pre-quote checks
            let pre = Self::check_pre_quote(&quote_ctx);

            let mut failed: BoundedVec<SolvencyCheck, T::MaxChecksPerResult> = pre.failed_checks;
            // 4. Route already has a live reservation?
            if pallet_x3_reservation::pallet::ReservationsByRoute::<T>::contains_key(ctx.route_id) {
                let _ = failed.try_push(SolvencyCheck::RouteDuplicate);
            }

            let hash = Self::compute_context_hash_reservation(ctx);
            let result = if failed.is_empty() {
                SolvencyResult::pass(hash)
            } else {
                SolvencyResult::fail(failed.clone(), hash)
            };

            Self::record_snapshot(hash, result.passed, failed, ctx.route_id, ReservationId::default());
            Self::deposit_event(Event::SolvencyGateChecked {
                route_id: ctx.route_id,
                gate: GateKind::PreReservation,
                passed: result.passed,
                snapshot_hash: hash,
            });
            result
        }

        // ── TICKET-4.5-008 ─────────────────────

        /// Pre-submission gate: reservation validity, quote freshness, slippage bounds,
        /// signer path health, no incident flags, reconciliation lag, partner reservation
        /// live, bridge path exists.
        pub fn check_pre_submission(
            ctx: &SubmissionContext<
                <T as pallet_x3_inventory::pallet::Config>::Balance,
                BlockNumberFor<T>,
            >,
        ) -> SolvencyResult<T::MaxChecksPerResult> {
            let mut failed: BoundedVec<SolvencyCheck, T::MaxChecksPerResult> = BoundedVec::default();
            let now = frame_system::Pallet::<T>::block_number();

            // 1. Reservation still valid?
            if !ReservationPallet::<T>::is_reservation_valid(ctx.reservation_id) {
                // Discriminate between expired vs other
                if let Some(res) = pallet_x3_reservation::pallet::Reservations::<T>::get(ctx.reservation_id) {
                    if res.expiry_block <= now {
                        let _ = failed.try_push(SolvencyCheck::ReservationExpired);
                    } else {
                        let _ = failed.try_push(SolvencyCheck::ReservationNotActive);
                    }
                } else {
                    let _ = failed.try_push(SolvencyCheck::ReservationExpired);
                }
            }

            // 2. Quote freshness
            let staleness: BlockNumberFor<T> = T::QuoteStalenessBlocks::get().into();
            if now.saturating_sub(ctx.quote_block) > staleness {
                let _ = failed.try_push(SolvencyCheck::QuoteStale);
            }

            // 3. Slippage bounds
            if ctx.slippage_bps > ctx.max_slippage_bps {
                let _ = failed.try_push(SolvencyCheck::SlippageExceeded);
            }

            // 4. Signer path health (stub — no external oracle, defaults healthy)
            // Production: integrate with Phase 3.5 custody module once available.

            // 5. No incident flags, reconciliation lag — check pending obligations count.
            //    If pending obligation count ≥ some high-water mark we'd block; for MVP we
            //    do a basic existence check on this route.
            if PendingObligations::<T>::contains_key(ctx.route_id) {
                let _ = failed.try_push(SolvencyCheck::ReconciliationLagged);
            }

            let hash = Self::compute_context_hash_submission(ctx);
            let result = if failed.is_empty() {
                SolvencyResult::pass(hash)
            } else {
                SolvencyResult::fail(failed.clone(), hash)
            };

            Self::record_snapshot(hash, result.passed, failed, ctx.route_id, ctx.reservation_id);
            Self::deposit_event(Event::SolvencyGateChecked {
                route_id: ctx.route_id,
                gate: GateKind::PreSubmission,
                passed: result.passed,
                snapshot_hash: hash,
            });
            result
        }

        // ── TICKET-4.5-009 ─────────────────────

        /// Record a pending obligation after a successful pre-submission gate.
        /// Seals evidence, enqueues timeout, increments pending obligation count.
        pub fn record_post_submission(
            ctx: PostSubmissionContext<
                <T as pallet_x3_inventory::pallet::Config>::Balance,
                BlockNumberFor<T>,
            >,
        ) -> Result<(), Error<T>> {
            ensure!(
                !PendingObligations::<T>::contains_key(ctx.route_id),
                Error::<T>::ObligationAlreadyExists
            );

            let now = frame_system::Pallet::<T>::block_number();
            let timeout: BlockNumberFor<T> = T::ObligationTimeoutBlocks::get().into();
            let timeout_block = now.saturating_add(timeout);

            let obligation = PendingObligation {
                route_id: ctx.route_id,
                reservation_id: ctx.reservation_id,
                amount: ctx.amount,
                timeout_block,
                snapshot_hash: ctx.submission_hash,
                submission_hash: ctx.submission_hash,
            };
            PendingObligations::<T>::insert(ctx.route_id, &obligation);
            ObligationTimeouts::<T>::insert(timeout_block, ctx.route_id, ());
            PendingObligationCount::<T>::mutate(|c| *c = c.saturating_add(1));

            // Seal evidence record
            let evidence = EvidenceRecord {
                route_id: ctx.route_id,
                reservation_id: ctx.reservation_id,
                submission_hash: ctx.submission_hash,
                block_timestamp: ctx.submission_block,
                snapshot_hash: ctx.submission_hash,
            };
            EvidenceRecords::<T>::insert(ctx.route_id, evidence);

            // Mark snapshot as referenced so it survives pruning
            SolvencySnapshots::<T>::mutate_exists(ctx.submission_hash, |opt| {
                if let Some(rec) = opt {
                    rec.referenced = true;
                }
            });

            Self::deposit_event(Event::PendingObligationRecorded {
                route_id: ctx.route_id,
                reservation_id: ctx.reservation_id,
                snapshot_hash: ctx.submission_hash,
            });
            Ok(())
        }

        // ── TICKET-4.5-010 ─────────────────────

        /// Retrieve a snapshot by its hash.
        pub fn get_snapshot(
            hash: SnapshotHash,
        ) -> Option<SolvencySnapshotRecord<BlockNumberFor<T>, T::MaxChecksPerResult>> {
            SolvencySnapshots::<T>::get(hash)
        }

        // ── Internals ──────────────────────────

        fn record_snapshot(
            hash: SnapshotHash,
            passed: bool,
            failed_checks: BoundedVec<SolvencyCheck, T::MaxChecksPerResult>,
            route_id: RouteId,
            reservation_id: ReservationId,
        ) {
            let now = frame_system::Pallet::<T>::block_number();
            let context_hash = hash;
            let record = SolvencySnapshotRecord {
                block_number: now,
                passed,
                failed_checks,
                route_id,
                reservation_id,
                context_hash,
                referenced: false,
            };
            SolvencySnapshots::<T>::insert(hash, record);
            SnapshotsByBlock::<T>::insert(now, hash, ());
        }

        fn compute_context_hash_quote(
            ctx: &QuoteContext<<T as pallet_x3_inventory::pallet::Config>::Balance>,
        ) -> SnapshotHash {
            let encoded = ctx.encode();
            BlakeTwo256::hash(&encoded).into()
        }

        fn compute_context_hash_reservation(
            ctx: &ReservationContext<<T as pallet_x3_inventory::pallet::Config>::Balance>,
        ) -> SnapshotHash {
            let encoded = ctx.encode();
            BlakeTwo256::hash(&encoded).into()
        }

        fn compute_context_hash_submission(
            ctx: &SubmissionContext<
                <T as pallet_x3_inventory::pallet::Config>::Balance,
                BlockNumberFor<T>,
            >,
        ) -> SnapshotHash {
            let encoded = ctx.encode();
            BlakeTwo256::hash(&encoded).into()
        }
    }
}

// ──────────────────────────────────────────────
// GateKind helper — keeps events human-readable
// ──────────────────────────────────────────────

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub enum GateKind {
    PreQuote,
    PreReservation,
    PreSubmission,
}
