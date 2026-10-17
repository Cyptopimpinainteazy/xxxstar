//! Differential suite: the production engine (`crates/x3-vm`) against the nested reference oracle
//! (`x3-lang/vm` `TradingVm`), on the same source and the same seeded venues.
//!
//! The nested VM stays as a reference oracle until this suite shows parity
//! (docs/roadmap/X3LANG_PRODUCTION_UNIFICATION.md §0). Parity is defined on the trade itself —
//! debt principal and fee, each leg's output, and the per-asset delta before the chain's own
//! charges (gas, settlement, relay, protocol, oracle), which the reference VM has no notion of.

#![cfg(all(feature = "std", feature = "compile"))]

mod trading_support;

use std::collections::BTreeMap;

use trading_support::*;
use x3_lang_compiler::ir::{AssetKey, TradingOperation};
use x3_lang_compiler::semantic::CompilationMode;
use x3_lang_vm::trading::{
    BorrowRequest, BorrowResult, CapabilityManifest, CapabilityMode, CommittedCost, ExecutionMode,
    HostError, PriceSource, QuoteRequest, QuoteResult, RepayRequest, RepayResult, SwapRequest,
    SwapResult, TradeExecutionContext, TradingHost, TradingVm,
};
use x3_vm::trading::TradingPreState;
use x3_x3_integration::trading::execute_trade;
use x3bc_core::receipt::CostKind;

const GAS_LIMIT: u64 = 1_000_000;

/// Test-only reference host over the same seeded state: the same pools, the same formula.
struct OracleHost {
    manifest: CapabilityManifest,
    pre: TradingPreState,
    reserves: BTreeMap<String, (u128, u128)>,
    debts: BTreeMap<String, (u128, u128)>,
}

fn asset_id(key: &AssetKey) -> u32 {
    key.canonical_id
        .trim_start_matches("asset_")
        .parse()
        .expect("x3 asset id")
}

impl OracleHost {
    fn new(pre: TradingPreState) -> Self {
        let reserves = pre
            .venues
            .iter()
            .map(|venue| (venue.name.clone(), (venue.reserve_a, venue.reserve_b)))
            .collect();
        let manifest = CapabilityManifest {
            mode: CapabilityMode::Fixture,
            version: "trading-policy-v1".into(),
            chain: "x3chain".into(),
            state_commitment: [0u8; 32],
            private_submission: true,
            providers: pre
                .flash_pools
                .iter()
                .map(|pool| pool.provider.clone())
                .collect(),
            venues: pre.venues.iter().map(|venue| venue.name.clone()).collect(),
            bridges: Default::default(),
        };
        Self {
            manifest,
            pre,
            reserves,
            debts: BTreeMap::new(),
        }
    }

    fn out(&self, venue: &str, from: u32, input: u128) -> u128 {
        let state = self
            .pre
            .venues
            .iter()
            .find(|v| v.name == venue)
            .expect("venue");
        let (a, b) = self.reserves[venue];
        let (r_in, r_out) = if from == state.asset_a {
            (a, b)
        } else {
            (b, a)
        };
        cp_out(input, r_in, r_out, state.fee_bps).0
    }
}

impl TradingHost for OracleHost {
    fn capabilities(&self) -> &CapabilityManifest {
        &self.manifest
    }
    fn open_debt(&mut self, request: BorrowRequest) -> Result<BorrowResult, HostError> {
        let pool = &self.pre.flash_pools[0];
        let fee = request.principal * u128::from(pool.fee_bps) / 10_000;
        self.debts
            .insert(request.provider.clone(), (request.principal, fee));
        Ok(BorrowResult {
            asset: request.asset,
            principal: request.principal,
            fee,
            state_commitment: [0u8; 32],
        })
    }
    fn quote(&self, request: QuoteRequest) -> Result<QuoteResult, HostError> {
        let expected_output = self.out(&request.venue, asset_id(&request.from), request.input);
        Ok(QuoteResult {
            expected_output,
            sources: vec![PriceSource {
                name: "x3_oracle".into(),
                expected_output,
            }],
            quote_block: EXECUTED_AT,
        })
    }
    fn swap(&mut self, request: SwapRequest) -> Result<SwapResult, HostError> {
        let from = asset_id(&request.from);
        let output = self.out(&request.venue, from, request.input);
        let state = self
            .pre
            .venues
            .iter()
            .find(|v| v.name == request.venue)
            .expect("venue");
        let reserves = self.reserves.get_mut(&request.venue).expect("venue");
        if from == state.asset_a {
            *reserves = (reserves.0 + request.input, reserves.1 - output);
        } else {
            *reserves = (reserves.0 - output, reserves.1 + request.input);
        }
        Ok(SwapResult {
            from: request.from.clone(),
            to: request.to,
            input: request.input,
            output,
            // The venue fee is inside the constant-product output, as on chain.
            fee: 0,
            fee_asset: request.from,
            state_commitment: [0u8; 32],
        })
    }
    fn close_debt(&mut self, request: RepayRequest) -> Result<RepayResult, HostError> {
        let (principal, fee) = self.debts.values().next().copied().expect("open debt");
        Ok(RepayResult {
            debt_id: request.debt_id,
            asset: request.asset,
            amount_paid: principal + fee,
            fee,
            state_commitment: [0u8; 32],
        })
    }
    fn execution_costs(&self) -> Result<Vec<CommittedCost>, HostError> {
        Ok(Vec::new())
    }
}

fn reference_ops(source: &str) -> Vec<TradingOperation> {
    let bytes = x3_lang_compiler::compile_with_mode(source, CompilationMode::Testnet)
        .expect("reference compile");
    x3_lang_compiler::emitter::decode_trading_program(&bytes).expect("reference decode")
}

fn reference_usdx_delta(pre: &TradingPreState, source: &str) -> i128 {
    let mut host = OracleHost::new(pre.clone());
    let mut vm = TradingVm::new();
    let execution = vm
        .execute_atomic(
            &reference_ops(source),
            &mut host,
            TradeExecutionContext {
                mode: ExecutionMode::Development,
                current_block: EXECUTED_AT,
            },
        )
        .expect("the reference oracle commits the trade");
    let usdx = execution
        .committed_state
        .net_deltas
        .iter()
        .find(|(asset, _)| asset.canonical_id == "asset_1")
        .map(|(_, delta)| *delta)
        .expect("USDX moved");
    usdx
}

fn production_delta_before_chain_charges(pre: &TradingPreState, source: &str) -> i128 {
    let artifact = compile(source);
    let run = execute_trade(&artifact.bytes, pre, &anchor(), GAS_LIMIT).expect("runs");
    assert!(run.committed, "{:?}", run.receipt.outcome);
    let chain_charges: u128 = run
        .receipt
        .costs
        .iter()
        .filter(|cost| {
            matches!(
                cost.kind,
                CostKind::Gas
                    | CostKind::SettlementFee
                    | CostKind::RelayFee
                    | CostKind::ProtocolFee
                    | CostKind::OracleFee
            )
        })
        .map(|cost| cost.amount)
        .sum();
    run.receipt.profit.net + chain_charges as i128
}

#[test]
fn production_and_reference_agree_on_the_flagship_trade() {
    let pre = pre_state();
    assert_eq!(
        production_delta_before_chain_charges(&pre, ARB_SOURCE),
        reference_usdx_delta(&pre, ARB_SOURCE)
    );
}

#[test]
fn production_and_reference_agree_across_pool_depths_and_fees() {
    for (depth, fee_bps, flash_bps) in [(1u128, 30u16, 5u16), (2, 5, 1), (3, 100, 9), (5, 1, 0)] {
        let mut pre = pre_state();
        for venue in &mut pre.venues {
            venue.reserve_a *= depth;
            venue.reserve_b *= depth;
            venue.fee_bps = fee_bps;
        }
        pre.flash_pools[0].fee_bps = flash_bps;
        // Only cases the policy admits are compared; refusals are covered by the negative suite.
        let artifact = compile(ARB_SOURCE);
        let run = execute_trade(&artifact.bytes, &pre, &anchor(), GAS_LIMIT).expect("runs");
        if !run.committed {
            continue;
        }
        assert_eq!(
            production_delta_before_chain_charges(&pre, ARB_SOURCE),
            reference_usdx_delta(&pre, ARB_SOURCE),
            "depth {depth}, fee {fee_bps}, flash {flash_bps}"
        );
    }
}
