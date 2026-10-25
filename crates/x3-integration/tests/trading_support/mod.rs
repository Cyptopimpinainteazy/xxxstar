//! Shared fixtures for the X3 Lang production trading suites.
//!
//! Everything here is **input** to the production path: the `.x3` source, the chain state a block
//! would seed the trade with, and an attestor key. Nothing here computes an expected result by
//! calling the code under test — `expected_*` helpers re-derive figures from the constant-product
//! formula and the cost schedule independently, so a test that compares against them is comparing
//! the engine with arithmetic, not with itself.

#![allow(dead_code)]

use std::collections::BTreeMap;

use x3_lang_compiler::semantic::CompilationMode;
use x3_lang_compiler::x3bc::{compile_trading_x3bc, X3bcArtifact};
use x3_vm::trading::{
    ChainAnchor, CostSchedule, Finality, FlashPoolState, OracleObservation, TradingPreState,
    VenueState,
};
use x3_x3_integration::trading::{AttestorKey, AttestorRegistry};

/// The flagship program: flash borrow → pool 1 → pool 2 → repay → profit floor.
pub const ARB_SOURCE: &str = include_str!("../../../../x3-lang/examples/x3chain_flash_arb.x3");

/// The X3 chain's id (`runtime/src/lib.rs` `ChainId`).
pub const X3_CHAIN_ID: u64 = 650_000;
pub const USDX: u32 = 1;
pub const WETX: u32 = 2;
/// 1 USDX / 1 WETX in base units (6 decimals).
pub const UNIT: u128 = 1_000_000;

pub const TRADER_USDX: u128 = 50 * UNIT;
pub const BORROW: u128 = 1_000 * UNIT;
pub const FLASH_FEE_BPS: u16 = 5;
pub const POOL1: (u128, u128, u16) = (1_000_000 * UNIT, 500_000 * UNIT, 30); // USDX, WETX
pub const POOL2: (u128, u128, u16) = (400_000 * UNIT, 830_000 * UNIT, 30); // WETX, USDX

pub const GAS_PRICE: u128 = 10; // USDX base units per gas unit
pub const SETTLEMENT_FEE: u128 = 200_000;
pub const RELAY_FEE: u128 = 300_000;
pub const PROTOCOL_FEE: u128 = 100_000;
pub const ORACLE_FEE: u128 = 10_000;

pub const SUBMITTED_AT: u64 = 1_000;
pub const EXECUTED_AT: u64 = 1_002;

pub fn compile(source: &str) -> X3bcArtifact {
    compile_trading_x3bc(source, CompilationMode::Testnet)
        .unwrap_or_else(|error| panic!("the trading source must compile to X3BC: {error:?}"))
}

pub fn compile_err(source: &str) -> String {
    match compile_trading_x3bc(source, CompilationMode::Testnet) {
        Ok(_) => panic!("the source must be refused at compile time"),
        Err(error) => format!("{error:?}"),
    }
}

/// The chain state a block seeds the trade with: the trader's balances, the flash pool, both dex
/// pools, oracle observations, the cost schedule and the capability versions the runtime offers.
pub fn pre_state() -> TradingPreState {
    let mut trader_balances = BTreeMap::new();
    trader_balances.insert(USDX, TRADER_USDX);
    TradingPreState {
        chain_id: X3_CHAIN_ID,
        block_number: EXECUTED_AT,
        submitted_at_block: SUBMITTED_AT,
        trader: [7u8; 32],
        trader_balances,
        flash_pools: vec![FlashPoolState {
            provider: "x3_flashloan".into(),
            version: 1,
            asset: USDX,
            liquidity: 5_000_000 * UNIT,
            fee_bps: FLASH_FEE_BPS,
        }],
        venues: vec![
            VenueState {
                name: "x3dex_pool_1".into(),
                version: 1,
                asset_a: USDX,
                asset_b: WETX,
                reserve_a: POOL1.0,
                reserve_b: POOL1.1,
                fee_bps: POOL1.2,
            },
            VenueState {
                name: "x3dex_pool_2".into(),
                version: 1,
                asset_a: WETX,
                asset_b: USDX,
                reserve_a: POOL2.0,
                reserve_b: POOL2.1,
                fee_bps: POOL2.2,
            },
        ],
        oracle: vec![OracleObservation {
            base: WETX,
            quote: USDX,
            // 2 USDX per WETX.
            price_num: 2,
            price_den: 1,
            observed_at_block: EXECUTED_AT - 1,
            source: "x3_oracle".into(),
        }],
        costs: CostSchedule {
            fee_asset: USDX,
            gas_price: GAS_PRICE,
            settlement_fee: SETTLEMENT_FEE,
            relay_fee: RELAY_FEE,
            protocol_fee: PROTOCOL_FEE,
            oracle_fee: ORACLE_FEE,
            solver_fee: 0,
            cross_domain_fee: 0,
        },
        private_channel: true,
    }
}

/// Where the trade ran. The execution-time half is known inside the block; `finality` is attached by
/// the attestor from the finalized header (GRANDPA) before it signs.
pub fn anchor() -> ChainAnchor {
    ChainAnchor {
        spec_version: 23,
        transaction_version: 1,
        block_number: EXECUTED_AT,
        parent_hash: [0xB0; 32],
        extrinsic_index: 3,
        finality: Some(Finality {
            block_number: EXECUTED_AT,
            block_hash: [0xB1; 32],
            state_root: [0x52; 32],
            parent_state_root: [0x51; 32],
        }),
    }
}

pub fn attestor() -> AttestorKey {
    AttestorKey::from_seed("x3-attestor-1", [42u8; 32])
}

pub fn registry() -> AttestorRegistry {
    let mut registry = AttestorRegistry::new();
    registry.trust(&attestor());
    registry
}

/// `in_after_fee * reserve_out / (reserve_in + in_after_fee)` with the fee taken from the input.
pub fn cp_out(amount_in: u128, reserve_in: u128, reserve_out: u128, fee_bps: u16) -> (u128, u128) {
    let fee = amount_in * u128::from(fee_bps) / 10_000;
    let net = amount_in - fee;
    let out = (primitive_types_mul_div(net, reserve_out, reserve_in + net)).expect("no overflow");
    (out, fee)
}

fn primitive_types_mul_div(a: u128, b: u128, d: u128) -> Option<u128> {
    // Exact for the magnitudes these fixtures use (a * b < 2^128).
    a.checked_mul(b).map(|product| product / d)
}

/// The figures the flagship trade must produce, from the formula and the schedule alone.
pub struct ExpectedArb {
    pub swap1_out: u128,
    pub swap1_fee: u128,
    pub swap2_out: u128,
    pub swap2_fee: u128,
    pub flash_fee: u128,
    /// Every non-gas cost the chain charges in USDX.
    pub fixed_costs: u128,
}

pub fn expected_arb() -> ExpectedArb {
    let (swap1_out, swap1_fee) = cp_out(BORROW, POOL1.0, POOL1.1, POOL1.2);
    let (swap2_out, swap2_fee) = cp_out(swap1_out, POOL2.0, POOL2.1, POOL2.2);
    let flash_fee = BORROW * u128::from(FLASH_FEE_BPS) / 10_000;
    // One oracle read per swap (two swaps, each priced against the oracle).
    let fixed_costs = SETTLEMENT_FEE + RELAY_FEE + PROTOCOL_FEE + 2 * ORACLE_FEE;
    ExpectedArb {
        swap1_out,
        swap1_fee,
        swap2_out,
        swap2_fee,
        flash_fee,
        fixed_costs,
    }
}
