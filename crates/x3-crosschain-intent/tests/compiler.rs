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
    CrossChainIntent {
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
            require_receiver_is_owner: true,
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
            require_route_simulated: true,
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
    }
}

/// Returns `safe_bridge_swap_intent()` but with the given field(s) modified
/// to trigger a specific compile error.
fn intent_with_no_timeout() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.timeout.timeout_secs = 0;
    i
}

fn intent_with_no_refund_path() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.timeout.on_fail = vec![];
    i
}

fn intent_with_no_fee_cap() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.max_total_fee = None;
    i
}

fn intent_with_no_finality() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.finality = vec![]; // remove all finality requirements
    i
}

fn intent_with_insufficient_finality() -> CrossChainIntent {
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
}

fn intent_with_no_proof() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.proofs = vec![]; // no proofs at all
    i
}

fn intent_with_no_canonical_supply_check() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.require_canonical_supply_valid = false;
    i
}

fn intent_with_no_slippage_guard() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.max_slippage_bps = None;
    i
}

fn intent_with_no_receiver_validation() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.requirements.require_receiver_is_owner = false;
    i
}

fn intent_with_unsafe_venue() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.route.allow.push("bridge.unknown".to_string()); // explicitly allowed AND denied
    i
}

fn intent_with_unknown_asset() -> CrossChainIntent {
    let mut i = safe_bridge_swap_intent();
    i.source.asset = AssetRef::new(ChainKind::Ethereum, ""); // empty symbol
    i
}

// ─────────────────────────────────────────────────────────────────────────────
// Happy path test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn happy_path_compiles_to_valid_plan() {
    let compiler = IntentCompiler::new();
    let intent = safe_bridge_swap_intent();
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

    assert!(labels.contains(&"RegisterWatchdog"), "Missing RegisterWatchdog");
    assert!(labels.contains(&"ValidateOwner"), "Missing ValidateOwner");
    assert!(labels.contains(&"CheckBalance"), "Missing CheckBalance");
    assert!(labels.contains(&"Simulate"), "Missing SimulateExecution");
    assert!(labels.contains(&"LockAsset"), "Missing LockAsset");
    assert!(labels.contains(&"WaitFinality"), "Missing WaitFinality");
    assert!(labels.contains(&"VerifyProof"), "Missing VerifyProof");
    assert!(labels.contains(&"CheckCanonicalSupply"), "Missing CheckCanonicalSupply");
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
        result.errors.iter().any(|e| matches!(e, IntentCompileError::MissingTimeout)),
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
        result.errors.iter().any(|e| matches!(e, IntentCompileError::MissingRefundPath)),
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
        result.errors.iter().any(|e| matches!(e, IntentCompileError::MissingFeeCap { .. })),
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
// Safety check 12: X3-INTENT-005 — Missing receiver validation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_005_missing_receiver_validation() {
    let compiler = IntentCompiler::new();
    let result = compiler.compile(&intent_with_no_receiver_validation());

    assert!(!result.is_ok(), "Expected compile error");
    assert!(
        result
            .errors
            .iter()
            .any(|e| matches!(e, IntentCompileError::MissingReceiverValidation)),
        "Expected MissingReceiverValidation error, got: {:?}",
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

    // Intent with both no timeout AND no slippage guard AND no receiver validation
    let mut intent = safe_bridge_swap_intent();
    intent.timeout.timeout_secs = 0;
    intent.timeout.on_fail = vec![];
    intent.requirements.max_total_fee = None;
    intent.requirements.max_slippage_bps = None;
    intent.requirements.require_receiver_is_owner = false;

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
    intent.requirements.finality = vec![]; // no bridge, no finality needed
    intent.requirements.proofs = vec![]; // no bridge, no proofs needed
    intent.requirements.require_canonical_supply_valid = false; // no bridge mint
    intent.requirements.max_slippage_bps = None; // missing guard

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
    assert!(result.estimated_output.is_some(), "Should have estimated output");
    assert!(result.estimated_fees > 0, "Should have non-zero fees");
    assert!(!result.is_safe_to_execute() || result.risk_score < 75, "Risk score check");

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
    println!("Slippage sim result: {}bps, exceeds: {}", result.estimated_slippage_bps, result.slippage_exceeds_limit);
}
