//! Runtime storage migrations for `pallet-x3-kernel`.

use frame_support::traits::{Get, OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Current migration: record storage version 2 (`X3ExecutionReceipts` was added, and it
        // starts empty, so nothing has to be rewritten — only the version has to move so an
        // operator can tell an upgraded chain from one still on the old layout).
        if StorageVersion::get::<pallet::Pallet<T>>() < pallet::STORAGE_VERSION {
            pallet::STORAGE_VERSION.put::<pallet::Pallet<T>>();
            // Reads: get; Writes: put
            T::DbWeight::get().reads_writes(1, 1)
        } else {
            Weight::zero()
        }
    }
}
