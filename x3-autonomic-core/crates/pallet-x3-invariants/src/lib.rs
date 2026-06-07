//! Pallet X3 Invariants
//! 
//! Substrate pallet for invariant checking on X3 chain

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use x3_autonomic_types::{AutonomyLevel, InvariantDefinition, Severity};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Configuration for this pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Storage for registered invariants
    #[pallet::storage]
    #[pallet::getter(fn invariants)]
    pub type Invariants<T> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        InvariantDefinition,
        OptionQuery,
    >;

    /// Current autonomy level
    #[pallet::storage]
    #[pallet::getter(fn autonomy_level)]
    pub type AutonomyLevel<T> = StorageValue<_, x3_autonomic_types::AutonomyLevel, ValueQuery>;

    /// Whether invariant checking is enabled
    #[pallet::storage]
    #[pallet::getter(fn checking_enabled)]
    pub type CheckingEnabled<T> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T> {
        /// An invariant was registered
        InvariantRegistered { id: Vec<u8> },
        /// An invariant check failed
        InvariantFailed { id: Vec<u8>, severity: Severity },
        /// Autonomy level changed
        AutonomyLevelChanged { old: AutonomyLevel, new: AutonomyLevel },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invariant already exists
        InvariantExists,
        /// Invariant not found
        InvariantNotFound,
        /// Not authorized
        NotAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new invariant
        #[pallet::call_index(0)]
        pub fn register_invariant(
            origin: OriginFor<T>,
            invariant: InvariantDefinition,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            ensure!(
                !Invariants::<T>::contains_key(&invariant.id),
                Error::<T>::InvariantExists
            );
            Invariants::<T>::insert(&invariant.id, &invariant);
            Self::deposit_event(Event::InvariantRegistered { id: invariant.id });
            Ok(())
        }

        /// Set autonomy level
        #[pallet::call_index(1)]
        pub fn set_autonomy_level(
            origin: OriginFor<T>,
            level: AutonomyLevel,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let old = AutonomyLevel::<T>::get();
            AutonomyLevel::<T>::set(level.clone());
            Self::deposit_event(Event::AutonomyLevelChanged { old, new: level });
            Ok(())
        }

        /// Enable or disable invariant checking
        #[pallet::call_index(2)]
        pub fn set_checking_enabled(
            origin: OriginFor<T>,
            enabled: bool,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            CheckingEnabled::<T>::set(enabled);
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Check if an invariant is satisfied
    pub fn check_invariant(id: &[u8]) -> bool {
        // Simplified - real implementation would execute invariant logic
        true
    }

    /// Emit an invariant failure event
    pub fn emit_invariant_failure(id: Vec<u8>, severity: Severity) {
        Self::deposit_event(Event::InvariantFailed { id, severity });
    }
}