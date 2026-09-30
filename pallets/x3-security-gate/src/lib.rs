// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-security-gate — versioned Guardian security policy.
//
// Holds the rules an application must satisfy: required test classes, severity
// thresholds, and per-category / per-VM requirements. Policies are versioned
// and activated, never silently reinterpreted (spec §28).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}
}
