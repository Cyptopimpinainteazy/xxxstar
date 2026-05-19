#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
//! # pallet-x3-reconciliation
//!
//! Supply reconciliation and governance power mapping for the X3 wrapped-token
//! system (Phase 5 of the X3 Wrapped Token Specification).
//!
//! ## Specification alignment
//!
//! - Canonical supply = non-wrapped treasury X3 + sum of wrapped X3 on all chains.
//! - 24-hour reconciliation cycles with 0.01 % tolerance (0.1 % = degraded,
//!   >0.1 % unresolved after 1 hr → halt minting).
//! - Governance power = token balance; same weight regardless of chain.
//! - Automatic alert if power diverges >1 % across chain tallies.
//! - All minting halts if supply divergence unresolved after 1 hour (~600 blocks at 6 s).

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// ── Public types (outside pallet macro so callers can import them) ────────────

/// Status enum returned by reconciliation checks.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    codec::Encode,
    codec::Decode,
    codec::DecodeWithMemTracking,
    scale_info::TypeInfo,
    codec::MaxEncodedLen,
    sp_runtime::RuntimeDebug,
)]
pub enum ReconciliationStatus {
    /// Divergence within tolerance (≤ ToleranceBps).
    Passing,
    /// Divergence exceeds tolerance but minting is not yet halted.
    Degraded,
    /// Divergence persisted past `MintHaltThresholdBlocks`; minting is halted.
    Halted,
}

// ── FRAME pallet ──────────────────────────────────────────────────────────────

#[frame_support::pallet]
pub mod pallet {
    use super::ReconciliationStatus;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    // ── Data types ────────────────────────────────────────────────────────────

    /// Per-chain supply report submitted by a trusted bridge validator.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        TypeInfo,
        MaxEncodedLen,
        RuntimeDebug,
    )]
    pub struct ChainSupplyReport<BlockNumber> {
        /// Identifier of the remote chain.
        pub chain_id: u32,
        /// Amount of wrapped X3 currently circulating on that chain.
        pub wrapped_supply: u128,
        /// Block number at which this report was submitted.
        pub reported_at: BlockNumber,
    }

    /// Outcome of a single reconciliation run.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        TypeInfo,
        MaxEncodedLen,
        RuntimeDebug,
    )]
    pub struct ReconciliationRecord<BlockNumber> {
        /// The canonical supply at the time of reconciliation.
        pub canonical_supply: u128,
        /// Sum of all wrapped supplies across all chains.
        pub sum_wrapped: u128,
        /// Absolute divergence expressed in basis points (1 bps = 0.01 %).
        pub divergence_bps: u32,
        /// `true` when divergence_bps ≤ T::ToleranceBps.
        pub passed: bool,
        /// `true` when this run triggered a mint halt.
        pub halted_minting: bool,
        /// Block number at which this record was produced.
        pub executed_at: BlockNumber,
    }

    // ── Pallet struct ─────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Config ────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin allowed to submit chain supply reports, update canonical supply,
        /// update governance power, and lift mint halts.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Upper bound on the number of chains tracked simultaneously.
        type MaxSupportedChains: Get<u32>;
        /// Blocks in one reconciliation cycle (e.g. 14 400 ≈ 24 h at 6 s).
        type ReconciliationCycleBlocks: Get<BlockNumberFor<Self>>;
        /// Blocks before an unresolved divergence triggers a mint halt
        /// (e.g. 600 ≈ 1 h at 6 s).
        type MintHaltThresholdBlocks: Get<BlockNumberFor<Self>>;
        /// Maximum acceptable divergence in basis points (1 = 0.01 %).
        type ToleranceBps: Get<u32>;
        /// Basis-point threshold above which a governance power imbalance alert is
        /// emitted (100 = 1 %).
        type GovernanceDivergenceAlertBps: Get<u32>;
    }

    // ── Storage ───────────────────────────────────────────────────────────────

    /// Per-chain wrapped supply reports: chain_id → (report).
    #[pallet::storage]
    pub type ChainSupplyReports<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        ChainSupplyReport<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Canonical supply as declared by governance / trusted bridge validators.
    #[pallet::storage]
    pub type CanonicalSupply<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Most recent reconciliation record.
    #[pallet::storage]
    pub type LastReconciliation<T: Config> =
        StorageValue<_, ReconciliationRecord<BlockNumberFor<T>>, OptionQuery>;

    /// If `Some(block)`, minting was halted at that block due to unresolved
    /// supply divergence.
    #[pallet::storage]
    pub type MintHaltSince<T: Config> = StorageValue<_, Option<BlockNumberFor<T>>, ValueQuery>;

    /// Governance power reported per chain.
    #[pallet::storage]
    pub type GovernancePowerByChain<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, u128, ValueQuery>;

    /// Total aggregated governance power (sum of all chains).
    #[pallet::storage]
    pub type TotalGovernancePower<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Block number of the most recent governance-power aggregation.
    #[pallet::storage]
    pub type LastPowerReconciliation<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A chain supply report was submitted.
        ChainSupplyReported {
            chain_id: u32,
            wrapped_supply: u128,
            reported_at: BlockNumberFor<T>,
        },
        /// The canonical supply was updated by governance.
        CanonicalSupplyUpdated { new_canonical: u128 },
        /// A reconciliation cycle completed.
        ReconciliationExecuted {
            divergence_bps: u32,
            passed: bool,
        },
        /// Minting was halted because supply divergence was not resolved in time.
        MintHaltTriggered { at_block: BlockNumberFor<T> },
        /// Governance lifted the mint halt after divergence fell within tolerance.
        MintHaltLifted { at_block: BlockNumberFor<T> },
        /// Governance power for a chain was updated.
        GovernancePowerUpdated { chain_id: u32, power: u128 },
        /// Governance power diverged by more than `GovernanceDivergenceAlertBps` across chains.
        GovernancePowerDivergenceAlert { max_divergence_bps: u32 },
        /// All chain power entries were re-summed into `TotalGovernancePower`.
        GovernancePowerAggregated { total: u128 },
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// The caller is not a governance origin.
        UnauthorizedOrigin,
        /// The referenced chain has no registered supply report.
        ChainNotRegistered,
        /// Divergence is above tolerance; the requested action is rejected.
        DivergenceTooHigh,
        /// A mint halt is already in progress.
        MintCurrentlyHalted,
        /// Attempted to lift a halt that is not active.
        HaltNotActive,
    }

    // ── Hooks ─────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Each block: if divergence has been above tolerance for at least
        /// `MintHaltThresholdBlocks` and no halt is currently active, trigger
        /// a mint halt automatically.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Only act when no halt is already in force.
            if MintHaltSince::<T>::get().is_some() {
                return Weight::zero();
            }

            if let Some(record) = LastReconciliation::<T>::get() {
                if record.divergence_bps > T::ToleranceBps::get() {
                    let elapsed = now.saturating_sub(record.executed_at);
                    if elapsed >= T::MintHaltThresholdBlocks::get() {
                        MintHaltSince::<T>::put(Some(now));
                        Self::deposit_event(Event::MintHaltTriggered { at_block: now });
                        return Weight::from_parts(10_000, 0);
                    }
                }
            }

            Weight::zero()
        }
    }

    // ── Calls ─────────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a wrapped supply report for a given chain.
        ///
        /// Only governance is permitted.  Overwrites any previous report for
        /// the same chain.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn submit_chain_supply_report(
            origin: OriginFor<T>,
            chain_id: u32,
            wrapped_supply: u128,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            let report = ChainSupplyReport {
                chain_id,
                wrapped_supply,
                reported_at: now,
            };
            ChainSupplyReports::<T>::insert(chain_id, report);
            Self::deposit_event(Event::ChainSupplyReported {
                chain_id,
                wrapped_supply,
                reported_at: now,
            });
            Ok(())
        }

        /// Set the canonical supply (non-wrapped treasury X3 base).
        ///
        /// Only governance is permitted.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn set_canonical_supply(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            CanonicalSupply::<T>::put(amount);
            Self::deposit_event(Event::CanonicalSupplyUpdated { new_canonical: amount });
            Ok(())
        }

        /// Run a reconciliation cycle.
        ///
        /// Open to any origin.  Computes the divergence between canonical supply
        /// and the sum of all wrapped chain reports, updates `LastReconciliation`,
        /// and may trigger a halt via the `on_initialize` hook on the next block.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(30_000, 0))]
        pub fn run_reconciliation(origin: OriginFor<T>) -> DispatchResult {
            let _who = ensure_signed_or_root(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            let canonical = CanonicalSupply::<T>::get();

            // Sum all wrapped supplies.
            let sum_wrapped: u128 = ChainSupplyReports::<T>::iter()
                .map(|(_, r)| r.wrapped_supply)
                .fold(0u128, |acc, v| acc.saturating_add(v));

            // Compute divergence in basis points relative to canonical supply.
            let divergence_bps = if canonical == 0 {
                // Avoid division by zero; treat as zero divergence when no supply reported.
                0u32
            } else {
                let diff = if canonical >= sum_wrapped {
                    canonical - sum_wrapped
                } else {
                    sum_wrapped - canonical
                };
                // divergence_bps = diff * 10_000 / canonical, saturating
                let bps = diff.saturating_mul(10_000) / canonical;
                // Cap at u32::MAX in the extreme case.
                bps.min(u32::MAX as u128) as u32
            };

            let passed = divergence_bps <= T::ToleranceBps::get();
            // Determine whether this run itself triggers a halt.
            // The halt may also be triggered lazily by on_initialize on a later
            // block; here we handle the immediate case where halt threshold = 0.
            let halted_minting = !passed
                && T::MintHaltThresholdBlocks::get() == BlockNumberFor::<T>::from(0u32)
                && MintHaltSince::<T>::get().is_none();

            if halted_minting {
                MintHaltSince::<T>::put(Some(now));
                Self::deposit_event(Event::MintHaltTriggered { at_block: now });
            }

            let record = ReconciliationRecord {
                canonical_supply: canonical,
                sum_wrapped,
                divergence_bps,
                passed,
                halted_minting,
                executed_at: now,
            };
            LastReconciliation::<T>::put(record);
            Self::deposit_event(Event::ReconciliationExecuted { divergence_bps, passed });
            Ok(())
        }

        /// Lift an active mint halt.
        ///
        /// Only governance is permitted.  Rejected if current divergence still
        /// exceeds `ToleranceBps`.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn lift_mint_halt(origin: OriginFor<T>) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            // Must have an active halt.
            ensure!(MintHaltSince::<T>::get().is_some(), Error::<T>::HaltNotActive);

            // Reject if divergence is still above tolerance.
            if let Some(record) = LastReconciliation::<T>::get() {
                ensure!(
                    record.divergence_bps <= T::ToleranceBps::get(),
                    Error::<T>::DivergenceTooHigh
                );
            }

            let now = frame_system::Pallet::<T>::block_number();
            MintHaltSince::<T>::put::<Option<BlockNumberFor<T>>>(None);
            Self::deposit_event(Event::MintHaltLifted { at_block: now });
            Ok(())
        }

        /// Update the governance power contribution of a single chain.
        ///
        /// Recomputes `TotalGovernancePower` and emits a divergence alert if
        /// any single chain power deviates from the per-chain average by more
        /// than `GovernanceDivergenceAlertBps`.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn update_governance_power(
            origin: OriginFor<T>,
            chain_id: u32,
            power: u128,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            GovernancePowerByChain::<T>::insert(chain_id, power);

            // Recompute total.
            let total: u128 = GovernancePowerByChain::<T>::iter()
                .map(|(_, p)| p)
                .fold(0u128, |acc, v| acc.saturating_add(v));
            TotalGovernancePower::<T>::put(total);

            Self::deposit_event(Event::GovernancePowerUpdated { chain_id, power });

            // Check divergence: compare each chain's power share to the average.
            let count = GovernancePowerByChain::<T>::iter().count() as u128;
            if count > 0 && total > 0 {
                let average = total / count;
                let max_div_bps = GovernancePowerByChain::<T>::iter()
                    .map(|(_, p)| {
                        let diff = if p >= average { p - average } else { average - p };
                        // basis points relative to total power.
                        (diff.saturating_mul(10_000) / total).min(u32::MAX as u128) as u32
                    })
                    .max()
                    .unwrap_or(0);

                if max_div_bps > T::GovernanceDivergenceAlertBps::get() {
                    Self::deposit_event(Event::GovernancePowerDivergenceAlert { max_divergence_bps: max_div_bps });
                }
            }

            Ok(())
        }

        /// Re-sum all chain power entries into `TotalGovernancePower`.
        ///
        /// Open to any origin.  Useful after manual chain power pruning.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn aggregate_governance_power(origin: OriginFor<T>) -> DispatchResult {
            let _who = ensure_signed_or_root(origin)?;

            let total: u128 = GovernancePowerByChain::<T>::iter()
                .map(|(_, p)| p)
                .fold(0u128, |acc, v| acc.saturating_add(v));
            TotalGovernancePower::<T>::put(total);

            let now = frame_system::Pallet::<T>::block_number();
            LastPowerReconciliation::<T>::put(now);

            Self::deposit_event(Event::GovernancePowerAggregated { total });
            Ok(())
        }
    }

    // ── Public helpers ────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Returns `true` when a mint halt is currently active.
        pub fn is_minting_halted() -> bool {
            MintHaltSince::<T>::get().is_some()
        }

        /// Returns the divergence (in basis points) from the most recent
        /// reconciliation record, or `None` if no reconciliation has run yet.
        pub fn current_divergence_bps() -> Option<u32> {
            LastReconciliation::<T>::get().map(|r| r.divergence_bps)
        }

        /// Returns the current total aggregated governance power.
        pub fn get_total_governance_power() -> u128 {
            TotalGovernancePower::<T>::get()
        }

        /// Returns the current reconciliation status.
        pub fn reconciliation_status() -> ReconciliationStatus {
            match LastReconciliation::<T>::get() {
                None => ReconciliationStatus::Passing,
                Some(record) => {
                    if MintHaltSince::<T>::get().is_some() {
                        ReconciliationStatus::Halted
                    } else if record.divergence_bps > T::ToleranceBps::get() {
                        ReconciliationStatus::Degraded
                    } else {
                        ReconciliationStatus::Passing
                    }
                }
            }
        }
    }
}
