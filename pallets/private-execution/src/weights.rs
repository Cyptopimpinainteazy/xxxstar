//! Weight info for Private Execution pallet.
//!
//! These weights account for TEE attestation verification (CPU-heavy),
//! ZK-proof validation, and threshold cryptography overhead.
//! Re-run benchmarks on TEE-equipped hardware before mainnet.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions for the private execution pallet.
pub trait WeightInfo {
    fn register_confidential_validator() -> Weight;
    fn deregister_confidential_validator() -> Weight;
    fn refresh_attestation() -> Weight;
    fn submit_private_transaction() -> Weight;
    fn commit_encrypted_state_diff() -> Weight;
    fn set_committee_key() -> Weight;
    fn set_enabled() -> Weight;
}

/// Production weights using runtime-configurable DB costs.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Includes TEE attestation parse + signature verify (~50M extra).
    /// Storage: `PrivateExecution::Validators` (r:1 w:1), `Balances::Reserves` (r:1 w:1).
    fn register_confidential_validator() -> Weight {
        Weight::from_parts(112_000_000, 2_048)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn deregister_confidential_validator() -> Weight {
        Weight::from_parts(42_000_000, 1_024)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    /// Includes TEE attestation re-verification (~50M extra).
    fn refresh_attestation() -> Weight {
        Weight::from_parts(102_000_000, 2_048)
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    /// Includes ZK-SNARK proof verify (~80M extra) + threshold signature check.
    /// Storage: `PrivateExecution::PendingTxs` (r:1 w:1), `PrivateExecution::Validators` (r:2 w:0).
    fn submit_private_transaction() -> Weight {
        Weight::from_parts(162_000_000, 2_048)
            .saturating_add(T::DbWeight::get().reads(3_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    /// Threshold signature verification over encrypted state diff.
    fn commit_encrypted_state_diff() -> Weight {
        Weight::from_parts(182_000_000, 3_072)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(2_u64))
    }
    fn set_committee_key() -> Weight {
        Weight::from_parts(22_000_000, 512)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
    fn set_enabled() -> Weight {
        Weight::from_parts(12_000_000, 128)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

impl WeightInfo for () {
    fn register_confidential_validator() -> Weight {
        Weight::from_parts(112_000_000, 2_048).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn deregister_confidential_validator() -> Weight {
        Weight::from_parts(42_000_000, 1_024).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn refresh_attestation() -> Weight {
        Weight::from_parts(102_000_000, 2_048).saturating_add(RocksDbWeight::get().reads_writes(1, 1))
    }
    fn submit_private_transaction() -> Weight {
        Weight::from_parts(162_000_000, 2_048).saturating_add(RocksDbWeight::get().reads_writes(3, 2))
    }
    fn commit_encrypted_state_diff() -> Weight {
        Weight::from_parts(182_000_000, 3_072).saturating_add(RocksDbWeight::get().reads_writes(2, 2))
    }
    fn set_committee_key() -> Weight {
        Weight::from_parts(22_000_000, 512).saturating_add(RocksDbWeight::get().writes(1))
    }
    fn set_enabled() -> Weight {
        Weight::from_parts(12_000_000, 128).saturating_add(RocksDbWeight::get().writes(1))
    }
}
