// SPDX-License-Identifier: Apache-2.0
//
// tests_public_api.rs — the public query API is part of the product, so it is tested.
//
// `is_nonce_used`, `get_current_nonce`, `fetch_asset_metadata` and `enforce_supply_policy` are
// `pub fn`s a wallet, UI or runtime wires against, but nothing in-tree called them and nothing
// asserted their results. cargo-mutants proved it: replacing each body with a constant
// (`true`, `false`, `0`, `1`, `Ok(Default::default())`, `Ok(())`) survived the whole suite.
// These tests pin the observable behaviour through the public entry points instead.

use crate::mock::{asset, ledger_of, new_test_ext, register_asset, set_supply_policy, Test};
use crate::Pallet;
use frame_support::assert_ok;
use x3_asset_kernel_types::traits::{EconomicHaltInspect, SupplyLedgerGovern, SupplyLedgerWrite};
use x3_asset_kernel_types::{AssetStatus, DomainId, SupplyPolicy};

/// A governed mint has to be observable through both nonce queries: the nonce it consumed
/// reads as used, the next one does not, and the counter advances past both constants a
/// mutant would return.
#[test]
fn nonce_queries_track_every_recorded_mint() {
    new_test_ext().execute_with(|| {
        let asset_id = asset(11);
        register_asset(asset_id, 1_000_000, 0);
        frame_system::Pallet::<Test>::set_block_number(1);
        let who: u64 = 7;
        let origin: crate::mock::RuntimeOrigin = frame_system::RawOrigin::Signed(who).into();

        assert!(
            !Pallet::<Test>::is_nonce_used(&who, 0),
            "nothing is used before the first mint"
        );
        assert_eq!(Pallet::<Test>::get_current_nonce(&who), 0);

        assert_ok!(Pallet::<Test>::mint_canonical(
            origin.clone(),
            asset_id,
            DomainId::X3Native,
            500,
            0
        ));
        assert!(
            Pallet::<Test>::is_nonce_used(&who, 0),
            "the minted nonce is recorded"
        );
        assert!(
            !Pallet::<Test>::is_nonce_used(&who, 1),
            "the next nonce is still free"
        );
        assert_eq!(Pallet::<Test>::get_current_nonce(&who), 1);

        assert_ok!(Pallet::<Test>::mint_canonical(
            origin,
            asset_id,
            DomainId::X3Native,
            500,
            1
        ));
        assert!(Pallet::<Test>::is_nonce_used(&who, 1));
        assert_eq!(
            Pallet::<Test>::get_current_nonce(&who),
            2,
            "the counter keeps moving"
        );
    });
}

/// Registry lookups must distinguish a live asset from an unknown one — the mutant answered
/// `Ok(AssetStatus::default())` (= `Registered`) for everything.
#[test]
fn registry_metadata_reports_active_and_unknown_assets() {
    new_test_ext().execute_with(|| {
        let registered = asset(12);
        register_asset(registered, 1_000_000, 0);
        assert_eq!(
            Pallet::<Test>::fetch_asset_metadata(&registered),
            Ok(AssetStatus::Active),
            "a seeded ledger is an active asset"
        );
        assert!(
            Pallet::<Test>::fetch_asset_metadata(&asset(99)).is_err(),
            "an unknown asset is not Ok(Registered)"
        );
    });
}

/// Only native mint/burn is implemented here; every other policy must be refused rather than
/// sail through an unconditional `Ok(())`.
#[test]
fn enforce_supply_policy_accepts_native_and_refuses_the_rest() {
    new_test_ext().execute_with(|| {
        let native = asset(13);
        register_asset(native, 1_000_000, 0);
        assert!(
            Pallet::<Test>::enforce_supply_policy(&native).is_ok(),
            "native mint/burn is the policy this pallet implements"
        );

        let wrapped = asset(14);
        set_supply_policy(wrapped, SupplyPolicy::LockMint);
        assert!(
            Pallet::<Test>::enforce_supply_policy(&wrapped).is_err(),
            "lock-and-mint is not native mint/burn"
        );
    });
}

// ── the `SupplyLedgerWrite` / `SupplyLedgerGovern` / `EconomicHaltInspect` impls ──────────────
// `do_mint_canonical`, `do_burn_canonical`, `ledger` and `is_halted` are the trait surface the
// runtime, token factory and settlement paths call through. cargo-mutants replaced each with a
// constant (`Ok(())`, `None`, `Some(default)`, `true`, `false`) and the suite stayed green, so
// these pin the contracts: a read returns the stored ledger, a governor mint/burn really moves
// canonical supply, refusals stay refusals, and the halt flag is reported as stored.

/// The read side of the trait: a known asset's stored ledger comes back intact — not `None`,
/// not `Default::default()` — and an unknown asset is `None`, never a fabricated ledger.
#[test]
fn trait_ledger_read_returns_the_stored_ledger() {
    new_test_ext().execute_with(|| {
        let asset_id = asset(15);
        let stored = register_asset(asset_id, 500_000, 300_000);
        let read = <Pallet<Test> as SupplyLedgerWrite>::ledger(&asset_id);
        assert_eq!(
            read,
            Some(stored),
            "the reader returns the ledger that was stored"
        );
        assert_ne!(read, Some(Default::default()), "not the zeroed default");
        assert_eq!(
            <Pallet<Test> as SupplyLedgerWrite>::ledger(&asset(97)),
            None,
            "an unknown asset has no ledger"
        );
    });
}

/// The governor mint/burn must move canonical supply and the named domain slot, refuse unknown
/// assets, and refuse to burn more than the ledger holds. Replacing either body with `Ok(())`
/// survived the suite before this test existed.
#[test]
fn governing_mint_and_burn_move_canonical_supply() {
    new_test_ext().execute_with(|| {
        let asset_id = asset(16);
        let initial = register_asset(asset_id, 1_000_000, 1_000_000);

        assert_ok!(<Pallet<Test> as SupplyLedgerGovern>::do_mint_canonical(
            &asset_id,
            DomainId::X3Evm,
            400
        ));
        let minted = ledger_of(asset_id);
        assert_eq!(minted.evm_supply, 400, "the mint credits the named domain");
        assert_eq!(
            minted.canonical_supply, 1_000_400,
            "and grows canonical supply"
        );
        assert_ne!(minted, initial, "a mint that does nothing is not a mint");

        assert_ok!(<Pallet<Test> as SupplyLedgerGovern>::do_burn_canonical(
            &asset_id,
            DomainId::X3Evm,
            400
        ));
        assert_eq!(
            ledger_of(asset_id),
            initial,
            "the burn undoes the mint exactly"
        );

        let unknown = asset(97);
        assert!(
            <Pallet<Test> as SupplyLedgerGovern>::do_mint_canonical(
                &unknown,
                DomainId::X3Native,
                1
            )
            .is_err(),
            "minting an unknown asset is refused, not silently Ok"
        );
        assert!(
            <Pallet<Test> as SupplyLedgerGovern>::do_burn_canonical(
                &unknown,
                DomainId::X3Native,
                1
            )
            .is_err(),
            "burning an unknown asset is refused, not silently Ok"
        );
        assert!(
            <Pallet<Test> as SupplyLedgerGovern>::do_burn_canonical(
                &asset_id,
                DomainId::X3Native,
                2_000_000
            )
            .is_err(),
            "burning beyond the domain balance is refused"
        );
    });
}

/// `is_halted` reports the transfer-halt flag both ways. Constants `true`/`false` both survived.
#[test]
fn halt_inspect_reports_the_transfer_halt_flag() {
    new_test_ext().execute_with(|| {
        assert!(
            !<Pallet<Test> as EconomicHaltInspect>::is_halted(),
            "a fresh ledger is not halted"
        );
        crate::TransferHalted::<Test>::put(true);
        assert!(
            <Pallet<Test> as EconomicHaltInspect>::is_halted(),
            "the flag is reported once set"
        );
        crate::TransferHalted::<Test>::put(false);
        assert!(
            !<Pallet<Test> as EconomicHaltInspect>::is_halted(),
            "and clearing it is reported too"
        );
    });
}
