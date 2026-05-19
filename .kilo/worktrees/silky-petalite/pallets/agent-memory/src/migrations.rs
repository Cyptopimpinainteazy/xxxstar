//! Runtime storage migrations for `pallet-agent-memory`.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Use an explicit target StorageVersion to avoid referencing the pallet's
        // private `STORAGE_VERSION` constant from this sibling module.
        let target = StorageVersion::new(1);
        if StorageVersion::get::<pallet::Pallet<T>>() < target {
            StorageVersion::put::<pallet::Pallet<T>>(&target);
            Weight::from_parts(2u64, 0)
        } else {
            Weight::zero()
        }
    }
}
