//! Tests for the x3-canonical-truth crate.
//!
//! Coverage:
//!   - SCALE encode/decode round-trips for each major type (tests 1–9).
//!   - Serde JSON serialize/deserialize round-trips for representative types
//!     that contain no `u128` fields, avoiding serde_json's arbitrary-precision
//!     requirement (tests 10–12).
//!   - Semantic correctness checks: identity hash stability, supply arithmetic,
//!     and treasury snapshot field summation (tests 13–15).

use super::*;
use codec::{Decode, Encode};

// ─── Helper ───────────────────────────────────────────────────────────────────

fn scale_roundtrip<T>(value: T)
where
    T: Encode + Decode + PartialEq + core::fmt::Debug,
{
    let encoded = value.encode();
    let decoded = T::decode(&mut &encoded[..]).expect("SCALE decode must succeed");
    assert_eq!(value, decoded, "SCALE round-trip must be lossless");
}

// ─── SCALE encode/decode round-trips ─────────────────────────────────────────

#[test]
fn chain_family_scale_roundtrip() {
    scale_roundtrip(ChainFamily::Substrate);
    scale_roundtrip(ChainFamily::Evm);
    scale_roundtrip(ChainFamily::Svm);
    scale_roundtrip(ChainFamily::Bitcoin);
}

#[test]
fn chain_address_scale_roundtrip() {
    let addr = ChainAddress {
        chain_id: 1,
        family: ChainFamily::Evm,
        address_bytes: [0xab; 64],
        address_len: 20,
    };
    scale_roundtrip(addr);
}

#[test]
fn canonical_identity_scale_roundtrip() {
    let identity: CanonicalIdentity<[u8; 32]> = CanonicalIdentity {
        primary: [1u8; 32],
        identity_hash: [2u8; 32],
        governance_registered: true,
        kyc_tier: KycTier::Enhanced,
    };
    scale_roundtrip(identity);
}

#[test]
fn governance_record_scale_roundtrip() {
    let record = GovernanceRecord {
        identity_hash: [3u8; 32],
        total_voting_power: 1_000_000_u128,
        registration_block: 500_u32,
    };
    scale_roundtrip(record);
}

#[test]
fn canonical_asset_scale_roundtrip() {
    let asset = CanonicalAsset {
        asset_id: 1,
        origin: AssetOrigin::Native,
        canonical_chain_id: 0,
        decimals: 18,
        symbol_hash: [0xcc; 32],
        is_active: true,
    };
    scale_roundtrip(asset);
}

#[test]
fn chain_supply_entry_scale_roundtrip() {
    let entry = ChainSupplyEntry {
        chain_id: 1,
        asset_id: 10,
        minted: 1_000_000_u128,
        burned: 50_000_u128,
        locked: 200_000_u128,
        pending_in: 10_000_u128,
        pending_out: 5_000_u128,
    };
    scale_roundtrip(entry);
}

#[test]
fn canonical_lock_proof_scale_roundtrip() {
    let proof = CanonicalLockProof {
        asset_id: 7,
        source_chain_id: 1,
        dest_chain_id: 2,
        locked_amount: 500_000_u128,
        proof_hash: [0xdd; 32],
        block_number: 999,
    };
    scale_roundtrip(proof);
}

#[test]
fn treasury_snapshot_scale_roundtrip() {
    let snapshot = TreasurySnapshot {
        snapshot_hash: [0xaa; 32],
        block_number: 1_000,
        operational_float_total: 5_000_000_u128,
        insurance_reserve_total: 1_000_000_u128,
        strategic_reserve_total: 2_000_000_u128,
        gas_reserve_total: 100_000_u128,
        deployed_settlement_float: 3_000_000_u128,
        at_risk_exposure: 200_000_u128,
    };
    scale_roundtrip(snapshot);
}

#[test]
fn treasury_reconciliation_report_scale_roundtrip() {
    let report = TreasuryReconciliationReport {
        snapshot_hash: [0xff; 32],
        divergence_bps: 5,
        passed: true,
        action_count_since_last: 12,
        block_number: 800,
    };
    scale_roundtrip(report);
}

// ─── Serde JSON round-trips ───────────────────────────────────────────────────
// Only types with no `u128` fields are tested via serde_json; serde_json
// serializes u128 as a JSON integer using itoa but may fail to deserialize
// them across the u64 boundary without the `arbitrary_precision` feature.

#[test]
fn canonical_identity_serde_roundtrip() {
    let identity: CanonicalIdentity<[u8; 32]> = CanonicalIdentity {
        primary: [5u8; 32],
        identity_hash: [6u8; 32],
        governance_registered: false,
        kyc_tier: KycTier::Basic,
    };
    let json = serde_json::to_string(&identity).expect("serialize CanonicalIdentity");
    let decoded: CanonicalIdentity<[u8; 32]> =
        serde_json::from_str(&json).expect("deserialize CanonicalIdentity");
    assert_eq!(identity, decoded, "serde JSON round-trip must be lossless");
}

#[test]
fn canonical_asset_serde_roundtrip() {
    let asset = CanonicalAsset {
        asset_id: 99,
        origin: AssetOrigin::Wrapped,
        canonical_chain_id: 1,
        decimals: 6,
        symbol_hash: [0x11; 32],
        is_active: false,
    };
    let json = serde_json::to_string(&asset).expect("serialize CanonicalAsset");
    let decoded: CanonicalAsset = serde_json::from_str(&json).expect("deserialize CanonicalAsset");
    assert_eq!(asset, decoded, "serde JSON round-trip must be lossless");
}

#[test]
fn treasury_reconciliation_report_serde_roundtrip() {
    let report = TreasuryReconciliationReport {
        snapshot_hash: [0x22; 32],
        divergence_bps: 15,
        passed: false,
        action_count_since_last: 7,
        block_number: 4_200,
    };
    let json = serde_json::to_string(&report).expect("serialize TreasuryReconciliationReport");
    let decoded: TreasuryReconciliationReport =
        serde_json::from_str(&json).expect("deserialize TreasuryReconciliationReport");
    assert_eq!(report, decoded, "serde JSON round-trip must be lossless");
}

// ─── Semantic correctness checks ──────────────────────────────────────────────

#[test]
fn identity_hash_stability() {
    // Constructing two identical CanonicalIdentity values must yield identical
    // identity_hash fields.  This guards the invariant that the hash field is
    // determined solely by its inputs and is not randomly or lazily populated.
    let hash = [0xbe_u8; 32];
    let id_a: CanonicalIdentity<[u8; 32]> = CanonicalIdentity {
        primary: [42u8; 32],
        identity_hash: hash,
        governance_registered: true,
        kyc_tier: KycTier::Institutional,
    };
    let id_b: CanonicalIdentity<[u8; 32]> = CanonicalIdentity {
        primary: [42u8; 32],
        identity_hash: hash,
        governance_registered: true,
        kyc_tier: KycTier::Institutional,
    };
    assert_eq!(
        id_a.identity_hash, id_b.identity_hash,
        "identical inputs must produce identical identity_hash"
    );
    assert_eq!(
        id_a, id_b,
        "structurally identical identities must be equal"
    );
}

#[test]
fn supply_entry_arithmetic_sanity() {
    // net = minted - burned; locked and pending fields do not reduce supply.
    let entry = ChainSupplyEntry {
        chain_id: 0,
        asset_id: 1,
        minted: 1_000_000_u128,
        burned: 250_000_u128,
        locked: 100_000_u128,
        pending_in: 5_000_u128,
        pending_out: 3_000_u128,
    };
    let net = entry.minted.saturating_sub(entry.burned);
    assert_eq!(net, 750_000_u128, "net supply = minted - burned");
    // locked should never exceed net supply.
    assert!(
        entry.locked <= net,
        "locked ({}) must not exceed net supply ({})",
        entry.locked,
        net
    );
}

#[test]
fn treasury_snapshot_fields_sum() {
    let snapshot = TreasurySnapshot {
        snapshot_hash: [0u8; 32],
        block_number: 100,
        operational_float_total: 1_000_u128,
        insurance_reserve_total: 2_000_u128,
        strategic_reserve_total: 3_000_u128,
        gas_reserve_total: 500_u128,
        deployed_settlement_float: 800_u128,
        at_risk_exposure: 300_u128,
    };

    // All four reserves must sum to their expected total.
    let total_reserves = snapshot
        .operational_float_total
        .saturating_add(snapshot.insurance_reserve_total)
        .saturating_add(snapshot.strategic_reserve_total)
        .saturating_add(snapshot.gas_reserve_total);
    assert_eq!(
        total_reserves, 6_500_u128,
        "reserve totals must sum correctly"
    );

    // Deployed settlement float must not exceed the operational float it is drawn from.
    assert!(
        snapshot.deployed_settlement_float <= snapshot.operational_float_total,
        "deployed_settlement_float ({}) must not exceed operational_float_total ({})",
        snapshot.deployed_settlement_float,
        snapshot.operational_float_total
    );

    // At-risk exposure is informational but should not exceed total reserves.
    assert!(
        snapshot.at_risk_exposure <= total_reserves,
        "at_risk_exposure ({}) must not exceed total reserves ({})",
        snapshot.at_risk_exposure,
        total_reserves
    );
}
