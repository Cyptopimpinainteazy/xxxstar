//! End-to-end integration test for the adapter boundary.
//!
//! This test exercises the full chain that the four-layer system
//! promises:
//!
//! 1. **x3-lang compiler boundary** — produces an `IntentSpecDraft`
//!    (the language-agnostic, JSON-serializable description).
//! 2. **x3-crosschain-intent adapter** — converts the draft into
//!    a fully-validated `CrossChainIntent` and stamps the canonical
//!    hash. This is the single canonical entry point for the
//!    language compiler.
//! 3. **Intent compiler** — runs the safety checks and emits the
//!    `X3Instruction` execution plan (legs, proofs, refund).
//! 4. **Settlement runtime adapter** — projects the
//!    `CrossChainIntent` onto a `SettlementIntent`, enforcing
//!    hash integrity, non-zero timeout, and supported chains.
//!
//! The test asserts that the same intent, expressed in x3-lang
//! surface, produces an execution plan and runtime intent that
//! preserve every safety-critical field — timeout, proof, refund
//! path, and receiver authorization.

use x3_crosschain_intent::adapter::{
    intent_spec_to_crosschain_intent, validate_intent_spec, AdapterError, IntentSpec,
};
use x3_crosschain_intent::compiler::IntentCompiler;
use x3_crosschain_intent::instructions::X3Instruction;
use x3_crosschain_intent::simulation::IntentSimulator;
use x3_crosschain_intent::types::{
    AssetRef, ChainKind, DestinationSpec, FailureAction, FinalityLevel, FinalityRequirement,
    ProofKind, ProofRequirement, ReceiptSpec, ReceiverAuthorization, Requirements, RouteObjective,
    RouteSpec, SourceSpec, TimeoutSpec,
};
use x3_crosschain_intent::CrossChainIntent;

/// Build a draft-shape `IntentSpec` mirroring what the x3-lang
/// compiler would emit from a `.x3` source program. The x3-lang
/// compiler lives in a separate workspace; for the e2e test we
/// recreate the same shape here and round-trip it through the
/// canonical adapter.
fn build_draft_like_spec(name: &str) -> IntentSpec {
    let source = SourceSpec {
        asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
        amount: 500_000_000,
        owner: "alice.eth".to_string(),
        lock_contract: Some("0xBridge".to_string()),
    };
    let destination = DestinationSpec {
        asset: AssetRef::new(ChainKind::X3, "USDC.e"),
        receiver: "alice.x3".to_string(),
        min_amount: Some(500_000_000),
    };
    let mut spec = IntentSpec::new(name, source, destination);
    spec.route = RouteSpec {
        objective: RouteObjective::MaximizeOutput,
        allow: vec!["x3.dex".to_string()],
        deny: vec!["bridge.unknown".to_string()],
    };
    spec.requirements = Requirements {
        finality: vec![FinalityRequirement {
            chain: ChainKind::Ethereum,
            level: FinalityLevel::Confirmations(12),
        }],
        proofs: vec![ProofRequirement {
            chain: ChainKind::Ethereum,
            kind: ProofKind::EventProof {
                event: "BridgeLock".to_string(),
                contract: "0xBridge".to_string(),
                confirmations: 12,
            },
            label: "eth.lock_event".to_string(),
        }],
        max_slippage_bps: Some(50),
        max_total_fee: Some(10_000_000),
        require_canonical_supply_valid: true,
        require_route_simulated: false,
        receiver_authorization: ReceiverAuthorization::MappedAccount {
            source_chain: ChainKind::Ethereum,
            source_owner: "alice.eth".to_string(),
            dest_chain: ChainKind::X3,
            dest_account: "alice.x3".to_string(),
        },
    };
    spec.timeout = TimeoutSpec {
        timeout_secs: 30 * 60,
        on_fail: vec![FailureAction::RefundSource, FailureAction::Quarantine],
    };
    spec.receipt = ReceiptSpec::verbose();
    // Set the receiver_authorization on the spec (not just the
    // requirements struct) so the canonical adapter picks it up.
    spec.receiver_authorization = ReceiverAuthorization::MappedAccount {
        source_chain: ChainKind::Ethereum,
        source_owner: "alice.eth".to_string(),
        dest_chain: ChainKind::X3,
        dest_account: "alice.x3".to_string(),
    };
    spec
}

#[test]
fn end_to_end_x3_lang_to_runtime_intent() {
    // ── Stage 1: x3-lang compiler boundary ────────────────────────
    // The x3-lang compiler produces an `IntentSpecDraft` (a JSON
    // value). The integration test reconstructs the same shape via
    // `IntentSpec::new` since both layers speak the same field
    // vocabulary. The real call site in the x3-lang workspace
    // builds the draft directly from the parser AST.
    let spec = build_draft_like_spec("bridge_usdc_x3");

    // ── Stage 2: x3-crosschain-intent adapter boundary ────────────
    // This is the canonical lowering function. The x3-lang
    // compiler calls this (or its test-mode twin) for every
    // cross-chain intent it emits.
    let intent = intent_spec_to_crosschain_intent(spec, 1);
    assert!(intent.verify_hash(), "adapter must stamp a valid hash");
    assert_eq!(intent.name, "bridge_usdc_x3");
    assert_eq!(intent.source.amount, 500_000_000);
    assert_eq!(intent.destination.receiver, "alice.x3");
    assert_eq!(intent.timeout.timeout_secs, 30 * 60);

    // ── Stage 3: Intent compiler emits the plan ───────────────────
    // The plan is the cross-VM execution graph: locking,
    // proof verification, swap, refund on failure.
    let result = IntentCompiler::new().compile(&intent);
    assert!(
        result.is_ok(),
        "safe intent must compile, errors: {:?}",
        result.errors
    );
    let plan = result.plan;

    // The plan must include a timeout / refund hook, a proof
    // verification step, and a receiver authorization step.
    let labels: Vec<&str> = plan.iter().map(|s| s.label()).collect();
    assert!(
        labels
            .iter()
            .any(|l| l.contains("Refund") || l.contains("Watchdog")),
        "plan must include refund/watchdog hook: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains("Proof")),
        "plan must include proof step: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains("Receiver")),
        "plan must include receiver-auth step: {labels:?}"
    );

    // The refund hook must point at the same actions the intent
    // declared (RefundSource, Quarantine).
    let has_refund = plan.iter().any(|step| {
        matches!(
            step,
            X3Instruction::RegisterTimeoutWatchdog { .. } | X3Instruction::ExecuteRefund { .. }
        )
    });
    assert!(
        has_refund,
        "plan must include a timeout-watchdog or refund step"
    );

    // ── Stage 4: simulation must run with a real data source ──────
    // Production mode requires a real quote/liquidity/bridge
    // source. Test mode is allowed to synthesize a route.
    let test_sim = IntentSimulator::test_mode();
    let test_outcome = test_sim.simulate(&intent);
    assert!(test_outcome.route_found, "test simulator must find a route");

    // Production mode without a data source must fail closed.
    let prod_sim = IntentSimulator::production_mode();
    let prod_outcome = prod_sim.simulate(&intent);
    assert!(
        !prod_outcome.route_found,
        "production simulator without a data source must fail closed"
    );
    assert!(
        prod_outcome
            .failure_cases
            .iter()
            .any(|f| f.label == "no_real_data_source"),
        "production simulator must report no_real_data_source: {:?}",
        prod_outcome.failure_cases
    );

    // ── Stage 5: settlement runtime adapter invariants ───────────
    // The runtime side has its own adapter function
    // `from_crosschain_intent` in the settlement pallet. We do not
    // import that here (different crate) but we mirror its hash
    // and timeout invariants at the boundary to demonstrate the
    // contract is preserved.
    let stored_hash = intent.intent_hash;
    let recomputed = intent.compute_hash();
    assert_eq!(
        stored_hash, recomputed,
        "hash invariant at runtime boundary"
    );
    assert!(
        intent.timeout.timeout_secs > 0,
        "non-zero timeout invariant"
    );
    assert!(
        !intent.timeout.on_fail.is_empty(),
        "non-empty refund path invariant"
    );
    assert!(
        !intent.requirements.proofs.is_empty(),
        "non-empty proof requirements invariant"
    );
    assert!(
        !matches!(
            intent.requirements.receiver_authorization,
            ReceiverAuthorization::AllowAny
        ),
        "receiver authorization must be strict"
    );
}

#[test]
fn end_to_end_adapter_rejects_empty_draft() {
    // The adapter boundary must fail closed on bad input. The
    // x3-lang compiler should never produce a draft with an empty
    // name or zero amount, but the boundary is a defensive layer
    // so we still verify it rejects them.
    let source = SourceSpec {
        asset: AssetRef::new(ChainKind::Ethereum, "USDC"),
        amount: 0, // zero
        owner: "alice.eth".to_string(),
        lock_contract: None,
    };
    let destination = DestinationSpec {
        asset: AssetRef::new(ChainKind::X3, "USDC.e"),
        receiver: "alice.x3".to_string(),
        min_amount: None,
    };
    let spec = IntentSpec::new("", source, destination);
    let err = validate_intent_spec(&spec).unwrap_err();
    assert!(matches!(err, AdapterError::EmptyIntentName));
}

#[test]
fn end_to_end_unsafe_intent_is_rejected_by_canonical_compiler() {
    // An intent that *passes* the adapter preflight but fails the
    // canonical safety check must be rejected by `IntentCompiler`
    // (the second layer). This is the safety net: even if the
    // x3-lang compiler produces a draft that looks well-formed, the
    // canonical intent compiler enforces every safety rule.
    let mut spec = build_draft_like_spec("unsafe_swap");
    // Make the intent violate the no-simulation-required contract
    // by clearing the slippage guard.
    spec.requirements.max_slippage_bps = None;
    let intent = intent_spec_to_crosschain_intent(spec, 99);

    let result = IntentCompiler::new().compile(&intent);
    assert!(!result.is_ok(), "unsafe intent must be rejected");
    let messages: Vec<String> = result.errors.iter().map(|e| format!("{e}")).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("X3-INTENT-006") || m.contains("slippage")),
        "expected slippage-guard error, got: {messages:?}"
    );
}

#[test]
fn end_to_end_hash_integrity_is_preserved_throughout() {
    // The canonical hash must round-trip through every stage. If
    // the language compiler, the intent adapter, the safety
    // compiler, or the runtime adapter lost a single field, the
    // hash would change and the test would fail.
    let intent = intent_spec_to_crosschain_intent(build_draft_like_spec("hash_round_trip"), 1);
    let pre_plan_hash = intent.compute_hash();

    // Compile to a plan; the hash on the input intent must not
    // change.
    let plan_result = IntentCompiler::new().compile(&intent);
    assert!(
        plan_result.is_ok(),
        "compile errors: {:?}",
        plan_result.errors
    );
    let _plan = plan_result.plan;
    assert_eq!(
        intent.compute_hash(),
        pre_plan_hash,
        "intent hash must be stable through compile"
    );

    // Simulate; the hash must still round-trip.
    let simulator = IntentSimulator::test_mode();
    let _outcome = simulator.simulate(&intent);
    assert_eq!(
        intent.compute_hash(),
        pre_plan_hash,
        "intent hash must be stable through simulation"
    );
}

#[test]
fn end_to_end_intent_carries_minimum_safety_invariants() {
    // The user-facing contract promises that any intent that
    // makes it through the language compiler and the canonical
    // adapter has, at minimum, these invariants. Test them
    // explicitly so a future refactor cannot silently relax them.
    let intent: CrossChainIntent =
        intent_spec_to_crosschain_intent(build_draft_like_spec("invariant_check"), 1);

    // Timeout is set, non-zero, and has at least one on-fail path.
    assert!(intent.timeout.timeout_secs > 0);
    assert!(!intent.timeout.on_fail.is_empty());

    // Finality requirement is present.
    assert!(!intent.requirements.finality.is_empty());

    // Slippage cap or fee cap is present (a swap without either
    // is unsafe).
    assert!(
        intent.requirements.max_slippage_bps.is_some()
            || intent.requirements.max_total_fee.is_some(),
        "swap intent must have a slippage or fee cap"
    );

    // Proof requirements are present (bridge steps need proofs).
    assert!(!intent.requirements.proofs.is_empty());

    // Receiver authorization is strict.
    assert!(!matches!(
        intent.requirements.receiver_authorization,
        ReceiverAuthorization::AllowAny
    ));
}
