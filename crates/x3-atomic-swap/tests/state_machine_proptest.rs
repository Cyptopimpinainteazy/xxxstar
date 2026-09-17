//! Proptest state-machine tests for atomic swap transitions.
//!
//! Verifies the HTLC state machine, proof ledger invariants, and scoreboard
//! computations across random sequences of valid and invalid operations.
//!
//! Uses a deterministic xorshift64 PRNG (no external proptest crate dependency)
//! following the pattern established in pallet-x3-cross-vm-router tests.

use sha2::Digest;
use x3_atomic_swap::error::SwapError;
use x3_atomic_swap::intent::{
    AtomicIntent, AtomicIntentBuilder, AtomicSwapStatus, ChainKind, FinalityLevel,
    FinalityRequirement, RefundPath, RouteMode,
};
use x3_atomic_swap::ledger::{ProofLedger, ProofRecord};
use x3_atomic_swap::scoreboard::SwapScoreboard;

// ── xorshift64 PRNG ──────────────────────────────────────────────────────

struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() & 0xffffffff) as u32
    }
    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }
    fn next_in_range(&mut self, lo: usize, hi: usize) -> usize {
        let span = (hi.saturating_sub(lo)).max(1) as u64;
        lo + (self.next_u64() % span) as usize
    }
}

// ── Helper: build a valid test intent ─────────────────────────────────────

fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn build_test_intent(intent_id: u64) -> AtomicIntent {
    let hashlock = sha256_hash(b"x3-test-preimage-00123456");

    AtomicIntentBuilder::new()
        .source_chain(ChainKind::Ethereum)
        .destination_chain(ChainKind::Solana)
        .source_asset("USDC")
        .destination_asset("SOL")
        .amount_in(2_100_000_000)   // 2100 USDC (6 decimals)
        .min_amount_out(500_000_000) // 0.5 SOL (9 decimals)
        .receiver("sol-wallet-address")
        .hashlock(hashlock)
        .source_timeout(1_000_000)
        .destination_timeout(500_000)
        .add_finality(FinalityRequirement {
            chain: ChainKind::Ethereum,
            level: FinalityLevel::Confirmations(12),
        })
        .add_finality(FinalityRequirement {
            chain: ChainKind::Solana,
            level: FinalityLevel::Finalized,
        })
        .refund_path(RefundPath {
            chain: ChainKind::Ethereum,
            address: "eth-refund-address".into(),
            asset: None,
        })
        .route_mode(RouteMode::DirectHtlc)
        .max_slippage_bps(50)
        .relayer_quorum(3)
        .build(intent_id)
        .expect("test intent should build")
}

// ============================================================================
// State-machine property tests
// ============================================================================

/// Property: the `valid_transitions()` table for every status is non-empty
/// unless the status is terminal.
#[test]
fn proptest_all_non_terminal_statuses_have_at_least_one_transition() {
    let statuses = [
        AtomicSwapStatus::Pending,
        AtomicSwapStatus::RouteQuoted,
        AtomicSwapStatus::SolverAssigned,
        AtomicSwapStatus::RelayersAssigned,
        AtomicSwapStatus::SourceLocked,
        AtomicSwapStatus::DestinationLocked,
        AtomicSwapStatus::BothLocked,
        AtomicSwapStatus::FinalityPending,
        AtomicSwapStatus::Claimable,
        AtomicSwapStatus::PreimageRevealed,
        AtomicSwapStatus::ClaimSubmitted,
        AtomicSwapStatus::Refundable,
        AtomicSwapStatus::RefundableSource,
        AtomicSwapStatus::RefundableDestination,
        AtomicSwapStatus::Refunding,
        AtomicSwapStatus::Disputed,
        AtomicSwapStatus::Blocked,
    ];
    let all_states = [
        AtomicSwapStatus::Pending,
        AtomicSwapStatus::RouteQuoted,
        AtomicSwapStatus::SolverAssigned,
        AtomicSwapStatus::RelayersAssigned,
        AtomicSwapStatus::SourceLocked,
        AtomicSwapStatus::DestinationLocked,
        AtomicSwapStatus::BothLocked,
        AtomicSwapStatus::FinalityPending,
        AtomicSwapStatus::Claimable,
        AtomicSwapStatus::PreimageRevealed,
        AtomicSwapStatus::ClaimSubmitted,
        AtomicSwapStatus::Claimed,
        AtomicSwapStatus::Completed,
        AtomicSwapStatus::Refundable,
        AtomicSwapStatus::RefundableSource,
        AtomicSwapStatus::RefundableDestination,
        AtomicSwapStatus::Refunding,
        AtomicSwapStatus::Refunded,
        AtomicSwapStatus::Expired,
        AtomicSwapStatus::ExpiredUnsafe,
        AtomicSwapStatus::Disputed,
        AtomicSwapStatus::Failed,
        AtomicSwapStatus::Blocked,
    ];

    for s in &all_states {
        if s.is_terminal() {
            // terminal states verify they have no transitions
            assert!(
                s.valid_transitions().is_empty(),
                "terminal {} must have zero transitions",
                s.display_label()
            );
        } else {
            // non-terminal states must have at least one transition
            assert!(
                !s.valid_transitions().is_empty(),
                "{} should have at least one transition",
                s.display_label()
            );
        }
    }
}

/// Property: every valid transition produces a different status.
#[test]
fn proptest_transitions_are_non_identity() {
    let all = [
        AtomicSwapStatus::Pending,
        AtomicSwapStatus::RouteQuoted,
        AtomicSwapStatus::SolverAssigned,
        AtomicSwapStatus::RelayersAssigned,
        AtomicSwapStatus::SourceLocked,
        AtomicSwapStatus::DestinationLocked,
        AtomicSwapStatus::BothLocked,
        AtomicSwapStatus::FinalityPending,
        AtomicSwapStatus::Claimable,
        AtomicSwapStatus::PreimageRevealed,
        AtomicSwapStatus::ClaimSubmitted,
        AtomicSwapStatus::Claimed,
        AtomicSwapStatus::Completed,
        AtomicSwapStatus::Refundable,
        AtomicSwapStatus::RefundableSource,
        AtomicSwapStatus::RefundableDestination,
        AtomicSwapStatus::Refunding,
        AtomicSwapStatus::Refunded,
        AtomicSwapStatus::Expired,
        AtomicSwapStatus::ExpiredUnsafe,
        AtomicSwapStatus::Disputed,
        AtomicSwapStatus::Failed,
        AtomicSwapStatus::Blocked,
    ];
    for from in &all {
        for to in from.valid_transitions() {
            assert_ne!(
                from, to,
                "transition {:?} -> {:?} must change state",
                from, to
            );
        }
    }
}

/// Property: transition guards are symmetric — `can_transition_to` matches
/// `valid_transitions().contains()`.
#[test]
fn proptest_can_transition_to_matches_valid_transitions() {
    let all = [
        AtomicSwapStatus::Pending,
        AtomicSwapStatus::RouteQuoted,
        AtomicSwapStatus::SolverAssigned,
        AtomicSwapStatus::RelayersAssigned,
        AtomicSwapStatus::SourceLocked,
        AtomicSwapStatus::DestinationLocked,
        AtomicSwapStatus::BothLocked,
        AtomicSwapStatus::FinalityPending,
        AtomicSwapStatus::Claimable,
        AtomicSwapStatus::PreimageRevealed,
        AtomicSwapStatus::ClaimSubmitted,
        AtomicSwapStatus::Claimed,
        AtomicSwapStatus::Completed,
        AtomicSwapStatus::Refundable,
        AtomicSwapStatus::RefundableSource,
        AtomicSwapStatus::RefundableDestination,
        AtomicSwapStatus::Refunding,
        AtomicSwapStatus::Refunded,
        AtomicSwapStatus::Expired,
        AtomicSwapStatus::ExpiredUnsafe,
        AtomicSwapStatus::Disputed,
        AtomicSwapStatus::Failed,
        AtomicSwapStatus::Blocked,
    ];
    for from in &all {
        for to in &all {
            let by_table = from.valid_transitions().contains(to);
            let by_fn = from.can_transition_to(*to);
            assert_eq!(
                by_table, by_fn,
                "mismatch: {:?}.can_transition_to({:?}) = {}, table says {}",
                from, to, by_fn, by_table
            );
        }
    }
}

/// Property: random valid walk through state machine preserves intent hash
/// and never reaches an illegal state.
#[test]
fn proptest_random_valid_walk_preserves_hash_and_reaches_terminal() {
    let mut rng = XorShift::new(0xDEAD_BEEF_0000_0001);
    let intent = build_test_intent(1);
    let original_hash = intent.intent_hash;

    let walk_end = walk_state_machine(&intent, &mut rng);
    assert_eq!(walk_end.intent_hash, original_hash, "hash must be preserved");
    assert!(
        walk_end.status.is_terminal(),
        "random walk must reach terminal state, got {:?}",
        walk_end.status
    );
}

/// Walk the intent state machine randomly from its current status to a terminal.
fn walk_state_machine(intent: &AtomicIntent, rng: &mut XorShift) -> AtomicIntent {
    let mut current = intent.clone();
    let max_steps = 50;
    for _step in 0..max_steps {
        if current.status.is_terminal() {
            break;
        }
        let transitions = current.status.valid_transitions();
        if transitions.is_empty() {
            break;
        }
        let idx = rng.next_in_range(0, transitions.len());
        let target = transitions[idx];

        // Attempt the transition — some require preconditions.
        let success = match (current.status, target) {
            // SourceLocked → RefundableSource needs timeout
            (AtomicSwapStatus::SourceLocked, AtomicSwapStatus::RefundableSource) => {
                // Skip in random walk (needs time advance)
                false
            }
            // DestinationLocked → BothLocked
            (AtomicSwapStatus::DestinationLocked, AtomicSwapStatus::BothLocked) => {
                current.set_status(target).is_ok()
            }
            // BothLocked → FinalityPending
            (AtomicSwapStatus::BothLocked, AtomicSwapStatus::FinalityPending) => {
                current.set_status(target).is_ok()
            }
            // FinalityPending → Claimable
            (AtomicSwapStatus::FinalityPending, AtomicSwapStatus::Claimable) => {
                current.set_status(target).is_ok()
            }
            // Claimable → Claimed
            (AtomicSwapStatus::Claimable, AtomicSwapStatus::Claimed) => {
                current.set_status(target).is_ok()
            }
            // Refundable → Refunded
            (AtomicSwapStatus::Refundable, AtomicSwapStatus::Refunded) => {
                current.set_status(target).is_ok()
            }
            // anything else
            _ => current.set_status(target).is_ok(),
        };

        if success {
            // Status changed — verify hash preserved
            assert_eq!(
                current.intent_hash, intent.intent_hash,
                "hash must not change during transition {:?} -> {:?}",
                intent.status, target
            );
        }
    }
    current
}

/// Property: wrong preimage is always rejected.
#[test]
fn proptest_wrong_preimage_rejected() {
    let intent = build_test_intent(1);
    let wrong = b"wrong-preimage-data-12345";
    assert!(!intent.verify_preimage(wrong));
}

/// Property: correct preimage is always accepted.
#[test]
fn proptest_correct_preimage_accepted() {
    let intent = build_test_intent(1);
    let correct = b"x3-test-preimage-00123456";
    assert!(intent.verify_preimage(correct));
}

/// Property: terminal status rejects all further transitions.
#[test]
fn proptest_terminal_status_rejects_all_transitions() {
    let intent = build_test_intent(1);
    let terminals = [
        AtomicSwapStatus::Claimed,
        AtomicSwapStatus::Completed,
        AtomicSwapStatus::Refunded,
        AtomicSwapStatus::ExpiredUnsafe,
        AtomicSwapStatus::Failed,
        AtomicSwapStatus::Blocked,
    ];
    for terminal in &terminals {
        let mut t = intent.clone();
        t.status = *terminal;
        for target in [
            AtomicSwapStatus::SourceLocked,
            AtomicSwapStatus::Claimable,
            AtomicSwapStatus::Claimed,
            AtomicSwapStatus::Refunded,
        ] {
            let result = t.set_status(target);
            assert!(
                result.is_err(),
                "terminal {:?} must reject transition to {:?}",
                terminal,
                target
            );
        }
    }
}

// ============================================================================
// Scoreboard property tests
// ============================================================================

/// Property: a fresh/empty scoreboard scores 0/100.
#[test]
fn proptest_fresh_scoreboard_scores_zero() {
    let mut record = ProofRecord::new(0, 1, "relayer-0".into(), 0);
    let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 0, false);
    assert_eq!(scoreboard.total_score, 0);
    assert!(!scoreboard.is_perfect());
}

/// Property: a fully proven swap scores 100/100.
#[test]
fn proptest_fully_proven_swap_scores_100() {
    let intent = build_test_intent(1);
    let mut record = ProofRecord::new(0, intent.intent_id, "relayer-1".into(), 1000);

    // Record all required proofs
    record.record_source_lock("0xsource".into(), 100, 1000);
    record.record_destination_lock("0xdest".into(), 200, 1500);
    record.record_hashlock_match(true, 2000);
    record.record_timeout_order(true, 2000);
    record.record_finality_verified(true, 2000);
    record.record_secret_reveal("0xreveal".into(), 2500);
    record.record_claim("0xclaim".into(), 300, 3000);

    let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);

    assert_eq!(scoreboard.total_score, 100, "should score 100, got {}", scoreboard.total_score);
    assert!(scoreboard.is_perfect());
}

/// Property: missing claim proof prevents 100/100.
#[test]
fn proptest_missing_claim_cannot_score_100() {
    let intent = build_test_intent(1);
    let mut record = ProofRecord::new(0, intent.intent_id, "relayer-1".into(), 1000);

    record.record_source_lock("0xsource".into(), 100, 1000);
    record.record_destination_lock("0xdest".into(), 200, 1500);
    record.record_hashlock_match(true, 2000);
    record.record_timeout_order(true, 2000);
    record.record_finality_verified(true, 2000);
    record.record_secret_reveal("0xreveal".into(), 2500);
    // NO claim recorded

    let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
    assert!(scoreboard.total_score < 100, "score should be < 100, got {}", scoreboard.total_score);
    assert!(!scoreboard.is_perfect());
}

/// Property: missing finality proof prevents 100/100.
#[test]
fn proptest_missing_finality_cannot_score_100() {
    let intent = build_test_intent(1);
    let mut record = ProofRecord::new(0, intent.intent_id, "relayer-1".into(), 1000);

    record.record_source_lock("0xsource".into(), 100, 1000);
    record.record_destination_lock("0xdest".into(), 200, 1500);
    record.record_hashlock_match(true, 2000);
    record.record_timeout_order(true, 2000);
    // NO finality verified
    record.record_secret_reveal("0xreveal".into(), 2500);
    record.record_claim("0xclaim".into(), 300, 3000);

    let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
    assert!(scoreboard.total_score < 100, "score should be < 100, got {}", scoreboard.total_score);
}

// ============================================================================
// Randomized stress tests
// ============================================================================

/// 100 random intent builds — all valid ones pass timeout validation.
#[test]
fn proptest_random_intent_builds_valid_timeouts() {
    let mut rng = XorShift::new(0xC0FFEE);
    let hashlock = sha256_hash(b"test-preimage");

    for i in 0..100 {
        let src_timeout = rng.next_u64() % 10_000_000 + 1_000;
        let dest_timeout = src_timeout.saturating_sub(rng.next_u64() % src_timeout).max(1);
        let amount_in = rng.next_u64() as u128 % 1_000_000 + 1;
        let min_out = rng.next_u64() as u128 % amount_in;
        let slippage = (rng.next_u64() % 1001) as u16; // 0-1000 bps

        let result = AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(amount_in)
            .min_amount_out(min_out)
            .receiver("dest-addr")
            .hashlock(hashlock)
            .source_timeout(src_timeout)
            .destination_timeout(dest_timeout)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "refund".into(),
                asset: None,
            })
            .route_mode(RouteMode::DirectHtlc)
            .max_slippage_bps(slippage)
            .relayer_quorum(1)
            .build(i);

        match result {
            Ok(intent) => {
                assert!(dest_timeout < src_timeout, "valid intent must have dest < src");
                assert!(intent.verify_hash(), "hash must verify for built intent");
            }
            Err(e) => {
                // Only acceptable error is InvalidTimeoutOrdering
                assert!(
                    matches!(e, SwapError::InvalidTimeoutOrdering { .. }),
                    "unexpected error for intent {i}: {e:?}"
                );
            }
        }
    }
}

/// Property: 200 random nonce+chain combos parse/unparse correctly.
#[test]
fn proptest_chain_kind_roundtrip() {
    let mut rng = XorShift::new(0xDEAD_BEEF_CAFE_D00D);
    for _ in 0..200 {
        let idx = rng.next_in_range(0, 11);
        let chain = match idx {
            0 => ChainKind::Ethereum,
            1 => ChainKind::Solana,
            2 => ChainKind::Bitcoin,
            3 => ChainKind::X3,
            4 => ChainKind::Base,
            5 => ChainKind::Arbitrum,
            6 => ChainKind::Optimism,
            7 => ChainKind::Bsc,
            8 => ChainKind::Polygon,
            9 => ChainKind::Avalanche,
            _ => ChainKind::Cosmos,
        };
        let s = chain.as_str();
        let parsed = ChainKind::parse(s).expect("must parse back");
        assert_eq!(parsed, chain, "{s} -> {:?} failed roundtrip", chain);
    }
}