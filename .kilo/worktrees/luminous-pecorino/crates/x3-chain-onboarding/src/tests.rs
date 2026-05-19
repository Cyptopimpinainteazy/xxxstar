//! Unit tests for `x3-chain-onboarding`.
//!
//! Coverage (10 tests):
//!  1. compute_composite — all-perfect scores yield 100.
//!  2. compute_composite — all-zero scores yield 0.
//!  3. compute_composite — known mixed scores match manual calculation.
//!  4. compute_composite — only technical weight (30 %) applied correctly.
//!  5. meets_approval_threshold — exact boundary values return true.
//!  6. meets_approval_threshold — composite one below threshold returns false.
//!  7. meets_approval_threshold — compliance one below threshold returns false.
//!  8. meets_approval_threshold — technical one below threshold returns false.
//!  9. SCALE codec roundtrip for ChainOnboardingRecord.
//! 10. SCALE codec roundtrip for WeeklyProvingCycleSummary.

use super::*;
use codec::{Decode, Encode};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn base_record() -> ChainOnboardingRecord {
    ChainOnboardingRecord {
        chain_id: 42,
        phase: OnboardingPhase::ComplianceReview,
        technical_score: 70,
        economic_score: 75,
        liquidity_score: 80,
        compliance_score: 85,
        composite_score: 0,
        risk_tier: ChainRisk::Medium,
        week_number: 20,
    }
}

// ─── 1. All-perfect scores ───────────────────────────────────────────────────

#[test]
fn compute_composite_all_100() {
    let mut rec = base_record();
    rec.technical_score = 100;
    rec.economic_score = 100;
    rec.liquidity_score = 100;
    rec.compliance_score = 100;
    // (100*30 + 100*30 + 100*25 + 100*15) / 100 = 10000 / 100 = 100
    assert_eq!(rec.compute_composite(), 100);
}

// ─── 2. All-zero scores ──────────────────────────────────────────────────────

#[test]
fn compute_composite_all_zero() {
    let mut rec = base_record();
    rec.technical_score = 0;
    rec.economic_score = 0;
    rec.liquidity_score = 0;
    rec.compliance_score = 0;
    assert_eq!(rec.compute_composite(), 0);
}

// ─── 3. Known mixed scores match manual calculation ──────────────────────────

#[test]
fn compute_composite_known_mixed() {
    let mut rec = base_record();
    rec.technical_score = 80;
    rec.economic_score = 90;
    rec.liquidity_score = 70;
    rec.compliance_score = 85;
    // 80*30=2400  90*30=2700  70*25=1750  85*15=1275  sum=8125 / 100 = 81
    assert_eq!(rec.compute_composite(), 81);
}

// ─── 4. Only technical weight applied correctly ──────────────────────────────

#[test]
fn compute_composite_only_technical() {
    let mut rec = base_record();
    rec.technical_score = 100;
    rec.economic_score = 0;
    rec.liquidity_score = 0;
    rec.compliance_score = 0;
    // 100*30 / 100 = 30
    assert_eq!(rec.compute_composite(), 30);
}

// ─── 5. Exact boundary values return true ────────────────────────────────────

#[test]
fn meets_approval_threshold_exact_boundary_passes() {
    let mut rec = base_record();
    // Minimum valid values: composite=70, compliance=80, technical=60
    rec.composite_score = 70;
    rec.compliance_score = 80;
    rec.technical_score = 60;
    assert!(
        rec.meets_approval_threshold(),
        "exact boundary values must return true"
    );
}

// ─── 6. composite one below threshold returns false ──────────────────────────

#[test]
fn meets_approval_threshold_composite_below_fails() {
    let mut rec = base_record();
    rec.composite_score = 69;
    rec.compliance_score = 80;
    rec.technical_score = 60;
    assert!(
        !rec.meets_approval_threshold(),
        "composite=69 must fail (threshold is 70)"
    );
}

// ─── 7. compliance one below threshold returns false ─────────────────────────

#[test]
fn meets_approval_threshold_compliance_below_fails() {
    let mut rec = base_record();
    rec.composite_score = 70;
    rec.compliance_score = 79;
    rec.technical_score = 60;
    assert!(
        !rec.meets_approval_threshold(),
        "compliance=79 must fail (threshold is 80)"
    );
}

// ─── 8. technical one below threshold returns false ──────────────────────────

#[test]
fn meets_approval_threshold_technical_below_fails() {
    let mut rec = base_record();
    rec.composite_score = 70;
    rec.compliance_score = 80;
    rec.technical_score = 59;
    assert!(
        !rec.meets_approval_threshold(),
        "technical=59 must fail (threshold is 60)"
    );
}

// ─── 9. SCALE codec roundtrip — ChainOnboardingRecord ────────────────────────

#[test]
fn chain_onboarding_record_scale_roundtrip() {
    let rec = ChainOnboardingRecord {
        chain_id: 1,
        phase: OnboardingPhase::Approved,
        technical_score: 90,
        economic_score: 85,
        liquidity_score: 78,
        compliance_score: 95,
        composite_score: 87,
        risk_tier: ChainRisk::Low,
        week_number: 52,
    };

    let encoded = rec.encode();
    let decoded =
        ChainOnboardingRecord::decode(&mut &encoded[..]).expect("SCALE decode must succeed");
    assert_eq!(rec, decoded, "roundtrip equality failed for ChainOnboardingRecord");
}

// ─── 10. SCALE codec roundtrip — WeeklyProvingCycleSummary ───────────────────

#[test]
fn weekly_proving_cycle_summary_scale_roundtrip() {
    let summary = WeeklyProvingCycleSummary {
        week_number: 20,
        chains_evaluated: 12,
        chains_approved: 8,
        chains_rejected: 4,
        avg_composite_score: 76,
    };

    let encoded = summary.encode();
    let decoded = WeeklyProvingCycleSummary::decode(&mut &encoded[..])
        .expect("SCALE decode must succeed");
    assert_eq!(
        summary, decoded,
        "roundtrip equality failed for WeeklyProvingCycleSummary"
    );
}

// ─── Serde roundtrips (std only) ─────────────────────────────────────────────

#[cfg(feature = "std")]
mod serde_tests {
    use super::*;

    #[test]
    fn chain_onboarding_record_serde_roundtrip() {
        let rec = ChainOnboardingRecord {
            chain_id: 7,
            phase: OnboardingPhase::Rejected,
            technical_score: 40,
            economic_score: 55,
            liquidity_score: 30,
            compliance_score: 60,
            composite_score: 46,
            risk_tier: ChainRisk::Critical,
            week_number: 1,
        };

        let json = serde_json::to_string(&rec).expect("serde_json serialization must succeed");
        let decoded: ChainOnboardingRecord =
            serde_json::from_str(&json).expect("serde_json deserialization must succeed");
        assert_eq!(rec, decoded, "serde roundtrip equality failed for ChainOnboardingRecord");
    }

    #[test]
    fn weekly_proving_cycle_summary_serde_roundtrip() {
        let summary = WeeklyProvingCycleSummary {
            week_number: 44,
            chains_evaluated: 5,
            chains_approved: 3,
            chains_rejected: 2,
            avg_composite_score: 72,
        };

        let json =
            serde_json::to_string(&summary).expect("serde_json serialization must succeed");
        let decoded: WeeklyProvingCycleSummary =
            serde_json::from_str(&json).expect("serde_json deserialization must succeed");
        assert_eq!(
            summary, decoded,
            "serde roundtrip equality failed for WeeklyProvingCycleSummary"
        );
    }
}
