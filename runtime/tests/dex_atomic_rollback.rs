//! Exercise DEX custody through the runtime's real canonical ledger adapter.
#![cfg(not(feature = "mainnet-rc1"))]

use frame_support::{
    assert_noop, assert_ok,
    storage::{with_transaction, TransactionOutcome},
};
use pallet_x3_dex::{Error, NextPoolId};
use sp_runtime::DispatchError;
use x3_chain_runtime::{AccountId, Runtime, RuntimeOrigin};
use x3_dex::amm_pools::{AMMPool, TokenId};

type Dex = pallet_x3_dex::Pallet<Runtime>;
type Kernel = pallet_x3_kernel::Pallet<Runtime>;
type System = frame_system::Pallet<Runtime>;

#[test]
fn failed_second_asset_transfers_revert_the_real_dex_entrypoints() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let provider = AccountId::from([1; 32]);
        let trader = AccountId::from([2; 32]);
        let asset_a = Kernel::issue_ledger_asset(b"A", 6).unwrap();
        let asset_b = Kernel::issue_ledger_asset(b"B", 6).unwrap();
        let token = |asset| TokenId {
            chain_id: <Runtime as pallet_x3_dex::Config>::LedgerChainId::get(),
            asset_id: u128::from(asset),
        };
        assert_ok!(Kernel::mint_to_ledger(asset_a, &provider, 20_000));
        assert_ok!(Kernel::mint_to_ledger(asset_a, &trader, 1_000));
        assert_ok!(Dex::create_pool(
            RuntimeOrigin::signed(provider.clone()),
            token(asset_a),
            token(asset_b),
            30,
        ));
        let pool = NextPoolId::<Runtime>::get();
        let custody = Dex::pool_account(pool);
        // The first deposit transfer can succeed, but the provider owns no B.
        // No outer transaction supplied by this test: exercise the dispatchable's layer.
        assert_noop!(
            Dex::add_liquidity(
                RuntimeOrigin::signed(provider.clone()),
                pool,
                10_000,
                10_000,
                1,
                1,
            ),
            pallet_x3_kernel::Error::<Runtime>::InsufficientLedgerBalance
        );
        assert_eq!(Kernel::ledger_balance_of(asset_a, &provider), 20_000);
        assert_eq!(Kernel::ledger_balance_of(asset_a, &custody), 0);

        assert_ok!(Kernel::mint_to_ledger(asset_b, &provider, 10_000));
        assert_ok!(Dex::add_liquidity(
            RuntimeOrigin::signed(provider),
            pool,
            10_000,
            10_000,
            1,
            1,
        ));
        // Inject insolvency through a real ledger debit, leaving recorded reserves intact.
        assert_ok!(Kernel::burn_from_ledger(asset_b, &custody, 10_000));
        assert_noop!(
            Dex::swap(
                RuntimeOrigin::signed(trader.clone()),
                pool,
                token(asset_a),
                100,
                1,
            ),
            pallet_x3_kernel::Error::<Runtime>::InsufficientLedgerBalance
        );
        assert_eq!(Kernel::ledger_balance_of(asset_a, &trader), 1_000);
        assert_eq!(Kernel::ledger_balance_of(asset_a, &custody), 10_000);
        assert_eq!(Kernel::ledger_balance_of(asset_b, &custody), 0);
    });
}

#[test]
fn canonical_custody_rolls_back_both_hops_and_allows_an_atomic_retry() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let provider = AccountId::from([1; 32]);
        let trader = AccountId::from([2; 32]);
        let assets = [
            Kernel::issue_ledger_asset(b"A", 6).unwrap(),
            Kernel::issue_ledger_asset(b"B", 6).unwrap(),
            Kernel::issue_ledger_asset(b"C", 6).unwrap(),
        ];
        for asset in assets {
            assert_ok!(Kernel::mint_to_ledger(asset, &provider, 20_000));
        }
        assert_ok!(Kernel::mint_to_ledger(assets[0], &trader, 1_000));
        let tokens = assets.map(|asset| TokenId {
            chain_id: <Runtime as pallet_x3_dex::Config>::LedgerChainId::get(),
            asset_id: u128::from(asset),
        });
        let mut pools = Vec::new();
        for pair in tokens.windows(2) {
            assert_ok!(Dex::create_pool(
                RuntimeOrigin::signed(provider.clone()),
                pair[0].clone(),
                pair[1].clone(),
                30,
            ));
            let id = NextPoolId::<Runtime>::get();
            assert_ok!(Dex::add_liquidity(
                RuntimeOrigin::signed(provider.clone()),
                id,
                10_000,
                10_000,
                1,
                1,
            ));
            pools.push(id);
        }
        let accounts = [
            provider,
            trader.clone(),
            Dex::pool_account(pools[0]),
            Dex::pool_account(pools[1]),
        ];
        for (index, account) in accounts.iter().enumerate() {
            for other in &accounts[index + 1..] {
                assert_ne!(
                    account, other,
                    "custody and trader accounts must be distinct"
                );
            }
        }
        let balances = || {
            assets.map(|asset| {
                accounts
                    .each_ref()
                    .map(|who| Kernel::ledger_balance_of(asset, who))
            })
        };
        let before = balances();
        let pool_before = [Dex::pools(pools[0]).unwrap(), Dex::pools(pools[1]).unwrap()];
        let events_before = System::events();
        let carry = AMMPool::swap_calculate(&pool_before[0], &tokens[0], 100, 1).unwrap();
        let output = AMMPool::swap_calculate(&pool_before[1], &tokens[1], carry, 1).unwrap();

        let failed: Result<(), DispatchError> = with_transaction(|| {
            assert_ok!(Dex::swap(
                RuntimeOrigin::signed(trader.clone()),
                pools[0],
                tokens[0].clone(),
                100,
                1,
            ));
            assert_ne!(
                balances(),
                before,
                "the first hop must move canonical assets"
            );
            assert_ne!(Dex::pools(pools[0]).unwrap(), pool_before[0]);
            match Dex::swap(
                RuntimeOrigin::signed(trader.clone()),
                pools[1],
                tokens[1].clone(),
                carry,
                u128::MAX,
            ) {
                Ok(()) => TransactionOutcome::Commit(Ok(())),
                Err(error) => TransactionOutcome::Rollback(Err(error)),
            }
        });
        assert_eq!(failed, Err(Error::<Runtime>::SlippageExceeded.into()));
        assert_eq!(balances(), before);
        for (id, expected) in pools.iter().zip(&pool_before) {
            assert_eq!(Dex::pools(id).unwrap(), *expected);
        }
        assert_eq!(System::events(), events_before);

        let retry: Result<(), DispatchError> = with_transaction(|| {
            for (index, amount) in [100, carry].into_iter().enumerate() {
                if let Err(error) = Dex::swap(
                    RuntimeOrigin::signed(trader.clone()),
                    pools[index],
                    tokens[index].clone(),
                    amount,
                    1,
                ) {
                    return TransactionOutcome::Rollback(Err(error));
                }
            }
            TransactionOutcome::Commit(Ok(()))
        });
        assert_ok!(retry);
        let after = balances();
        for (initial, final_balances) in before.iter().zip(&after) {
            assert_eq!(
                initial.iter().sum::<u128>(),
                final_balances.iter().sum::<u128>()
            );
        }
        assert_eq!(after[0][1], before[0][1] - 100);
        assert_eq!(after[1][1], before[1][1]);
        assert_eq!(after[2][1], before[2][1] + output);
        for (index, id) in pools.iter().enumerate() {
            let pool = Dex::pools(id).unwrap();
            assert_eq!(
                Kernel::ledger_balance_of(assets[index], &accounts[index + 2]),
                pool.reserve_a
            );
            assert_eq!(
                Kernel::ledger_balance_of(assets[index + 1], &accounts[index + 2]),
                pool.reserve_b
            );
        }
        assert!(System::events().len() > events_before.len());
    });
}
