#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 Consensus Pallet
//!
//! Integrates Aura block production and Grandpa finality with validator management
//! and slashing for consensus violations.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::weights::WeightInfo as _;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_runtime::{traits::Saturating, Perbill};
    use sp_staking::{
        offence::{OffenceDetails, OnOffenceHandler},
        SessionIndex,
    };
    use sp_std::vec::Vec;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_aura::Config + pallet_grandpa::Config + pallet_session::Config
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of active validators
        #[pallet::constant]
        type MaxValidators: Get<u32>;

        /// Fraction slashed per offence type, in basis points (0–10000).
        /// E.g. 1000 means 10 % of the validator's current stake is deducted.
        type SlashFraction: Get<u32>;

        /// Minimum stake that must remain after any slash.
        /// Prevents zeroing the stake entirely in a single report.
        type MinStakeAfterSlash: Get<u128>;

        /// Weight information for extrinsics
        type WeightInfo: crate::weights::WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Current active validator set
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxValidators>, ValueQuery>;

    /// Next validator set (pending activation)
    #[pallet::storage]
    #[pallet::getter(fn next_validators)]
    pub type NextValidators<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxValidators>, ValueQuery>;

    /// Block number when next validator set should be activated
    #[pallet::storage]
    pub type ValidatorSetActivationBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, OptionQuery>;

    /// Per-validator stake tracking used for slashing.
    /// Populated when a validator is registered; updated on every slash.
    #[pallet::storage]
    #[pallet::getter(fn validator_stake)]
    pub type ValidatorStake<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorInfo, OptionQuery>;

    /// Consensus state tracking
    #[pallet::storage]
    #[pallet::getter(fn consensus_state)]
    pub type ConsensusState<T: Config> =
        StorageValue<_, ConsensusInfo<BlockNumberFor<T>>, ValueQuery>;

    /// Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New validator set activated
        ValidatorSetChanged { validators: Vec<T::AccountId> },
        /// Consensus state updated
        ConsensusStateUpdated { block_number: BlockNumberFor<T> },
        /// Validator slashed for misbehavior
        ValidatorSlashed {
            validator: T::AccountId,
            reason: SlashReason,
        },
        /// Slash amount actually deducted from validator stake
        SlashApplied {
            validator: T::AccountId,
            slash_amount: u128,
            new_stake: u128,
        },
    }

    /// Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Too many validators specified
        TooManyValidators,
        /// Invalid validator set
        InvalidValidatorSet,
        /// Consensus not initialized
        ConsensusNotInitialized,
        /// Validator not found in the stake registry
        ValidatorNotFound,
    }

    /// Per-validator stake record stored in [`ValidatorStake`].
    #[derive(
        Clone,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        Debug,
        PartialEq,
        Eq,
        Default,
    )]
    pub struct ValidatorInfo {
        /// Staked balance in the smallest token unit.
        pub stake: u128,
        /// Whether the validator is currently participating in block production.
        /// Set to `false` when the stake falls to or below `MinStakeAfterSlash`.
        pub is_active: bool,
    }

    /// Trait to convert a consensus identification tuple into the underlying validator account.
    pub trait IdentificationToAccountId<AccountId> {
        fn validator_account(&self) -> &AccountId;
    }

    impl<AccountId, Extra> IdentificationToAccountId<AccountId> for (AccountId, Extra) {
        fn validator_account(&self) -> &AccountId {
            &self.0
        }
    }

    /// Consensus information snapshot. Authorities are stored as encoded bytes to keep the
    /// snapshot type stable across runtime upgrades that may change Aura/Grandpa authority types.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq,
    )]
    #[scale_info(skip_type_params(BlockNumber))]
    pub struct ConsensusInfo<BlockNumber: MaxEncodedLen> {
        /// Current block number
        pub block_number: BlockNumber,
        /// Number of active Aura authorities
        pub aura_authority_count: u32,
        /// Number of active Grandpa authorities
        pub grandpa_authority_count: u32,
        /// Last finalized grandpa set id
        pub last_finalized_set_id: u64,
    }

    /// Slash reasons
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq,
    )]
    pub enum SlashReason {
        /// Double signing detected
        DoubleSign,
        /// Equivocation in consensus
        Equivocation,
        /// Missing blocks
        MissingBlocks,
        /// Invalid finality proof
        InvalidFinality,
    }

    impl<BlockNumber> Default for ConsensusInfo<BlockNumber>
    where
        BlockNumber: Default + MaxEncodedLen,
    {
        fn default() -> Self {
            Self {
                block_number: Default::default(),
                aura_authority_count: 0,
                grandpa_authority_count: 0,
                last_finalized_set_id: 0,
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the next validator set (requires governance approval)
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::set_validators())]
        pub fn set_validators(
            origin: OriginFor<T>,
            validators: Vec<T::AccountId>,
            activation_delay: BlockNumberFor<T>,
        ) -> DispatchResult {
            // Only governance can change validators
            ensure_root(origin)?;

            let bounded_validators =
                BoundedVec::try_from(validators).map_err(|_| Error::<T>::TooManyValidators)?;

            let activation_block =
                frame_system::Pallet::<T>::block_number().saturating_add(activation_delay);

            NextValidators::<T>::put(bounded_validators.clone());
            ValidatorSetActivationBlock::<T>::put(activation_block);

            Self::deposit_event(Event::ValidatorSetChanged {
                validators: bounded_validators.into_inner(),
            });

            Ok(())
        }

        /// Report validator misbehavior and apply a proportional stake slash.
        ///
        /// The caller must be a signed account. The slash amount is computed as
        /// `SlashFraction` basis points of the validator's current stake, with
        /// the result floored at `MinStakeAfterSlash` to prevent a complete
        /// zeroing in a single report. When the remaining stake reaches that
        /// floor the validator is also marked inactive.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::report_misbehavior())]
        pub fn report_misbehavior(
            origin: OriginFor<T>,
            validator: T::AccountId,
            reason: SlashReason,
        ) -> DispatchResult {
            let _reporter = ensure_signed(origin)?;

            let slash_fraction =
                Perbill::from_parts(T::SlashFraction::get().saturating_mul(100_000));
            Self::slash_validator(&validator, reason, slash_fraction);

            Ok(())
        }
    }

    impl<T: Config + pallet_offences::Config>
        OnOffenceHandler<T::AccountId, T::IdentificationTuple, Weight> for Pallet<T>
    where
        T::IdentificationTuple: IdentificationToAccountId<T::AccountId> + Clone,
    {
        fn on_offence(
            offenders: &[OffenceDetails<T::AccountId, T::IdentificationTuple>],
            slash_fraction: &[Perbill],
            _session: SessionIndex,
        ) -> Weight {
            for (offence, fraction) in offenders.iter().zip(slash_fraction.iter()) {
                let validator = offence.offender.validator_account().clone();
                Self::slash_validator(&validator, SlashReason::Equivocation, *fraction);
            }
            Weight::zero()
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // Check if we need to activate a new validator set
            if let Some(activation_block) = ValidatorSetActivationBlock::<T>::get() {
                if n >= activation_block {
                    Self::activate_validator_set();
                    ValidatorSetActivationBlock::<T>::kill();
                }
            }

            // Update consensus state
            Self::update_consensus_state(n);

            <T as Config>::WeightInfo::on_initialize()
        }
    }

    impl<T: Config> Pallet<T> {
        /// Activate the next validator set
        fn activate_validator_set() {
            if let Ok(next_validators) = NextValidators::<T>::try_get() {
                Validators::<T>::put(next_validators.clone());
                Self::deposit_event(Event::ValidatorSetChanged {
                    validators: next_validators.into_inner(),
                });
            }
        }

        /// Update the current consensus state
        fn update_consensus_state(current_block: BlockNumberFor<T>) {
            let aura_authority_count = pallet_aura::Authorities::<T>::get().len() as u32;
            let grandpa_authority_count =
                pallet_grandpa::Pallet::<T>::grandpa_authorities().len() as u32;

            let consensus_info = ConsensusInfo {
                block_number: current_block,
                aura_authority_count,
                grandpa_authority_count,
                last_finalized_set_id: pallet_grandpa::Pallet::<T>::current_set_id(),
            };

            ConsensusState::<T>::put(consensus_info);

            Self::deposit_event(Event::ConsensusStateUpdated {
                block_number: current_block,
            });
        }

        /// Get current validator set
        pub fn current_validators() -> Vec<T::AccountId> {
            Validators::<T>::get().into_inner()
        }

        /// Check if account is a validator
        pub fn is_validator(who: &T::AccountId) -> bool {
            Validators::<T>::get().contains(who)
        }

        /// Slash a validator by a dynamic perbill fraction.
        pub fn slash_validator(
            validator: &T::AccountId,
            reason: SlashReason,
            slash_fraction: Perbill,
        ) {
            if let Some(mut info) = ValidatorStake::<T>::get(validator) {
                let slash_amount = slash_fraction * info.stake;
                let min_remaining = T::MinStakeAfterSlash::get();
                let new_stake = info.stake.saturating_sub(slash_amount).max(min_remaining);
                let actual_slash = info.stake.saturating_sub(new_stake);
                info.stake = new_stake;

                if new_stake <= min_remaining {
                    info.is_active = false;
                }

                ValidatorStake::<T>::insert(validator, &info);
                Self::deposit_event(Event::ValidatorSlashed {
                    validator: validator.clone(),
                    reason,
                });
                Self::deposit_event(Event::SlashApplied {
                    validator: validator.clone(),
                    slash_amount: actual_slash,
                    new_stake,
                });
            }
        }
    }

    impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
        fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
            // Return the pending validator set if one is queued, else None (keep current).
            // `try_get` returns Err(()) when the key is absent in storage (ValueQuery
            // default is not in storage), correctly signalling "no rotation needed" to
            // the session pallet when nothing has been queued via `set_validators`.
            NextValidators::<T>::try_get()
                .ok()
                .map(|set| set.into_inner())
        }

        fn end_session(_end_index: u32) {}

        fn start_session(_start_index: u32) {
            // Clear pending set once the session pallet has activated it.
            NextValidators::<T>::kill();
        }
    }
}

/// Weight information for the consensus pallet
pub mod weights {
    use frame_support::weights::Weight;

    /// Weight functions for pallet_x3_consensus
    pub trait WeightInfo {
        fn set_validators() -> Weight;
        fn report_misbehavior() -> Weight;
        fn on_initialize() -> Weight;
    }

    /// Default weight implementation
    pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

    impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
        fn set_validators() -> Weight {
            Weight::from_parts(10_000_000, 0)
        }

        fn report_misbehavior() -> Weight {
            Weight::from_parts(5_000_000, 0)
        }

        fn on_initialize() -> Weight {
            Weight::from_parts(1_000_000, 0)
        }
    }
}
