// SPDX-License-Identifier: Apache-2.0
//
// Weights for pallet-x3-security-gate.
//
// NOTE: these are NOT benchmark-measured. They are a DB-access-weighted
// fallback (`RocksDbWeight` reads/writes) so that every dispatchable charges
// *something* honest instead of a hand-picked literal. Before mainnet, run the
// FRAME benchmark CLI for this pallet and replace this file with the generated
// weights.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions needed for `pallet_x3_security_gate`.
pub trait WeightInfo {
	fn create_ruleset() -> Weight;
	fn set_category_policy() -> Weight;
	fn set_vm_policy() -> Weight;
	fn activate_ruleset() -> Weight;
	fn deprecate_ruleset() -> Weight;
	fn record_severity_exception() -> Weight;
	fn revoke_severity_exception() -> Weight;
}

/// Provisional weights using the node's configured DB weight.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn create_ruleset() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}

	fn set_category_policy() -> Weight {
		Weight::from_parts(30_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}

	fn set_vm_policy() -> Weight {
		Weight::from_parts(30_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}

	fn activate_ruleset() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}

	fn deprecate_ruleset() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}

	fn record_severity_exception() -> Weight {
		Weight::from_parts(35_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}

	fn revoke_severity_exception() -> Weight {
		Weight::from_parts(25_000_000, 0)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

impl WeightInfo for () {
	fn create_ruleset() -> Weight {
		RocksDbWeight::get().reads_writes(1_u64, 2_u64)
	}

	fn set_category_policy() -> Weight {
		RocksDbWeight::get().reads_writes(2_u64, 2_u64)
	}

	fn set_vm_policy() -> Weight {
		RocksDbWeight::get().reads_writes(2_u64, 2_u64)
	}

	fn activate_ruleset() -> Weight {
		RocksDbWeight::get().reads_writes(1_u64, 2_u64)
	}

	fn deprecate_ruleset() -> Weight {
		RocksDbWeight::get().reads_writes(2_u64, 1_u64)
	}

	fn record_severity_exception() -> Weight {
		RocksDbWeight::get().reads_writes(2_u64, 1_u64)
	}

	fn revoke_severity_exception() -> Weight {
		RocksDbWeight::get().reads_writes(1_u64, 1_u64)
	}
}
