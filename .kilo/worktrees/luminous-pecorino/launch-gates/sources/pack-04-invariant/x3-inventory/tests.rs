use crate::{mock::*, pallet::*, types::*};
use frame_support::{assert_noop, assert_ok, BoundedVec};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn vault_id(seed: u8) -> VaultId {
    [seed; 32]
}
fn lane_id(seed: u8) -> LaneId {
    [seed; 32]
}

fn create_test_vault(id: VaultId, critical_min: u128, min: u128, target: u128, max: u128) {
    assert_ok!(X3Inventory::create_vault(
        RuntimeOrigin::root(),
        id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        1,   // chain_id
        100, // asset_id
        critical_min,
        min,
        target,
        max,
    ));
}

fn register_test_lane(id: LaneId) {
    let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ExternalMarket]).unwrap();
    assert_ok!(X3Inventory::register_lane(
        RuntimeOrigin::root(),
        id,
        1,
        2,
        100,
        200,
        LaneClass::A,
        sources,
        1_000_000,
        500_000,
    ));
}

// ---------------------------------------------------------------------------
// TICKET-4.5-002: Vault storage and band invariants
// ---------------------------------------------------------------------------

#[test]
fn create_vault_stores_with_active_status() {
    new_test_ext().execute_with(|| {
        let id = vault_id(1);
        create_test_vault(id, 100, 500, 1_000, 2_000);
        // A brand-new vault with 0 balance is Frozen until funded.
        // Fund above min_band to confirm the Active transition works.
        assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), id, 1_500));
        let vault = Vaults::<Test>::get(id).expect("vault must exist");
        assert_eq!(vault.status, VaultStatus::Active);
        assert_eq!(vault.available_balance, 1_500u128);
    });
}

#[test]
fn create_vault_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = vault_id(2);
        create_test_vault(id, 100, 500, 1_000, 2_000);
        System::assert_has_event(
            Event::VaultCreated {
                vault_id: id,
                vault_type: VaultType::SettlementFloat,
                chain_id: 1,
                asset_id: 100,
            }
            .into(),
        );
    });
}

#[test]
fn duplicate_vault_returns_error() {
    new_test_ext().execute_with(|| {
        let id = vault_id(3);
        create_test_vault(id, 100, 500, 1_000, 2_000);
        assert_noop!(
            X3Inventory::create_vault(
                RuntimeOrigin::root(),
                id,
                VaultType::Gas,
                OwnerType::Protocol,
                1,
                100,
                100,
                500,
                1_000,
                2_000,
            ),
            Error::<Test>::VaultAlreadyExists
        );
    });
}

#[test]
fn invalid_band_order_rejected() {
    new_test_ext().execute_with(|| {
        // min > target  →  invalid
        assert_noop!(
            X3Inventory::create_vault(
                RuntimeOrigin::root(),
                vault_id(4),
                VaultType::Gas,
                OwnerType::Protocol,
                1,
                100,
                100,
                2_000,
                500,
                3_000,
            ),
            Error::<Test>::InvalidBandOrder
        );
    });
}

#[test]
fn check_band_status_returns_frozen_below_critical_min() {
    new_test_ext().execute_with(|| {
        let id = vault_id(5);
        create_test_vault(id, 500, 1_000, 2_000, 5_000);

        // Manually lower balance to below critical_min for the status check helper.
        Vaults::<Test>::mutate(id, |v| {
            let v = v.as_mut().unwrap();
            v.available_balance = 400; // below critical_min=500
        });

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(X3Inventory::check_band_status(&vault), VaultStatus::Frozen);
    });
}

#[test]
fn check_band_status_returns_degraded_below_min() {
    new_test_ext().execute_with(|| {
        let id = vault_id(6);
        create_test_vault(id, 100, 500, 1_000, 2_000);

        Vaults::<Test>::mutate(id, |v| {
            let v = v.as_mut().unwrap();
            v.available_balance = 300; // above critical_min=100, below min=500
        });

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(
            X3Inventory::check_band_status(&vault),
            VaultStatus::Degraded
        );
    });
}

#[test]
fn check_band_status_returns_active_above_min() {
    new_test_ext().execute_with(|| {
        let id = vault_id(7);
        create_test_vault(id, 100, 500, 1_000, 2_000);

        Vaults::<Test>::mutate(id, |v| {
            let v = v.as_mut().unwrap();
            v.available_balance = 800; // above min=500
        });

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(X3Inventory::check_band_status(&vault), VaultStatus::Active);
    });
}

#[test]
fn update_vault_bands_emits_event_and_persists() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = vault_id(8);
        create_test_vault(id, 100, 500, 1_000, 2_000);

        assert_ok!(X3Inventory::update_vault_bands(
            RuntimeOrigin::root(),
            id,
            200,
            600,
            1_200,
            3_000
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.min_band, 600);
        assert_eq!(vault.max_band, 3_000);
        System::assert_has_event(Event::VaultBandsUpdated { vault_id: id }.into());
    });
}

#[test]
fn update_vault_bands_re_evaluates_status_to_frozen() {
    new_test_ext().execute_with(|| {
        let id = vault_id(9);
        create_test_vault(id, 100, 500, 1_000, 2_000);

        // Give the vault some balance first
        Vaults::<Test>::mutate(id, |v| {
            v.as_mut().unwrap().available_balance = 800;
        });

        // Raise critical_min above current balance — should trigger Frozen
        assert_ok!(X3Inventory::update_vault_bands(
            RuntimeOrigin::root(),
            id,
            1_000,
            2_000,
            3_000,
            5_000
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.status, VaultStatus::Frozen);
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-003: Lane storage and freeze mechanics
// ---------------------------------------------------------------------------

#[test]
fn register_lane_stores_with_active_status() {
    new_test_ext().execute_with(|| {
        let id = lane_id(1);
        register_test_lane(id);
        let lane = Lanes::<Test>::get(id).expect("lane must exist");
        assert_eq!(lane.status, LaneStatus::Active);
    });
}

#[test]
fn register_lane_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = lane_id(2);
        register_test_lane(id);
        System::assert_has_event(
            Event::LaneRegistered {
                lane_id: id,
                lane_class: LaneClass::A,
            }
            .into(),
        );
    });
}

#[test]
fn duplicate_lane_returns_error() {
    new_test_ext().execute_with(|| {
        let id = lane_id(3);
        register_test_lane(id);
        let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
            BoundedVec::try_from(vec![LiquiditySourceType::ExternalMarket]).unwrap();
        assert_noop!(
            X3Inventory::register_lane(
                RuntimeOrigin::root(),
                id,
                1,
                2,
                100,
                200,
                LaneClass::A,
                sources,
                1_000_000,
                500_000,
            ),
            Error::<Test>::LaneAlreadyExists
        );
    });
}

#[test]
fn freeze_lane_transitions_to_frozen_and_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = lane_id(4);
        register_test_lane(id);

        assert_ok!(X3Inventory::freeze_lane(
            RuntimeOrigin::root(),
            id,
            FreezeReason::BalanceBelowCriticalMin,
        ));

        let lane = Lanes::<Test>::get(id).unwrap();
        assert_eq!(lane.status, LaneStatus::Frozen);
        System::assert_has_event(
            Event::LaneFrozen {
                lane_id: id,
                reason: FreezeReason::BalanceBelowCriticalMin,
            }
            .into(),
        );
    });
}

#[test]
fn freeze_already_frozen_lane_returns_error() {
    new_test_ext().execute_with(|| {
        let id = lane_id(5);
        register_test_lane(id);
        assert_ok!(X3Inventory::freeze_lane(
            RuntimeOrigin::root(),
            id,
            FreezeReason::OperatorManual
        ));
        assert_noop!(
            X3Inventory::freeze_lane(RuntimeOrigin::root(), id, FreezeReason::OperatorManual),
            Error::<Test>::AlreadyFrozen
        );
    });
}

#[test]
fn unfreeze_lane_restores_active_and_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = lane_id(6);
        register_test_lane(id);
        assert_ok!(X3Inventory::freeze_lane(
            RuntimeOrigin::root(),
            id,
            FreezeReason::OperatorManual
        ));

        let evidence = OperatorEvidence {
            description_hash: [0xaa; 32],
            submitted_at_block: 10,
            operator_id: [0xbb; 32],
        };
        assert_ok!(X3Inventory::unfreeze_lane(
            RuntimeOrigin::root(),
            id,
            evidence.clone()
        ));

        let lane = Lanes::<Test>::get(id).unwrap();
        assert_eq!(lane.status, LaneStatus::Active);
        System::assert_has_event(
            Event::LaneUnfrozen {
                lane_id: id,
                operator_id: [0xbb; 32],
            }
            .into(),
        );
    });
}

#[test]
fn unfreeze_non_frozen_lane_returns_error() {
    new_test_ext().execute_with(|| {
        let id = lane_id(7);
        register_test_lane(id);
        let evidence = OperatorEvidence {
            description_hash: [0x00; 32],
            submitted_at_block: 1,
            operator_id: [0x01; 32],
        };
        assert_noop!(
            X3Inventory::unfreeze_lane(RuntimeOrigin::root(), id, evidence),
            Error::<Test>::NotFrozen
        );
    });
}

#[test]
fn freeze_nonexistent_lane_returns_error() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Inventory::freeze_lane(
                RuntimeOrigin::root(),
                lane_id(99),
                FreezeReason::OperatorManual
            ),
            Error::<Test>::LaneNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// TICKET-4.5-004: Inventory reserve and release
// ---------------------------------------------------------------------------

fn funded_vault(id: VaultId, balance: u128) -> VaultId {
    create_test_vault(id, 100, 500, 1_000, 5_000);
    assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), id, balance));
    id
}

// --- reserve_inventory ---

#[test]
fn reserve_inventory_moves_balance_from_available_to_reserved() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(20), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            400
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.available_balance, 600);
        assert_eq!(vault.reserved_balance, 400);
    });
}

#[test]
fn reserve_inventory_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = funded_vault(vault_id(21), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            300
        ));
        System::assert_has_event(
            Event::InventoryReserved {
                vault_id: id,
                amount: 300,
            }
            .into(),
        );
    });
}

#[test]
fn reserve_inventory_rejects_insufficient_available() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(22), 200);
        assert_noop!(
            X3Inventory::reserve_inventory(RuntimeOrigin::root(), id, 201),
            Error::<Test>::InsufficientAvailableBalance
        );
    });
}

#[test]
fn reserve_inventory_rejects_frozen_vault() {
    new_test_ext().execute_with(|| {
        // critical_min=500, fund with 400 → vault is Frozen on creation path
        create_test_vault(vault_id(23), 500, 1_000, 2_000, 5_000);
        // Fund below critical_min so vault freezes
        assert_ok!(X3Inventory::fund_vault(
            RuntimeOrigin::root(),
            vault_id(23),
            400
        ));
        // The vault should be Frozen now
        let vault = Vaults::<Test>::get(vault_id(23)).unwrap();
        assert_eq!(vault.status, VaultStatus::Frozen);

        assert_noop!(
            X3Inventory::reserve_inventory(RuntimeOrigin::root(), vault_id(23), 100),
            Error::<Test>::VaultFrozen
        );
    });
}

#[test]
fn reserve_inventory_balance_invariant_holds() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(24), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            400
        ));
        let vault = Vaults::<Test>::get(id).unwrap();
        // available + reserved + pending_out must equal funded amount
        assert_eq!(
            vault.available_balance + vault.reserved_balance + vault.pending_out_balance,
            1_000
        );
    });
}

// --- release_inventory ---

#[test]
fn release_inventory_moves_balance_from_reserved_to_available() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(25), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            400
        ));
        assert_ok!(X3Inventory::release_inventory(
            RuntimeOrigin::root(),
            id,
            200
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.available_balance, 800);
        assert_eq!(vault.reserved_balance, 200);
    });
}

#[test]
fn release_inventory_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = funded_vault(vault_id(26), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            400
        ));
        assert_ok!(X3Inventory::release_inventory(
            RuntimeOrigin::root(),
            id,
            400
        ));
        System::assert_has_event(
            Event::InventoryReleased {
                vault_id: id,
                amount: 400,
            }
            .into(),
        );
    });
}

#[test]
fn release_inventory_rejects_insufficient_reserved() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(27), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            300
        ));
        assert_noop!(
            X3Inventory::release_inventory(RuntimeOrigin::root(), id, 301),
            Error::<Test>::InsufficientReservedBalance
        );
    });
}

#[test]
fn release_inventory_restores_available_balance_invariant() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(28), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            600
        ));
        assert_ok!(X3Inventory::release_inventory(
            RuntimeOrigin::root(),
            id,
            600
        ));
        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.available_balance, 1_000);
        assert_eq!(vault.reserved_balance, 0);
    });
}

// --- record_pending_out ---

#[test]
fn record_pending_out_moves_from_available_to_pending_out() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(29), 1_000);
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            250
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.available_balance, 750);
        assert_eq!(vault.pending_out_balance, 250);
    });
}

#[test]
fn record_pending_out_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = funded_vault(vault_id(30), 1_000);
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            100
        ));
        System::assert_has_event(
            Event::PendingOutRecorded {
                vault_id: id,
                amount: 100,
            }
            .into(),
        );
    });
}

#[test]
fn record_pending_out_rejects_insufficient_available() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(31), 500);
        assert_noop!(
            X3Inventory::record_pending_out(RuntimeOrigin::root(), id, 501),
            Error::<Test>::InsufficientAvailableBalance
        );
    });
}

#[test]
fn record_pending_out_balance_invariant_holds() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(32), 1_000);
        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            300
        ));
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            200
        ));
        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(
            vault.available_balance + vault.reserved_balance + vault.pending_out_balance,
            1_000
        );
    });
}

// --- confirm_settlement ---

#[test]
fn confirm_settlement_reduces_pending_out_balance() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(33), 1_000);
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            300
        ));
        assert_ok!(X3Inventory::confirm_settlement(
            RuntimeOrigin::root(),
            id,
            300
        ));

        let vault = Vaults::<Test>::get(id).unwrap();
        assert_eq!(vault.pending_out_balance, 0);
        // available is NOT restored — funds have left the system
        assert_eq!(vault.available_balance, 700);
    });
}

#[test]
fn confirm_settlement_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        let id = funded_vault(vault_id(34), 1_000);
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            400
        ));
        assert_ok!(X3Inventory::confirm_settlement(
            RuntimeOrigin::root(),
            id,
            400
        ));
        System::assert_has_event(
            Event::SettlementConfirmed {
                vault_id: id,
                amount: 400,
            }
            .into(),
        );
    });
}

#[test]
fn confirm_settlement_rejects_insufficient_pending_out() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(35), 1_000);
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            200
        ));
        assert_noop!(
            X3Inventory::confirm_settlement(RuntimeOrigin::root(), id, 201),
            Error::<Test>::InsufficientPendingOutBalance
        );
    });
}

// --- fund_vault ---

#[test]
fn fund_vault_increases_available_balance() {
    new_test_ext().execute_with(|| {
        create_test_vault(vault_id(36), 100, 500, 1_000, 5_000);
        assert_ok!(X3Inventory::fund_vault(
            RuntimeOrigin::root(),
            vault_id(36),
            2_000
        ));
        let vault = Vaults::<Test>::get(vault_id(36)).unwrap();
        assert_eq!(vault.available_balance, 2_000);
    });
}

#[test]
fn fund_vault_emits_event() {
    new_test_ext().execute_with(|| {
        System::reset_events();
        create_test_vault(vault_id(37), 100, 500, 1_000, 5_000);
        assert_ok!(X3Inventory::fund_vault(
            RuntimeOrigin::root(),
            vault_id(37),
            1_500
        ));
        System::assert_has_event(
            Event::VaultFunded {
                vault_id: vault_id(37),
                amount: 1_500,
            }
            .into(),
        );
    });
}

#[test]
fn fund_vault_transitions_frozen_vault_to_active() {
    new_test_ext().execute_with(|| {
        // Vault starts with 0 balance → Frozen (below critical_min=500)
        create_test_vault(vault_id(38), 500, 1_000, 2_000, 5_000);
        let vault = Vaults::<Test>::get(vault_id(38)).unwrap();
        assert_eq!(vault.status, VaultStatus::Frozen);

        // Fund above min_band → should become Active
        assert_ok!(X3Inventory::fund_vault(
            RuntimeOrigin::root(),
            vault_id(38),
            1_500
        ));
        let vault = Vaults::<Test>::get(vault_id(38)).unwrap();
        assert_eq!(vault.status, VaultStatus::Active);
    });
}

// --- combined invariant property test ---

#[test]
fn combined_operations_maintain_balance_invariant() {
    new_test_ext().execute_with(|| {
        let id = funded_vault(vault_id(39), 5_000);

        assert_ok!(X3Inventory::reserve_inventory(
            RuntimeOrigin::root(),
            id,
            1_000
        ));
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            id,
            500
        ));
        assert_ok!(X3Inventory::release_inventory(
            RuntimeOrigin::root(),
            id,
            500
        ));
        assert_ok!(X3Inventory::confirm_settlement(
            RuntimeOrigin::root(),
            id,
            500
        ));
        assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), id, 200));

        let vault = Vaults::<Test>::get(id).unwrap();
        // Initial 5_000 + 200 funded - 500 confirmed (left system) = 4_700
        assert_eq!(
            vault.available_balance + vault.reserved_balance + vault.pending_out_balance,
            4_700
        );
    });
}
