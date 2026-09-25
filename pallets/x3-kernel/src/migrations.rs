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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};

    /// The version move a chain performs when it upgrades into this build.
    ///
    /// `X3ExecutionReceipts` arrived at storage version 2 and starts empty, so the migration has no
    /// data to rewrite — but it still has to record the version. Without that, an operator cannot
    /// tell an upgraded chain from one still on the old layout, and the pallet's own
    /// `on_runtime_upgrade` would run its version check on every later upgrade too.
    #[test]
    fn the_migration_records_storage_version_two() {
        new_test_ext().execute_with(|| {
            StorageVersion::put::<pallet::Pallet<Test>>(&StorageVersion::new(1));

            let weight = <Migration<Test> as OnRuntimeUpgrade>::on_runtime_upgrade();

            assert_eq!(
                StorageVersion::get::<pallet::Pallet<Test>>(),
                pallet::STORAGE_VERSION,
                "the migration must record the version this build declares"
            );
            let db_weight: frame_support::weights::RuntimeDbWeight =
                <Test as frame_system::Config>::DbWeight::get();
            assert_eq!(
                weight,
                db_weight.reads_writes(1, 1),
                "and it must charge exactly the read and write it performs"
            );
            assert_eq!(
                <Migration<Test> as OnRuntimeUpgrade>::on_runtime_upgrade(),
                Weight::zero(),
                "running it again on an already-migrated chain is a no-op, not another write"
            );
        });
    }

    /// The map the version move announced is real, and an upgraded chain starts with no receipts
    /// rather than with a fabricated one.
    #[test]
    fn the_receipt_map_starts_empty_on_an_upgraded_chain() {
        new_test_ext().execute_with(|| {
            StorageVersion::put::<pallet::Pallet<Test>>(&StorageVersion::new(1));
            let _ = <Migration<Test> as OnRuntimeUpgrade>::on_runtime_upgrade();

            let comit = sp_core::H256::from([0x11u8; 32]);
            assert!(
                pallet::X3ExecutionReceipts::<Test>::get(comit).is_none(),
                "a chain that has just upgraded has executed no X3 comit yet"
            );
        });
    }
}
