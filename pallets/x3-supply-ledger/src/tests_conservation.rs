// SPDX-License-Identifier: Apache-2.0
//
// tests_conservation.rs — supply conservation across the transitions the chain actually performs.
//
// The pallet's S0-1 suite builds `SupplyLedger` values by hand and checks `check_invariant()` on
// them, which is arithmetic on a struct. What every cross-domain operation goes through is
// `SupplyLedgerWrite` — debit a source leg into `pending_supply`, credit a destination leg out of
// it, or refund it back — and those three functions had no runtime to run against until `mock.rs`
// existed. These tests drive them.
//
// Two laws are asserted, and they are the whole point of the ledger:
//
//   1. A transition that succeeds never changes the represented total. Debit moves source ->
//      pending, credit moves pending -> destination, refund moves pending -> source: each is a
//      relabelling, so `native + evm + svm + external_locked + pending` is invariant. That is what
//      makes "the same value, one place" true across domains.
//   2. A transition that is refused does not mutate the ledger at all. Without this, a caller could
//      read a failure and still have moved supply.

use crate::mock::{
    asset, ledger_of, new_test_ext, pause, register_asset, represented_of, unpause, Test,
};
use crate::Pallet;
use frame_support::assert_ok;
use sp_core::H256;
use x3_asset_kernel_types::traits::SupplyLedgerWrite;
use x3_asset_kernel_types::{DomainId, SupplyLedger};

/// The representative asset each test uses, with a ceiling and a fully-native starting balance.
const CANONICAL: u128 = 1_000_000;
const NATIVE: u128 = 1_000_000;

fn seeded(seed: u8) -> H256 {
    let asset_id = asset(seed);
    register_asset(asset_id, CANONICAL, NATIVE);
    asset_id
}

fn debit(asset_id: H256, domain: DomainId, amount: u128) -> Result<(), sp_runtime::DispatchError> {
    Pallet::<Test>::debit_source_to_pending(&asset_id, domain, amount)
}

fn credit(asset_id: H256, domain: DomainId, amount: u128) -> Result<(), sp_runtime::DispatchError> {
    Pallet::<Test>::credit_destination_from_pending(&asset_id, domain, amount)
}

fn refund(asset_id: H256, domain: DomainId, amount: u128) -> Result<(), sp_runtime::DispatchError> {
    Pallet::<Test>::refund_pending_to_source(&asset_id, domain, amount)
}

/// The success path, the rollback path and the external route, with the total checked at every step.
#[test]
fn every_leg_of_the_lifecycle_conserves_the_represented_total() {
    new_test_ext().execute_with(|| {
        let asset_id = seeded(1);
        assert_eq!(represented_of(asset_id), CANONICAL);

        // Success: X3 native -> X3 EVM.
        debit(asset_id, DomainId::X3Native, 400).expect("the source leg has the balance");
        assert_eq!(ledger_of(asset_id).pending_supply, 400);
        assert_eq!(
            represented_of(asset_id),
            CANONICAL,
            "a debit must only relabel supply"
        );
        credit(asset_id, DomainId::X3Evm, 400).expect("the pending leg was debited");
        let after_success = ledger_of(asset_id);
        assert_eq!(after_success.pending_supply, 0);
        assert_eq!(after_success.evm_supply, 400);
        assert_eq!(represented_of(asset_id), CANONICAL);

        // External route: the same shape, landing in the external-locked line.
        debit(asset_id, DomainId::X3Native, 100).expect("the source leg has the balance");
        credit(asset_id, DomainId::Ethereum, 100).expect("the pending leg was debited");
        assert_eq!(ledger_of(asset_id).external_locked_supply, 100);
        assert_eq!(represented_of(asset_id), CANONICAL);

        // Rollback: a refund restores the exact ledger the debit left behind.
        let before_debit = ledger_of(asset_id);
        debit(asset_id, DomainId::X3Native, 250).expect("the source leg has the balance");
        refund(asset_id, DomainId::X3Native, 250).expect("the pending leg was debited");
        assert_eq!(
            ledger_of(asset_id),
            before_debit,
            "debit then refund must be a no-op on the ledger, not merely on the total"
        );
        assert_eq!(represented_of(asset_id), CANONICAL);

        assert!(
            ledger_of(asset_id).check_invariant().is_ok(),
            "the king invariant holds after every one of those steps"
        );
        // And the ledger as a whole is publishable: block finalization finds no violation, so it
        // does not halt transfers.
        <Pallet<Test> as frame_support::traits::Hooks<u64>>::on_finalize(1);
        assert!(
            !crate::TransferHalted::<Test>::get(),
            "a conserved ledger must not trip the on-finalize violation response"
        );
    });
}

/// A refused transition leaves the ledger byte-for-byte identical.
#[test]
fn a_refused_transition_does_not_mutate_the_ledger() {
    new_test_ext().execute_with(|| {
        let asset_id = seeded(2);

        let before = ledger_of(asset_id);
        assert!(
            debit(asset_id, DomainId::X3Native, NATIVE + 1).is_err(),
            "debit more than the source domain holds"
        );
        assert_eq!(
            ledger_of(asset_id),
            before,
            "and it must not mutate anything"
        );

        assert!(
            credit(asset_id, DomainId::X3Evm, 1).is_err(),
            "credit with nothing pending"
        );
        assert_eq!(ledger_of(asset_id), before);

        assert!(
            refund(asset_id, DomainId::X3Native, 1).is_err(),
            "refund with nothing pending"
        );
        assert_eq!(ledger_of(asset_id), before);

        // A settled leg cannot be settled twice: the second credit has no pending supply behind it,
        // so the message can be replayed without double-crediting the destination.
        debit(asset_id, DomainId::X3Native, 40).expect("fund the pending leg");
        credit(asset_id, DomainId::X3Evm, 40).expect("settle it once");
        let after_first_settle = ledger_of(asset_id);
        assert!(
            credit(asset_id, DomainId::X3Evm, 40).is_err(),
            "a duplicate settle must be refused, not applied a second time"
        );
        assert_eq!(ledger_of(asset_id), after_first_settle);
        assert_eq!(represented_of(asset_id), CANONICAL);
    });
}

/// `pending_supply` returns to its starting value once every leg settles or refunds.
#[test]
fn pending_supply_returns_to_zero_when_every_leg_is_resolved() {
    new_test_ext().execute_with(|| {
        let asset_id = seeded(3);

        for amount in [1u128, 7, 999, 250_000, 500_000] {
            debit(asset_id, DomainId::X3Native, amount).expect("debit");
            assert_eq!(ledger_of(asset_id).pending_supply, amount);
            if amount % 2 == 0 {
                credit(asset_id, DomainId::X3Svm, amount).expect("settle");
            } else {
                refund(asset_id, DomainId::X3Native, amount).expect("refund");
            }
            assert_eq!(
                ledger_of(asset_id).pending_supply,
                0,
                "pending supply is transient: it must not accumulate"
            );
            assert_eq!(represented_of(asset_id), CANONICAL);
        }
    });
}

/// Halting new legs must not strand what is already pending, and pausing an asset must not either.
#[test]
fn a_halt_or_a_pause_blocks_new_legs_but_still_lets_the_pending_one_come_back() {
    new_test_ext().execute_with(|| {
        let asset_id = seeded(4);

        // The asset is paused while a leg is in flight.
        debit(asset_id, DomainId::X3Native, 500).expect("debit before the pause");
        pause(asset_id);
        assert!(
            debit(asset_id, DomainId::X3Native, 1).is_err(),
            "a paused asset accepts no new debit"
        );
        assert!(
            credit(asset_id, DomainId::X3Evm, 1).is_err(),
            "and no new destination credit"
        );
        refund(asset_id, DomainId::X3Native, 500)
            .expect("a refund must still work while the asset is paused");
        assert_eq!(
            ledger_of(asset_id),
            register_asset(asset_id, CANONICAL, NATIVE)
        );

        // And the same through the transfer halt: new legs refused, refunds allowed.
        unpause(asset_id);
        crate::Pallet::<Test>::halt_transfers(frame_system::RawOrigin::Signed(1).into())
            .expect("the mock's supply governance is any signed account");
        assert!(debit(asset_id, DomainId::X3Native, 5).is_err());
        let assets_before = ledger_of(asset_id);
        debit_unhalted(&asset_id, 25);
        assert!(credit(asset_id, DomainId::X3Evm, 25).is_err());
        refund(asset_id, DomainId::X3Native, 25).expect("refunds are allowed while halted");
        assert_eq!(ledger_of(asset_id), assets_before);
    });
}

/// Debit while the halt flag is still set, to leave a pending leg to refund.
fn debit_unhalted(asset_id: &H256, amount: u128) {
    crate::TransferHalted::<Test>::put(false);
    debit(*asset_id, DomainId::X3Native, amount).expect("debit while unhalted");
    crate::TransferHalted::<Test>::put(true);
}

// ── Property: random sequences of legs ───────────────────────────────────────────────────────

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Leg {
    Debit { domain: DomainId, amount: u128 },
    Credit { domain: DomainId, amount: u128 },
    Refund { domain: DomainId, amount: u128 },
}

const DOMAINS: [DomainId; 4] = [
    DomainId::X3Native,
    DomainId::X3Evm,
    DomainId::X3Svm,
    DomainId::Ethereum,
];

impl Arbitrary for Leg {
    fn arbitrary(g: &mut Gen) -> Self {
        let domain = *g.choose(&DOMAINS).expect("the domain list is not empty");
        let amount = u128::arbitrary(g) % 2_000;
        match u8::arbitrary(g) % 3 {
            0 => Leg::Debit { domain, amount },
            1 => Leg::Credit { domain, amount },
            _ => Leg::Refund { domain, amount },
        }
    }
}

fn apply(leg: &Leg, asset_id: H256) -> Result<(), sp_runtime::DispatchError> {
    match leg {
        Leg::Debit { domain, amount } => debit(asset_id, *domain, *amount),
        Leg::Credit { domain, amount } => credit(asset_id, *domain, *amount),
        Leg::Refund { domain, amount } => refund(asset_id, *domain, *amount),
    }
}

fn prop_legs_conserve(legs: Vec<Leg>) -> TestResult {
    if legs.len() > 40 {
        return TestResult::discard();
    }
    let asset_id = asset(9);
    let outcome = std::panic::catch_unwind(move || {
        new_test_ext().execute_with(|| {
            register_asset(asset_id, CANONICAL, NATIVE);
            for (index, leg) in legs.iter().enumerate() {
                let before = ledger_of(asset_id);
                let total_before = represented_of(asset_id);
                let result = apply(leg, asset_id);
                let after = ledger_of(asset_id);
                match result {
                    Ok(()) => assert_eq!(
                        represented_of(asset_id),
                        total_before,
                        "step {index}: a successful {leg:?} changed the represented total"
                    ),
                    Err(_) => assert_eq!(
                        after, before,
                        "step {index}: a refused {leg:?} mutated the ledger"
                    ),
                }
                assert!(
                    after.check_invariant().is_ok(),
                    "step {index}: the invariant broke after {leg:?}"
                );
                assert_eq!(
                    after.represented(),
                    Some(represented_of(asset_id)),
                    "step {index}: represented() disagrees with the summed fields"
                );
            }
        })
    });
    match outcome {
        Ok(()) => TestResult::passed(),
        Err(_) => TestResult::failed(),
    }
}

#[test]
fn prop_random_leg_sequences_conserve_supply() {
    QuickCheck::new()
        .tests(200)
        .max_tests(1_000)
        .quickcheck(prop_legs_conserve as fn(Vec<Leg>) -> TestResult);
}

/// The mint path's own conservation: a governance mint raises the ceiling and the represented
/// total together, and a replayed nonce does neither.
#[test]
fn a_replayed_mint_nonce_cannot_mint_twice() {
    new_test_ext().execute_with(|| {
        let asset_id = seeded(5);
        let before: SupplyLedger = ledger_of(asset_id);

        frame_system::Pallet::<Test>::set_block_number(1);
        let origin: crate::mock::RuntimeOrigin = frame_system::RawOrigin::Signed(7).into();
        assert_ok!(Pallet::<Test>::mint_canonical(
            origin.clone(),
            asset_id,
            DomainId::X3Native,
            500,
            0
        ));
        let after_mint = ledger_of(asset_id);
        assert_eq!(after_mint.native_supply, before.native_supply + 500);
        assert_eq!(
            after_mint.canonical_supply,
            before.canonical_supply + 500,
            "a mint raises the ceiling with the supply, so the invariant still holds"
        );
        assert!(after_mint.check_invariant().is_ok());

        assert!(
            Pallet::<Test>::mint_canonical(origin, asset_id, DomainId::X3Native, 500, 0).is_err(),
            "the same nonce must not mint a second time"
        );
        assert_eq!(
            ledger_of(asset_id),
            after_mint,
            "and the refused replay must leave the ledger untouched"
        );
    });
}
