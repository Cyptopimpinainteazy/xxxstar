#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 DEX Pallet
//!
//! Substrate FRAME pallet providing access to X3 DEX functionality including
//! AMM pools, limit orders, concentrated liquidity, and trading operations.
//!
//! ## Custody
//!
//! A pool's reserves are balances on the kernel's `CanonicalLedger`, held by the pool's own
//! account ([`Pallet::pool_account`]): adding liquidity moves the provider's funds in, removing it
//! pays the position's owner out, and a swap takes the input from the trader and pays the output
//! to them. Until 2026-10-17 every call here only edited the reserve numbers — no balance moved,
//! a "swap" paid nothing and an LP position had no owner, so anyone could withdraw any position.
//! Pools created before that (no [`CustodiedPools`] entry) hold no funds and are frozen: every
//! call on them is refused rather than letting a swap take real funds against an unfunded pool.

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
mod tests_custody;
#[cfg(test)]
mod tests_liquidity_provision;
#[cfg(test)]
mod tests_swapping;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use frame_support::PalletId;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::AccountIdConversion;
    use x3_asset_kernel_types::traits::{CanonicalLedgerTransfer, EconomicHaltInspect};
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

        /// The ledger pool reserves are custodied on (the kernel's `CanonicalLedger`).
        type Ledger: CanonicalLedgerTransfer<Self::AccountId>;

        /// Derives each pool's custody account.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The `TokenId::chain_id` whose `asset_id`s are ledger asset ids. A token on any other
        /// chain has no balance on this ledger to custody, so a pool of it is refused.
        #[pallet::constant]
        type LedgerChainId: Get<u32>;

        /// Funds benchmark accounts on the ledger (benchmarks only).
        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkHelper: x3_asset_kernel_types::traits::LedgerBenchmarkFunding<Self::AccountId>;
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

    /// Next position ID counter
    #[pallet::storage]
    pub type NextPositionId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// The last pool id `create_pool` assigned; the next pool is the next unused id after it.
    ///
    /// Pool ids used to come from `x3_dex::amm_pools::AMMPool::generate_pool_id`, which hashes the
    /// chain ids and only the upper 64 bits of each asset id: every pool between two tokens on the
    /// same chain got the same id, so a second pool was refused as a duplicate. Ids are now
    /// sequential from 1, which is also how a program names a venue (`x3dex_pool_<id>`).
    #[pallet::storage]
    pub type NextPoolId<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Pools whose reserves are custodied on the ledger. A pool without an entry predates custody,
    /// holds no funds, and is frozen.
    #[pallet::storage]
    pub type CustodiedPools<T: Config> = StorageMap<_, Blake2_128Concat, u64, (), OptionQuery>;

    /// Who may withdraw an LP position.
    #[pallet::storage]
    pub type PositionOwner<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, T::AccountId, OptionQuery>;

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
        /// An LP position changed hands (see `transfer_position`).
        PositionTransferred {
            position_id: u64,
            from: T::AccountId,
            to: T::AccountId,
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
        /// The pool predates custody: it holds no funds and is frozen.
        LegacyPoolWithoutCustody,
        /// Only the position's owner may withdraw it.
        NotPositionOwner,
        /// The token is not an asset on this chain's ledger.
        TokenNotOnLedger,
        /// Checked pool arithmetic overflowed.
        ArithmeticOverflow,
        /// A reserve would exceed what the pool's custody account holds.
        ReserveExceedsCustody,
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
            Self::ledger_asset(&token_a)?;
            Self::ledger_asset(&token_b)?;

            // Validate the pair and fee with the DEX crate; the id is the pallet's.
            let mut pool = AMMPool::create_pool(token_a.clone(), token_b.clone(), fee_basis_points)
                .map_err(|_| Error::<T>::InvalidTokenPair)?;

            // The next unused sequential id. A pool from before sequential ids sits at a hashed
            // id; one of those is skipped, and a run of them is refused rather than searched.
            let mut pool_id = NextPoolId::<T>::get();
            let mut attempts = 0u8;
            loop {
                pool_id = pool_id
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                if !Pools::<T>::contains_key(pool_id) {
                    break;
                }
                attempts += 1;
                ensure!(attempts < 8, Error::<T>::PoolAlreadyExists);
            }
            NextPoolId::<T>::put(pool_id);
            pool.pool_id = pool_id;

            // Store pool
            Pools::<T>::insert(pool_id, pool);
            CustodiedPools::<T>::insert(pool_id, ());

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
            let who = ensure_signed(origin)?;

            // Get pool
            let mut pool = Self::custodied_pool(pool_id)?;

            // Add liquidity using DEX logic
            let (amount_a, amount_b, lp_tokens) = AMMPool::add_liquidity_calculate(
                &pool,
                amount_a_desired,
                amount_b_desired,
                amount_a_min,
                amount_b_min,
            )
            .map_err(|_| Error::<T>::InsufficientLiquidity)?;

            // The provider's funds move into the pool's custody account first: a deposit nobody
            // paid for is the bug this pallet used to have.
            let account = Self::pool_account(pool_id);
            T::Ledger::ledger_transfer(
                Self::ledger_asset(&pool.token_a)?,
                &who,
                &account,
                amount_a,
            )?;
            T::Ledger::ledger_transfer(
                Self::ledger_asset(&pool.token_b)?,
                &who,
                &account,
                amount_b,
            )?;

            // Update pool reserves
            pool.reserve_a = pool
                .reserve_a
                .checked_add(amount_a)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            pool.reserve_b = pool
                .reserve_b
                .checked_add(amount_b)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            pool.total_lp_supply = pool
                .total_lp_supply
                .checked_add(lp_tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

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
            PositionOwner::<T>::insert(position_id, who);
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
            let who = ensure_signed(origin)?;

            // Get position
            let position = LPPositions::<T>::get(position_id).ok_or(Error::<T>::PoolNotFound)?;
            let pool_id = position.pool_id;
            ensure!(
                PositionOwner::<T>::get(position_id).as_ref() == Some(&who),
                Error::<T>::NotPositionOwner
            );
            ensure!(
                lp_amount <= position.lp_balance,
                Error::<T>::InsufficientLiquidity
            );

            // Get pool
            let mut pool = Self::custodied_pool(pool_id)?;

            // Remove liquidity using DEX logic
            let (amount_a, amount_b) =
                AMMPool::remove_liquidity_calculate(&pool, lp_amount, amount_a_min, amount_b_min)
                    .map_err(|_| Error::<T>::InsufficientLiquidity)?;

            // Update pool reserves
            pool.reserve_a = pool
                .reserve_a
                .checked_sub(amount_a)
                .ok_or(Error::<T>::InsufficientLiquidity)?;
            pool.reserve_b = pool
                .reserve_b
                .checked_sub(amount_b)
                .ok_or(Error::<T>::InsufficientLiquidity)?;
            pool.total_lp_supply = pool
                .total_lp_supply
                .checked_sub(lp_amount)
                .ok_or(Error::<T>::InsufficientLiquidity)?;

            // Pay the owner out of the pool's custody.
            let account = Self::pool_account(pool_id);
            T::Ledger::ledger_transfer(
                Self::ledger_asset(&pool.token_a)?,
                &account,
                &who,
                amount_a,
            )?;
            T::Ledger::ledger_transfer(
                Self::ledger_asset(&pool.token_b)?,
                &account,
                &who,
                amount_b,
            )?;

            // Update position
            let mut updated_position = position;
            updated_position.lp_balance = updated_position
                .lp_balance
                .checked_sub(lp_amount)
                .ok_or(Error::<T>::InsufficientLiquidity)?;

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
            let mut pool = Self::custodied_pool(pool_id)?;

            // Execute swap using DEX logic
            let amount_out = AMMPool::swap_calculate(&pool, &token_in, amount_in, min_out)
                .map_err(|_| Error::<T>::SlippageExceeded)?;

            // Update pool reserves
            let token_out = if token_in == pool.token_a {
                pool.reserve_a = pool
                    .reserve_a
                    .checked_add(amount_in)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                pool.reserve_b = pool
                    .reserve_b
                    .checked_sub(amount_out)
                    .ok_or(Error::<T>::InsufficientLiquidity)?;
                pool.token_b.clone()
            } else {
                pool.reserve_b = pool
                    .reserve_b
                    .checked_add(amount_in)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                pool.reserve_a = pool
                    .reserve_a
                    .checked_sub(amount_out)
                    .ok_or(Error::<T>::InsufficientLiquidity)?;
                pool.token_a.clone()
            };

            // The trader pays the input in and is paid the output: both legs or neither (the
            // extrinsic's storage layer reverts the first if the second fails).
            let account = Self::pool_account(pool_id);
            T::Ledger::ledger_transfer(Self::ledger_asset(&token_in)?, &who, &account, amount_in)?;
            T::Ledger::ledger_transfer(
                Self::ledger_asset(&token_out)?,
                &account,
                &who,
                amount_out,
            )?;

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

        /// The account that custodies `pool_id`'s reserves.
        /// Hand LP position `position_id` from its owner `from` to `to`. For pallets that hold a
        /// position on someone's behalf (a launch's locked liquidity); there is no extrinsic, and
        /// the transfer is refused unless `from` owns the position.
        pub fn transfer_position(
            position_id: u64,
            from: &T::AccountId,
            to: &T::AccountId,
        ) -> DispatchResult {
            ensure!(
                PositionOwner::<T>::get(position_id).as_ref() == Some(from),
                Error::<T>::NotPositionOwner
            );
            PositionOwner::<T>::insert(position_id, to.clone());
            Self::deposit_event(Event::PositionTransferred {
                position_id,
                from: from.clone(),
                to: to.clone(),
            });
            Ok(())
        }

        /// The account that owns LP position `position_id`.
        pub fn position_owner(position_id: u64) -> Option<T::AccountId> {
            PositionOwner::<T>::get(position_id)
        }

        pub fn pool_account(pool_id: u64) -> T::AccountId {
            T::PalletId::get().into_sub_account_truncating(pool_id)
        }

        /// The ledger asset id of a token, refused unless the token is on this chain's ledger and
        /// its id fits the ledger's `u32` asset type (checked conversion).
        pub fn ledger_asset(token: &TokenId) -> Result<u32, DispatchError> {
            ensure!(
                token.chain_id == T::LedgerChainId::get(),
                Error::<T>::TokenNotOnLedger
            );
            u32::try_from(token.asset_id).map_err(|_| Error::<T>::TokenNotOnLedger.into())
        }

        fn custodied_pool(pool_id: u64) -> Result<LiquidityPool, DispatchError> {
            let pool = Pools::<T>::get(pool_id).ok_or(Error::<T>::PoolNotFound)?;
            ensure!(
                CustodiedPools::<T>::contains_key(pool_id),
                Error::<T>::LegacyPoolWithoutCustody
            );
            Ok(pool)
        }

        /// A custodied pool as the trading extension sees it: ledger asset ids, reserves, fee.
        pub fn trading_pool(pool_id: u64) -> Option<TradingPool> {
            let pool = Self::custodied_pool(pool_id).ok()?;
            Some(TradingPool {
                asset_a: Self::ledger_asset(&pool.token_a).ok()?,
                asset_b: Self::ledger_asset(&pool.token_b).ok()?,
                reserve_a: pool.reserve_a,
                reserve_b: pool.reserve_b,
                fee_bps: u16::try_from(pool.fee_basis_points).ok()?,
            })
        }

        /// Record the reserves a committed atomic trade left `pool_id` with.
        ///
        /// The trade's own balance movements (into and out of [`Pallet::pool_account`]) are
        /// applied by the kernel on the same ledger, in the same storage transaction, before this
        /// is called; this refuses reserves the custody account does not actually hold.
        pub fn apply_trading_reserves(
            pool_id: u64,
            reserve_a: u128,
            reserve_b: u128,
        ) -> DispatchResult {
            let mut pool = Self::custodied_pool(pool_id)?;
            let account = Self::pool_account(pool_id);
            ensure!(
                reserve_a
                    <= T::Ledger::ledger_balance(Self::ledger_asset(&pool.token_a)?, &account)
                    && reserve_b
                        <= T::Ledger::ledger_balance(Self::ledger_asset(&pool.token_b)?, &account),
                Error::<T>::ReserveExceedsCustody
            );
            pool.reserve_a = reserve_a;
            pool.reserve_b = reserve_b;
            Pools::<T>::insert(pool_id, pool);
            Ok(())
        }
    }

    /// A custodied pool's trading view.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct TradingPool {
        pub asset_a: u32,
        pub asset_b: u32,
        pub reserve_a: u128,
        pub reserve_b: u128,
        pub fee_bps: u16,
    }
}
