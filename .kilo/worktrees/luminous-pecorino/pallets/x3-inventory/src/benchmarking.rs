//! Benchmarking setup for pallet-x3-inventory

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::types::*;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use sp_runtime::traits::Zero;

// Use zero for balance values — Config does not guarantee From<u32>
fn zero_balance<T: Config>() -> T::Balance {
    T::Balance::zero()
}

benchmarks! {
    create_vault {
        let vault_id: VaultId = [0u8; 32];
        let vault_type = VaultType::Gas;
        let owner_type = OwnerType::Protocol;
        let chain_id: ChainId = 1;
        let asset_id: AssetId = 1;
        let zero = zero_balance::<T>();
    }: _(RawOrigin::Root, vault_id, vault_type, owner_type, chain_id, asset_id, zero, zero, zero, zero)
    verify {
        assert!(Vaults::<T>::contains_key(vault_id));
    }

    update_vault_bands {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
    }: _(RawOrigin::Root, vault_id, zero, zero, zero, zero)

    register_lane {
        let lane_id: LaneId = [0u8; 32];
        let allowed: BoundedVec<LiquiditySourceType, T::MaxLiquiditySources> = BoundedVec::default();
        let zero = zero_balance::<T>();
    }: _(RawOrigin::Root, lane_id, 1u32, 2u32, 1u32, 2u32, LaneClass::A, allowed, zero, zero)

    freeze_lane {
        let lane_id: LaneId = [0u8; 32];
        let allowed: BoundedVec<LiquiditySourceType, T::MaxLiquiditySources> = BoundedVec::default();
        let zero = zero_balance::<T>();

        Pallet::<T>::register_lane(
            RawOrigin::Root.into(),
            lane_id, 1u32, 2u32, 1u32, 2u32, LaneClass::A, allowed, zero, zero,
        ).unwrap();
    }: _(RawOrigin::Root, lane_id, FreezeReason::OperatorManual)

    unfreeze_lane {
        let lane_id: LaneId = [0u8; 32];
        let allowed: BoundedVec<LiquiditySourceType, T::MaxLiquiditySources> = BoundedVec::default();
        let zero = zero_balance::<T>();
        let evidence = OperatorEvidence {
            description_hash: [0u8; 32],
            submitted_at_block: 1u32,
            operator_id: [1u8; 32],
        };

        Pallet::<T>::register_lane(
            RawOrigin::Root.into(),
            lane_id, 1u32, 2u32, 1u32, 2u32, LaneClass::A, allowed, zero, zero,
        ).unwrap();
        Pallet::<T>::freeze_lane(
            RawOrigin::Root.into(),
            lane_id, FreezeReason::OperatorManual,
        ).unwrap();
    }: _(RawOrigin::Root, lane_id, evidence)

    reserve_inventory {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
    }: _(RawOrigin::Root, vault_id, zero)

    release_inventory {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
        Pallet::<T>::reserve_inventory(RawOrigin::Root.into(), vault_id, zero).unwrap();
    }: _(RawOrigin::Root, vault_id, zero)

    record_pending_out {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
    }: _(RawOrigin::Root, vault_id, zero)

    confirm_settlement {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
        Pallet::<T>::record_pending_out(RawOrigin::Root.into(), vault_id, zero).unwrap();
    }: _(RawOrigin::Root, vault_id, zero)

    fund_vault {
        let vault_id: VaultId = [0u8; 32];
        let zero = zero_balance::<T>();

        Pallet::<T>::create_vault(
            RawOrigin::Root.into(),
            vault_id, VaultType::Gas, OwnerType::Protocol, 1, 1,
            zero, zero, zero, zero,
        ).unwrap();
    }: _(RawOrigin::Root, vault_id, zero)

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
