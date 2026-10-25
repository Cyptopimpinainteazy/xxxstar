//! A token launch on this runtime, against real pallets: contributions paid into the launch's
//! custody on the kernel's ledger, a token launched through the token factory (registry and
//! supply ledger included) with its supply on that ledger, a real `pallet_x3_dex` pool seeded from
//! custody with half the raise, contributors paid their tokens, the creator paid the other half,
//! and the locked LP position released to the creator after the lock.

use crate::{AccountId, AtlasKernel, Runtime, RuntimeOrigin, System, X3Dex, X3Launchpad};
use frame_support::assert_ok;

const QUOTE: u32 = 0;
const PRICE: u128 = 10;
const START: u32 = 10;
const END: u32 = START + 3_000;
const LOCK: u32 = 100;

fn account(seed: u8) -> AccountId {
    AccountId::new([seed; 32])
}

fn balance(who: &AccountId, asset: u32) -> u128 {
    pallet_x3_kernel::Pallet::<Runtime>::ledger_balance_of(asset, who)
}

#[test]
fn a_launch_graduates_into_a_funded_pool_and_every_unit_is_accounted_for() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let (creator, alice, bob) = (account(1), account(2), account(3));
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            QUOTE,
            b"X3".to_vec(),
            12
        ));
        for who in [&alice, &bob] {
            pallet_x3_kernel::CanonicalLedger::<Runtime>::insert(who, QUOTE, 10_000_000u128);
        }

        assert_ok!(X3Launchpad::create_launch(
            RuntimeOrigin::signed(creator.clone()),
            0,
            1_000_000,
            10_000_000,
            PRICE,
            START,
            END,
            LOCK
        ));
        let id = pallet_x3_launchpad::NextLaunchId::<Runtime>::get() - 1;
        let custody = X3Launchpad::launch_account(id);

        System::set_block_number(START);
        assert_ok!(X3Launchpad::contribute(
            RuntimeOrigin::signed(alice.clone()),
            id,
            1_200_000
        ));
        assert_ok!(X3Launchpad::contribute(
            RuntimeOrigin::signed(bob.clone()),
            id,
            800_000
        ));
        assert_eq!(
            balance(&custody, QUOTE),
            2_000_000,
            "contributions are in custody"
        );

        System::set_block_number(END + 1);
        assert_ok!(X3Launchpad::finalize_launch(
            RuntimeOrigin::signed(alice.clone()),
            id
        ));
        assert_ok!(X3Launchpad::graduate_launch(
            RuntimeOrigin::signed(creator.clone()),
            id
        ));
        let graduation = pallet_x3_launchpad::Graduations::<Runtime>::get(id).expect("graduated");
        let token = graduation.token_asset;
        assert!(
            token >= pallet_x3_kernel::trading::ISSUED_ASSET_BASE,
            "the token is an issued ledger asset"
        );

        // 2 000 000 raised at 10: 200 000 tokens for contributors; 1 000 000 quote and 100 000
        // tokens seed the pool, which custody owns.
        let pool = X3Dex::pools(graduation.pool_id).expect("a real pool");
        assert_eq!((pool.reserve_a, pool.reserve_b), (1_000_000, 100_000));
        let pool_account = pallet_x3_dex::Pallet::<Runtime>::pool_account(graduation.pool_id);
        assert_eq!(balance(&pool_account, QUOTE), 1_000_000);
        assert_eq!(balance(&pool_account, token), 100_000);
        assert_eq!(
            pallet_x3_dex::Pallet::<Runtime>::position_owner(graduation.position_id),
            Some(custody.clone())
        );

        assert_ok!(X3Launchpad::claim_allocation(
            RuntimeOrigin::signed(alice.clone()),
            id
        ));
        assert_ok!(X3Launchpad::claim_allocation(
            RuntimeOrigin::signed(bob.clone()),
            id
        ));
        assert_eq!(balance(&alice, token), 120_000);
        assert_eq!(balance(&bob, token), 80_000);
        assert_ok!(X3Launchpad::withdraw_raised_funds(
            RuntimeOrigin::signed(creator.clone()),
            id
        ));
        assert_eq!(balance(&creator, QUOTE), 1_000_000);
        assert_eq!(balance(&custody, QUOTE), 0);
        assert_eq!(balance(&custody, token), 0);

        // The token's ledger supply is exactly what graduation minted, and it matches the supply
        // ledger's native leg for the factory asset.
        let token_supply = [&alice, &bob, &custody, &pool_account, &creator]
            .iter()
            .map(|who| balance(who, token))
            .sum::<u128>();
        assert_eq!(token_supply, 300_000);
        let factory_asset = pallet_x3_token_factory::LedgerAssetOf::<Runtime>::iter()
            .find_map(|(asset, ledger)| (ledger == token).then_some(asset))
            .expect("the factory knows the token");
        assert_eq!(
            pallet_x3_supply_ledger::Ledgers::<Runtime>::get(factory_asset)
                .expect("supply ledger")
                .native_supply,
            token_supply,
            "account balances sum to the supply ledger's native leg"
        );

        // The LP position stays in custody until the lock expires, then the creator can take
        // the liquidity out of the pool.
        assert!(X3Launchpad::claim_lp(RuntimeOrigin::signed(creator.clone()), id).is_err());
        System::set_block_number(END + 1 + LOCK);
        assert_ok!(X3Launchpad::claim_lp(
            RuntimeOrigin::signed(creator.clone()),
            id
        ));
        let lp = X3Dex::lp_positions(graduation.position_id)
            .expect("position")
            .lp_balance;
        assert_ok!(X3Dex::remove_liquidity(
            RuntimeOrigin::signed(creator.clone()),
            graduation.position_id,
            lp,
            0,
            0
        ));
        assert_eq!(balance(&creator, token), 100_000);
        assert_eq!(balance(&creator, QUOTE), 2_000_000);
    });
}
