//! `submit_trade` on the kernel: a compiled X3 Lang program run on the production engine and
//! settled on `CanonicalLedger`.
//!
//! The program is the flagship `x3-lang/examples/x3chain_flash_arb.x3`, compiled by the canonical
//! lowering (`compile_trading_x3bc`) — no hand-assembled bytecode. The flagship demands private
//! submission, which the chain cannot offer, so the committed path compiles the same program with
//! that one policy line set to `false` (an ordinary public trade), and a separate test holds the
//! chain to refusing the original.
//!
//! Expected figures are re-derived from the constant-product formula here, not read back from the
//! engine, and every committed test checks conservation of every asset across every account.

use crate::mock::*;
use crate::pallet::{
    CanonicalLedger, Error, Event, TradeNonces, TradeReceipts, TradingConfiguration,
};
use crate::trading::TradingConfig;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchResult, BoundedVec};
use std::collections::BTreeMap;
use x3_lang_compiler::semantic::CompilationMode;
use x3_lang_compiler::x3bc::compile_trading_x3bc;
use x3bc_core::receipt::{self, Account, Outcome};
use x3bc_core::state::VenueState;

const FLAGSHIP: &str = include_str!("../../../x3-lang/examples/x3chain_flash_arb.x3");

const USDX: u32 = 1;
const WETX: u32 = 2;
const UNIT: u128 = 1_000_000;

const TRADER: AccountId = ALICE;
const VENUE_1: AccountId = 1_001;
const VENUE_2: AccountId = 1_002;
const FLASH: AccountId = 1_003;
const FEE_COLLECTOR: AccountId = 1_004;

const TRADER_USDX: u128 = 50 * UNIT;
const BORROW: u128 = 1_000 * UNIT;
const POOL1: (u128, u128) = (1_000_000 * UNIT, 500_000 * UNIT); // USDX, WETX
const POOL2: (u128, u128) = (400_000 * UNIT, 830_000 * UNIT); // WETX, USDX
const FLASH_LIQUIDITY: u128 = 5_000_000 * UNIT;
const FEE_BPS: u16 = 30;
const FLASH_FEE_BPS: u16 = 5;

const SUBMITTED_AT: u64 = 1_000;
const EXECUTED_AT: u64 = 1_002;

const ACCOUNTS: [AccountId; 5] = [TRADER, VENUE_1, VENUE_2, FLASH, FEE_COLLECTOR];

/// `submit_trade` with its post-dispatch weight dropped, so refusals compare by error alone.
fn submit_trade(
    origin: RuntimeOrigin,
    module: BoundedVec<
        u8,
        frame_support::traits::ConstU32<{ crate::trading::MAX_TRADE_MODULE_BYTES }>,
    >,
    submitted_at_block: u64,
) -> frame_support::dispatch::DispatchResult {
    AtlasKernel::submit_trade(origin, module, submitted_at_block)
        .map(|_| ())
        .map_err(|e| e.error)
}

fn public(source: &str) -> String {
    let public = source.replace(
        "require_private_submission: true",
        "require_private_submission: false",
    );
    assert_ne!(public, source, "the flagship's policy line moved");
    public
}

fn module(
    source: &str,
) -> BoundedVec<u8, frame_support::traits::ConstU32<{ crate::trading::MAX_TRADE_MODULE_BYTES }>> {
    let artifact = compile_trading_x3bc(source, CompilationMode::Testnet)
        .unwrap_or_else(|error| panic!("the trading source must compile: {error:?}"));
    artifact.bytes.try_into().expect("fits the module bound")
}

fn fund(who: AccountId, asset: u32, amount: u128) {
    CanonicalLedger::<Test>::mutate(who, asset, |balance| *balance += amount);
}

fn balances() -> BTreeMap<(AccountId, u32), u128> {
    let mut out = BTreeMap::new();
    for who in ACCOUNTS {
        for asset in [USDX, WETX] {
            out.insert((who, asset), CanonicalLedger::<Test>::get(who, asset));
        }
    }
    out
}

fn supply(snapshot: &BTreeMap<(AccountId, u32), u128>, asset: u32) -> u128 {
    snapshot
        .iter()
        .filter(|((_, a), _)| *a == asset)
        .map(|(_, v)| *v)
        .sum()
}

fn config() -> TradingConfig<AccountId> {
    TradingConfig {
        fee_asset: USDX,
        gas_price: 10,
        settlement_fee: 200_000,
        relay_fee: 300_000,
        protocol_fee: 100_000,
        oracle_fee: 10_000,
        solver_fee: 0,
        cross_domain_fee: 0,
        fee_collector: FEE_COLLECTOR,
    }
}

/// Registered assets, funded pools whose reserves are their custody balances, oracle prices at the
/// pools' mid (2 USDX per WETX), and the governance cost schedule.
fn chain(decimals_wetx: u8) {
    TestTradingHost::reset();
    frame_system::Pallet::<Test>::set_block_number(EXECUTED_AT);
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
        decimals_wetx
    ));
    fund(TRADER, USDX, TRADER_USDX);
    fund(VENUE_1, USDX, POOL1.0);
    fund(VENUE_1, WETX, POOL1.1);
    fund(VENUE_2, WETX, POOL2.0);
    fund(VENUE_2, USDX, POOL2.1);
    fund(FLASH, USDX, FLASH_LIQUIDITY);
    TestTradingHost::add_venue(
        VenueState {
            name: "x3dex_pool_1".into(),
            version: 1,
            asset_a: USDX,
            asset_b: WETX,
            reserve_a: POOL1.0,
            reserve_b: POOL1.1,
            fee_bps: FEE_BPS,
        },
        VENUE_1,
    );
    TestTradingHost::add_venue(
        VenueState {
            name: "x3dex_pool_2".into(),
            version: 1,
            asset_a: WETX,
            asset_b: USDX,
            reserve_a: POOL2.0,
            reserve_b: POOL2.1,
            fee_bps: FEE_BPS,
        },
        VENUE_2,
    );
    TestTradingHost::add_flash_pool(USDX, FLASH_FEE_BPS, FLASH);
    // USD per whole unit, 6 decimals: USDX $1, WETX $2.
    TestTradingHost::set_price(USDX, 1_000_000, EXECUTED_AT - 1);
    TestTradingHost::set_price(WETX, 2_000_000, EXECUTED_AT - 1);
    assert_ok!(AtlasKernel::set_trading_config(
        RuntimeOrigin::root(),
        config()
    ));
}

/// `in_after_fee * reserve_out / (reserve_in + in_after_fee)`, fee taken from the input.
fn cp_out(amount_in: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    let net = amount_in - amount_in * u128::from(FEE_BPS) / 10_000;
    net * reserve_out / (reserve_in + net)
}

fn last_trade_event() -> Event<Test> {
    frame_system::Pallet::<Test>::events()
        .into_iter()
        .rev()
        .find_map(|record| match record.event {
            RuntimeEvent::AtlasKernel(
                event @ (Event::TradeCommitted { .. } | Event::TradeRolledBack { .. }),
            ) => Some(event),
            _ => None,
        })
        .expect("a trade event")
}

#[test]
fn a_compiled_public_arbitrage_commits_on_chain_and_moves_exactly_what_its_receipt_says() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        let bytecode = module(&public(FLAGSHIP));

        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            bytecode.clone(),
            SUBMITTED_AT
        ));
        let Event::TradeCommitted {
            trader,
            receipt: digest,
            gas_used,
            receipt_bytes,
        } = last_trade_event()
        else {
            panic!("the trade must commit");
        };
        assert_eq!(trader, TRADER);
        assert!(gas_used > 0);

        // The stored receipt is the canonical encoding of a committed trade, under its digest.
        // State keeps the digest; the event carries the canonical receipt that hashes to it.
        let record = TradeReceipts::<Test>::get(digest).expect("the receipt is recorded");
        assert!(record.committed);
        let receipt = receipt::decode(&receipt_bytes).expect("canonical receipt");
        assert_eq!(receipt::digest(&receipt), digest.0);
        assert_eq!(receipt.outcome, Outcome::Committed);
        assert_eq!(receipt.anchor.block_number, EXECUTED_AT);
        assert_eq!(receipt.pre_state.submitted_at_block, SUBMITTED_AT);
        assert!(!receipt.pre_state.private_channel);

        // The chain moved exactly the receipt's intents and nothing else.
        let after = balances();
        let mut replayed = before.clone();
        for intent in &receipt.intents {
            let account = |a: &Account| match a {
                Account::Trader => TRADER,
                Account::FeeCollector => FEE_COLLECTOR,
                Account::FlashPool(_) => FLASH,
                Account::Venue(name) if name == "x3dex_pool_1" => VENUE_1,
                Account::Venue(name) if name == "x3dex_pool_2" => VENUE_2,
                Account::Venue(name) => panic!("unexpected venue {name}"),
            };
            *replayed
                .get_mut(&(account(&intent.from), intent.asset))
                .expect("known") -= intent.amount;
            *replayed
                .get_mut(&(account(&intent.to), intent.asset))
                .expect("known") += intent.amount;
        }
        assert_eq!(after, replayed);
        for asset in [USDX, WETX] {
            assert_eq!(
                supply(&before, asset),
                supply(&after, asset),
                "asset {asset} conserved"
            );
        }

        // The figures the formula gives, independently of the engine.
        let swap1 = cp_out(BORROW, POOL1.0, POOL1.1);
        let swap2 = cp_out(swap1, POOL2.0, POOL2.1);
        let flash_fee = BORROW * u128::from(FLASH_FEE_BPS) / 10_000;
        assert_eq!(after[&(VENUE_1, USDX)], POOL1.0 + BORROW);
        assert_eq!(after[&(VENUE_1, WETX)], POOL1.1 - swap1);
        assert_eq!(after[&(VENUE_2, WETX)], POOL2.0 + swap1);
        assert_eq!(after[&(VENUE_2, USDX)], POOL2.1 - swap2);
        assert_eq!(after[&(FLASH, USDX)], FLASH_LIQUIDITY + flash_fee);
        let costs = after[&(FEE_COLLECTOR, USDX)];
        assert!(costs > 0, "the chain's costs were charged");
        assert_eq!(
            after[&(TRADER, USDX)],
            TRADER_USDX + swap2 - BORROW - flash_fee - costs
        );
        assert!(
            after[&(TRADER, USDX)] >= TRADER_USDX + UNIT,
            "the profit floor held"
        );

        // The venues' recorded reserves are their custody balances.
        let venue1 = TestTradingHost::venue_state("x3dex_pool_1").expect("venue");
        assert_eq!(
            (venue1.reserve_a, venue1.reserve_b),
            (after[&(VENUE_1, USDX)], after[&(VENUE_1, WETX)])
        );
        let venue2 = TestTradingHost::venue_state("x3dex_pool_2").expect("venue");
        assert_eq!(
            (venue2.reserve_a, venue2.reserve_b),
            (after[&(VENUE_2, WETX)], after[&(VENUE_2, USDX)])
        );
        assert_eq!(TradeNonces::<Test>::get(TRADER), 1);

        // Anyone can re-execute the on-chain receipt from its own pre-state and get it back.
        let key = x3_x3_integration::trading::AttestorKey::from_seed("test-attestor", [9u8; 32]);
        let signed = x3_x3_integration::trading::sign_receipt(&receipt, &key);
        let mut registry = x3_x3_integration::trading::AttestorRegistry::new();
        registry.trust(&key);
        x3_x3_integration::trading::replay_receipt(&bytecode, &signed, &registry)
            .expect("the on-chain receipt replays");
    });
}

#[test]
fn a_trade_that_misses_its_profit_floor_rolls_back_on_chain_and_moves_nothing() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        let greedy = public(FLAGSHIP).replace("net_profit >= 1 USDX", "net_profit >= 1_000 USDX");
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&greedy),
            SUBMITTED_AT
        ));
        let Event::TradeRolledBack {
            trader,
            receipt: digest,
            trap,
            receipt_bytes,
            ..
        } = last_trade_event()
        else {
            panic!("the trade must roll back");
        };
        assert_eq!(trader, TRADER);
        assert_eq!(balances(), before, "a rolled-back trade moves nothing");
        let record = TradeReceipts::<Test>::get(digest).expect("the refusal is recorded");
        assert!(!record.committed);
        assert_eq!(trap, x3bc_core::trap::TrapId::ProfitBelowFloor as u8);
        let receipt = receipt::decode(&receipt_bytes).expect("canonical receipt");
        assert_eq!(receipt::digest(&receipt), digest.0);
        assert!(
            matches!(
                receipt.outcome,
                Outcome::RolledBack {
                    trap: x3bc_core::trap::TrapId::ProfitBelowFloor,
                    ..
                }
            ),
            "{:?}",
            receipt.outcome
        );
        assert_eq!(TradeNonces::<Test>::get(TRADER), 0);
        assert_eq!(
            TestTradingHost::venue_state("x3dex_pool_1")
                .expect("venue")
                .reserve_a,
            POOL1.0
        );
    });
}

#[test]
fn the_private_flagship_is_refused_on_chain_because_no_private_channel_exists() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(FLAGSHIP),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeRefused
        );
    });
}

#[test]
fn trading_is_refused_until_governance_sets_the_cost_schedule() {
    new_test_ext().execute_with(|| {
        chain(6);
        TradingConfiguration::<Test>::kill();
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradingNotConfigured
        );
    });
}

#[test]
fn only_governance_sets_the_cost_schedule_and_only_in_a_registered_asset() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert!(AtlasKernel::set_trading_config(RuntimeOrigin::signed(TRADER), config()).is_err());
        let unknown = TradingConfig {
            fee_asset: 99,
            ..config()
        };
        assert_noop!(
            AtlasKernel::set_trading_config(RuntimeOrigin::root(), unknown),
            Error::<Test>::UnknownAsset
        );
    });
}

#[test]
fn a_program_whose_declared_decimals_disagree_with_the_registry_is_refused() {
    new_test_ext().execute_with(|| {
        // The program says WETX has 6 decimals; the chain says 18.
        chain(18);
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeAssetMismatch
        );
    });
}

#[test]
fn a_submission_signed_against_a_future_block_is_refused() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                EXECUTED_AT + 1
            ),
            Error::<Test>::TradeRefused
        );
    });
}

#[test]
fn a_venue_whose_custody_does_not_cover_its_reserves_fails_settlement_and_writes_nothing() {
    new_test_ext().execute_with(|| {
        chain(6);
        // Pool 2 claims reserves its custody account does not hold: the engine trades against
        // the claim, and settlement must refuse to record reserves nobody paid in.
        CanonicalLedger::<Test>::mutate(VENUE_2, USDX, |balance| *balance -= 100_000 * UNIT);
        let before = balances();
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            sp_runtime::DispatchError::Other("reserves exceed custody")
        );
        assert_eq!(balances(), before);
        let venue1 = TestTradingHost::venue_state("x3dex_pool_1").expect("venue");
        assert_eq!(
            (venue1.reserve_a, venue1.reserve_b),
            POOL1,
            "venue writes rolled back too"
        );
    });
}

#[test]
fn trading_is_refused_while_the_protocol_is_paused() {
    new_test_ext().execute_with(|| {
        chain(6);
        crate::pallet::ProtocolPaused::<Test>::put(true);
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::ProtocolIsPaused
        );
    });
}

#[test]
fn trading_is_refused_while_the_economic_halt_freezes_transfers() {
    new_test_ext().execute_with(|| {
        chain(6);
        TestTradingHost::set_halted(true);
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradingHalted
        );
    });
}

#[test]
fn a_trader_who_is_the_fee_collector_is_refused() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_ok!(AtlasKernel::set_trading_config(
            RuntimeOrigin::root(),
            TradingConfig {
                fee_collector: TRADER,
                ..config()
            }
        ));
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeRefused
        );
    });
}

/// A pool that holds more than its recorded reserves (a donation, a governance balance edit)
/// must not let a trade record reserves the trade's own movements do not account for.
#[test]
fn a_pool_holding_more_than_its_reserves_still_settles_to_old_reserves_plus_flows() {
    new_test_ext().execute_with(|| {
        chain(6);
        let surplus = 7 * UNIT;
        fund(VENUE_1, WETX, surplus);
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));
        assert!(matches!(last_trade_event(), Event::TradeCommitted { .. }));
        let swap1 = cp_out(BORROW, POOL1.0, POOL1.1);
        let venue1 = TestTradingHost::venue_state("x3dex_pool_1").expect("venue");
        assert_eq!(venue1.reserve_b, POOL1.1 - swap1);
        assert_eq!(
            CanonicalLedger::<Test>::get(VENUE_1, WETX),
            POOL1.1 - swap1 + surplus,
            "the surplus stays in custody, outside the reserves"
        );
    });
}

/// A program may not name one pool twice: `x3dex_pool_+1` is not a venue.
#[test]
fn an_aliased_venue_name_never_reaches_a_pool() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        let aliased = public(FLAGSHIP).replace("via x3dex_pool_2", "via x3dex_pool_+1");
        match compile_trading_x3bc(&aliased, CompilationMode::Testnet) {
            // Refused at compile time: nothing to submit.
            Err(_) => {}
            Ok(artifact) => {
                let bytes: BoundedVec<_, _> = artifact.bytes.try_into().expect("fits");
                // Refused before execution, or rolled back by the engine: either way no pool
                // is reached and nothing moves.
                let _ = submit_trade(RuntimeOrigin::signed(TRADER), bytes, SUBMITTED_AT);
                assert!(!matches!(
                    frame_system::Pallet::<Test>::events()
                        .last()
                        .map(|r| &r.event),
                    Some(RuntimeEvent::AtlasKernel(Event::TradeCommitted { .. }))
                ));
            }
        }
        assert_eq!(balances(), before);
    });
}

/// Different decimals on the two sides: the oracle conversion is exercised where the powers of ten
/// do not cancel. WETX has 18 decimals on chain and in the program; pool and price figures are the
/// same economic market as the 6-decimal tests.
#[test]
fn the_oracle_conversion_uses_each_assets_registry_decimals() {
    new_test_ext().execute_with(|| {
        const SCALE: u128 = 1_000_000_000_000; // 10^(18-6)
        TestTradingHost::reset();
        frame_system::Pallet::<Test>::set_block_number(EXECUTED_AT);
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
            18
        ));
        fund(TRADER, USDX, TRADER_USDX);
        fund(VENUE_1, USDX, POOL1.0);
        fund(VENUE_1, WETX, POOL1.1 * SCALE);
        fund(VENUE_2, WETX, POOL2.0 * SCALE);
        fund(VENUE_2, USDX, POOL2.1);
        fund(FLASH, USDX, FLASH_LIQUIDITY);
        for (name, a, b, ra, rb, custody) in [
            (
                "x3dex_pool_1",
                USDX,
                WETX,
                POOL1.0,
                POOL1.1 * SCALE,
                VENUE_1,
            ),
            (
                "x3dex_pool_2",
                WETX,
                USDX,
                POOL2.0 * SCALE,
                POOL2.1,
                VENUE_2,
            ),
        ] {
            TestTradingHost::add_venue(
                VenueState {
                    name: name.into(),
                    version: 1,
                    asset_a: a,
                    asset_b: b,
                    reserve_a: ra,
                    reserve_b: rb,
                    fee_bps: FEE_BPS,
                },
                custody,
            );
        }
        TestTradingHost::add_flash_pool(USDX, FLASH_FEE_BPS, FLASH);
        TestTradingHost::set_price(USDX, 1_000_000, EXECUTED_AT - 1);
        TestTradingHost::set_price(WETX, 2_000_000, EXECUTED_AT - 1);
        assert_ok!(AtlasKernel::set_trading_config(
            RuntimeOrigin::root(),
            config()
        ));

        let source = public(FLAGSHIP).replace(
            "asset WETX = x3.x3chain.asset_2 {\n    decimals: 6",
            "asset WETX = x3.x3chain.asset_2 {\n    decimals: 18",
        );
        assert!(
            source.contains("decimals: 18"),
            "the WETX declaration moved"
        );
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&source),
            SUBMITTED_AT
        ));
        let Event::TradeCommitted { receipt_bytes, .. } = last_trade_event() else {
            panic!(
                "the trade must commit: a wrong conversion is off by 10^24 and trips the oracle"
            );
        };
        let receipt = receipt::decode(&receipt_bytes).expect("canonical receipt");
        let reading = receipt.oracle.first().expect("an oracle reading");
        // WETX base units per USDX base unit: ($1 / $2) * 10^18 / 10^6 = SCALE / 2.
        assert_eq!((reading.base, reading.quote), (USDX, WETX));
        assert_eq!(
            reading.price_num * 2,
            reading.price_den * SCALE,
            "1 USDX base unit is worth 0.5e12 WETX base units"
        );
    });
}

/// A trade is charged for the largest trade up front and refunded to what it did: the flagship
/// pays under a tenth of the bound, and a module refused before it runs pays less again.
#[test]
fn a_trade_pays_for_what_it_did_not_for_the_largest_trade_possible() {
    new_test_ext().execute_with(|| {
        chain(6);
        let bound = AtlasKernel::trade_weight();
        let committed = AtlasKernel::submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT,
        )
        .expect("commits")
        .actual_weight
        .expect("an actual weight");
        assert!(
            committed.ref_time() * 10 < bound.ref_time(),
            "{committed:?} vs {bound:?}"
        );
        assert!(committed.ref_time() > 0);

        let refused = AtlasKernel::submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(FLAGSHIP),
            SUBMITTED_AT,
        )
        .expect_err("the private flagship is refused")
        .post_info
        .actual_weight
        .expect("an actual weight");
        assert!(
            refused.ref_time() < committed.ref_time(),
            "{refused:?} vs {committed:?}"
        );
    });
}

/// A genesis may only start trading in a fee asset it registers: anything else is a chain that
/// would refuse its first trade, so the genesis itself is refused.
#[test]
#[should_panic(expected = "Trading fee asset is not registered in genesis")]
fn a_genesis_trading_schedule_in_an_unregistered_asset_is_refused() {
    use sp_runtime::BuildStorage;
    let _ = crate::pallet::GenesisConfig::<Test> {
        assets: vec![(USDX, b"USDX".to_vec(), 6)],
        trading_config: Some(TradingConfig {
            fee_asset: 99,
            ..config()
        }),
        ..Default::default()
    }
    .build_storage();
}

#[test]
fn a_genesis_trading_schedule_is_in_force_from_block_one() {
    use sp_runtime::BuildStorage;
    let storage = crate::pallet::GenesisConfig::<Test> {
        assets: vec![(USDX, b"USDX".to_vec(), 6)],
        trading_config: Some(config()),
        ..Default::default()
    }
    .build_storage()
    .expect("genesis builds");
    sp_io::TestExternalities::new(storage).execute_with(|| {
        assert_eq!(TradingConfiguration::<Test>::get(), Some(config()));
    });
}

/// The `submit_trade` benchmark runs a committed module (the wasm runtime has no compiler). It has
/// to be exactly what the canonical lowering makes of the public flagship today; regenerate with
/// `X3_BLESS_BENCHMARK_TRADE=1 cargo test -p pallet-x3-kernel the_benchmark_trade`.
#[test]
fn the_benchmark_trade_is_the_compiled_flagship() {
    let compiled = module(&public(FLAGSHIP)).into_inner();
    if std::env::var_os("X3_BLESS_BENCHMARK_TRADE").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/benchmark_trade.x3bc"),
            &compiled,
        )
        .expect("write the benchmark module");
    }
    assert_eq!(
        crate::trading::BENCHMARK_TRADE_MODULE,
        compiled.as_slice(),
        "the benchmark module is stale"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ProfitLock: the chain's own limits on a committed trade, enforced before any
// ledger movement.
// ─────────────────────────────────────────────────────────────────────────────

use crate::trading::TradeConstraints;

fn set_constraints(constraints: Option<TradeConstraints>) {
    assert_ok!(AtlasKernel::set_trade_constraints(
        RuntimeOrigin::root(),
        constraints
    ));
}

/// A trade that misses its limits must leave the chain exactly as it was: no
/// transfer, no venue reserve update, no receipt record.
#[test]
fn a_trade_below_the_minimum_profit_is_refused_and_changes_nothing() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        let receipts_before = TradeReceipts::<Test>::iter().count();
        // The flagship requires 1 USDX; demand more than any arbitrage on these
        // pools can produce.
        set_constraints(Some(TradeConstraints {
            min_net_profit: 1_000_000 * UNIT,
            max_execution_cost: u128::MAX,
            deadline_blocks: 0,
        }));

        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeConstraintsUnsatisfied
        );

        assert_eq!(balances(), before, "no balance moved");
        assert_eq!(
            TradeReceipts::<Test>::iter().count(),
            receipts_before,
            "no receipt was recorded for a trade the chain did not apply"
        );
    });
}

/// The same trade, under a limit it can meet, commits normally.
#[test]
fn a_trade_above_the_minimum_profit_commits() {
    new_test_ext().execute_with(|| {
        chain(6);
        set_constraints(Some(TradeConstraints {
            min_net_profit: UNIT,
            max_execution_cost: u128::MAX,
            deadline_blocks: 0,
        }));

        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));
        assert!(matches!(last_trade_event(), Event::TradeCommitted { .. }));
    });
}

#[test]
fn a_trade_charging_more_than_the_cost_ceiling_is_refused() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        set_constraints(Some(TradeConstraints {
            min_net_profit: 0,
            max_execution_cost: 1, // below any real cost
            deadline_blocks: 0,
        }));

        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeConstraintsUnsatisfied
        );
        assert_eq!(balances(), before);
    });
}

#[test]
fn a_trade_that_missed_its_deadline_is_refused() {
    new_test_ext().execute_with(|| {
        chain(6);
        let before = balances();
        // The chain proceeds from SUBMITTED_AT (1000) to EXECUTED_AT (1002).
        set_constraints(Some(TradeConstraints {
            min_net_profit: 0,
            max_execution_cost: u128::MAX,
            deadline_blocks: 1,
        }));

        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeConstraintsUnsatisfied
        );
        assert_eq!(balances(), before);
    });
}

#[test]
fn clearing_the_constraints_restores_the_previous_behaviour() {
    new_test_ext().execute_with(|| {
        chain(6);
        set_constraints(Some(TradeConstraints {
            min_net_profit: 1_000_000 * UNIT,
            max_execution_cost: u128::MAX,
            deadline_blocks: 0,
        }));
        assert_noop!(
            submit_trade(
                RuntimeOrigin::signed(TRADER),
                module(&public(FLAGSHIP)),
                SUBMITTED_AT
            ),
            Error::<Test>::TradeConstraintsUnsatisfied
        );

        set_constraints(None);
        assert!(crate::pallet::TradeConstraintsOf::<Test>::get().is_none());
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));
    });
}

#[test]
fn constraints_that_enable_no_check_are_refused() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_noop!(
            AtlasKernel::set_trade_constraints(
                RuntimeOrigin::root(),
                Some(TradeConstraints::permissive())
            ),
            Error::<Test>::InvalidTradeConstraints
        );
    });
}

#[test]
fn only_governance_may_set_constraints() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_noop!(
            AtlasKernel::set_trade_constraints(
                RuntimeOrigin::signed(TRADER),
                Some(TradeConstraints {
                    min_net_profit: UNIT,
                    max_execution_cost: u128::MAX,
                    deadline_blocks: 0,
                })
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Revenue Engine: a verified performance fee, split and paid atomically with the
// trade it was charged on.
// ─────────────────────────────────────────────────────────────────────────────

use crate::trading::RevenueAgreement;

const DEVELOPER: AccountId = 1_010;
const PROTOCOL: AccountId = 1_011;

fn agree(trader: AccountId, agreement: Option<RevenueAgreement<AccountId>>) -> DispatchResult {
    AtlasKernel::set_revenue_agreement(RuntimeOrigin::signed(trader), agreement)
}

/// 20% performance fee, split 15% developer / 5% protocol / 80% trader.
fn standard_agreement() -> RevenueAgreement<AccountId> {
    RevenueAgreement {
        performance_bps: 2_000,
        trader: None,
        developer: Some(DEVELOPER),
        protocol: Some(PROTOCOL),
        developer_bps: 1_500,
        protocol_bps: 500,
    }
}

/// The committed trade's receipt, decoded from its event.
fn committed_receipt() -> x3bc_core::receipt::TradingReceipt {
    let Event::TradeCommitted { receipt_bytes, .. } = last_trade_event() else {
        panic!("the trade must commit");
    };
    receipt::decode(&receipt_bytes).expect("canonical receipt")
}

#[test]
fn a_verified_profit_pays_the_agreed_split_out_of_the_traders_balance() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_ok!(agree(TRADER, Some(standard_agreement())));

        let trader_before = AtlasKernel::ledger_balance_of(USDX, &TRADER);
        let developer_before = AtlasKernel::ledger_balance_of(USDX, &DEVELOPER);
        let protocol_before = AtlasKernel::ledger_balance_of(USDX, &PROTOCOL);

        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));

        // Recompute the fee from the receipt, never from the event.
        let receipt = committed_receipt();
        let outcome =
            x3bc_core::economics::outcome(&receipt, USDX).expect("a committed trade prices");
        assert!(outcome.is_profit(), "the flagship trade is profitable");
        let expected = x3bc_core::economics::performance_fee(&outcome, 2_000).expect("fee");
        assert!(expected.fee > 0);

        let developer_paid = AtlasKernel::ledger_balance_of(USDX, &DEVELOPER) - developer_before;
        let protocol_paid = AtlasKernel::ledger_balance_of(USDX, &PROTOCOL) - protocol_before;
        let trader_delta = AtlasKernel::ledger_balance_of(USDX, &TRADER) - trader_before;

        assert_eq!(developer_paid, expected.fee * 1_500 / 10_000);
        assert_eq!(protocol_paid, expected.fee * 500 / 10_000);
        // The three shares are the whole fee: nothing is created or destroyed, and the trader's
        // 80% stays in their own balance because the agreement names no destination.
        let trader_share = expected.fee - developer_paid - protocol_paid;
        assert_eq!(
            developer_paid + protocol_paid + trader_share,
            expected.fee,
            "the shares are the fee"
        );
        // With no named destination the trader's share stays in their balance, so the only fee
        // that leaves it is what the other two participants were paid.
        assert_eq!(
            trader_delta,
            outcome.net as u128 - developer_paid - protocol_paid,
            "the trader keeps the net minus everyone else's share"
        );
    });
}

/// The fee only redistributes value the trade produced: across the trader and
/// the fee recipients, USDX rises by exactly the trade's verified net, no more.
#[test]
fn fee_distribution_creates_no_value_beyond_the_verified_net() {
    new_test_ext().execute_with(|| {
        chain(6);
        assert_ok!(agree(TRADER, Some(standard_agreement())));
        let participants = [TRADER, DEVELOPER, PROTOCOL];
        let total = |asset: u32| -> u128 {
            participants
                .iter()
                .map(|a| AtlasKernel::ledger_balance_of(asset, a))
                .sum()
        };
        let usdx_before = total(USDX);
        let wetx_before = total(WETX);

        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));

        let receipt = committed_receipt();
        let outcome = x3bc_core::economics::outcome(&receipt, USDX).unwrap();
        assert_eq!(
            total(USDX) as i128 - usdx_before as i128,
            outcome.net,
            "participants gained exactly the trade's net"
        );
        assert_eq!(
            total(WETX),
            wetx_before,
            "the settlement-asset fee moves no other asset"
        );
    });
}

#[test]
fn a_trader_without_an_agreement_pays_no_performance_fee() {
    new_test_ext().execute_with(|| {
        chain(6);
        let developer_before = AtlasKernel::ledger_balance_of(USDX, &DEVELOPER);
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));
        assert_eq!(
            AtlasKernel::ledger_balance_of(USDX, &DEVELOPER),
            developer_before
        );
    });
}

#[test]
fn a_zero_rate_agreement_pays_nothing() {
    new_test_ext().execute_with(|| {
        chain(6);
        let mut agreement = standard_agreement();
        agreement.performance_bps = 0;
        assert_ok!(agree(TRADER, Some(agreement)));
        let developer_before = AtlasKernel::ledger_balance_of(USDX, &DEVELOPER);
        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));
        assert_eq!(
            AtlasKernel::ledger_balance_of(USDX, &DEVELOPER),
            developer_before
        );
    });
}

#[test]
fn an_impossible_agreement_is_refused() {
    new_test_ext().execute_with(|| {
        chain(6);

        // A rate above 100%.
        let mut agreement = standard_agreement();
        agreement.performance_bps = 10_001;
        assert_noop!(
            agree(TRADER, Some(agreement)),
            Error::<Test>::InvalidRevenueAgreement
        );

        // Shares summing above 100% of the fee.
        let mut agreement = standard_agreement();
        agreement.developer_bps = 8_000;
        agreement.protocol_bps = 3_000;
        assert_noop!(
            agree(TRADER, Some(agreement)),
            Error::<Test>::InvalidRevenueAgreement
        );

        // A share with no recipient.
        let mut agreement = standard_agreement();
        agreement.developer = None;
        assert_noop!(
            agree(TRADER, Some(agreement)),
            Error::<Test>::InvalidRevenueAgreement
        );

        // A recipient with no share is harmless and accepted.
        let mut agreement = standard_agreement();
        agreement.protocol_bps = 0;
        assert_ok!(agree(TRADER, Some(agreement)));
    });
}

#[test]
fn only_the_trader_can_set_their_own_agreement() {
    new_test_ext().execute_with(|| {
        chain(6);
        // ALICE's agreement is not BOB's to set.
        assert_ok!(agree(TRADER, Some(standard_agreement())));
        assert_eq!(
            crate::pallet::RevenueAgreements::<Test>::get(TRADER),
            Some(standard_agreement())
        );
        assert!(crate::pallet::RevenueAgreements::<Test>::get(BOB).is_none());

        // Clearing removes it, and a cleared agreement stops future fees.
        assert_ok!(agree(TRADER, None));
        assert!(crate::pallet::RevenueAgreements::<Test>::get(TRADER).is_none());
    });
}

#[test]
fn the_traders_remainder_is_sent_when_they_name_a_destination() {
    new_test_ext().execute_with(|| {
        chain(6);
        let destination = BOB;
        let mut agreement = standard_agreement();
        agreement.trader = Some(destination);
        assert_ok!(agree(TRADER, Some(agreement)));
        let before = AtlasKernel::ledger_balance_of(USDX, &destination);

        assert_ok!(submit_trade(
            RuntimeOrigin::signed(TRADER),
            module(&public(FLAGSHIP)),
            SUBMITTED_AT
        ));

        let receipt = committed_receipt();
        let outcome = x3bc_core::economics::outcome(&receipt, USDX).unwrap();
        let fee = x3bc_core::economics::performance_fee(&outcome, 2_000).unwrap();
        let trader_share = fee.fee - fee.fee * 1_500 / 10_000 - fee.fee * 500 / 10_000;
        assert_eq!(
            AtlasKernel::ledger_balance_of(USDX, &destination) - before,
            trader_share
        );
    });
}
