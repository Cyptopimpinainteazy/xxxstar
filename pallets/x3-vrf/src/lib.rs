#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 VRF Pallet
//!
//! Provides verifiable randomness generation using VRF (Verifiable Random Function).
//! Enables fair lotteries, random selections, and cryptographic randomness proofs.

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
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::{H256, U256};
    use sp_runtime::{
        traits::{AtLeast32BitUnsigned, CheckedAdd, SaturatedConversion},
        Saturating,
    };
    use sp_std::vec::Vec;
    use x3_vrf::{RandomnessRequest, RandomnessResult};

    /// Maximum number of pending randomness requests per account
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency trait for fee deduction and balance management.
        type Currency: frame_support::traits::ReservableCurrency<Self::AccountId>;

        /// Balance type used for fees.
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaxEncodedLen
            + CheckedAdd
            + From<u128>
            + Saturating
            + From<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>
            + Into<<Self::Currency as frame_support::traits::Currency<Self::AccountId>>::Balance>;

        /// Maximum pending requests per account
        #[pallet::constant]
        type MaxPendingRequests: Get<u32>;

        /// Base fee for randomness requests
        #[pallet::constant]
        type BaseFee: Get<Self::Balance>;

        /// Fee per byte of seed data
        #[pallet::constant]
        type FeePerByte: Get<Self::Balance>;

        /// Maximum seed length in bytes
        #[pallet::constant]
        type MaxSeedLength: Get<u32>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Pending randomness requests
    #[pallet::storage]
    #[pallet::getter(fn pending_requests)]
    pub type PendingRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // request_id
        RandomnessRequest,
        OptionQuery,
    >;

    /// Fulfilled randomness results
    #[pallet::storage]
    #[pallet::getter(fn fulfilled_requests)]
    pub type FulfilledRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // request_id
        RandomnessResult,
        OptionQuery,
    >;

    /// Request IDs per account (for iteration and limits)
    #[pallet::storage]
    #[pallet::getter(fn account_requests)]
    pub type AccountRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<H256, <T as Config>::MaxPendingRequests>,
        ValueQuery,
    >;

    /// Global request counter for unique IDs
    #[pallet::storage]
    pub type RequestCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A randomness request was submitted
        RandomnessRequested {
            request_id: H256,
            requester: T::AccountId,
            fee: T::Balance,
        },
        /// Randomness request was fulfilled
        RandomnessFulfilled {
            request_id: H256,
            randomness: [u8; 32],
        },
        /// Randomness request was cancelled
        RandomnessCancelled {
            request_id: H256,
            requester: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Request ID not found
        RequestNotFound,
        /// Request already fulfilled
        RequestAlreadyFulfilled,
        /// Request not owned by caller
        NotRequestOwner,
        /// Too many pending requests for account
        TooManyPendingRequests,
        /// Insufficient balance for fee
        InsufficientBalance,
        /// Seed data too long
        SeedTooLong,
        /// VRF generation failed
        VrfGenerationFailed,
        /// Fee calculation overflow
        FeeOverflow,
        /// Invalid request ID
        InvalidRequestId,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request verifiable randomness
        ///
        /// The fee is calculated as: base_fee + (seed_length * fee_per_byte)
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::request_randomness())]
        pub fn request_randomness(
            origin: OriginFor<T>,
            seed: Vec<u8>,
            max_fee: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate seed length
            ensure!(
                seed.len() <= T::MaxSeedLength::get() as usize,
                Error::<T>::SeedTooLong
            );

            // Check pending request limit
            let current_requests = AccountRequests::<T>::get(&who);
            ensure!(
                current_requests.len() < T::MaxPendingRequests::get() as usize,
                Error::<T>::TooManyPendingRequests
            );

            // Calculate fee
            let seed_fee = T::FeePerByte::get().saturating_mul((seed.len() as u32).into());
            let total_fee = T::BaseFee::get().saturating_add(seed_fee);
            ensure!(total_fee <= max_fee, Error::<T>::FeeOverflow);

            // Check balance
            ensure!(
                T::Currency::free_balance(&who) >= total_fee.into(),
                Error::<T>::InsufficientBalance
            );

            // Generate request ID
            let request_counter = RequestCounter::<T>::get();
            let request_id = Self::generate_request_id(request_counter, &who, &seed);
            RequestCounter::<T>::put(request_counter.saturating_add(1));

            // Reserve fee
            T::Currency::reserve(&who, total_fee.into())?;

            // Create seed hash for VRF
            let seed_hash = sp_io::hashing::blake2_256(&seed);
            let seed_h256 = H256::from(seed_hash);

            // Create request
            let current_block = frame_system::Pallet::<T>::block_number();
            let request = RandomnessRequest {
                request_id,
                seed: seed_h256,
                block_number: current_block.saturated_into::<u64>(),
                max_fee: U256::from(total_fee.saturated_into::<u128>()),
            };

            // Store request
            PendingRequests::<T>::insert(request_id, request);
            AccountRequests::<T>::mutate(&who, |requests| {
                requests.try_push(request_id).ok();
            });

            Self::deposit_event(Event::RandomnessRequested {
                request_id,
                requester: who,
                fee: total_fee,
            });

            Ok(())
        }

        /// Fulfill a pending randomness request (typically called by off-chain worker)
        ///
        /// In production, this would be restricted to authorized fulfillers
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::fulfill_randomness())]
        pub fn fulfill_randomness(origin: OriginFor<T>, request_id: H256) -> DispatchResult {
            let _ = ensure_signed(origin)?; // In production, check if authorized fulfiller

            // Check not already fulfilled (do this first, before checking pending)
            ensure!(
                !FulfilledRequests::<T>::contains_key(request_id),
                Error::<T>::RequestAlreadyFulfilled
            );

            // Get pending request
            let request =
                PendingRequests::<T>::get(request_id).ok_or(Error::<T>::RequestNotFound)?;

            // Generate VRF randomness
            let vrf_provider = x3_vrf::get_vrf_provider();
            let proof = vrf_provider
                .prove(&request.seed.0)
                .map_err(|_| Error::<T>::VrfGenerationFailed)?;

            // Create result
            let current_block = frame_system::Pallet::<T>::block_number();
            let result = RandomnessResult {
                request_id,
                randomness: proof.output,
                proof,
                fulfilled_block: current_block.saturated_into::<u64>(),
            };

            // Store result
            FulfilledRequests::<T>::insert(request_id, result.clone());

            // Remove from pending
            PendingRequests::<T>::remove(request_id);

            // Find and remove from account's requests
            let requester = Self::find_requester(request_id)?;
            AccountRequests::<T>::mutate(&requester, |requests| {
                if let Some(pos) = requests.iter().position(|&id| id == request_id) {
                    requests.swap_remove(pos);
                }
            });

            // Unreserve fee (fulfillment successful)
            let fee_amount: T::Balance = request.max_fee.saturated_into::<u128>().into();
            T::Currency::unreserve(&requester, fee_amount.into());

            Self::deposit_event(Event::RandomnessFulfilled {
                request_id,
                randomness: result.randomness,
            });

            Ok(())
        }

        /// Cancel a pending randomness request and refund the fee
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::cancel_randomness())]
        pub fn cancel_randomness(origin: OriginFor<T>, request_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check not already fulfilled (do this first, before checking pending)
            ensure!(
                !FulfilledRequests::<T>::contains_key(request_id),
                Error::<T>::RequestAlreadyFulfilled
            );

            // Get pending request
            let request =
                PendingRequests::<T>::get(request_id).ok_or(Error::<T>::RequestNotFound)?;

            // Check ownership
            ensure!(
                Self::is_request_owner(&who, request_id),
                Error::<T>::NotRequestOwner
            );

            // Remove from storage
            PendingRequests::<T>::remove(request_id);
            AccountRequests::<T>::mutate(&who, |requests| {
                if let Some(pos) = requests.iter().position(|&id| id == request_id) {
                    requests.swap_remove(pos);
                }
            });

            // Refund fee
            let fee_amount: T::Balance = request.max_fee.saturated_into::<u128>().into();
            T::Currency::unreserve(&who, fee_amount.into());

            Self::deposit_event(Event::RandomnessCancelled {
                request_id,
                requester: who,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Generate a unique request ID
        fn generate_request_id(counter: u64, account: &T::AccountId, seed: &[u8]) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(&counter.to_le_bytes());
            data.extend_from_slice(&account.encode());
            data.extend_from_slice(seed);
            H256::from(sp_io::hashing::blake2_256(&data))
        }

        /// Find the account that owns a request
        fn find_requester(request_id: H256) -> Result<T::AccountId, Error<T>> {
            for (account, requests) in AccountRequests::<T>::iter() {
                if requests.contains(&request_id) {
                    return Ok(account);
                }
            }
            Err(Error::<T>::RequestNotFound)
        }

        /// Check if account owns the request
        fn is_request_owner(account: &T::AccountId, request_id: H256) -> bool {
            AccountRequests::<T>::get(account).contains(&request_id)
        }

        /// Get randomness result (public read function)
        pub fn get_randomness(request_id: H256) -> Option<RandomnessResult> {
            FulfilledRequests::<T>::get(request_id)
        }

        /// Get pending request (public read function)
        pub fn get_pending_request(request_id: H256) -> Option<RandomnessRequest> {
            PendingRequests::<T>::get(request_id)
        }
    }
}
