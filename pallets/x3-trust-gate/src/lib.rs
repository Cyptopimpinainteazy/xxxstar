// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-trust-gate — X3 Guardian trust gate.
//
// Separates "the code does what it says" from "what the code is allowed to do to
// users". Risky capabilities (mint, freeze, seize, drain, upgrade, tax, oracle
// control) are recorded as a machine-readable Privilege Map and judged against
// the declared application category. Risky functionality is surfaced, never
// assumed malicious (spec §2B, §16).

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
