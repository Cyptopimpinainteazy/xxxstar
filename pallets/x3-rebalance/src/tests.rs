use crate::{mock::*, pallet::*};
use frame_support::{assert_noop, assert_ok, traits::Hooks, BoundedVec};
use pallet_x3_inventory::{pallet::Vaults, types::*};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn vault_id(seed: u8) -> VaultId {
    [seed; 32]
}

fn lane_id(seed: u8) -> LaneId {
    [seed; 32]
}

/// Create an active vault.  Bands: critical_min=100, min_band=500, target_band=1000,
/// max_band=5_000.  The vault starts with zero balance (below min_band by default).
fn setup_vault(id: VaultId) {
    assert_ok!(X3Inventory::create_vault(
        RuntimeOrigin::root(),
        id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        1,     // chain_id
        100,   // asset_id
        100,   // critical_min
        500,   // min_band
        1_000, // target_band
        5_000, // max_band
    ));
}

/// Create an active vault whose step_amount (target_band - available = 10_000 - 0) exceeds
/// the test runtime's MaxDailyRebalanceVolume of 5_000.
fn setup_large_vault(id: VaultId) {
    assert_ok!(X3Inventory::create_vault(
        RuntimeOrigin::root(),
        id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        2,      // chain_id
        200,    // asset_id
        100,    // critical_min
        500,    // min_band
        10_000, // target_band — step_amount = 10_000 > cap of 5_000
        50_000, // max_band
    ));
}

/// Create a vault with chain_id=3, asset_id=300 for TreasuryRefill lane tests.
fn setup_vault_chain3(id: VaultId) {
    assert_ok!(X3Inventory::create_vault(
        RuntimeOrigin::root(),
        id,
        VaultType::SettlementFloat,
        OwnerType::Protocol,
        3,     // chain_id
        300,   // asset_id
        100,   // critical_min
        500,   // min_band
        1_000, // target_band
        5_000, // max_band
    ));
}

/// Register a lane for the given lane_id with the specified class, using chain_id=1 /
/// asset_id=100 (matching `setup_vault`).
fn setup_lane_with_class(id: LaneId, class: LaneClass) {
    let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
    assert_ok!(X3Inventory::register_lane(
        RuntimeOrigin::root(),
        id,
        1,   // source_chain — matches setup_vault chain_id
        10,  // dest_chain
        100, // source_asset — matches setup_vault asset_id
        999, // dest_asset
        class,
        sources,
        1_000_000, // exposure_cap
        1_000_000, // unsettled_cap
    ));
}

/// Register a lane with chain_id=3 / asset_id=300 (matching setup_vault_chain3) and the
/// given class.
fn setup_lane_chain3_with_class(id: LaneId, class: LaneClass) {
    let sources: BoundedVec<LiquiditySourceType, MaxLiquiditySources> =
        BoundedVec::try_from(vec![LiquiditySourceType::ProtocolFloat]).unwrap();
    assert_ok!(X3Inventory::register_lane(
        RuntimeOrigin::root(),
        id,
        3,   // source_chain — matches setup_vault_chain3
        10,  // dest_chain
        300, // source_asset — matches setup_vault_chain3
        999, // dest_asset
        class,
        sources,
        1_000_000,
        1_000_000,
    ));
}

// ---------------------------------------------------------------------------
// 1. trigger_rebalance — enqueues correctly
// ---------------------------------------------------------------------------

#[test]
fn trigger_rebalance_enqueues_and_emits_event() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(1);
        let trigger = RebalanceTrigger::BelowMinBand { vault_id: vid };

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(42),
            trigger.clone(),
        ));

        // Entry is in storage.
        let entry = PendingRebalances::<Test>::get(vid).expect("entry must be stored");
        assert_eq!(entry.0, trigger);
        assert_eq!(entry.1, 1u64); // scheduled at block 1

        // Count incremented.
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);

        // Event emitted.
        System::assert_has_event(
            Event::RebalanceTriggered {
                vault_id: vid,
                trigger: RebalanceTrigger::BelowMinBand { vault_id: vid },
                scheduled_block: 1,
            }
            .into(),
        );
    });
}

#[test]
fn trigger_rebalance_overwrites_existing_without_count_change() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(2);
        let trigger = RebalanceTrigger::BelowMinBand { vault_id: vid };

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            trigger.clone(),
        ));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);

        // Second call for the same key — must overwrite, count stays at 1.
        System::set_block_number(5);
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            trigger.clone(),
        ));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);

        // Scheduled block updated to block 5.
        let entry = PendingRebalances::<Test>::get(vid).unwrap();
        assert_eq!(entry.1, 5u64);
    });
}

#[test]
fn trigger_rebalance_with_demand_spike_uses_lane_as_key() {
    new_test_ext().execute_with(|| {
        let lid = lane_id(0xAB);
        let trigger = RebalanceTrigger::DemandSpike { lane_id: lid };

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            trigger.clone(),
        ));

        // The queue key is the lane_id itself.
        assert!(PendingRebalances::<Test>::contains_key(lid));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);
    });
}

// ---------------------------------------------------------------------------
// 2. execute_rebalance_step — TreasuryRefill rejected on Class A/B lanes
// ---------------------------------------------------------------------------

#[test]
fn execute_rebalance_step_treasury_refill_rejected_on_class_a_lane() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(10);
        let lid = lane_id(10);

        setup_vault(vid);
        // Register a Class A lane whose chain+asset match the vault.
        setup_lane_with_class(lid, LaneClass::A);

        // Vault is at 0 balance (below min_band=500).
        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid,
                RebalanceMethod::TreasuryRefill,
            ),
            Error::<Test>::TreasuryRefillNotAllowedOnABLane
        );
    });
}

#[test]
fn execute_rebalance_step_treasury_refill_rejected_on_class_b_lane() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(11);
        let lid = lane_id(11);

        setup_vault(vid);
        setup_lane_with_class(lid, LaneClass::B);

        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid,
                RebalanceMethod::TreasuryRefill,
            ),
            Error::<Test>::TreasuryRefillNotAllowedOnABLane
        );
    });
}

#[test]
fn execute_rebalance_step_treasury_refill_allowed_on_class_c_lane() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(12);
        let lid = lane_id(12);

        // Use chain 3 / asset 300 for this vault so Class C lane matches it.
        setup_vault_chain3(vid);
        setup_lane_chain3_with_class(lid, LaneClass::C);

        // Vault is at 0 balance (below min_band=500); step_amount=1000 < cap=5000.
        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid,
            RebalanceMethod::TreasuryRefill,
        ));

        // Vault funded to target_band=1000.
        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 1_000);

        // RebalanceCompleted emitted since 1000 >= min_band 500.
        System::assert_has_event(
            Event::RebalanceCompleted {
                vault_id: vid,
                method: RebalanceMethod::TreasuryRefill,
                total_moved: 1_000,
            }
            .into(),
        );
    });
}

#[test]
fn execute_rebalance_step_treasury_refill_allowed_when_no_lanes_present() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(13);
        setup_vault(vid);

        // No lanes registered → no A/B lane → TreasuryRefill is allowed.
        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid,
            RebalanceMethod::TreasuryRefill,
        ));
    });
}

// ---------------------------------------------------------------------------
// 3. execute_rebalance_step — daily cap breach
// ---------------------------------------------------------------------------

#[test]
fn execute_rebalance_step_cap_exceeded_returns_daily_cap_exceeded_error() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(20);
        // Vault with target_band=10_000.  step_amount=10_000 > cap=5_000.
        setup_large_vault(vid);

        // The call must return DailyCapExceeded.
        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid,
                RebalanceMethod::InternalNetting,
            ),
            Error::<Test>::DailyCapExceeded
        );

        // Vault must NOT have been funded (fund_vault is reached only after the cap check).
        let vault = Vaults::<Test>::get(vid).unwrap();
        assert_eq!(vault.available_balance, 0u128);

        // DailyRebalanceVolume must remain zero (the failed call's storage was rolled back).
        // Block 1, day_index = 0.
        assert_eq!(DailyRebalanceVolume::<Test>::get(0u32), 0u128);
    });
}

#[test]
fn execute_rebalance_step_second_step_can_breach_cap() {
    new_test_ext().execute_with(|| {
        // Two vaults whose individual step_amounts (2500 each) are within cap (5000),
        // but whose combined total exceeds cap.
        let vid1 = vault_id(21);
        let vid2 = vault_id(22);

        // vault1: target=2500, available=0.  step_amount=2500 <= 5000 — succeeds.
        assert_ok!(X3Inventory::create_vault(
            RuntimeOrigin::root(),
            vid1,
            VaultType::SettlementFloat,
            OwnerType::Protocol,
            4,
            400,
            100,
            500,
            2_500,
            10_000,
        ));

        // vault2: target=3000, available=0.  step_amount=3000; running total=5500 > 5000.
        assert_ok!(X3Inventory::create_vault(
            RuntimeOrigin::root(),
            vid2,
            VaultType::SettlementFloat,
            OwnerType::Protocol,
            5,
            500,
            100,
            500,
            3_000,
            10_000,
        ));

        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid1,
            RebalanceMethod::InternalNetting,
        ));

        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid2,
                RebalanceMethod::InternalNetting,
            ),
            Error::<Test>::DailyCapExceeded
        );
    });
}

// ---------------------------------------------------------------------------
// 4. Cooldown enforcement
// ---------------------------------------------------------------------------

#[test]
fn execute_rebalance_step_cooldown_blocks_second_call() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(30);
        setup_vault(vid);

        // Block 1: first step succeeds, vault funded to 1000.
        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid,
            RebalanceMethod::InternalNetting,
        ));

        // Drain vault below min_band so it needs rebalancing again.
        // record_pending_out moves available → pending_out.
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            vid,
            900
        ));
        // available_balance is now 100 < min_band 500.

        // Still block 1: cooldown=10 blocks; elapsed=0 → rejected.
        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid,
                RebalanceMethod::InternalNetting,
            ),
            Error::<Test>::RebalanceCooldownActive
        );
    });
}

#[test]
fn execute_rebalance_step_succeeds_after_cooldown() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(31);
        setup_vault(vid);

        // Block 1: first step.
        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid,
            RebalanceMethod::InternalNetting,
        ));

        // Drain vault.
        assert_ok!(X3Inventory::record_pending_out(
            RuntimeOrigin::root(),
            vid,
            900
        ));

        // Block 11: elapsed = 10 >= cooldown of 10 → allowed.
        System::set_block_number(11);
        assert_ok!(X3Rebalance::execute_rebalance_step(
            RuntimeOrigin::signed(1),
            vid,
            RebalanceMethod::InternalNetting,
        ));
    });
}

// ---------------------------------------------------------------------------
// 5. Queue full returns error
// ---------------------------------------------------------------------------

#[test]
fn trigger_rebalance_queue_full_returns_error() {
    new_test_ext().execute_with(|| {
        // MaxPendingRebalances = 3 in test runtime.

        // Fill the queue with three distinct entries.
        let v1 = vault_id(40);
        let v2 = vault_id(41);
        let v3 = vault_id(42);
        let v4 = vault_id(43);

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v1 },
        ));
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v2 },
        ));
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v3 },
        ));

        assert_eq!(PendingRebalanceCount::<Test>::get(), 3);

        // Fourth distinct entry must fail.
        assert_noop!(
            X3Rebalance::trigger_rebalance(
                RuntimeOrigin::signed(1),
                RebalanceTrigger::BelowMinBand { vault_id: v4 },
            ),
            Error::<Test>::RebalanceQueueFull
        );

        // Count unchanged.
        assert_eq!(PendingRebalanceCount::<Test>::get(), 3);
    });
}

#[test]
fn trigger_rebalance_queue_full_allows_overwrite_of_existing_key() {
    new_test_ext().execute_with(|| {
        let v1 = vault_id(44);
        let v2 = vault_id(45);
        let v3 = vault_id(46);

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v1 },
        ));
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v2 },
        ));
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v3 },
        ));

        // Queue full, but re-triggering an existing key must succeed (overwrite).
        System::set_block_number(5);
        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: v1 },
        ));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 3);
    });
}

// ---------------------------------------------------------------------------
// Additional: execute_rebalance_step error cases
// ---------------------------------------------------------------------------

#[test]
fn execute_rebalance_step_vault_not_found_returns_error() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vault_id(0xFF),
                RebalanceMethod::InternalNetting,
            ),
            Error::<Test>::VaultNotFound
        );
    });
}

#[test]
fn execute_rebalance_step_no_rebalance_needed_when_above_min_band() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(50);
        setup_vault(vid);

        // Fund vault to target_band so it is above min_band.
        assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), vid, 1_000,));

        assert_noop!(
            X3Rebalance::execute_rebalance_step(
                RuntimeOrigin::signed(1),
                vid,
                RebalanceMethod::InternalNetting,
            ),
            Error::<Test>::NoRebalanceNeeded
        );
    });
}

// ---------------------------------------------------------------------------
// Additional: on_initialize hook
// ---------------------------------------------------------------------------

#[test]
fn on_initialize_removes_due_entries_and_emits_event_when_vault_below_min_band() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(60);
        setup_vault(vid); // available=0, min_band=500 → below min_band

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: vid },
        ));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);

        // Advance to block 2; scheduled_block=1 <= 2 → due.
        System::set_block_number(2);
        X3Rebalance::on_initialize(2);

        // Entry removed from queue.
        assert!(PendingRebalances::<Test>::get(vid).is_none());
        assert_eq!(PendingRebalanceCount::<Test>::get(), 0);

        // Readiness event emitted.
        System::assert_has_event(
            Event::RebalanceTriggered {
                vault_id: vid,
                trigger: RebalanceTrigger::BelowMinBand { vault_id: vid },
                scheduled_block: 1,
            }
            .into(),
        );
    });
}

#[test]
fn on_initialize_suppresses_event_when_vault_recovered() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(61);
        setup_vault(vid);

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: vid },
        ));

        // Fund vault so it is above min_band before on_initialize fires.
        assert_ok!(X3Inventory::fund_vault(RuntimeOrigin::root(), vid, 1_000,));

        // Snapshot event count before on_initialize so we can isolate events it emits.
        let events_before = System::events().len();

        System::set_block_number(2);
        X3Rebalance::on_initialize(2);

        // Entry still removed (processed regardless of emit decision).
        assert!(PendingRebalances::<Test>::get(vid).is_none());
        assert_eq!(PendingRebalanceCount::<Test>::get(), 0);

        // No RebalanceTriggered event for this vault must appear in events emitted
        // *during* on_initialize (vault was healthy, so readiness signal is suppressed).
        let new_events = System::events();
        let new_events_slice = &new_events[events_before..];
        let has_triggered = new_events_slice.iter().any(|r| {
            matches!(
                r.event,
                RuntimeEvent::X3Rebalance(Event::RebalanceTriggered {
                    vault_id: emitted_vid,
                    ..
                }) if emitted_vid == vid
            )
        });
        assert!(
            !has_triggered,
            "on_initialize must not emit RebalanceTriggered for a recovered vault"
        );
    });
}

#[test]
fn on_initialize_does_not_remove_future_entries() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(62);

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: vid },
        ));
        // scheduled_block = 1; run on_initialize at block 1 — entry is due (1 <= 1).
        // To test "not yet due", we'd need scheduled_block > now.
        // We can't set a future scheduled_block directly, but we can test that
        // if we set block_number to 0 (before scheduled_block=1) it stays.
        // Actually on_initialize(0): 1 <= 0 is false → entry kept.

        // Use block number 0 explicitly.
        X3Rebalance::on_initialize(0u64);

        // Entry must still be present.
        assert!(PendingRebalances::<Test>::contains_key(vid));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);
    });
}

// ---------------------------------------------------------------------------
// Additional: clear_rebalance
// ---------------------------------------------------------------------------

#[test]
fn clear_rebalance_removes_entry_and_emits_event() {
    new_test_ext().execute_with(|| {
        let vid = vault_id(70);

        assert_ok!(X3Rebalance::trigger_rebalance(
            RuntimeOrigin::signed(1),
            RebalanceTrigger::BelowMinBand { vault_id: vid },
        ));
        assert_eq!(PendingRebalanceCount::<Test>::get(), 1);

        assert_ok!(X3Rebalance::clear_rebalance(RuntimeOrigin::root(), vid));

        assert!(PendingRebalances::<Test>::get(vid).is_none());
        assert_eq!(PendingRebalanceCount::<Test>::get(), 0);
        System::assert_has_event(Event::RebalanceCleared { vault_id: vid }.into());
    });
}

#[test]
fn clear_rebalance_silently_succeeds_when_not_queued() {
    new_test_ext().execute_with(|| {
        // No entry in queue — should not error.
        assert_ok!(X3Rebalance::clear_rebalance(
            RuntimeOrigin::root(),
            vault_id(71),
        ));
    });
}

#[test]
fn clear_rebalance_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3Rebalance::clear_rebalance(RuntimeOrigin::signed(1), vault_id(72)),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}
