//! Weight information for x3-slash pallet extrinsics.

use frame_support::traits::Get;
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

/// Weight functions trait.
pub trait WeightInfo {
    fn post_bond() -> Weight;
    fn release_bond() -> Weight;
    fn slash_bond() -> Weight;
    fn process_expirations() -> Weight;
}

/// Default weights for testing.
impl WeightInfo for () {
    fn post_bond() -> Weight {
        Weight::from_parts(75_000_000, 0)
    }
    fn release_bond() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }
    fn slash_bond() -> Weight {
        Weight::from_parts(150_000_000, 0)
    }
    fn process_expirations() -> Weight {
        Weight::from_parts(200_000_000, 0)
    }
}

/// Substrate weight implementation (derived from benchmarks).
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn post_bond() -> Weight {
        Weight::from_parts(75_000_000, 4000)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn release_bond() -> Weight {
        Weight::from_parts(50_000_000, 3000)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn slash_bond() -> Weight {
        // Slashing involves:
        // - Reading bond state
        // - Computing slash amount
        // - Updating bond status
        // - Recording slash
        // - Updating reputation
        Weight::from_parts(150_000_000, 8000)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn process_expirations() -> Weight {
        // This is variable based on number of expired bonds
        // Using a conservative estimate of 100 bonds processed
        Weight::from_parts(200_000_000, 10000)
            .saturating_add(T::DbWeight::get().reads(150))
            .saturating_add(T::DbWeight::get().writes(150))
    }
}
