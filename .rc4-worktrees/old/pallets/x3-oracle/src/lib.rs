#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Oracle Pallet
//!
//! Native oracle pallet providing decentralized price feeds through signed submissions.
//! Supports multiple oracle nodes submitting signed price data that gets aggregated
//! into median prices for use by DEX, lending, and other protocols.

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::DecodeWithMemTracking;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::vec::Vec;

    /// Asset identifier type for price feeds
    pub type AssetId = u32;

    /// Price represented as fixed-point with 6 decimals (e.g., 123456 = $1.23456)
    pub type Price = u64;

    /// Timestamp in seconds since Unix epoch
    pub type Timestamp = u64;

    /// Price submission data
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq,
    )]
    pub struct PriceSubmission<BlockNumber> {
        /// Submitted price
        pub price: Price,
        /// Block number when submitted
        pub block: BlockNumber,
        /// Timestamp when submitted
        pub timestamp: Timestamp,
    }

    /// Computed price data with metadata
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq,
    )]
    pub struct PriceData<BlockNumber> {
        /// Median price across submissions
        pub price: Price,
        /// Number of submissions used in median calculation
        pub submission_count: u32,
        /// Block when price was last updated
        pub last_updated: BlockNumber,
        /// Timestamp when price was last updated
        pub timestamp: Timestamp,
    }

    /// Maximum number of authorized oracle accounts
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of price submissions per block per oracle
        #[pallet::constant]
        type MaxSubmissionsPerBlock: Get<u32>;

        /// Maximum number of assets that can be tracked
        #[pallet::constant]
        type MaxAssets: Get<u32>;

        /// Maximum number of submissions to keep per asset
        #[pallet::constant]
        type MaxSubmissionsPerAsset: Get<u32>;

        /// Minimum number of submissions required for a valid median
        #[pallet::constant]
        type MinSubmissionsForMedian: Get<u32>;

        /// Maximum age of submissions to consider (in seconds)
        #[pallet::constant]
        type MaxSubmissionAge: Get<u64>;

        /// Origin that can authorize/deauthorize oracle accounts
        type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Storage for authorized oracle accounts
    #[pallet::storage]
    #[pallet::getter(fn is_authorized_oracle)]
    pub type AuthorizedOracles<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// Current price submissions for each asset
    /// Maps (asset_id, oracle_account) -> PriceSubmission
    #[pallet::storage]
    pub type PriceSubmissions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AssetId,
        Blake2_128Concat,
        T::AccountId,
        PriceSubmission<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Computed median prices for each asset
    #[pallet::storage]
    #[pallet::getter(fn get_price)]
    pub type AssetPrices<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetId, PriceData<BlockNumberFor<T>>, OptionQuery>;

    /// Track submission counts per block per oracle to prevent spam
    #[pallet::storage]
    pub type SubmissionsThisBlock<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        BlockNumberFor<T>,
        u32,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An oracle account was authorized
        OracleAuthorized { account: T::AccountId },
        /// An oracle account was deauthorized
        OracleDeauthorized { account: T::AccountId },
        /// A price was submitted by an oracle
        PriceSubmitted {
            asset_id: AssetId,
            oracle: T::AccountId,
            price: Price,
            block: BlockNumberFor<T>,
        },
        /// Asset price was updated with new median
        PriceUpdated {
            asset_id: AssetId,
            price: Price,
            submission_count: u32,
            median_block: BlockNumberFor<T>,
        },
        /// Price submission was rejected (invalid oracle, rate limit, etc.)
        PriceSubmissionRejected {
            asset_id: AssetId,
            oracle: T::AccountId,
            reason: SubmissionRejectionReason,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account is not authorized as an oracle
        NotAuthorizedOracle,
        /// Rate limit exceeded for submissions this block
        SubmissionRateLimitExceeded,
        /// Price submission is too old
        SubmissionTooOld,
        /// Invalid price value (zero or unreasonable)
        InvalidPrice,
        /// Insufficient submissions to compute median
        InsufficientSubmissions,
        /// Asset ID is not supported
        UnsupportedAsset,
        /// Arithmetic overflow in price calculations
        PriceCalculationOverflow,
    }

    /// Reasons why a price submission was rejected
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq,
    )]
    pub enum SubmissionRejectionReason {
        /// Oracle not authorized
        NotAuthorized,
        /// Rate limit exceeded
        RateLimitExceeded,
        /// Submission too old
        TooOld,
        /// Invalid price
        InvalidPrice,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Authorize an account to submit oracle prices
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::authorize_oracle())]
        pub fn authorize_oracle(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            AuthorizedOracles::<T>::insert(&account, true);
            Self::deposit_event(Event::OracleAuthorized { account });
            Ok(())
        }

        /// Deauthorize an oracle account
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::deauthorize_oracle())]
        pub fn deauthorize_oracle(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            AuthorizedOracles::<T>::remove(&account);
            Self::deposit_event(Event::OracleDeauthorized { account });
            Ok(())
        }

        /// Submit a price for an asset
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_price())]
        pub fn submit_price(
            origin: OriginFor<T>,
            asset_id: AssetId,
            price: Price,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check oracle authorization
            ensure!(
                AuthorizedOracles::<T>::get(&who),
                Error::<T>::NotAuthorizedOracle
            );

            // Validate price
            ensure!(price > 0, Error::<T>::InvalidPrice);

            let current_block = frame_system::Pallet::<T>::block_number();
            let current_timestamp = Self::current_timestamp();

            // Check rate limit
            let submissions_this_block = SubmissionsThisBlock::<T>::get(&who, current_block);
            ensure!(
                submissions_this_block < T::MaxSubmissionsPerBlock::get(),
                Error::<T>::SubmissionRateLimitExceeded
            );

            // Create submission
            let submission = PriceSubmission::<BlockNumberFor<T>> {
                price,
                block: current_block,
                timestamp: current_timestamp,
            };

            // Store submission
            PriceSubmissions::<T>::insert(asset_id, &who, &submission);

            // Update rate limit counter
            SubmissionsThisBlock::<T>::mutate(&who, current_block, |count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::PriceSubmitted {
                asset_id,
                oracle: who,
                price,
                block: current_block,
            });

            // Update median price for this asset
            Self::update_median_price(asset_id)?;

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get current timestamp (placeholder - should use pallet-timestamp)
        fn current_timestamp() -> Timestamp {
            pallet_timestamp::Pallet::<T>::get().saturated_into::<u64>()
        }

        /// Update the median price for an asset based on current submissions
        fn update_median_price(asset_id: AssetId) -> DispatchResult {
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_timestamp = Self::current_timestamp();
            let max_age = T::MaxSubmissionAge::get();

            // Collect valid submissions
            let mut prices = Vec::new();
            for (_oracle, submission) in PriceSubmissions::<T>::iter_prefix(asset_id) {
                // Check age
                if current_timestamp.saturating_sub(submission.timestamp) > max_age {
                    continue;
                }
                prices.push(submission.price);
            }

            let submission_count = prices.len() as u32;

            // Need minimum submissions for valid median
            if submission_count < T::MinSubmissionsForMedian::get() {
                return Ok(());
            }

            // Sort prices for median calculation
            prices.sort_unstable();

            // Calculate median (simple average of middle values for even count)
            let median_price = if prices.len() % 2 == 0 {
                let mid = prices.len() / 2;
                let a = prices[mid - 1];
                let b = prices[mid];
                // Simple average - in production might want more sophisticated median
                (a.saturating_add(b)) / 2
            } else {
                prices[prices.len() / 2]
            };

            let price_data = PriceData::<BlockNumberFor<T>> {
                price: median_price,
                submission_count,
                last_updated: current_block,
                timestamp: current_timestamp,
            };

            AssetPrices::<T>::insert(asset_id, price_data);

            Self::deposit_event(Event::PriceUpdated {
                asset_id,
                price: median_price,
                submission_count,
                median_block: current_block,
            });

            Ok(())
        }

        /// Clean up old submissions (called by off-chain worker or governance)
        pub fn cleanup_old_submissions(asset_id: AssetId, max_age: Timestamp) -> u32 {
            let current_timestamp = Self::current_timestamp();
            let mut removed = 0u32;

            // This is a simplified cleanup - in production would need more efficient iteration
            let keys_to_remove: Vec<T::AccountId> = PriceSubmissions::<T>::iter_prefix(asset_id)
                .filter(|(_oracle, submission)| {
                    current_timestamp.saturating_sub(submission.timestamp) > max_age
                })
                .map(|(oracle, _)| oracle)
                .collect();

            for oracle in keys_to_remove {
                PriceSubmissions::<T>::remove(asset_id, &oracle);
                removed = removed.saturating_add(1);
            }

            removed
        }
    }
}
