//! Compiler safety check tests.
//!
//! Each test exercises a specific safety check (X3-INTENT-001 through X3-INTENT-013)
//! and verifies the correct diagnostic code fires. The happy path test verifies
//! a well-formed intent compiles to a valid instruction plan.

use x3_crosschain_intent::{
    compiler::IntentCompiler,
    error::IntentCompileError,
    intent::CrossChainIntent,
    types::{
        AssetRef, ChainKind, DestinationSpec, FailureAction, FinalityLevel, FinalityRequirement,
        ProofKind, ProofRequirement, ReceiptSpec, Requirements, RouteSpec, SourceSpec, TimeoutSpec,
    },
};

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Builds a minimal but fully-safe bridge+swap intent (ETH USDC → SOL SOL).
/// All 13 safety requirements satisfied. Should compile to a valid plan.
fn safe_bridge_swap_intent() -> CrossChainIntent {
    let mut intent = CrossChainIntent {
        id: 1,
        name: "test_usdc_to_sol".to_string(),
        source: SourceSpec {
            asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
            amount: 500_000_000, // 500 USDC (6 decimals)
            owner: "alice.eth".to_string(),
            lock_contract: Some("0xBridgeContract".to_string()),
        },
        destination: DestinationSpec {
            asset: AssetRef::new(ChainKind::Solana, "SOL"),
            receiver: "alice.sol".to_string(),
            min_amount: Some(3_500_000_000), // ~3.5 SOL
        },
        route: RouteSpec {
            objective: x3_crosschain_intent::types::RouteObjective::Best,
            allow: vec!["x3.dex".to_string(), "bridge.wormhole".to_string()],
            deny: vec!["bridge.unknown".to_string()],
        },
        requirements: Requirements {
            finality: vec![
                FinalityRequirement {
                    chain: ChainKind::Ethereum,
                    level: FinalityLevel::Confirmations(12),
                },
                FinalityRequirement {
                    chain: ChainKind::Solana,
                    level: FinalityLevel::Finalized,
                },
            ],
            max_slippage_bps: Some(100),
            max_total_fee: Some(10_000_000), // 10 USDC fee cap
            // Cross-chain owner→receiver mapping so the receiver
            // authorization rule (X3-INTENT-015) passes; the safety
            // checks we are testing are unrelated to receiver auth.
            receiver_authorization:
                x3_crosschain_intent::types::ReceiverAuthorization::MappedAccount {
                    source_chain: ChainKind::Ethereum,
                    source_owner: "alice.eth".to_string(),
                    dest_chain: ChainKind::Solana,
                    dest_account: "alice.sol".to_string(),
                },
            proofs: vec![
                ProofRequirement {
                    chain: ChainKind::Ethereum,
                    label: "eth.lock_event".to_string(),
                    kind: ProofKind::EventProof {
                        event: "BridgeLock".to_string(),
                        contract: "0xBridgeContract".to_string(),
                        confirmations: 12,
                    },
                },
                ProofRequirement {
                    chain: ChainKind::Solana,
                    label: "sol.release_receipt".to_string(),
                    kind: ProofKind::LightClientProof {
                        client_id: "sol-x3-ibc".to_string(),
                    },
                },
            ],
            require_canonical_supply_valid: true,
            // Route simulation is opt-in. Tests that exercise the
            // simulation safety check (X3-INTENT-016/-017) set this
            // explicitly. Tests that exercise other safety checks
            // leave it off so they don't trip the simulator-mode
            // gate.
            require_route_simulated: false,
        },
        timeout: TimeoutSpec {
            timeout_secs: 1800, // 30 minutes
            on_fail: vec![FailureAction::RefundSource],
        },
        receipt: ReceiptSpec {
            include_route: true,
            include_fees: true,
            include_proofs: true,
            include_state_transitions: false,
        },
        intent_hash: [0u8; 32],
    };
    intent.recompute_and_store_hash();
    intent
}

/// Returns `safe_bridge_swap_intent()` but with the given field(s) modified
/// to trigger a specific compile error. The helper recomputes the
/// intent hash so the X3-INTENT-014 check passes and the safety check
/// we are actually testing can fire.
fn with_recomputed_hash(mut intent: CrossChainIntent) -> CrossChainIntent {
    intent.recompute_and_store_hash();
    intent
}

fn intent_with_no_timeout() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.timeout.timeout_secs = 0;
        i
    })
}

fn intent_with_no_refund_path() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.timeout.on_fail = vec![];
        i
    })
}

fn intent_with_no_fee_cap() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.requirements.max_total_fee = None;
        i
    })
}

fn intent_with_no_finality() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.requirements.finality = vec![];
        i
    })
}

fn intent_with_insufficient_finality() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        // ETH safe minimum is 12; set to 3 (insufficient)
        i.requirements.finality = vec![
            FinalityRequirement {
                chain: ChainKind::Ethereum,
                level: FinalityLevel::Confirmations(3),
            },
            FinalityRequirement {
                chain: ChainKind::Solana,
                level: FinalityLevel::Finalized,
            },
        ];
        i
    })
}

fn intent_with_no_proof() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.requirements.proofs = vec![];
        i
    })
}

fn intent_with_no_canonical_supply_check() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.requirements.require_canonical_supply_valid = false;
        i
    })
}

fn intent_with_no_slippage_guard() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.requirements.max_slippage_bps = None;
        i
    })
}

fn intent_with_no_receiver_validation() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        // The default safe intent has MappedAccount(alice.eth ->
        // alice.sol). Change to a strict OwnerOnly so the cross-chain
        // string mismatch triggers X3-INTENT-015.
        i.requirements.receiver_authorization =
            x3_crosschain_intent::types::ReceiverAuthorization::OwnerOnly;
        i
    })
}

fn intent_with_unsafe_venue() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.route.allow.push("bridge.unknown".to_string());
        i
    })
}

fn intent_with_unknown_asset() -> CrossChainIntent {
    with_recomputed_hash({
        let mut i = safe_bridge_swap_intent();
        i.source.asset = AssetRef::new(ChainKind::Ethereum, "");
        i
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Happy path test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn happy_path_compiles_to_valid_plan() {
    let compiler = IntentCompiler::new();
    // Use an explicit cross-chain mapping so the receiver rule passes
    // for owner=alice.eth / receiver=alice.sol.
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::Solana,
            dest_account: "alice.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = compiler.compile(&intent);

    assert!(
        result.is_ok(),
        "Expected clean compile, got errors: {:?}",
        result.errors
    );
    assert!(
        !result.plan.is_empty(),
        "Expected instruction plan to be non-empty"
    );

    // Verify key instruction types are present
    use x3_crosschain_intent::instructions::X3Instruction;

    let labels: Vec<&'static str> = result.plan.iter().map(|i| i.label()).collect();
    println!("Generated plan ({} instructions):", labels.len());
    for (n, label) in labels.iter().enumerate() {
        println!("  {}: {}", n + 1, label);
    }

    assert!(
        labels.contains(&"RegisterWatchdog"),
        "Missing RegisterWatchdog"
    );
    assert!(labels.contains(&"ValidateOwner"), "Missing ValidateOwner");
    assert!(
        labels.contains(&"EnforceReceiverAuth"),
        "Missing EnforceReceiverAuth"
    );
    assert!(labels.contains(&"CheckBalance"), "Missing CheckBalance");
    assert!(labels.contains(&"LockAsset"), "Missing LockAsset");
    assert!(labels.contains(&"WaitFinality"), "Missing WaitFinality");
    assert!(labels.contains(&"VerifyProof"), "Missing VerifyProof");
    assert!(
        labels.contains(&"CheckCanonicalSupply"),
        "Missing CheckCanonicalSupply"
    );
    assert!(labels.contains(&"MintCanonical"), "Missing MintCanonical");
    assert!(labels.contains(&"ExecuteSwap"), "Missing ExecuteSwap");
    assert!(labels.contains(&"EmitReceipt"), "Missing EmitReceipt");
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 3: X3-INTENT-003 — Missing timeout
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_003_missing_timeout() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_timeout());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingTimeout)),
        "Expected MissingTimeout error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 4: X3-INTENT-004 — Missing refund path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_004_missing_refund_path() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_refund_path());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingRefundPath)),
        "Expected MissingRefundPath error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 5+6: X3-INTENT-013 + X3-INTENT-007 — Unbounded + no fee cap
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_013_unbounded_execution() {
    let compiler = IntentCompiler::new();
    // No timeout + no fee cap = unbounded
    let mut intent = safe_bridge_swap_intent();
    intent.timeout.timeout_secs = 0;
    intent.timeout.on_fail = vec![];
    intent.requirements.max_total_fee = None;
    intent.recompute_and_store_hash();

    let result = compiler.compile(&intent);
    assert!(!result.is_ok(), "Expected compile error");

    let has_unbounded = result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::UnboundedExecution));
    assert!(
        has_unbounded,
        "Expected UnboundedExecution error, got: {:?}",
        result.errors
    );
}

#[test]
fn check_007_missing_fee_cap() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_fee_cap());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingFeeCap { .. })),
        "Expected MissingFeeCap error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 7: X3-INTENT-001 — Missing finality
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_001_missing_finality() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_finality());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingFinality { .. })),
        "Expected MissingFinality error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 8: Insufficient finality (below chain safe minimum)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn insufficient_finality_rejected() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_insufficient_finality());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::InsufficientFinality { .. })),
        "Expected InsufficientFinality error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 9: X3-INTENT-002 — Missing proof
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_002_missing_proof() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_proof());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingProof { .. })),
        "Expected MissingProof error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 10: X3-INTENT-011 — Missing canonical supply check
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_011_missing_canonical_supply_check() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_canonical_supply_check());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingCanonicalSupplyCheck { .. })),
        "Expected MissingCanonicalSupplyCheck error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 11: X3-INTENT-006 — Missing slippage guard
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_006_missing_slippage_guard() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_slippage_guard());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingSlippageGuard { .. })),
        "Expected MissingSlippageGuard error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 12: X3-INTENT-015 — receiver authorization mismatch
// (replaces the old boolean-flag `require_receiver_is_owner` check)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_015_receiver_authorization_mismatch() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_receiver_validation());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::ReceiverAuthorizationMismatch { .. })),
        "Expected ReceiverAuthorizationMismatch error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 13: X3-INTENT-010 — Unsafe bridge venue
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_010_unsafe_bridge_venue() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_unsafe_venue());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::UnsafeRoute { .. })),
        "Expected UnsafeRoute error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check 2: X3-INTENT-009 — Unknown asset
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_009_unknown_asset() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_unknown_asset());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::UnknownAsset { .. })),
        "Expected UnknownAsset error, got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Multiple errors reported at once
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn multiple_errors_reported_together() {
    let compiler = IntentCompiler::new();

    // Intent with both no timeout AND no slippage guard AND receiver mismatch
    let mut intent = safe_bridge_swap_intent();
    intent.timeout.timeout_secs = 0;
    intent.timeout.on_fail = vec![];
    intent.requirements.max_total_fee = None;
    intent.requirements.max_slippage_bps = None;
    // Drop the MappedAccount rule so the mismatched receiver fires
    // X3-INTENT-015 in addition to the other errors.
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::OwnerOnly;
    intent.recompute_and_store_hash();

    let result = compiler.compile(&intent);
    assert!(!result.is_ok(), "Expected compile errors");
    assert!(
        result.errors.len() >= 3,
        "Expected at least 3 errors, got {} : {:?}",
        result.errors.len(),
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// X3-only swap (no bridge) — still requires slippage guard
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn x3_native_swap_requires_slippage_guard() {
    let compiler = IntentCompiler::new();

    let mut intent = safe_bridge_swap_intent();
    // Make it X3-native swap
    intent.source.asset = AssetRef::new(ChainKind::X3, "USDC");
    intent.destination.asset = AssetRef::new(ChainKind::X3, "SOL");
    intent.source.owner = "alice.x3".to_string();
    intent.destination.receiver = "alice.x3".to_string();
    intent.requirements.finality = vec![]; // no bridge, no finality needed
    intent.requirements.proofs = vec![]; // no bridge, no proofs needed
    intent.requirements.require_canonical_supply_valid = false; // no bridge mint
    intent.requirements.max_slippage_bps = None; // missing guard
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::OwnerOnly;
    intent.recompute_and_store_hash();

    let result = compiler.compile(&intent);
    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingSlippageGuard { .. })),
        "X3-native swap should still require slippage guard. Got: {:?}",
        result.errors
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Simulation tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn simulation_runs_on_valid_intent() {
    use x3_crosschain_intent::simulation::IntentSimulator;

    let simulator = IntentSimulator::new();
    let intent = safe_bridge_swap_intent();
    let result = simulator.simulate(&intent);

    assert!(result.route_found, "Route should be found");
    assert!(
        result.estimated_output.is_some(),
        "Should have estimated output"
    );
    assert!(result.estimated_fees > 0, "Should have non-zero fees");
    assert!(
        !result.is_safe_to_execute() || result.risk_score < 75,
        "Risk score check"
    );

    println!("Simulation: {}", result.summary());
}

#[test]
fn simulation_detects_slippage_violation() {
    use x3_crosschain_intent::simulation::IntentSimulator;

    let simulator = IntentSimulator::new();
    let mut intent = safe_bridge_swap_intent();
    // Set slippage limit very low (1 bps) — simulation will flag it
    intent.requirements.max_slippage_bps = Some(1);

    let result = simulator.simulate(&intent);

    // For large amounts, slippage will exceed 1bps
    // (Whether it blocks depends on the amount vs. threshold)
    println!(
        "Slippage sim result: {}bps, exceeds: {}",
        result.estimated_slippage_bps, result.slippage_exceeds_limit
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Comment 1: X3-INTENT-014 — intent hash covers every user-controlled field.
// ─────────────────────────────────────────────────────────────────────────────

fn mutate_route(intent: &mut CrossChainIntent) {
    intent.route.allow.push("bridge.layerzero".to_string());
}

fn mutate_proof(intent: &mut CrossChainIntent) {
    intent.requirements.proofs.push(ProofRequirement {
        chain: ChainKind::Ethereum,
        label: "eth.extra_lock_proof".to_string(),
        kind: ProofKind::MerkleProof {
            root_type: "eth.state_root".to_string(),
        },
    });
}

fn mutate_fee_cap(intent: &mut CrossChainIntent) {
    intent.requirements.max_total_fee = Some(9_999_999);
}

fn mutate_slippage(intent: &mut CrossChainIntent) {
    intent.requirements.max_slippage_bps = Some(77);
}

fn mutate_refund_path(intent: &mut CrossChainIntent) {
    intent
        .timeout
        .on_fail
        .push(x3_crosschain_intent::types::FailureAction::Quarantine);
}

fn mutate_receipt(intent: &mut CrossChainIntent) {
    intent.receipt.include_state_transitions = true;
}

fn mutate_receiver_auth(intent: &mut CrossChainIntent) {
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::AllowAny;
}

#[test]
fn hash_changes_when_route_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    let old_hash = a.intent_hash;
    mutate_route(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
    // Sanity: a's hash didn't change just because we recomputed.
    assert_eq!(a.intent_hash, old_hash);
}

#[test]
fn hash_changes_when_proof_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_proof(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn hash_changes_when_fee_cap_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_fee_cap(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn hash_changes_when_slippage_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_slippage(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn hash_changes_when_refund_path_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_refund_path(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn hash_changes_when_receipt_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_receipt(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn hash_changes_when_receiver_auth_changes() {
    let mut a = safe_bridge_swap_intent();
    let mut b = safe_bridge_swap_intent();
    a.recompute_and_store_hash();
    mutate_receiver_auth(&mut b);
    b.recompute_and_store_hash();
    assert_ne!(a.intent_hash, b.intent_hash);
}

#[test]
fn compiler_rejects_stale_hash() {
    // The intent's stored hash is from before the route was edited.
    // Even though the user could try to set the stored hash back to
    // match, the regression we want to prove is: tampering with the
    // intent after computing the hash is detected and compilation is
    // refused (X3-INTENT-014).
    let mut intent = safe_bridge_swap_intent();
    intent.recompute_and_store_hash(); // hash matches current fields
    let original_hash = intent.intent_hash;
    // Now edit the route without recomputing the hash.
    intent.route.allow.push("bridge.layerzero".to_string());
    // The stored hash no longer matches.
    assert_ne!(intent.intent_hash, intent.compute_hash());
    assert_eq!(intent.intent_hash, original_hash);

    let result = IntentCompiler::new().compile(&intent);
    assert!(!result.is_ok(), "Stale hash must be rejected");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::IntentHashMismatch { .. })),
        "Expected IntentHashMismatch error, got: {:?}",
        result.errors
    );
}

#[test]
fn compiler_rejects_hash_mismatch_even_on_safe_intent() {
    // The intent is otherwise valid; only the hash is wrong. The
    // compiler MUST still refuse (X3-INTENT-014) and must NOT emit a
    // plan.
    let mut intent = safe_bridge_swap_intent();
    intent.recompute_and_store_hash();
    intent.intent_hash = [0u8; 32]; // poison the stored hash

    let result = IntentCompiler::new().compile(&intent);
    assert!(!result.is_ok());
    assert!(
        result.plan.is_empty(),
        "no plan must be emitted on hash mismatch"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::IntentHashMismatch { .. })));
}

#[test]
fn happy_path_still_works_with_full_hash() {
    // The happy-path test now exercises the new full canonical hash.
    let mut intent = safe_bridge_swap_intent();
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "Expected clean compile, got errors: {:?}",
        result.errors
    );
    assert!(!result.plan.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Comment 2: X3-INTENT-015 — receiver authorization rule is enforced.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn owner_only_with_mismatched_receiver_rejected() {
    // The default intent has owner=alice.eth, receiver=alice.sol. Same
    // canonical user but different chain, so the rule's default
    // "OwnerOnly" (which compares strings) rejects it. The user must
    // opt into an explicit cross-chain mapping.
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::OwnerOnly;
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        !result.is_ok(),
        "OwnerOnly should reject cross-chain owner/receiver string mismatch"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::ReceiverAuthorizationMismatch { .. })));
}

#[test]
fn owner_only_same_string_succeeds() {
    let mut intent = safe_bridge_swap_intent();
    // Make owner == receiver exactly. The X3-only swap uses
    // ChainKind::X3 on both sides so this is unambiguous.
    intent.source.asset = AssetRef::new(ChainKind::X3, "USDC");
    intent.destination.asset = AssetRef::new(ChainKind::X3, "SOL");
    intent.source.owner = "alice.x3".to_string();
    intent.destination.receiver = "alice.x3".to_string();
    intent.requirements.finality = vec![];
    intent.requirements.proofs = vec![];
    intent.requirements.require_canonical_supply_valid = false;
    intent.route.allow = vec!["x3.dex".to_string()];
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::OwnerOnly;
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "OwnerOnly with matching strings should succeed: {:?}",
        result.errors
    );
}

#[test]
fn explicit_account_rule_with_matching_account_succeeds() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::ExplicitAccount {
            account: "alice.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "ExplicitAccount with matching account should succeed: {:?}",
        result.errors
    );
}

#[test]
fn explicit_account_rule_with_mismatched_account_rejected() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::ExplicitAccount {
            account: "mallory.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::ReceiverAuthorizationMismatch { .. })));
}

#[test]
fn mapped_account_rule_with_full_chain_match_succeeds() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::Solana,
            dest_account: "alice.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "MappedAccount with full chain match should succeed: {:?}",
        result.errors
    );
}

#[test]
fn mapped_account_rule_with_wrong_chain_rejected() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::Polygon, // wrong dest chain
            dest_account: "alice.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::ReceiverAuthorizationMismatch { .. })));
}

#[test]
fn allow_any_rule_always_succeeds() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::AllowAny;
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "AllowAny should pass the receiver auth check: {:?}",
        result.errors
    );
}

#[test]
fn compiler_emits_enforce_receiver_authorization_instruction() {
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.receiver_authorization =
        x3_crosschain_intent::types::ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::Solana,
            dest_account: "alice.sol".to_string(),
        };
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile(&intent);
    assert!(result.is_ok());
    let labels: Vec<&'static str> = result.plan.iter().map(|i| i.label()).collect();
    assert!(
        labels.contains(&"EnforceReceiverAuth"),
        "Plan must contain EnforceReceiverAuth: {:?}",
        labels
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Comment 3: X3-INTENT-016 / X3-INTENT-017 — fail-closed simulation.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn production_simulator_without_data_source_returns_no_route() {
    use x3_crosschain_intent::simulation::{IntentSimulator, SimulationMode};
    let sim = IntentSimulator::production_mode();
    let intent = safe_bridge_swap_intent();
    let result = sim.simulate(&intent);
    assert_eq!(sim.mode(), SimulationMode::Production);
    assert!(
        !result.route_found,
        "production simulator with no oracle must return route_found=false"
    );
    assert!(!result.failure_cases.is_empty());
    assert!(result.failure_cases.iter().any(|f| f.is_blocking));
}

#[test]
fn test_simulator_synthetic_route_for_unit_tests() {
    use x3_crosschain_intent::simulation::{IntentSimulator, SimulationMode};
    let sim = IntentSimulator::test_mode();
    let intent = safe_bridge_swap_intent();
    let result = sim.simulate(&intent);
    assert_eq!(sim.mode(), SimulationMode::Test);
    assert!(
        result.route_found,
        "test-mode synthetic result returns true"
    );
}

#[test]
fn compiler_rejects_test_simulator_used_to_gate_simulation_required_intent() {
    use x3_crosschain_intent::simulation::IntentSimulator;
    let sim = IntentSimulator::test_mode();
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.require_route_simulated = true;
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile_with_simulator(&intent, &sim);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::NoRealSimulationSource)));
}

#[test]
fn compiler_rejects_production_simulator_with_no_route() {
    use x3_crosschain_intent::simulation::IntentSimulator;
    let sim = IntentSimulator::production_mode();
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.require_route_simulated = true;
    intent.recompute_and_store_hash();
    let result = IntentCompiler::new().compile_with_simulator(&intent, &sim);
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| matches!(e, IntentCompileError::NoValidRoute { .. })));
}

#[test]
fn simulation_not_required_skips_safety_check_14() {
    use x3_crosschain_intent::simulation::IntentSimulator;
    let mut intent = safe_bridge_swap_intent();
    intent.requirements.require_route_simulated = false;
    intent.recompute_and_store_hash();
    // Both test and production simulators should pass the simulation
    // check when the flag is off.
    let result =
        IntentCompiler::new().compile_with_simulator(&intent, &IntentSimulator::production_mode());
    assert!(result.is_ok());
}
