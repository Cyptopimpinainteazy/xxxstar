//! Runtime storage migrations for `pallet-treasury`.

use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

use crate::pallet;

pub struct Migration<T>(PhantomData<T>);

impl<T: crate::Config> OnRuntimeUpgrade for Migration<T> {
    fn on_runtime_upgrade() -> Weight {
        // Read the pallet's declared version instead of restating it. A
        // hardcoded target silently disagrees with `STORAGE_VERSION` the moment
        // the pallet version is bumped: the on-chain version would stop at the
        // stale literal while the code moved on, which is precisely the
        // "bumped the version, changed nothing" failure this module must make
        // impossible. `runtime_upgrade_rehearsal_migrations_advance_behind_versions`
        // now fails if this module and the pallet ever disagree.
        let target = pallet::STORAGE_VERSION;
        if StorageVersion::get::<pallet::Pallet<T>>() < target {
            StorageVersion::put::<pallet::Pallet<T>>(&target);
            Weight::from_parts(2u64, 0)
        } else {
            Weight::zero()
        }
    }
}
