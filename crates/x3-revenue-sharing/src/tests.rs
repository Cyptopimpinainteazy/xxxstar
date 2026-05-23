//! Tests for x3-revenue-sharing.

use codec::{Decode, Encode};

use crate::{
    validate_split, ApprovalStatus, EarningsSummary, PlacementTier, RevenueDestination,
    RevenueSplitEntry, RevenueSplitPolicy,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn empty_entry() -> RevenueSplitEntry {
    RevenueSplitEntry {
        destination: RevenueDestination::Treasury,
        share_bps: 0,
    }
}

fn two_entry_policy(bps_a: u32, bps_b: u32) -> RevenueSplitPolicy {
    RevenueSplitPolicy {
        policy_id: 1,
        entries_len: 2,
        entries: [
            RevenueSplitEntry {
                destination: RevenueDestination::Treasury,
                share_bps: bps_a,
            },
            RevenueSplitEntry {
                destination: RevenueDestination::DeveloperAccount,
                share_bps: bps_b,
            },
            empty_entry(),
            empty_entry(),
            empty_entry(),
            empty_entry(),
            empty_entry(),
            empty_entry(),
        ],
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

/// 1. A policy whose active entries sum to 10 000 bps is valid.
#[test]
fn validate_split_valid() {
    let policy = two_entry_policy(3_000, 7_000);
    assert!(
        validate_split(&policy),
        "3000 + 7000 == 10000 must be valid"
    );
}

/// 2. A policy whose entries do NOT sum to 10 000 bps is invalid.
#[test]
fn validate_split_invalid() {
    // 3000 + 6999 = 9999
    let policy = two_entry_policy(3_000, 6_999);
    assert!(!validate_split(&policy), "9999 bps must be invalid");
}

/// 3. `RevenueSplitPolicy` round-trips through SCALE encode / decode.
#[test]
fn revenue_split_policy_roundtrip() {
    let policy = RevenueSplitPolicy {
        policy_id: 42,
        entries_len: 3,
        entries: [
            RevenueSplitEntry {
                destination: RevenueDestination::Treasury,
                share_bps: 2_000,
            },
            RevenueSplitEntry {
                destination: RevenueDestination::DeveloperAccount,
                share_bps: 7_000,
            },
            RevenueSplitEntry {
                destination: RevenueDestination::ProtocolBurn,
                share_bps: 1_000,
            },
            empty_entry(),
            empty_entry(),
            empty_entry(),
            empty_entry(),
            empty_entry(),
        ],
    };

    let encoded = policy.encode();
    let decoded = RevenueSplitPolicy::decode(&mut &encoded[..]).expect("decode must succeed");
    assert_eq!(policy, decoded, "round-trip must be lossless");
}

/// 4. `EarningsSummary` fields are accessible and carry correct values.
#[test]
fn earnings_summary_field_check() {
    let summary = EarningsSummary {
        total_revenue: 1_000_000,
        developer_share: 700_000,
        protocol_share: 250_000,
        burn_amount: 50_000,
    };
    assert_eq!(summary.total_revenue, 1_000_000);
    assert_eq!(summary.developer_share, 700_000);
    assert_eq!(summary.protocol_share, 250_000);
    assert_eq!(summary.burn_amount, 50_000);
}

/// 5. `PlacementTier` encodes as expected SCALE byte values.
#[test]
fn placement_tier_encode() {
    assert_eq!(
        PlacementTier::Standard.encode(),
        vec![0u8],
        "Standard = index 0"
    );
    assert_eq!(
        PlacementTier::Featured.encode(),
        vec![1u8],
        "Featured = index 1"
    );
    assert_eq!(
        PlacementTier::Premium.encode(),
        vec![2u8],
        "Premium  = index 2"
    );
}

/// 6. `RevenueDestination` encodes as expected SCALE byte values.
#[test]
fn revenue_destination_encode() {
    assert_eq!(
        RevenueDestination::Treasury.encode(),
        vec![0u8],
        "Treasury = index 0"
    );
    assert_eq!(
        RevenueDestination::DeveloperAccount.encode(),
        vec![1u8],
        "DeveloperAccount = index 1"
    );
    assert_eq!(
        RevenueDestination::ProtocolBurn.encode(),
        vec![2u8],
        "ProtocolBurn = index 2"
    );
    assert_eq!(
        RevenueDestination::LiquidityPool.encode(),
        vec![3u8],
        "LiquidityPool = index 3"
    );
    assert_eq!(
        RevenueDestination::Stakers.encode(),
        vec![4u8],
        "Stakers = index 4"
    );
}

/// Bonus: `ApprovalStatus` round-trips correctly.
#[test]
fn approval_status_roundtrip() {
    for status in [
        ApprovalStatus::Pending,
        ApprovalStatus::Approved,
        ApprovalStatus::Rejected,
        ApprovalStatus::Suspended,
    ] {
        let encoded = status.clone().encode();
        let decoded = ApprovalStatus::decode(&mut &encoded[..]).expect("decode ok");
        assert_eq!(status, decoded);
    }
}
