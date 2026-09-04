//! Runtime storage migrations for `pallet-treasury`.

use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Avoid referencing the pallet's private `STORAGE_VERSION` from this module.
        let target = StorageVersion::new(1);
        if StorageVersion::get::<pallet::Pallet<T>>() < target {
            StorageVersion::put::<pallet::Pallet<T>>(&target);
            Weight::from_parts(2u64, 0)
        } else {
            Weight::zero()
        }
    }
}
