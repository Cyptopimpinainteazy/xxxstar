//! Unit tests for `x3-accounting-events`.
//!
//! Coverage:
//! 1. SCALE codec roundtrip for a fully-populated `AccountingEvent<u32, u128>`.
//! 2. `RevenueModule::Other` roundtrip (the forward-compat variant).
//! 3. `fee_collected` builder — correct `kind` and `is_cross_chain = false`.
//! 4. `cross_chain_fee` builder — `is_cross_chain = true`, chains differ.
//! 5. `NoOpSpine::emit` compiles and runs without panicking.

use super::*;
use codec::{Decode, Encode};

// ─── Helper: a deterministic receipt id ──────────────────────────────────────

fn receipt(seed: u8) -> [u8; 32] {
    [seed; 32]
}

// ─── 1. Full AccountingEvent codec roundtrip ─────────────────────────────────

#[test]
fn accounting_event_codec_roundtrip() {
    let event: AccountingEvent<u32, u128> = AccountingEvent {
        module: RevenueModule::Swap,
        kind: AccountingEventKind::FeeCollected,
        source_chain: 0,
        dest_chain: 0,
        asset_id: 1,
        principal: 1_000_000_000_u128,
        fee: 3_000_u128,
        fee_destination: FeeDestination::Split,
        splits: FeeSplits {
            entries: sp_std::vec![
                FeeSplitEntry {
                    destination: FeeDestination::ProtocolTreasury,
                    amount: 2_000_u128,
                    basis_points: 6_667,
                },
                FeeSplitEntry {
                    destination: FeeDestination::LiquidityProviders,
                    amount: 1_000_u128,
                    basis_points: 3_333,
                },
            ],
        },
        receipt_id: receipt(0xAB),
        block: 42_u32,
        is_cross_chain: false,
    };

    let encoded = event.encode();
    let decoded =
        AccountingEvent::<u32, u128>::decode(&mut &encoded[..]).expect("decode must succeed");
    assert_eq!(event, decoded, "roundtrip equality failed");
}

// ─── 2. RevenueModule::Other roundtrip ───────────────────────────────────────

#[test]
fn revenue_module_other_roundtrip() {
    // Simulate blake2_256("x3-future-service") — just a byte pattern here.
    let hash: [u8; 32] = [
        0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
        0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1A, 0x1B,
    ];
    let module = RevenueModule::Other(hash);
    let encoded = module.encode();
    let decoded = RevenueModule::decode(&mut &encoded[..]).expect("decode must succeed");
    assert_eq!(module, decoded);
    // Confirm the variant discriminant is not one of the named variants.
    assert!(matches!(decoded, RevenueModule::Other(_)));
}

// ─── 3. fee_collected builder ────────────────────────────────────────────────

#[test]
fn fee_collected_builder_correct_fields() {
    let event = AccountingEvent::<u32, u128>::fee_collected(
        RevenueModule::Lending,
        /* source_chain */ 0,
        /* asset_id */ 5,
        /* principal */ 500_000_u128,
        /* fee */ 250_u128,
        FeeDestination::ProtocolTreasury,
        receipt(0x01),
        /* block */ 100_u32,
    );

    assert_eq!(event.kind, AccountingEventKind::FeeCollected, "kind must be FeeCollected");
    assert!(!event.is_cross_chain, "local fee_collected must not be cross-chain");
    assert_eq!(event.dest_chain, event.source_chain, "dest_chain must equal source_chain");
    assert!(
        event.splits.entries.is_empty(),
        "splits must be empty from the builder"
    );
    assert_eq!(event.module, RevenueModule::Lending);
    assert_eq!(event.principal, 500_000_u128);
    assert_eq!(event.fee, 250_u128);
    assert_eq!(event.block, 100_u32);
}

// ─── 4. cross_chain_fee builder ──────────────────────────────────────────────

#[test]
fn cross_chain_fee_builder_correct_fields() {
    let src = 0_u32;   // native X3
    let dst = 1_u32;   // external chain (CAIP-2 numeric 1 = Ethereum mainnet)

    let event = AccountingEvent::<u32, u128>::cross_chain_fee(
        RevenueModule::Bridge,
        src,
        dst,
        /* asset_id */ 2,
        /* principal */ 10_000_u128,
        /* fee */ 50_u128,
        receipt(0x02),
        /* block */ 200_u32,
    );

    assert!(event.is_cross_chain, "cross_chain_fee must set is_cross_chain = true");
    assert_ne!(
        event.source_chain, event.dest_chain,
        "source_chain and dest_chain must differ for cross-chain events"
    );
    assert_eq!(event.source_chain, src);
    assert_eq!(event.dest_chain, dst);
    assert_eq!(
        event.kind,
        AccountingEventKind::CrossChainFeeSettled,
        "kind must be CrossChainFeeSettled"
    );
    assert_eq!(
        event.fee_destination,
        FeeDestination::Split,
        "cross-chain fees default to Split destination"
    );
}

// ─── 5. NoOpSpine compiles and runs without panic ────────────────────────────

#[test]
fn no_op_spine_emit_does_not_panic() {
    let event = AccountingEvent::<u32, u128>::fee_collected(
        RevenueModule::RpcMonetization,
        0,
        7,
        1_000_u128,
        5_u128,
        FeeDestination::ProtocolTreasury,
        receipt(0xFF),
        999_u32,
    );
    // This must compile (proves the trait bound is satisfiable) and must not
    // panic at runtime.
    NoOpSpine::emit(event);
}
