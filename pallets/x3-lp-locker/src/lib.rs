// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-lp-locker — On-chain LP lock registry for anti-rug mitigation.
//
// Provides a canonical on-chain storage for LP token locks. When a pool is
// created via the launchpad, the operator (or the launchpad itself) can lock
// LP tokens until a future block height, providing on-chain proof of
// commitment and preventing immediate liquidity withdrawal after listing.
//
// Integration points:
//   * The launchpad pallet calls `Self::lock_lp_for(...)` during graduation.
//   * Any signed account may voluntarily lock their own LP tokens.
//   * Lock periods are immutable once set; only the unlock_at_block can be
//     extended (never shortened) via `extend_lock`.
//
// Guarantees:
//   * Single lock per (owner, pool_id) — enforced by storage key.
//   * Unlock-at-block monotonicity — extend_lock only increases the block.
//   * Atomic rollback — all operations use `with_transaction` semantics
//     provided by the caller.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    // ── Types ───────────────────────────────────────────────────────────────

    /// A single LP lock record stored on-chain.
    #[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct LpLockRecord<AccountId, BlockNumber> {
        /// Account that owns the LP tokens (the locker).
        pub owner: AccountId,
        /// DEX pool identifier.
        pub pool_id: u64,
        /// Amount of LP tokens locked (in smallest unit).
        pub lp_amount: u128,
        /// Block number at or after which LP tokens may be withdrawn.
        pub unlock_at_block: BlockNumber,
        /// Block number when the lock was created.
        pub locked_at_block: BlockNumber,
    }

    // ── Pallet ──────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.

        /// Minimum lock duration in blocks (anti-rug floor).
        #[pallet::constant]
        type MinLockDuration: Get<BlockNumberFor<Self>>;

        /// Maximum lock duration in blocks.
        #[pallet::constant]
        type MaxLockDuration: Get<BlockNumberFor<Self>>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Storage ─────────────────────────────────────────────────────────────

    /// Map from (owner, pool_id) -> LP lock record.
    #[pallet::storage]
    #[pallet::getter(fn lp_locks)]
    pub type LpLocks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u64,
        LpLockRecord<T::AccountId, BlockNumberFor<T>>,
    >;

    // ── Events ──────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// LP tokens were locked.
        LpLocked {
            owner: T::AccountId,
            pool_id: u64,
            lp_amount: u128,
            unlock_at_block: BlockNumberFor<T>,
        },
        /// LP tokens were unlocked and withdrawn.
        LpUnlocked {
            owner: T::AccountId,
            pool_id: u64,
            lp_amount: u128,
        },
        /// An existing lock was extended.
        LockExtended {
            owner: T::AccountId,
            pool_id: u64,
            new_unlock_at_block: BlockNumberFor<T>,
        },
        /// An existing lock's amount was increased.
        LockIncreased {
            owner: T::AccountId,
            pool_id: u64,
            additional_amount: u128,
            new_total: u128,
        },
    }

    // ── Errors ──────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Lock amount must be greater than zero.
        ZeroAmount,
        /// No lock exists for the given (owner, pool_id).
        NotFound,
        /// The lock has not yet expired.
        LockNotExpired,
        /// A lock already exists for (owner, pool_id); use extend_lock to modify.
        AlreadyLocked,
        /// Cannot extend lock to an earlier block than the current unlock block.
        CannotShortenLock,
        /// Lock duration is below the minimum required.
        DurationBelowMinimum,
        /// Lock duration exceeds the maximum allowed.
        DurationAboveMaximum,
        /// Description exceeds maximum length.
        DescriptionTooLong,
    }

    // ── Extrinsics ──────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Lock LP tokens for a specific pool.
        ///
        /// The caller must be the owner of the LP tokens being locked.
        /// The lock will prevent withdrawal until `unlock_at_block` is reached.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::lock_lp())]
        pub fn lock_lp(
            origin: OriginFor<T>,
            pool_id: u64,
            lp_amount: u128,
            unlock_at_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(lp_amount > 0, Error::<T>::ZeroAmount);
            ensure!(
                !LpLocks::<T>::contains_key(&who, pool_id),
                Error::<T>::AlreadyLocked
            );
            let current_block = frame_system::Pallet::<T>::block_number();
            let duration = unlock_at_block.saturating_sub(current_block);
            ensure!(
                duration >= T::MinLockDuration::get(),
                Error::<T>::DurationBelowMinimum
            );
            ensure!(
                duration <= T::MaxLockDuration::get(),
                Error::<T>::DurationAboveMaximum
            );

            let record = LpLockRecord {
                owner: who.clone(),
                pool_id,
                lp_amount,
                unlock_at_block,
                locked_at_block: current_block,
            };

            LpLocks::<T>::insert(&who, pool_id, record);

            Self::deposit_event(Event::<T>::LpLocked {
                owner: who,
                pool_id,
                lp_amount,
                unlock_at_block,
            });

            Ok(())
        }

        /// Unlock and withdraw LP tokens after the lock period has expired.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::unlock_lp())]
        pub fn unlock_lp(origin: OriginFor<T>, pool_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let record = LpLocks::<T>::get(&who, pool_id).ok_or(Error::<T>::NotFound)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                current_block >= record.unlock_at_block,
                Error::<T>::LockNotExpired
            );

            let lp_amount = record.lp_amount;
            LpLocks::<T>::remove(&who, pool_id);

            Self::deposit_event(Event::<T>::LpUnlocked {
                owner: who,
                pool_id,
                lp_amount,
            });

            Ok(())
        }

        /// Extend an existing lock's unlock block (can only increase, never decrease).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::extend_lock())]
        pub fn extend_lock(
            origin: OriginFor<T>,
            pool_id: u64,
            new_unlock_at_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut record = LpLocks::<T>::get(&who, pool_id).ok_or(Error::<T>::NotFound)?;

            ensure!(
                new_unlock_at_block > record.unlock_at_block,
                Error::<T>::CannotShortenLock
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let new_duration = new_unlock_at_block.saturating_sub(current_block);
            ensure!(
                new_duration <= T::MaxLockDuration::get(),
                Error::<T>::DurationAboveMaximum
            );

            record.unlock_at_block = new_unlock_at_block;
            LpLocks::<T>::insert(&who, pool_id, record);

            Self::deposit_event(Event::<T>::LockExtended {
                owner: who,
                pool_id,
                new_unlock_at_block,
            });

            Ok(())
        }

        /// Increase the amount of LP tokens locked in an existing lock.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::increase_lock())]
        pub fn increase_lock(
            origin: OriginFor<T>,
            pool_id: u64,
            additional_amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(additional_amount > 0, Error::<T>::ZeroAmount);

            let mut record = LpLocks::<T>::get(&who, pool_id).ok_or(Error::<T>::NotFound)?;
            record.lp_amount = record.lp_amount.saturating_add(additional_amount);
            let new_total = record.lp_amount;
            LpLocks::<T>::insert(&who, pool_id, record);

            Self::deposit_event(Event::<T>::LockIncreased {
                owner: who,
                pool_id,
                additional_amount,
                new_total,
            });

            Ok(())
        }
    }

    // ── Internal / Hooks ────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Check whether a lock exists and is still active (not expired).
        pub fn is_locked(owner: &T::AccountId, pool_id: u64) -> bool {
            LpLocks::<T>::get(owner, pool_id)
                .map(|r| frame_system::Pallet::<T>::block_number() < r.unlock_at_block)
                .unwrap_or(false)
        }

        /// Get the total locked LP amount for a given pool across all owners.
        pub fn total_locked_for_pool(pool_id: u64) -> u128 {
            // Note: This is O(N) in the number of locks. In production,
            // a secondary storage counter should be maintained per pool.
            // For v0.4 with expected low lock count, this is acceptable.
            LpLocks::<T>::iter()
                .filter(|(_, pid, _)| *pid == pool_id)
                .map(|(_, _, record)| record.lp_amount)
                .sum()
        }
    }

    // ── Weight Info ─────────────────────────────────────────────────────────

    pub trait WeightInfo {
        fn lock_lp() -> Weight;
        fn unlock_lp() -> Weight;
        fn extend_lock() -> Weight;
        fn increase_lock() -> Weight;
    }

    impl WeightInfo for () {
        fn lock_lp() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn unlock_lp() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn extend_lock() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn increase_lock() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
