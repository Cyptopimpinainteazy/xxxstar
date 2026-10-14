// SPDX-License-Identifier: Apache-2.0
//
// Weights for pallet-x3-app-registry.
//
// NOTE: these are NOT benchmark-measured. They are a DB-access-weighted fallback
// (`RocksDbWeight` reads/writes) so that every dispatchable charges *something
// honest* instead of a hand-picked literal. Before mainnet, run the FRAME
// benchmark CLI for this pallet (`scripts/run-frame-benchmarks.sh`) and replace
// this file with the generated weights.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions needed for `pallet_x3_app_registry`.
pub trait WeightInfo {
	fn register_application() -> Weight;
	fn add_version() -> Weight;
	fn submit_for_review() -> Weight;
	fn certify_application() -> Weight;
	fn restrict_application() -> Weight;
	fn revoke_application() -> Weight;
	fn bind_address() -> Weight;
	fn unbind_address() -> Weight;
	fn register_manifest() -> Weight;
}

/// Provisional weights using the node's configured DB weight.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn register_application() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn add_version() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn submit_for_review() -> Weight {
		Weight::from_parts(15_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn certify_application() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn restrict_application() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn revoke_application() -> Weight {
		Weight::from_parts(26_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	fn bind_address() -> Weight {
		Weight::from_parts(24_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn unbind_address() -> Weight {
		Weight::from_parts(20_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn register_manifest() -> Weight {
		Weight::from_parts(24_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}

impl WeightInfo for () {
	fn register_application() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn add_version() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn submit_for_review() -> Weight {
		Weight::from_parts(15_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn certify_application() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn restrict_application() -> Weight {
		Weight::from_parts(22_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn revoke_application() -> Weight {
		Weight::from_parts(26_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	fn bind_address() -> Weight {
		Weight::from_parts(24_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn unbind_address() -> Weight {
		Weight::from_parts(20_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn register_manifest() -> Weight {
		Weight::from_parts(24_000_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}
