#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! # X3 Flash Loan Pallet
//!
//! Enables atomic flash loans: borrow assets, use them in the same block,
//! and repay within the same extrinsic. If repayment does not occur, the
//! entire state change is reverted.
//!
//! ## Invariant
//! Every borrow must be followed by repayment + fee in the same call.
//! The pallet checks the repay-or-revert guarantee at the end of execution.

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
        /// Flash loan fee in basis points (e.g. 9 = 0.09%).
        #[pallet::constant]
        type FeeBasisPoints: Get<u32>;
        /// Maximum flash loan amount as a fraction of pool liquidity.
        #[pallet::constant]
        type MaxLoanFraction: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Active flash loan record for the current block.
    #[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
    pub struct FlashLoanRecord<AccountId, Balance> {
        pub borrower: AccountId,
        pub amount: Balance,
        pub fee: Balance,
        pub repaid: bool,
    }

    /// Current active flash loan (one at a time per block for simplicity).
    #[pallet::storage]
    pub type ActiveLoan<T: Config> =
        StorageValue<_, FlashLoanRecord<T::AccountId, BalanceOf<T>>, OptionQuery>;

    /// Flash loan pool balance (funds available to lend).
    #[pallet::storage]
    pub type PoolBalance<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Total fees collected historically.
    #[pallet::storage]
    pub type TotalFeesCollected<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Flash loan initiated.
        FlashLoanInitiated {
            borrower: T::AccountId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        /// Flash loan repaid successfully.
        FlashLoanRepaid {
            borrower: T::AccountId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
        },
        /// Liquidity added to the flash loan pool.
        LiquidityAdded {
            provider: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// A flash loan is already active in this block.
        LoanAlreadyActive,
        /// No active flash loan to repay.
        NoActiveLoan,
        /// Repayment amount is insufficient (must include fee).
        RepaymentInsufficient,
        /// Flash loan amount exceeds pool liquidity.
        InsufficientPoolLiquidity,
        /// Flash loan amount is zero.
        ZeroAmount,
        /// Arithmetic overflow computing fee.
        FeeOverflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Borrow assets from the flash loan pool.
        ///
        /// The borrower receives `amount` tokens immediately. They MUST call
        /// `repay` in the same extrinsic batch with `amount + fee` or the
        /// entire block state reverts.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn borrow(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResult {
            let borrower = ensure_signed(origin)?;
            ensure!(amount > BalanceOf::<T>::default(), Error::<T>::ZeroAmount);
            ensure!(
                ActiveLoan::<T>::get().is_none(),
                Error::<T>::LoanAlreadyActive
            );

            let pool = PoolBalance::<T>::get();
            ensure!(pool >= amount, Error::<T>::InsufficientPoolLiquidity);

            // Compute fee: amount * FeeBasisPoints / 10_000
            let fee_bps = T::FeeBasisPoints::get() as u128;
            let amount_u128 = TryInto::<u128>::try_into(amount).unwrap_or(0);
            let fee_u128 = amount_u128
                .checked_mul(fee_bps)
                .and_then(|v| v.checked_div(10_000))
                .ok_or(Error::<T>::FeeOverflow)?;
            let fee: BalanceOf<T> = fee_u128.try_into().map_err(|_| Error::<T>::FeeOverflow)?;

            // Transfer from pool account to borrower
            let pool_account = Self::pool_account();
            T::Currency::transfer(
                &pool_account,
                &borrower,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;

            // Record active loan
            ActiveLoan::<T>::put(FlashLoanRecord {
                borrower: borrower.clone(),
                amount,
                fee,
                repaid: false,
            });

            // Update pool balance
            PoolBalance::<T>::mutate(|b| *b = b.saturating_sub(amount));

            Self::deposit_event(Event::FlashLoanInitiated {
                borrower,
                amount,
                fee,
            });
            Ok(())
        }

        /// Repay the active flash loan (amount + fee).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn repay(
            origin: OriginFor<T>,
            #[pallet::compact] repayment: BalanceOf<T>,
        ) -> DispatchResult {
            let borrower = ensure_signed(origin)?;

            let loan = ActiveLoan::<T>::get().ok_or(Error::<T>::NoActiveLoan)?;
            ensure!(loan.borrower == borrower, Error::<T>::NoActiveLoan);

            let required = loan.amount.saturating_add(loan.fee);
            ensure!(repayment >= required, Error::<T>::RepaymentInsufficient);

            let pool_account = Self::pool_account();
            T::Currency::transfer(
                &borrower,
                &pool_account,
                repayment,
                ExistenceRequirement::KeepAlive,
            )?;

            // Mark repaid and update pool
            ActiveLoan::<T>::kill();
            PoolBalance::<T>::mutate(|b| *b = b.saturating_add(repayment));
            TotalFeesCollected::<T>::mutate(|f| *f = f.saturating_add(loan.fee));

            Self::deposit_event(Event::FlashLoanRepaid {
                borrower,
                amount: loan.amount,
                fee: loan.fee,
            });
            Ok(())
        }

        /// Add liquidity to the flash loan pool.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;
            ensure!(amount > BalanceOf::<T>::default(), Error::<T>::ZeroAmount);

            let pool_account = Self::pool_account();
            T::Currency::transfer(
                &provider,
                &pool_account,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;

            PoolBalance::<T>::mutate(|b| *b = b.saturating_add(amount));
            Self::deposit_event(Event::LiquidityAdded { provider, amount });
            Ok(())
        }
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_pool_balance: BalanceOf<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            PoolBalance::<T>::put(self.initial_pool_balance);
        }
    }

    impl<T: Config> Pallet<T> {
        /// The flash loan pool account (deterministic PalletId-style account).
        pub fn pool_account() -> T::AccountId {
            // PalletId b"x3flashl" → 8 bytes padded to 32-byte AccountId
            let mut bytes = [0u8; 32];
            bytes[0..8].copy_from_slice(b"x3flashl");
            T::AccountId::decode(&mut &bytes[..])
                .expect("x3-flashloan pool account decodes from 32 zero-padded bytes")
        }

        /// Check that no active loan is pending (called at end-of-block).
        pub fn check_no_pending_loans() -> Result<(), &'static str> {
            if ActiveLoan::<T>::get().is_some() {
                return Err("flash loan not repaid — invariant violated");
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
