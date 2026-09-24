//! Runtime storage migrations for `pallet-agent-accounts`.

use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Read the pallet's declared version instead of restating it: a literal
        // here would drift from `STORAGE_VERSION` on the next version bump and
        // leave the on-chain version behind the code.
        let target = pallet::STORAGE_VERSION;
        if StorageVersion::get::<pallet::Pallet<T>>() < target {
            StorageVersion::put::<pallet::Pallet<T>>(&target);
            Weight::from_parts(2u64, 0)
        } else {
            Weight::zero()
        }
    }
}
