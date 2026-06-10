//! Benchmarking for pallet-x3-agent-registry.
//!
//! Run with: `cargo bench --package pallet-x3-agent-registry`

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as AgentRegistry;
use frame_benchmarking::v2::*;
use frame_support::{
    assert_ok,
    traits::{Currency, ReservableCurrency},
    BoundedVec,
};
use frame_system::RawOrigin;
use sp_std::vec;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        // Fund the caller
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), operator, name, metadata, AgentKind::AutonomousAgent);
    }

    #[benchmark]
    fn bind_atlas_id() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), 0, 42u64);
    }

    #[benchmark]
    fn update_operator() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let new_operator: T::AccountId = account("new_operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), 0, new_operator);
    }

    #[benchmark]
    fn update_permissions() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        let permissions = AgentPermissions {
            can_deploy: true,
            can_stake: true,
            can_vote: true,
            can_trade: true,
            can_transfer: true,
            can_call_contracts: true,
            can_submit_proofs: true,
            can_validate: false,
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), 0, permissions);
    }

    #[benchmark]
    fn update_quota() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, 0, 2_000_000u128, 1_000_000u128, 200_000_000u128, 100_000_000u128);
    }

    #[benchmark]
    fn suspend_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<u8, ConstU32<256>> = b"benchmark".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, 0, reason);
    }

    #[benchmark]
    fn reactivate_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();
        let reason: BoundedVec<u8, ConstU32<256>> = b"benchmark".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));
        assert_ok!(AgentRegistry::<T>::suspend_agent(
            RawOrigin::Root.into(),
            0,
            reason,
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, 0);
    }

    #[benchmark]
    fn terminate_agent() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), 0);
    }

    #[benchmark]
    fn register_policy() {
        let agent: T::AccountId = account("agent", 0, 0);
        let policies = vec![PolicyRule::ReputationMinimum(50u64)];

        #[extrinsic_call]
        _(RawOrigin::Root, agent, policies);
    }

    #[benchmark]
    fn remove_blacklist() {
        let agent: T::AccountId = account("agent", 0, 0);

        #[extrinsic_call]
        _(RawOrigin::Root, agent);
    }

    #[benchmark]
    fn post_bond() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        let bond_amount = BalanceOf::<T>::from(2_000_000u32);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), bond_amount, None::<H256>);
    }

    #[benchmark]
    fn release_bond() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));
        assert_ok!(AgentRegistry::<T>::post_bond(
            RawOrigin::Signed(caller.clone()).into(),
            BalanceOf::<T>::from(2_000_000u32),
            None::<H256>,
        ));

        let bond_id = Bonds::<T>::iter_keys().next().unwrap();

        #[extrinsic_call]
        _(RawOrigin::Root, bond_id);
    }

    #[benchmark]
    fn slash_bond() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));
        assert_ok!(AgentRegistry::<T>::post_bond(
            RawOrigin::Signed(caller.clone()).into(),
            BalanceOf::<T>::from(2_000_000u32),
            None::<H256>,
        ));

        let bond_id = Bonds::<T>::iter_keys().next().unwrap();
        let reason = b"benchmark_slash".to_vec();

        #[extrinsic_call]
        _(RawOrigin::Root, bond_id, 2u8, reason);
    }

    #[benchmark]
    fn record_consumption() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0, 1000u128, 500u128);
    }

    #[benchmark]
    fn update_reputation() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, 0, 150u32);
    }

    #[benchmark]
    fn distribute_rewards() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Root, 0, BalanceOf::<T>::from(1000u32));
    }

    #[benchmark]
    fn emit_action() {
        let caller: T::AccountId = whitelisted_caller();
        let operator: T::AccountId = account("operator", 0, 0);
        let name: BoundedVec<u8, ConstU32<64>> = b"BenchAgent".to_vec().try_into().unwrap();
        let metadata: BoundedVec<u8, ConstU32<1024>> = b"{}".to_vec().try_into().unwrap();
        let data: BoundedVec<u8, ConstU32<512>> = b"benchmark_action".to_vec().try_into().unwrap();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        assert_ok!(AgentRegistry::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            operator,
            name,
            metadata,
            AgentKind::AutonomousAgent,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0, ActionType::ExecuteTrade, data);
    }

    impl_benchmark_test_suite!(AgentRegistry, crate::mock::new_test_ext(), crate::mock::Test);
}
