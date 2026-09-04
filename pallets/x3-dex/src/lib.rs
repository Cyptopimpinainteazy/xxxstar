#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 DEX Pallet
//!
//! Substrate FRAME pallet providing access to X3 DEX functionality including
//! AMM pools, limit orders, concentrated liquidity, and trading operations.

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_amm_math;
#[cfg(test)]
mod tests_liquidity_provision;
#[cfg(test)]
mod tests_swapping;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use x3_asset_kernel_types::traits::EconomicHaltInspect;
    use x3_dex::amm_pools::{AMMPool, LPPosition, LiquidityPool, TokenId};

    /// Maximum number of pools that can be tracked
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Maximum number of pools
        #[pallet::constant]
        type MaxPools: Get<u32>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;

        /// Read-only economic halt gate.
        type EconomicHalt: EconomicHaltInspect;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Storage for AMM liquidity pools
    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> = StorageMap<_, Blake2_128Concat, u64, LiquidityPool>;

    /// Storage for LP positions
    #[pallet::storage]
    #[pallet::getter(fn lp_positions)]
    pub type LPPositions<T: Config> = StorageMap<_, Blake2_128Concat, u64, LPPosition>;

    /// Next pool ID counter
    #[pallet::storage]
    pub type NextPoolId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Next position ID counter
    #[pallet::storage]
    pub type NextPositionId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new liquidity pool was created
        PoolCreated {
            pool_id: u64,
            token_a: TokenId,
            token_b: TokenId,
        },
        /// Liquidity was added to a pool
        LiquidityAdded {
            pool_id: u64,
            amount_a: u128,
            amount_b: u128,
            lp_tokens: u128,
        },
        /// Liquidity was removed from a pool
        LiquidityRemoved {
            pool_id: u64,
            amount_a: u128,
            amount_b: u128,
            lp_tokens: u128,
        },
        /// A swap was executed
        SwapExecuted {
            pool_id: u64,
            amount_in: u128,
            amount_out: u128,
            user: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Pool does not exist
        PoolNotFound,
        /// Insufficient liquidity
        InsufficientLiquidity,
        /// Invalid token pair
        InvalidTokenPair,
        /// Slippage tolerance exceeded
        SlippageExceeded,
        /// Pool already exists
        PoolAlreadyExists,
        /// New swaps are halted by economic safety policy.
        EconomicHaltActive,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new AMM liquidity pool
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_pool())]
        pub fn create_pool(
            origin: OriginFor<T>,
            token_a: TokenId,
            token_b: TokenId,
            fee_basis_points: u32,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Validate fee
            ensure!(fee_basis_points <= 10000, Error::<T>::InvalidTokenPair);

            // Create pool using the DEX crate
            let pool = AMMPool::create_pool(token_a.clone(), token_b.clone(), fee_basis_points)
                .map_err(|_| Error::<T>::InvalidTokenPair)?;

            // Check if pool already exists
            let pool_id = pool.pool_id;
            ensure!(
                !Pools::<T>::contains_key(pool_id),
                Error::<T>::PoolAlreadyExists
            );

            // Store pool
            Pools::<T>::insert(pool_id, pool);

            Self::deposit_event(Event::PoolCreated {
                pool_id,
                token_a,
                token_b,
            });
            Ok(())
        }

        /// Add liquidity to an existing pool
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_liquidity())]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            pool_id: u64,
            amount_a_desired: u128,
            amount_b_desired: u128,
            amount_a_min: u128,
            amount_b_min: u128,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Get pool
            let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;

            // Add liquidity using DEX logic
            let (amount_a, amount_b, lp_tokens) = AMMPool::add_liquidity_calculate(
                &pool,
                amount_a_desired,
                amount_b_desired,
                amount_a_min,
                amount_b_min,
            )
            .map_err(|_| Error::<T>::InsufficientLiquidity)?;

            // Update pool reserves
            pool.reserve_a = pool.reserve_a.saturating_add(amount_a);
            pool.reserve_b = pool.reserve_b.saturating_add(amount_b);
            pool.total_lp_supply = pool.total_lp_supply.saturating_add(lp_tokens);

            // Create LP position
            let position_id = NextPositionId::<T>::get();
            let position = LPPosition {
                position_id,
                pool_id,
                lp_balance: lp_tokens,
            };

            // Store updated data
            Pools::<T>::insert(pool_id, pool);
            LPPositions::<T>::insert(position_id, position);
            NextPositionId::<T>::mutate(|id| *id = id.saturating_add(1));

            Self::deposit_event(Event::LiquidityAdded {
                pool_id,
                amount_a,
                amount_b,
                lp_tokens,
            });
            Ok(())
        }

        /// Remove liquidity from a pool
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::remove_liquidity())]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            position_id: u64,
            lp_amount: u128,
            amount_a_min: u128,
            amount_b_min: u128,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Get position
            let position = LPPositions::<T>::get(position_id).ok_or(Error::<T>::PoolNotFound)?;
            let pool_id = position.pool_id;

            // Get pool
            let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;

            // Remove liquidity using DEX logic
            let (amount_a, amount_b) =
                AMMPool::remove_liquidity_calculate(&pool, lp_amount, amount_a_min, amount_b_min)
                    .map_err(|_| Error::<T>::InsufficientLiquidity)?;

            // Update pool reserves
            pool.reserve_a = pool.reserve_a.saturating_sub(amount_a);
            pool.reserve_b = pool.reserve_b.saturating_sub(amount_b);
            pool.total_lp_supply = pool.total_lp_supply.saturating_sub(lp_amount);

            // Update position
            let mut updated_position = position;
            updated_position.lp_balance = updated_position.lp_balance.saturating_sub(lp_amount);

            // Store updated data
            Pools::<T>::insert(pool_id, pool);
            LPPositions::<T>::insert(position_id, updated_position);

            Self::deposit_event(Event::LiquidityRemoved {
                pool_id,
                amount_a,
                amount_b,
                lp_tokens: lp_amount,
            });
            Ok(())
        }

        /// Execute a swap through an AMM pool
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::swap())]
        pub fn swap(
            origin: OriginFor<T>,
            pool_id: u64,
            token_in: TokenId,
            amount_in: u128,
            min_out: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !T::EconomicHalt::is_halted(),
                Error::<T>::EconomicHaltActive
            );

            // Get pool
            let mut pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;

            // Execute swap using DEX logic
            let amount_out = AMMPool::swap_calculate(&pool, &token_in, amount_in, min_out)
                .map_err(|_| Error::<T>::SlippageExceeded)?;

            // Update pool reserves
            if token_in == pool.token_a {
                pool.reserve_a = pool.reserve_a.saturating_add(amount_in);
                pool.reserve_b = pool.reserve_b.saturating_sub(amount_out);
            } else {
                pool.reserve_b = pool.reserve_b.saturating_add(amount_in);
                pool.reserve_a = pool.reserve_a.saturating_sub(amount_out);
            }

            // Store updated pool
            Pools::<T>::insert(pool_id, pool);

            Self::deposit_event(Event::SwapExecuted {
                pool_id,
                amount_in,
                amount_out,
                user: who,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get pool information
        pub fn get_pool(pool_id: u64) -> Option<LiquidityPool> {
            Pools::<T>::get(pool_id)
        }

        /// Get LP position information
        pub fn get_lp_position(position_id: u64) -> Option<LPPosition> {
            LPPositions::<T>::get(position_id)
        }
    }
}
