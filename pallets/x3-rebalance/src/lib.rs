#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # pallet-x3-rebalance
//!
//! Rebalance Engine (Module 4) for the X3 Phase 4.5 liquidity control plane.
//!
//! ## Responsibilities
//!
//! - Accept rebalance trigger notifications from any caller and enqueue them in a bounded
//!   pending queue (`PendingRebalances`).
//! - Fire readiness events in `on_initialize` for triggers that are due, so off-chain
//!   operators know which vaults to act on.
//! - Let operators execute individual rebalance steps with `execute_rebalance_step`, which
//!   calls `fund_vault` in `pallet-x3-inventory` to simulate capital arriving.
//! - Enforce a per-vault cooldown between consecutive steps.
//! - Enforce a rolling daily aggregate volume cap (`MaxDailyRebalanceVolume`).
//! - Restrict `TreasuryRefill` to vaults whose chain+asset are served solely by
//!   `LaneClass::C` corridors.
//! - Allow governance to remove stuck queue entries via `clear_rebalance`.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use pallet_x3_inventory::{
        pallet::{Lanes, Vaults},
        types::{LaneClass, RebalanceMethod, RebalanceTrigger, VaultId},
    };
    use sp_runtime::traits::{SaturatedConversion, Saturating, Zero};

    // -----------------------------------------------------------------------
    // Type alias
    // -----------------------------------------------------------------------

    /// Shorthand for the Balance type threaded through pallet-x3-inventory.
    type BalanceOf<T> = <T as pallet_x3_inventory::pallet::Config>::Balance;

    /// Approximate number of blocks in one calendar day at a 6-second block time.
    /// 24 * 60 * 60 / 6 = 14 400.
    const BLOCKS_PER_DAY: u32 = 14_400;

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
        /// The runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum total balance (in native units) that may be rebalanced across all vaults
        /// within a single calendar day. Rejects steps that would exceed this threshold.
        ///
        /// Uses `BalanceOf<Self>` so the constant can be compared directly with vault balances
        /// without any lossy numeric conversion. In the runtime, configure this with the same
        /// type used for `pallet_x3_inventory::Config::Balance` (typically `u128`).
        #[pallet::constant]
        type MaxDailyRebalanceVolume: Get<<Self as pallet_x3_inventory::pallet::Config>::Balance>;

        /// Minimum number of blocks that must elapse between consecutive rebalance steps on
        /// the same vault. Prevents rapid oscillation and fee extraction.
        #[pallet::constant]
        type RebalanceCooldownBlocks: Get<BlockNumberFor<Self>>;

        /// Maximum number of entries that may exist simultaneously in `PendingRebalances`.
        /// Bounds the per-block iteration cost of `on_initialize`.
        #[pallet::constant]
        type MaxPendingRebalances: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Active rebalance trigger queue.
    ///
    /// Key: a `[u8; 32]` queue key derived from the trigger's primary identifier:
    ///   - `BelowMinBand { vault_id }` → vault_id
    ///   - `DemandSpike { lane_id }` / `PersistentOneWayFlow { lane_id }` → lane_id
    ///   - `PartnerCapacityLoss { partner_id }` → partner_id
    ///   - `VenueLiquidityCollapse { venue_id }` → venue_id
    ///   - `ConcentrationBreach { chain_id }` / `ChainDegradation { chain_id }` →
    ///     chain_id encoded into bytes 0–3 with a discriminant in byte 4.
    ///
    /// Value: `(trigger, scheduled_block)` — the original trigger and the block at which it
    /// was enqueued.
    #[pallet::storage]
    #[pallet::getter(fn pending_rebalances)]
    pub type PendingRebalances<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        VaultId, // used as the generic [u8; 32] queue key
        (RebalanceTrigger, BlockNumberFor<T>),
        OptionQuery,
    >;

    /// Aggregate balance rebalanced per calendar day.
    ///
    /// Key: `day_index = block_number / BLOCKS_PER_DAY`.
    /// Value: running total for that day.
    #[pallet::storage]
    #[pallet::getter(fn daily_rebalance_volume)]
    pub type DailyRebalanceVolume<T: Config> =
        StorageMap<_, Twox64Concat, u32, BalanceOf<T>, ValueQuery>;

    /// Block number at which the most recent rebalance step was executed per vault.
    ///
    /// Used to enforce `RebalanceCooldownBlocks`.
    #[pallet::storage]
    #[pallet::getter(fn last_rebalance_block)]
    pub type LastRebalanceBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, VaultId, BlockNumberFor<T>, ValueQuery>;

    /// Number of entries currently held in `PendingRebalances`.
    ///
    /// Kept in strict sync with insertions and removals so that the queue cap can be
    /// enforced in O(1) without scanning the map.
    #[pallet::storage]
    #[pallet::getter(fn pending_rebalance_count)]
    pub type PendingRebalanceCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A rebalance trigger was enqueued or fired as due in `on_initialize`.
        ///
        /// `scheduled_block` is the block at which the trigger was first registered.
        /// Operators should call `execute_rebalance_step` for the indicated vault.
        RebalanceTriggered {
            vault_id: VaultId,
            trigger: RebalanceTrigger,
            scheduled_block: BlockNumberFor<T>,
        },

        /// An operator executed a rebalance step; the vault has received `amount` of capital
        /// but is still below its `min_band`.
        RebalanceStepExecuted {
            vault_id: VaultId,
            method: RebalanceMethod,
            amount: BalanceOf<T>,
        },

        /// The vault's available balance has returned to or above its `min_band`.
        /// `total_moved` is the amount added in the final step.
        RebalanceCompleted {
            vault_id: VaultId,
            method: RebalanceMethod,
            total_moved: BalanceOf<T>,
        },

        /// A rebalance step was rejected because it would have exceeded the daily aggregate cap.
        RebalanceCapExceeded {
            vault_id: VaultId,
            daily_volume: BalanceOf<T>,
            cap: BalanceOf<T>,
        },

        /// A stuck pending rebalance entry was removed by governance.
        RebalanceCleared { vault_id: VaultId },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// The referenced vault does not exist in `pallet-x3-inventory`.
        VaultNotFound,

        /// The pending rebalance queue is at capacity (`MaxPendingRebalances`).
        RebalanceQueueFull,

        /// A rebalance step was attempted before the per-vault cooldown has elapsed.
        RebalanceCooldownActive,

        /// The step amount would push the day's aggregate volume past `MaxDailyRebalanceVolume`.
        DailyCapExceeded,

        /// `TreasuryRefill` is only permitted on vaults whose chain+asset are served
        /// exclusively by `LaneClass::C` corridors.
        TreasuryRefillNotAllowedOnABLane,

        /// The vault's available balance is already at or above `min_band`; no rebalance
        /// is required.
        NoRebalanceNeeded,
    }

    // -----------------------------------------------------------------------
    // Hooks
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Scan the pending queue for entries that are due (`scheduled_block <= now`).
        ///
        /// For each due entry:
        ///   - `BelowMinBand` triggers: emit `RebalanceTriggered` only if the vault is
        ///     still below `min_band` (avoids spurious noise if the vault self-corrected).
        ///   - All other triggers: emit unconditionally since they represent systemic
        ///     conditions that operators must evaluate.
        ///
        /// Due entries are removed from the queue regardless of whether they emitted an
        /// event, so they do not block future triggers for the same key.
        ///
        /// Iteration is bounded by `MaxPendingRebalances` to keep per-block weight constant.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let max = T::MaxPendingRebalances::get();

            // Collect due entries.  The map is already bounded to MaxPendingRebalances, so
            // this iteration is O(MaxPendingRebalances) in the worst case.
            let due: sp_std::vec::Vec<(VaultId, RebalanceTrigger, BlockNumberFor<T>)> =
                PendingRebalances::<T>::iter()
                    .filter(|(_, (_, scheduled))| *scheduled <= now)
                    .take(max as usize)
                    .map(|(key, (trigger, scheduled))| (key, trigger, scheduled))
                    .collect();

            let processed = due.len() as u64;

            for (queue_key, trigger, scheduled) in due {
                PendingRebalances::<T>::remove(queue_key);
                PendingRebalanceCount::<T>::mutate(|c| {
                    *c = c.saturating_sub(1);
                });

                // Determine whether to emit the readiness event.
                let (should_emit, emit_vault_id) = match &trigger {
                    RebalanceTrigger::BelowMinBand { vault_id } => {
                        let below = Vaults::<T>::get(vault_id)
                            .map(|v| v.available_balance < v.min_band)
                            .unwrap_or(false);
                        (below, *vault_id)
                    }
                    _ => (true, queue_key),
                };

                if should_emit {
                    Self::deposit_event(Event::RebalanceTriggered {
                        vault_id: emit_vault_id,
                        trigger,
                        scheduled_block: scheduled,
                    });
                }
            }

            // Each processed entry costs: ~1 read (iter) + 1 write (remove) + 1 write (count).
            // Plus 1 read for the global count value.
            T::DbWeight::get()
                .reads(processed + 1)
                .saturating_add(T::DbWeight::get().writes(processed.saturating_mul(2)))
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Enqueue a rebalance trigger request.
        ///
        /// Anyone may call this extrinsic.  The trigger is stored in `PendingRebalances`
        /// under the key derived from its primary identifier (see `queue_key_for`).
        ///
        /// If an entry already exists for that key, it is overwritten and the count is
        /// unchanged (prevents stale triggers from accumulating duplicate queue slots).
        ///
        /// Returns `RebalanceQueueFull` if the queue is at capacity and no existing entry
        /// for this key can be updated.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn trigger_rebalance(
            origin: OriginFor<T>,
            trigger: RebalanceTrigger,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let queue_key = Self::queue_key_for(&trigger);
            let now = <frame_system::Pallet<T>>::block_number();

            if PendingRebalances::<T>::contains_key(queue_key) {
                // Key already present — overwrite trigger without touching the count.
                PendingRebalances::<T>::insert(queue_key, (trigger.clone(), now));
            } else {
                let count = PendingRebalanceCount::<T>::get();
                ensure!(
                    count < T::MaxPendingRebalances::get(),
                    Error::<T>::RebalanceQueueFull
                );
                PendingRebalances::<T>::insert(queue_key, (trigger.clone(), now));
                PendingRebalanceCount::<T>::put(count.saturating_add(1));
            }

            let emit_vault_id = match &trigger {
                RebalanceTrigger::BelowMinBand { vault_id } => *vault_id,
                _ => queue_key,
            };

            Self::deposit_event(Event::RebalanceTriggered {
                vault_id: emit_vault_id,
                trigger,
                scheduled_block: now,
            });

            Ok(())
        }

        /// Execute one rebalance step for `vault_id` using the given `method`.
        ///
        /// Checks (in order):
        /// 1. `vault_id` must exist in `pallet-x3-inventory`.
        /// 2. Vault `available_balance` must be below `min_band` (`NoRebalanceNeeded`
        ///    otherwise).
        /// 3. At least `RebalanceCooldownBlocks` must have passed since the last step on
        ///    this vault.
        /// 4. If `method == TreasuryRefill`, none of the lanes whose `source_chain` or
        ///    `dest_chain` matches the vault's `chain_id` and whose `source_asset` or
        ///    `dest_asset` matches the vault's `asset_id` may be `LaneClass::A` or `B`.
        /// 5. The step amount (`target_band - available_balance`) must not push the day's
        ///    aggregate volume past `MaxDailyRebalanceVolume`.
        ///
        /// On success, calls `fund_vault` in `pallet-x3-inventory` to credit the vault
        /// with the step amount.  Updates `LastRebalanceBlock` and `DailyRebalanceVolume`.
        ///
        /// Emits `RebalanceCompleted` if the vault is at or above `min_band` after the
        /// step, otherwise `RebalanceStepExecuted`.
        ///
        /// Emits `RebalanceCapExceeded` and returns `DailyCapExceeded` if the cap is hit.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(4, 3))]
        pub fn execute_rebalance_step(
            origin: OriginFor<T>,
            vault_id: VaultId,
            method: RebalanceMethod,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let now = <frame_system::Pallet<T>>::block_number();

            // 1. Vault must exist and need rebalancing.
            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            ensure!(
                vault.available_balance < vault.min_band,
                Error::<T>::NoRebalanceNeeded
            );

            // 2. Cooldown enforcement.
            let last = LastRebalanceBlock::<T>::get(vault_id);
            if !last.is_zero() {
                let elapsed = now.saturating_sub(last);
                ensure!(
                    elapsed >= T::RebalanceCooldownBlocks::get(),
                    Error::<T>::RebalanceCooldownActive
                );
            }

            // 3. TreasuryRefill is forbidden when the vault's chain+asset are served by any
            //    LaneClass::A or LaneClass::B corridor.
            if method == RebalanceMethod::TreasuryRefill {
                let has_ab_lane = Lanes::<T>::iter().any(|(_, lane)| {
                    let chain_matches =
                        lane.source_chain == vault.chain_id || lane.dest_chain == vault.chain_id;
                    let asset_matches =
                        lane.source_asset == vault.asset_id || lane.dest_asset == vault.asset_id;
                    let is_ab = lane.lane_class == LaneClass::A || lane.lane_class == LaneClass::B;
                    chain_matches && asset_matches && is_ab
                });
                ensure!(!has_ab_lane, Error::<T>::TreasuryRefillNotAllowedOnABLane);
            }

            // 4. Compute the step amount: bring available_balance up to target_band.
            //    Because available < min_band <= target_band, step_amount > 0.
            let step_amount = vault.target_band.saturating_sub(vault.available_balance);

            // 5. Enforce daily cap.
            let blocks_per_day: BlockNumberFor<T> = BlockNumberFor::<T>::from(BLOCKS_PER_DAY);
            let day: u32 = (now / blocks_per_day).saturated_into::<u32>();
            let current_volume = DailyRebalanceVolume::<T>::get(day);
            let new_volume = current_volume.saturating_add(step_amount);
            let cap: BalanceOf<T> = T::MaxDailyRebalanceVolume::get();

            if new_volume > cap {
                // Note: events emitted inside a #[pallet::call] that returns Err are rolled
                // back by FRAME's per-call storage transaction.  RebalanceCapExceeded is
                // therefore not deposited here; callers detect the cap breach via the
                // `DailyCapExceeded` error.  External monitoring tools can subscribe to
                // failed extrinsic receipts to detect this condition.
                return Err(Error::<T>::DailyCapExceeded.into());
            }

            // 6. Execute: credit the vault (funds are considered as having arrived).
            pallet_x3_inventory::inventory::fund_vault::<T>(vault_id, step_amount)?;

            // 7. Update tracking storage.
            DailyRebalanceVolume::<T>::insert(day, new_volume);
            LastRebalanceBlock::<T>::insert(vault_id, now);

            // 8. Emit completion or step event depending on whether the vault recovered.
            let updated = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            if updated.available_balance >= updated.min_band {
                Self::deposit_event(Event::RebalanceCompleted {
                    vault_id,
                    method,
                    total_moved: step_amount,
                });
            } else {
                Self::deposit_event(Event::RebalanceStepExecuted {
                    vault_id,
                    method,
                    amount: step_amount,
                });
            }

            Ok(())
        }

        /// Remove a stuck pending rebalance entry from the queue.
        ///
        /// Only callable by root (governance).  Use when a trigger cannot be processed
        /// because the associated vault or lane has been removed, or the trigger is
        /// permanently stale.
        ///
        /// Silently succeeds (no error) if `vault_id` is not currently in the queue.
        /// Emits `RebalanceCleared` when an entry is actually removed.
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn clear_rebalance(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            ensure_root(origin)?;

            if PendingRebalances::<T>::contains_key(vault_id) {
                PendingRebalances::<T>::remove(vault_id);
                PendingRebalanceCount::<T>::mutate(|c| {
                    *c = c.saturating_sub(1);
                });
                Self::deposit_event(Event::RebalanceCleared { vault_id });
            }

            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Derive the `[u8; 32]` storage key for a trigger.
        ///
        /// For triggers whose primary identifier is already a `[u8; 32]` (vault_id, lane_id,
        /// partner_id, venue_id), the identifier is used directly.
        ///
        /// For chain-scoped triggers (`ConcentrationBreach`, `ChainDegradation`), the `u32`
        /// chain_id is encoded into bytes 0–3 with a 1-byte discriminant in byte 4 to avoid
        /// key collisions between the two trigger variants for the same chain.
        pub(crate) fn queue_key_for(trigger: &RebalanceTrigger) -> VaultId {
            match trigger {
                RebalanceTrigger::BelowMinBand { vault_id } => *vault_id,
                RebalanceTrigger::DemandSpike { lane_id } => *lane_id,
                RebalanceTrigger::PersistentOneWayFlow { lane_id } => *lane_id,
                RebalanceTrigger::PartnerCapacityLoss { partner_id } => *partner_id,
                RebalanceTrigger::VenueLiquidityCollapse { venue_id } => *venue_id,
                RebalanceTrigger::ConcentrationBreach { chain_id } => {
                    let mut key = [0u8; 32];
                    key[..4].copy_from_slice(&chain_id.to_le_bytes());
                    // discriminant 0x00 (default zero bytes)
                    key
                }
                RebalanceTrigger::ChainDegradation { chain_id } => {
                    let mut key = [0u8; 32];
                    key[..4].copy_from_slice(&chain_id.to_le_bytes());
                    key[4] = 0x01; // discriminant distinguishes from ConcentrationBreach
                    key
                }
            }
        }
    }
}
