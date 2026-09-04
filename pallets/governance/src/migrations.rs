//! Runtime storage migrations for `pallet-governance`.

use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

#[allow(dead_code)]
pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        if StorageVersion::get::<pallet::Pallet<T>>() < pallet::STORAGE_VERSION {
            StorageVersion::put::<pallet::Pallet<T>>(&pallet::STORAGE_VERSION);
            Weight::from_parts(2_000, 0)
        } else {
            Weight::zero()
        }
    }
}
