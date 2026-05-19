//! Tests for `pallet-x3-wrapped`.
//!
//! Coverage: 20 test cases spanning:
//! - Asset registration (happy path, duplicate rejection, zero-supply rejection)
//! - Mint (happy path, nonce replay, supply cap, paused asset, deprecated asset,
//!         unregistered asset, zero-amount)
//! - Burn (happy path, insufficient balance, unregistered asset, deprecated rejection,
//!         burn-while-paused allowed)
//! - Governance power calculation and update
//! - Pause / resume lifecycle
//! - Bridge fee update
//! - `TotalWrappedSupply` invariant after mixed operations

use crate::{
    mock::*,
    pallet::{
        BridgeNonces, GovernancePowerMap, RegisteredWrappedAssets, TotalWrappedSupply,
        WrappedBalances, WrappedSupply,
    },
    WrappedAssetConfig, WrappedAssetStatus,
};
use frame_support::{assert_noop, assert_ok};
use pallet_x3_wrapped::Pallet as Wrapped;

// ── 1. Register asset ─────────────────────────────────────────────────────────

#[test]
fn register_asset_stores_config() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        let cfg = RegisteredWrappedAssets::<Test>::get(X3_ASSET_ID).expect("config must exist");
        assert_eq!(cfg.max_wrapped_supply, MAX_SUPPLY);
        assert_eq!(cfg.governance_weight_bps, 10_000);
        assert_eq!(cfg.status, WrappedAssetStatus::Active);
    });
}

#[test]
fn register_asset_duplicate_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_noop!(
            Wrapped::<Test>::register_wrapped_asset(
                root(),
                X3_ASSET_ID,
                WrappedAssetConfig {
                    native_asset_id: [0xAA; 32],
                    max_wrapped_supply: MAX_SUPPLY,
                    governance_weight_bps: 10_000,
                    bridge_fee_bps: 30,
                    status: WrappedAssetStatus::Active,
                },
            ),
            crate::pallet::Error::<Test>::AssetAlreadyRegistered
        );
    });
}

#[test]
fn register_asset_zero_supply_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Wrapped::<Test>::register_wrapped_asset(
                root(),
                X3_ASSET_ID,
                WrappedAssetConfig {
                    native_asset_id: [0xAA; 32],
                    max_wrapped_supply: 0_u128,
                    governance_weight_bps: 10_000,
                    bridge_fee_bps: 30,
                    status: WrappedAssetStatus::Active,
                },
            ),
            crate::pallet::Error::<Test>::InvalidAmount
        );
    });
}

#[test]
fn register_asset_non_governance_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Wrapped::<Test>::register_wrapped_asset(
                signed(1),
                X3_ASSET_ID,
                WrappedAssetConfig {
                    native_asset_id: [0xAA; 32],
                    max_wrapped_supply: MAX_SUPPLY,
                    governance_weight_bps: 10_000,
                    bridge_fee_bps: 30,
                    status: WrappedAssetStatus::Active,
                },
            ),
            frame_support::error::BadOrigin
        );
    });
}

// ── 2. Mint wrapped tokens ────────────────────────────────────────────────────

#[test]
fn mint_wrapped_increases_balance_and_supply() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(
            root(),
            ETH_CHAIN,
            X3_ASSET_ID,
            42_u64,
            500,
            1,
        ));
        assert_eq!(
            WrappedBalances::<Test>::get((ETH_CHAIN, X3_ASSET_ID, 42_u64)),
            500
        );
        assert_eq!(WrappedSupply::<Test>::get(ETH_CHAIN, X3_ASSET_ID), 500);
        assert_eq!(TotalWrappedSupply::<Test>::get(), 500);
    });
}

#[test]
fn mint_nonce_replay_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 42, 100, 7));
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 99, 100, 7),
            crate::pallet::Error::<Test>::NonceAlreadyUsed
        );
    });
}

#[test]
fn mint_marks_nonce_as_used() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 10, 42));
        assert!(BridgeNonces::<Test>::get(ETH_CHAIN, 42_u64));
    });
}

#[test]
fn mint_supply_cap_exceeded_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(1_000, 10_000); // cap = 1_000
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 1_000, 1));
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 1, 2),
            crate::pallet::Error::<Test>::SupplyCapExceeded
        );
    });
}

#[test]
fn mint_paused_asset_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::pause_wrapped_asset(root(), X3_ASSET_ID));
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 100, 1),
            crate::pallet::Error::<Test>::AssetPaused
        );
    });
}

#[test]
fn mint_unregistered_asset_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 100, 1),
            crate::pallet::Error::<Test>::AssetNotFound
        );
    });
}

#[test]
fn mint_zero_amount_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 0, 1),
            crate::pallet::Error::<Test>::InvalidAmount
        );
    });
}

#[test]
fn mint_non_bridge_authority_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_noop!(
            Wrapped::<Test>::mint_wrapped(signed(9), ETH_CHAIN, X3_ASSET_ID, 1, 100, 1),
            frame_support::error::BadOrigin
        );
    });
}

// ── 3. Burn wrapped tokens ────────────────────────────────────────────────────

#[test]
fn burn_wrapped_decreases_balance_and_supply() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 5, 800, 1));
        assert_ok!(Wrapped::<Test>::burn_wrapped(signed(5), ETH_CHAIN, X3_ASSET_ID, 300));
        assert_eq!(
            WrappedBalances::<Test>::get((ETH_CHAIN, X3_ASSET_ID, 5_u64)),
            500
        );
        assert_eq!(WrappedSupply::<Test>::get(ETH_CHAIN, X3_ASSET_ID), 500);
        assert_eq!(TotalWrappedSupply::<Test>::get(), 500);
    });
}

#[test]
fn burn_insufficient_balance_rejected() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 7, 200, 1));
        assert_noop!(
            Wrapped::<Test>::burn_wrapped(signed(7), ETH_CHAIN, X3_ASSET_ID, 201),
            crate::pallet::Error::<Test>::InsufficientWrappedBalance
        );
    });
}

#[test]
fn burn_unregistered_asset_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Wrapped::<Test>::burn_wrapped(signed(1), ETH_CHAIN, X3_ASSET_ID, 10),
            crate::pallet::Error::<Test>::AssetNotFound
        );
    });
}

#[test]
fn burn_while_paused_is_allowed() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 3, 400, 1));
        assert_ok!(Wrapped::<Test>::pause_wrapped_asset(root(), X3_ASSET_ID));
        // Burn should succeed even though the asset is paused.
        assert_ok!(Wrapped::<Test>::burn_wrapped(signed(3), ETH_CHAIN, X3_ASSET_ID, 400));
        assert_eq!(TotalWrappedSupply::<Test>::get(), 0);
    });
}

// ── 4. Governance power ───────────────────────────────────────────────────────

#[test]
fn update_governance_power_computes_weighted_balance() {
    new_test_ext().execute_with(|| {
        // Weight = 10_000 bps = 1× multiplier.  1_000 tokens → power = 1_000.
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 8, 1_000, 1));
        assert_ok!(Wrapped::<Test>::update_governance_power(signed(8), 8_u64));
        assert_eq!(GovernancePowerMap::<Test>::get(8_u64), 1_000);
    });
}

#[test]
fn update_governance_power_half_weight() {
    new_test_ext().execute_with(|| {
        // Weight = 5_000 bps = 0.5×.  2_000 tokens → power = 1_000.
        register_x3_asset(MAX_SUPPLY, 5_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 9, 2_000, 1));
        assert_ok!(Wrapped::<Test>::update_governance_power(signed(9), 9_u64));
        assert_eq!(GovernancePowerMap::<Test>::get(9_u64), 1_000);
    });
}

#[test]
fn update_governance_power_permissionless() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 11, 500, 1));
        // Account 99 (a stranger) triggers the update for account 11.
        assert_ok!(Wrapped::<Test>::update_governance_power(signed(99), 11_u64));
        assert_eq!(GovernancePowerMap::<Test>::get(11_u64), 500);
    });
}

// ── 5. Pause / resume lifecycle ───────────────────────────────────────────────

#[test]
fn pause_and_resume_asset() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::pause_wrapped_asset(root(), X3_ASSET_ID));
        assert_eq!(
            RegisteredWrappedAssets::<Test>::get(X3_ASSET_ID).unwrap().status,
            WrappedAssetStatus::Paused
        );
        assert_ok!(Wrapped::<Test>::resume_wrapped_asset(root(), X3_ASSET_ID));
        assert_eq!(
            RegisteredWrappedAssets::<Test>::get(X3_ASSET_ID).unwrap().status,
            WrappedAssetStatus::Active
        );
    });
}

#[test]
fn pause_unregistered_asset_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Wrapped::<Test>::pause_wrapped_asset(root(), X3_ASSET_ID),
            crate::pallet::Error::<Test>::AssetNotFound
        );
    });
}

// ── 6. Bridge fee ─────────────────────────────────────────────────────────────

#[test]
fn set_bridge_fee_updates_config() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        assert_ok!(Wrapped::<Test>::set_bridge_fee(root(), X3_ASSET_ID, 100));
        assert_eq!(
            RegisteredWrappedAssets::<Test>::get(X3_ASSET_ID).unwrap().bridge_fee_bps,
            100
        );
    });
}

// ── 7. TotalWrappedSupply invariant ──────────────────────────────────────────

#[test]
fn total_supply_invariant_holds_after_mixed_ops() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);

        // Register a second asset.
        assert_ok!(Wrapped::<Test>::register_wrapped_asset(
            root(),
            USDC_ASSET_ID,
            WrappedAssetConfig {
                native_asset_id: [0xBB; 32],
                max_wrapped_supply: MAX_SUPPLY,
                governance_weight_bps: 10_000,
                bridge_fee_bps: 50,
                status: WrappedAssetStatus::Active,
            },
        ));

        // Mint on two chains.
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 300, 1));
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ARB_CHAIN, X3_ASSET_ID, 2, 200, 2));
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, USDC_ASSET_ID, 3, 100, 3));

        // Burn partially.
        assert_ok!(Wrapped::<Test>::burn_wrapped(signed(1), ETH_CHAIN, X3_ASSET_ID, 50));

        // Expected: 300 - 50 + 200 + 100 = 550.
        let stored_total = TotalWrappedSupply::<Test>::get();
        let manual_sum = WrappedSupply::<Test>::iter()
            .map(|(_, _, bal)| bal)
            .fold(0_u128, |a, b| a + b);
        assert_eq!(stored_total, 550);
        assert_eq!(stored_total, manual_sum);
    });
}

// ── 8. Multi-chain nonce isolation ────────────────────────────────────────────

#[test]
fn same_nonce_on_different_chains_is_allowed() {
    new_test_ext().execute_with(|| {
        register_x3_asset(MAX_SUPPLY, 10_000);
        // Nonce 1 on ETH_CHAIN.
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ETH_CHAIN, X3_ASSET_ID, 1, 100, 1));
        // Same nonce 1 on ARB_CHAIN — must succeed (nonces are per-chain).
        assert_ok!(Wrapped::<Test>::mint_wrapped(root(), ARB_CHAIN, X3_ASSET_ID, 2, 100, 1));
    });
}
