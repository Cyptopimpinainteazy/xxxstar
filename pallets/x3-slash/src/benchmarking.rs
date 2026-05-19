//! Benchmarks for the x3-slash pallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_core::H256;

const SEED: u32 = 0;

benchmarks! {
    post_bond {
        let agent: T::AccountId = frame_benchmarking::account("agent", 0, SEED);
        let amount = T::MinBondAmount::get() * 10u32.into();
    }: _(RawOrigin::Signed(agent.clone()), amount, None)
    verify {
        let bonds = BondsByAgent::<T>::get(&agent);
        assert_eq!(bonds.len(), 1);
    }

    release_bond {
        let agent: T::AccountId = frame_benchmarking::account("agent", 0, SEED);
        let amount = T::MinBondAmount::get() * 10u32.into();

        // Post bond first
        Pallet::<T>::post_bond(
            RawOrigin::Signed(agent.clone()).into(),
            amount,
            None
        ).ok();

        let bonds = BondsByAgent::<T>::get(&agent);
        let bond_id = bonds[0];
    }: _(RawOrigin::Signed(agent.clone()), bond_id)
    verify {
        let bond = Bonds::<T>::get(bond_id).unwrap();
        assert_eq!(bond.status, BondStatus::Released);
    }

    slash_bond {
        let agent: T::AccountId = frame_benchmarking::account("agent", 0, SEED);
        let amount = T::MinBondAmount::get() * 10u32.into();

        // Post bond first
        Pallet::<T>::post_bond(
            RawOrigin::Signed(agent.clone()).into(),
            amount,
            None
        ).ok();

        let bonds = BondsByAgent::<T>::get(&agent);
        let bond_id = bonds[0];
    }: _(RawOrigin::Signed(agent.clone()), bond_id, 2u8, vec![1u8; 64])
    verify {
        let bond = Bonds::<T>::get(bond_id).unwrap();
        assert_eq!(bond.status, BondStatus::FullySlashed);
    }

    process_expirations {
    }: _(RawOrigin::Root)
    verify {
        // Just verify the extrinsic succeeds
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
