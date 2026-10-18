//! A compiled X3 Lang trade run on this runtime, against real pallets.
//!
//! The venues are `pallet_x3_dex` pools created and funded through their extrinsics, the flash
//! provider is a `pallet_x3_flashloan` asset pool opened by governance and funded by a provider,
//! and prices come from three authorized `pallet_x3_oracle` submitters. The only state written
//! directly is each participant's starting `CanonicalLedger` balance (what a mint or genesis would
//! give them). The program is `x3-lang/examples/x3chain_flash_arb.x3`, compiled by the canonical
//! lowering, with its private-submission line set to `false`: the chain has no private channel,
//! and `pallet_x3_kernel`'s own suite holds it to refusing the private original.

use crate::{
    AccountId, AtlasKernel, Runtime, RuntimeEvent, RuntimeOrigin, System, X3Dex, X3FlashLoan,
    X3Oracle,
};
use frame_support::assert_ok;
use x3_dex::TokenId;
use x3_lang_compiler::semantic::CompilationMode;
use x3_lang_compiler::x3bc::compile_trading_x3bc;
use x3bc_core::receipt::{self, Outcome};

const FLAGSHIP: &str = include_str!("../../x3-lang/examples/x3chain_flash_arb.x3");

const USDX: u32 = 1;
const WETX: u32 = 2;
const UNIT: u128 = 1_000_000;
const LEDGER_CHAIN: u32 = 650_000;

const POOL1: (u128, u128) = (1_000_000 * UNIT, 500_000 * UNIT); // USDX, WETX
const POOL2: (u128, u128) = (400_000 * UNIT, 830_000 * UNIT); // WETX, USDX
const FLASH_LIQUIDITY: u128 = 5_000_000 * UNIT;
const TRADER_USDX: u128 = 50 * UNIT;
const BORROW: u128 = 1_000 * UNIT;

const SUBMITTED_AT: u32 = 1_000;
const EXECUTED_AT: u32 = 1_002;

fn account(seed: u8) -> AccountId {
    AccountId::new([seed; 32])
}

fn token(asset: u32) -> TokenId {
    TokenId {
        chain_id: LEDGER_CHAIN,
        asset_id: u128::from(asset),
    }
}

fn balance(who: &AccountId, asset: u32) -> u128 {
    pallet_x3_kernel::Pallet::<Runtime>::ledger_balance_of(asset, who)
}

fn mint(who: &AccountId, asset: u32, amount: u128) {
    pallet_x3_kernel::CanonicalLedger::<Runtime>::mutate(who, asset, |b| *b += amount);
}

/// `in_after_fee * reserve_out / (reserve_in + in_after_fee)`, 30 bps taken from the input.
fn cp_out(amount_in: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    let net = amount_in - amount_in * 30 / 10_000;
    net * reserve_out / (reserve_in + net)
}

#[test]
fn a_compiled_x3lang_arbitrage_runs_on_this_runtime_against_real_dex_flash_and_oracle_pallets() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(EXECUTED_AT);
        let (lp, provider, trader, collector) = (account(1), account(2), account(3), account(4));

        // Governance: assets, the flash pool, oracle submitters, the cost schedule.
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            USDX,
            b"USDX".to_vec(),
            6
        ));
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            WETX,
            b"WETX".to_vec(),
            6
        ));
        assert_ok!(X3FlashLoan::create_asset_pool(
            RuntimeOrigin::root(),
            USDX,
            5
        ));
        let oracles = [account(10), account(11), account(12)];
        for oracle in &oracles {
            assert_ok!(X3Oracle::authorize_oracle(
                RuntimeOrigin::root(),
                oracle.clone()
            ));
        }
        assert_ok!(AtlasKernel::set_trading_config(
            RuntimeOrigin::root(),
            pallet_x3_kernel::TradingConfig {
                fee_asset: USDX,
                gas_price: 10,
                settlement_fee: 200_000,
                relay_fee: 300_000,
                protocol_fee: 100_000,
                oracle_fee: 10_000,
                solver_fee: 0,
                cross_domain_fee: 0,
                fee_collector: collector.clone(),
            }
        ));

        // Starting balances.
        mint(&lp, USDX, POOL1.0 + POOL2.1);
        mint(&lp, WETX, POOL1.1 + POOL2.0);
        mint(&provider, USDX, FLASH_LIQUIDITY);
        mint(&trader, USDX, TRADER_USDX);

        // Two pools of the same pair, ids 1 and 2, funded through the dex's own extrinsics.
        assert_ok!(X3Dex::create_pool(
            RuntimeOrigin::signed(lp.clone()),
            token(USDX),
            token(WETX),
            30
        ));
        assert_ok!(X3Dex::add_liquidity(
            RuntimeOrigin::signed(lp.clone()),
            1,
            POOL1.0,
            POOL1.1,
            0,
            0
        ));
        assert_ok!(X3Dex::create_pool(
            RuntimeOrigin::signed(lp.clone()),
            token(WETX),
            token(USDX),
            30
        ));
        assert_ok!(X3Dex::add_liquidity(
            RuntimeOrigin::signed(lp.clone()),
            2,
            POOL2.0,
            POOL2.1,
            0,
            0
        ));
        assert_ok!(X3FlashLoan::provide_asset_liquidity(
            RuntimeOrigin::signed(provider.clone()),
            USDX,
            FLASH_LIQUIDITY
        ));
        // $1 per USDX and $2 per WETX (6 decimals), from three submitters.
        for oracle in &oracles {
            assert_ok!(X3Oracle::submit_price(
                RuntimeOrigin::signed(oracle.clone()),
                USDX,
                1_000_000
            ));
            assert_ok!(X3Oracle::submit_price(
                RuntimeOrigin::signed(oracle.clone()),
                WETX,
                2_000_000
            ));
        }

        let pool1 = pallet_x3_dex::Pallet::<Runtime>::pool_account(1);
        let pool2 = pallet_x3_dex::Pallet::<Runtime>::pool_account(2);
        let flash = pallet_x3_flashloan::Pallet::<Runtime>::asset_pool_account(USDX);
        let everyone = [&lp, &provider, &trader, &collector, &pool1, &pool2, &flash];
        let supply = |asset| everyone.iter().map(|who| balance(who, asset)).sum::<u128>();
        let (usdx_supply, wetx_supply) = (supply(USDX), supply(WETX));

        // The trade, as the trader would submit it.
        let source = FLAGSHIP.replace(
            "require_private_submission: true",
            "require_private_submission: false",
        );
        let artifact = compile_trading_x3bc(&source, CompilationMode::Testnet).expect("compiles");
        assert_ok!(AtlasKernel::submit_trade(
            RuntimeOrigin::signed(trader.clone()),
            artifact.bytes.clone().try_into().expect("fits"),
            SUBMITTED_AT
        ));

        let digest = System::events()
            .into_iter()
            .find_map(|record| match record.event {
                RuntimeEvent::AtlasKernel(pallet_x3_kernel::Event::TradeCommitted {
                    receipt,
                    ..
                }) => Some(receipt),
                RuntimeEvent::AtlasKernel(pallet_x3_kernel::Event::TradeRolledBack {
                    trap,
                    op_index,
                    ..
                }) => panic!("the trade rolled back: trap {trap} at op {op_index}"),
                _ => None,
            })
            .expect("the trade committed");
        let record = pallet_x3_kernel::TradeReceipts::<Runtime>::get(digest).expect("recorded");
        let receipt = receipt::decode(&record.receipt).expect("canonical receipt");
        assert_eq!(receipt.outcome, Outcome::Committed);
        assert_eq!(receipt.anchor.spec_version, crate::VERSION.spec_version);

        // Exactly the formula's figures moved, and nothing was created or destroyed.
        let swap1 = cp_out(BORROW, POOL1.0, POOL1.1);
        let swap2 = cp_out(swap1, POOL2.0, POOL2.1);
        let flash_fee = BORROW * 5 / 10_000;
        assert_eq!(
            (balance(&pool1, USDX), balance(&pool1, WETX)),
            (POOL1.0 + BORROW, POOL1.1 - swap1)
        );
        assert_eq!(
            (balance(&pool2, WETX), balance(&pool2, USDX)),
            (POOL2.0 + swap1, POOL2.1 - swap2)
        );
        assert_eq!(balance(&flash, USDX), FLASH_LIQUIDITY + flash_fee);
        let costs = balance(&collector, USDX);
        assert!(costs > 0);
        assert_eq!(
            balance(&trader, USDX),
            TRADER_USDX + swap2 - BORROW - flash_fee - costs
        );
        assert!(
            balance(&trader, USDX) >= TRADER_USDX + UNIT,
            "the profit floor held"
        );
        assert_eq!((supply(USDX), supply(WETX)), (usdx_supply, wetx_supply));

        // The dex's own records agree with custody: the pools trade on from the new reserves.
        let after1 = X3Dex::pools(1).expect("pool 1");
        assert_eq!(
            (after1.reserve_a, after1.reserve_b),
            (POOL1.0 + BORROW, POOL1.1 - swap1)
        );
        let after2 = X3Dex::pools(2).expect("pool 2");
        assert_eq!(
            (after2.reserve_a, after2.reserve_b),
            (POOL2.0 + swap1, POOL2.1 - swap2)
        );

        // And the flash pool's providers own the fee.
        assert_ok!(X3FlashLoan::withdraw_asset_liquidity(
            RuntimeOrigin::signed(provider.clone()),
            USDX,
            FLASH_LIQUIDITY
        ));
        assert_eq!(balance(&provider, USDX), FLASH_LIQUIDITY + flash_fee);
    });
}
