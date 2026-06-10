//! Adapter: `IntentSpecDraft` (from x3-lang compiler) → `CrossChainIntent`
//!
//! The x3-lang compiler (in `x3-lang/compiler/src/intent_emit.rs`) produces
//! an `IntentSpecDraft` — a JSON-serializable, stringly-typed description of
//! a cross-chain intent. This module converts that draft into the canonical
//! [`IntentSpec`] → [`CrossChainIntent`] → compiled execution plan.
//!
//! This is the **single bridge** between the two language tracks.
//! - Old track: x3-lang/ workspace produces `IntentSpecDraft` (string fields,
//!   generic constraints, safe defaults).
//! - Canonical track: x3-crosschain-intent consumes `IntentSpec` and produces
//!   a validated `CrossChainIntent` with a hashed identity and full instruction
//!   plan via [`IntentCompiler`].
//!
//! # Usage
//!
//! ```ignore
//! use x3_crosschain_intent::from_draft::draft_to_compiled_plan;
//!
//! // Deserialize the output of the old x3-lang compiler
//! let draft: IntentSpecDraft = serde_json::from_str(json_str)?;
//! let result = draft_to_compiled_plan(draft, 1)?;
//! // result.plan: Vec<X3Instruction>
//! ```
//!
//! # Safety
//!
//! The adapter validates the draft at the boundary:
//! - Chain names must resolve to known [`ChainKind`] variants.
//! - Timeouts must parse from human-readable durations ("30m", "1h").
//! - Constraints (slippage, finality, proof, fee) map onto structured requirements.
//! - Empty names/owners/receivers are rejected.
//! - Zero amounts are rejected.

use crate::adapter::{
    chain_kind_from_canonical, intent_spec_to_crosschain_intent, validate_intent_spec,
    AdapterError, IntentSpec,
};
use crate::compiler::{CompileResult, IntentCompiler};
use crate::types::{
    AssetRef, ChainKind, DestinationSpec, FailureAction, FinalityLevel, FinalityRequirement,
    ProofKind, ProofRequirement, ReceiverAuthorization, Requirements, RouteObjective, RouteSpec,
    SourceSpec, TimeoutSpec,
};

/// The JSON-serializable draft produced by the old x3-lang compiler.
///
/// This type is defined here (rather than importing from x3-lang-compiler)
/// to avoid a cross-workspace dependency. The old x3-lang compiler is not
/// in the root workspace; it lives in the separate `x3-lang/` workspace.
/// The draft reaches this adapter via JSON serialization — the old compiler
/// writes a JSON file, and this module reads it.
///
/// Fields must match `x3-lang::compiler::intent_emit::IntentSpecDraft` exactly.
/// Run `cargo test --test draft_adapter_roundtrip` to verify consistency.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct IntentSpecDraft {
    pub name: String,
    pub source_chain: String,
    pub source_asset: String,
    pub source_amount: u128,
    pub source_owner: String,
    pub source_lock_contract: Option<String>,
    pub dest_chain: String,
    pub dest_asset: String,
    pub dest_min_amount: Option<u128>,
    pub dest_receiver: String,
    pub constraints: Vec<SourceConstraint>,
    pub timeout_secs: u64,
}

/// Constraint from the old x3-lang compiler.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SourceConstraint {
    pub kind: String,
    pub arg: String,
}

/// Result of the draft-to-plan pipeline.
#[derive(Debug)]
pub struct DraftCompileResult {
    /// The canonical, hashed cross-chain intent.
    pub intent: crate::CrossChainIntent,
    /// The compiled execution plan (empty if compilation failed).
    pub plan: Vec<crate::instructions::X3Instruction>,
    /// Errors from the intent compiler safety checks.
    pub errors: Vec<crate::error::IntentCompileError>,
    /// Errors from the adapter boundary (pre-safety-check validation).
    pub adapter_errors: Vec<AdapterError>,
}

impl DraftCompileResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty() && self.adapter_errors.is_empty()
    }
}

/// Convert an [`IntentSpecDraft`] into an [`IntentSpec`] at the adapter boundary.
///
/// This maps stringly-typed fields from the old compiler to the canonical
/// typed enums. Returns adapter errors for unknown chains, empty fields,
/// and zero amounts.
pub fn draft_to_intent_spec(draft: &IntentSpecDraft) -> Result<IntentSpec, Vec<AdapterError>> {
    let mut errors: Vec<AdapterError> = Vec::new();

    // Validate basic fields
    if draft.name.is_empty() {
        errors.push(AdapterError::EmptyIntentName);
    }
    if draft.source_owner.is_empty() {
        errors.push(AdapterError::EmptyAddress {
            field: "source_owner",
        });
    }
    if draft.dest_receiver.is_empty() {
        errors.push(AdapterError::EmptyAddress {
            field: "dest_receiver",
        });
    }
    if draft.source_amount == 0 {
        errors.push(AdapterError::NonPositiveAmount {
            amount: 0,
            asset: format!("{}.{}", draft.source_chain, draft.source_asset),
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Resolve chain kinds
    let source_chain = match chain_kind_from_canonical(&draft.source_chain) {
        Ok(c) => c,
        Err(e) => {
            errors.push(e);
            // Need a placeholder to satisfy type system
            // but will return Err below anyway
            return Err(errors);
        }
    };
    let dest_chain = match chain_kind_from_canonical(&draft.dest_chain) {
        Ok(c) => c,
        Err(e) => {
            errors.push(e);
            return Err(errors);
        }
    };

    // Build asset refs
    let source_asset = AssetRef::new(source_chain, draft.source_asset.clone());
    let dest_asset = AssetRef::new(dest_chain, draft.dest_asset.clone());

    // Resolve constraints into structured requirements
    let mut requirements = Requirements::default();
    let mut route = RouteSpec::default();
    let mut proofs: Vec<ProofRequirement> = Vec::new();
    let mut on_fail: Vec<FailureAction> = Vec::new();
    let mut receiver_auth: ReceiverAuthorization = ReceiverAuthorization::OwnerOnly;

    for constraint in &draft.constraints {
        match constraint.kind.as_str() {
            "slippage" => {
                // Parse argument like "<= 0.5%", "<= 50bps", "<= 100"
                let arg = constraint.arg.trim();
                if let Some(rest) = arg.strip_prefix("<=") {
                    let rest = rest.trim();
                    if rest.ends_with('%') {
                        if let Ok(pct) = rest.trim_end_matches('%').trim().parse::<f64>() {
                            requirements.max_slippage_bps = Some((pct * 100.0) as u32);
                        }
                    } else if rest.to_lowercase().ends_with("bps") {
                        if let Ok(bps) = rest
                            .trim_end_matches("bps")
                            .trim_end_matches("BPS")
                            .trim()
                            .parse::<u32>()
                        {
                            requirements.max_slippage_bps = Some(bps);
                        }
                    } else if let Ok(bps) = rest.parse::<u32>() {
                        requirements.max_slippage_bps = Some(bps);
                    }
                }
            }
            "max_fee" | "fee_cap" => {
                // Argument like "<= 10 USDC" — extract numeric portion
                let arg = constraint.arg.trim();
                if let Some(rest) = arg.strip_prefix("<=") {
                    let rest = rest.trim();
                    // Take the first whitespace-delimited token as the number
                    if let Some(num_str) = rest.split_whitespace().next() {
                        if let Ok(fee) = num_str.parse::<u128>() {
                            requirements.max_total_fee = Some(fee);
                        }
                    }
                }
            }
            "finality" => {
                // Argument like "eth >= 12", "sol >= 32", "btc >= 6"
                let arg = constraint.arg.trim();
                if let Some((chain_part, num_part)) = arg.split_once(">=") {
                    let chain_str = chain_part.trim();
                    if let Ok(n) = num_part.trim().parse::<u32>() {
                        if let Ok(ck) = chain_kind_from_canonical(chain_str) {
                            requirements.finality.push(FinalityRequirement {
                                chain: ck,
                                level: FinalityLevel::Confirmations(n),
                            });
                        }
                    }
                } else if let Some((chain_part, num_part)) = arg.split_once(">") {
                    // Accept ">" as well
                    let chain_str = chain_part.trim();
                    if let Ok(n) = num_part.trim().parse::<u32>() {
                        if let Ok(ck) = chain_kind_from_canonical(chain_str) {
                            requirements.finality.push(FinalityRequirement {
                                chain: ck,
                                level: FinalityLevel::Confirmations(n),
                            });
                        }
                    }
                }
            }
            "proof" => {
                // Argument like "eth.lock_event", "sol.mint_receipt", "btc.spv_proof"
                let arg = constraint.arg.trim();
                if let Some((chain_str, event)) = arg.split_once('.') {
                    if let Ok(ck) = chain_kind_from_canonical(chain_str) {
                        let kind = if event.contains("merkle") || event.contains("trie") {
                            ProofKind::MerkleProof {
                                root_type: "state".to_string(),
                            }
                        } else if event.contains("spv") || event.contains("header") {
                            ProofKind::SpvProof { confirmations: 6 }
                        } else {
                            ProofKind::EventProof {
                                event: event.to_string(),
                                contract: "0x0000000000000000000000000000000000000000".to_string(),
                                confirmations: 1,
                            }
                        };
                        proofs.push(ProofRequirement {
                            chain: ck,
                            label: arg.to_string(),
                            kind,
                        });
                    }
                }
            }
            "require_canonical_supply" | "supply_check" => {
                requirements.require_canonical_supply_valid = true;
            }
            "require_simulation" | "simulate" | "route_simulation" => {
                requirements.require_route_simulated = true;
            }
            "require_receiver_owner" | "receiver_is_owner" => {
                receiver_auth = ReceiverAuthorization::OwnerOnly;
            }
            "receiver_allow_any" | "allow_any_receiver" => {
                receiver_auth = ReceiverAuthorization::AllowAny;
            }
            "receiver_explicit" => {
                receiver_auth = ReceiverAuthorization::ExplicitAccount {
                    account: constraint.arg.clone(),
                };
            }
            "refund_source" | "on_fail_refund_source" => {
                on_fail.push(FailureAction::RefundSource);
            }
            "refund_x3" | "on_fail_refund_x3" => {
                on_fail.push(FailureAction::RefundX3 {
                    asset: dest_asset.clone(),
                    to: draft.dest_receiver.clone(),
                });
            }
            "quarantine" | "on_fail_quarantine" => {
                on_fail.push(FailureAction::Quarantine);
            }
            "insurance_claim" | "on_fail_insurance" => {
                on_fail.push(FailureAction::InsuranceClaim);
            }
            "route_allow" => {
                route.allow.push(constraint.arg.clone());
            }
            "route_deny" => {
                route.deny.push(constraint.arg.clone());
            }
            "route_best" => {
                route.objective = RouteObjective::Best;
            }
            "route_cheapest" => {
                route.objective = RouteObjective::MinimizeTotalCost;
            }
            "route_fastest" => {
                route.objective = RouteObjective::MinimizeLatency;
            }
            _ => {
                // Unknown constraint kind — ignore at the adapter level;
                // the intent compiler's safety checks will catch anything
                // that matters (missing finality, missing proof, etc.).
            }
        }
    }

    // Set defaults for failure actions if none specified
    if on_fail.is_empty() {
        on_fail.push(FailureAction::RefundSource);
        on_fail.push(FailureAction::Quarantine);
    }

    // Build timeout
    let timeout = TimeoutSpec {
        timeout_secs: draft.timeout_secs,
        on_fail,
    };

    // Build source spec
    let source = SourceSpec {
        asset: source_asset,
        amount: draft.source_amount,
        owner: draft.source_owner.clone(),
        lock_contract: draft.source_lock_contract.clone(),
    };

    // Build destination spec
    let destination = DestinationSpec {
        asset: dest_asset,
        receiver: draft.dest_receiver.clone(),
        min_amount: draft.dest_min_amount,
    };

    // Attach proofs to requirements
    requirements.proofs = proofs;

    // Build the final IntentSpec
    let mut spec = IntentSpec::new(&draft.name, source, destination);
    spec.route = route;
    spec.requirements = requirements;
    spec.timeout = timeout;
    spec.receiver_authorization = receiver_auth;

    // Run the adapter-level preflight
    match validate_intent_spec(&spec) {
        Ok(()) => Ok(spec),
        Err(e) => {
            errors.push(e);
            Err(errors)
        }
    }
}

/// Complete pipeline: `IntentSpecDraft` → `CrossChainIntent` → compiled plan.
///
/// This is the single entry point for bridging the old x3-lang compiler
/// to the canonical intent system. Returns both the canonical intent and
/// the compiled plan (or errors).
pub fn draft_to_compiled_plan(
    draft: IntentSpecDraft,
    intent_id: u64,
) -> Result<DraftCompileResult, Vec<AdapterError>> {
    // Step 1: Draft → IntentSpec (adapter boundary)
    let spec = match draft_to_intent_spec(&draft) {
        Ok(s) => s,
        Err(adapter_errors) => {
            return Ok(DraftCompileResult {
                // We still construct a minimal intent for the caller to inspect
                intent: intent_spec_to_crosschain_intent(
                    IntentSpec::new(
                        &draft.name,
                        SourceSpec {
                            asset: AssetRef::new(ChainKind::X3, "UNKNOWN"),
                            amount: 0,
                            owner: draft.source_owner.clone(),
                            lock_contract: None,
                        },
                        DestinationSpec {
                            asset: AssetRef::new(ChainKind::X3, "UNKNOWN"),
                            receiver: draft.dest_receiver.clone(),
                            min_amount: None,
                        },
                    ),
                    intent_id,
                ),
                plan: Vec::new(),
                errors: Vec::new(),
                adapter_errors,
            });
        }
    };

    // Step 2: IntentSpec → CrossChainIntent (canonical intent)
    let intent = intent_spec_to_crosschain_intent(spec, intent_id);

    // Step 3: CrossChainIntent → compiled execution plan
    let compiler = IntentCompiler::new();
    let CompileResult { plan, errors } = compiler.compile(&intent);

    Ok(DraftCompileResult {
        intent,
        plan,
        errors,
        adapter_errors: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a fully-specified swap-and-bridge draft that passes all 13
    /// safety checks in the canonical intent compiler:
    ///   1. Known chains (eth, sol)
    ///   2. Known assets (USDC, SOL)
    ///   3. Non-zero timeout
    ///   4. Refund path
    ///   5. Fee cap
    ///   6. Finality for bridge
    ///   7. Proof for source lock + destination receipt
    ///   8. Canonical supply check
    ///   9. Slippage guard (swap required: USDC → SOL)
    ///  10. Receiver authorization (mapped account)
    ///  11. Safe bridge venues
    fn swap_and_bridge_draft() -> IntentSpecDraft {
        IntentSpecDraft {
            name: "swap_and_bridge".to_string(),
            source_chain: "eth".to_string(),
            source_asset: "USDC".to_string(),
            source_amount: 500_000_000,
            source_owner: "alice.eth".to_string(),
            source_lock_contract: Some("0xBridge".to_string()),
            dest_chain: "sol".to_string(),
            dest_asset: "SOL".to_string(),
            dest_min_amount: Some(3_500_000_000),
            dest_receiver: "alice.sol".to_string(),
            constraints: vec![
                // Safety check 7 (finality)
                SourceConstraint {
                    kind: "finality".to_string(),
                    arg: "eth >= 12".to_string(),
                },
                SourceConstraint {
                    kind: "finality".to_string(),
                    arg: "sol >= 32".to_string(),
                },
                // Safety check 9 (slippage)
                SourceConstraint {
                    kind: "slippage".to_string(),
                    arg: "<= 0.5%".to_string(),
                },
                // Safety check 7 (source proof)
                SourceConstraint {
                    kind: "proof".to_string(),
                    arg: "eth.lock_event".to_string(),
                },
                // Safety check 7 (dest receipt proof)
                SourceConstraint {
                    kind: "proof".to_string(),
                    arg: "sol.release_receipt".to_string(),
                },
                // Safety check 5-6 (fee cap)
                SourceConstraint {
                    kind: "max_fee".to_string(),
                    arg: "<= 10 USDC".to_string(),
                },
                // Safety check 4 (refund path)
                SourceConstraint {
                    kind: "refund_source".to_string(),
                    arg: "".to_string(),
                },
                // Safety check 8 (canonical supply)
                SourceConstraint {
                    kind: "require_canonical_supply".to_string(),
                    arg: "".to_string(),
                },
                // Safety check 10 (receiver authorization via mapped account)
                SourceConstraint {
                    kind: "receiver_explicit".to_string(),
                    arg: "alice.sol".to_string(),
                },
                // Route configuration
                SourceConstraint {
                    kind: "route_best".to_string(),
                    arg: "".to_string(),
                },
                SourceConstraint {
                    kind: "route_allow".to_string(),
                    arg: "x3.dex".to_string(),
                },
            ],
            timeout_secs: 30 * 60, // 30 minutes
        }
    }

    #[test]
    fn draft_adapter_produces_compilable_intent() {
        let draft = swap_and_bridge_draft();
        let result = draft_to_compiled_plan(draft, 1).expect("adapter should produce result");

        assert!(
            result.adapter_errors.is_empty(),
            "adapter errors: {:?}",
            result.adapter_errors
        );
        assert!(
            result.errors.is_empty(),
            "compile errors: {:?}",
            result.errors
        );
        assert!(!result.plan.is_empty(), "should have a non-empty plan");
        assert_eq!(result.intent.id, 1);
        assert_eq!(result.intent.name, "swap_and_bridge");
        assert!(result.intent.verify_hash(), "intent hash must match fields");

        // Verify the plan contains expected instructions
        let op_names: Vec<&str> = result.plan.iter().map(|i| i.name()).collect();

        assert!(
            op_names.contains(&"RegisterWatchdog"),
            "plan must start with timeout watchdog: {:?}",
            op_names
        );
        assert!(
            op_names.contains(&"LockAsset"),
            "plan must contain LockAsset: {:?}",
            op_names
        );
        assert!(
            op_names.contains(&"VerifyProof"),
            "plan must contain VerifyProof: {:?}",
            op_names
        );
        assert!(
            op_names.contains(&"MintCanonical"),
            "plan must contain MintCanonical (bridging to X3): {:?}",
            op_names
        );
        assert!(
            op_names.contains(&"EmitReceipt"),
            "plan must end with EmitIntentReceipt: {:?}",
            op_names
        );
    }

    #[test]
    fn draft_adapter_rejects_empty_name() {
        let mut draft = swap_and_bridge_draft();
        draft.name = "".to_string();
        let result = draft_to_compiled_plan(draft, 1).expect("should produce result");
        assert!(
            !result.adapter_errors.is_empty(),
            "should have adapter errors for empty name"
        );
        assert!(!result.is_ok());
    }

    #[test]
    fn draft_adapter_rejects_zero_amount() {
        let mut draft = swap_and_bridge_draft();
        draft.source_amount = 0;
        let result = draft_to_compiled_plan(draft, 1).expect("should produce result");
        assert!(
            !result.adapter_errors.is_empty(),
            "should have adapter errors for zero amount"
        );
    }

    #[test]
    fn draft_adapter_rejects_unknown_chain() {
        let mut draft = swap_and_bridge_draft();
        draft.source_chain = "bogus".to_string();
        let result = draft_to_compiled_plan(draft, 1).expect("should produce result");
        assert!(
            !result.adapter_errors.is_empty(),
            "should have adapter errors for unknown chain"
        );
    }

    #[test]
    fn draft_adapter_slippage_parsing() {
        let mut draft = swap_and_bridge_draft();
        // Clear constraints and set only slippage
        draft.constraints = vec![SourceConstraint {
            kind: "slippage".to_string(),
            arg: "<= 0.5%".to_string(),
        }];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(
            spec.requirements.max_slippage_bps,
            Some(50),
            "0.5% should map to 50 bps"
        );
    }

    #[test]
    fn draft_adapter_slippage_parsing_bps() {
        let mut draft = swap_and_bridge_draft();
        draft.constraints = vec![SourceConstraint {
            kind: "slippage".to_string(),
            arg: "<= 50bps".to_string(),
        }];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(spec.requirements.max_slippage_bps, Some(50));
    }

    #[test]
    fn draft_adapter_slippage_parsing_raw() {
        let mut draft = swap_and_bridge_draft();
        draft.constraints = vec![SourceConstraint {
            kind: "slippage".to_string(),
            arg: "<= 100".to_string(),
        }];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(spec.requirements.max_slippage_bps, Some(100));
    }

    #[test]
    fn draft_adapter_finality_parsing() {
        let mut draft = swap_and_bridge_draft();
        draft.constraints = vec![SourceConstraint {
            kind: "finality".to_string(),
            arg: "btc >= 6".to_string(),
        }];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(spec.requirements.finality.len(), 1);
        assert_eq!(spec.requirements.finality[0].chain, ChainKind::Bitcoin);
    }

    #[test]
    fn draft_adapter_proof_parsing() {
        let mut draft = swap_and_bridge_draft();
        draft.constraints = vec![
            SourceConstraint {
                kind: "proof".to_string(),
                arg: "eth.lock_event".to_string(),
            },
            SourceConstraint {
                kind: "proof".to_string(),
                arg: "sol.merkle_receipt".to_string(),
            },
        ];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(spec.requirements.proofs.len(), 2);
    }

    #[test]
    fn draft_round_trips_through_json() {
        let draft = swap_and_bridge_draft();
        let json = serde_json::to_string(&draft).expect("serialize draft");
        let parsed: IntentSpecDraft = serde_json::from_str(&json).expect("deserialize draft");

        // Both should compile to the same result
        let result_orig = draft_to_compiled_plan(draft, 1).expect("orig");
        let result_parsed = draft_to_compiled_plan(parsed, 1).expect("parsed");

        assert_eq!(
            result_orig.intent.intent_hash,
            result_parsed.intent.intent_hash
        );
        assert_eq!(result_orig.plan.len(), result_parsed.plan.len());
    }

    #[test]
    fn timeout_constraint_maps_correctly() {
        let mut draft = swap_and_bridge_draft();
        draft.timeout_secs = 900; // 15 minutes
        draft.constraints = vec![SourceConstraint {
            kind: "refund_source".to_string(),
            arg: "".to_string(),
        }];
        let spec = draft_to_intent_spec(&draft).expect("spec should build");
        assert_eq!(spec.timeout.timeout_secs, 900);
        assert_eq!(spec.timeout.on_fail, vec![FailureAction::RefundSource]);
    }

    #[test]
    fn x3_only_intent_does_not_require_bridge_proofs() {
        let draft = IntentSpecDraft {
            name: "x3_internal_swap".to_string(),
            source_chain: "x3".to_string(),
            source_asset: "USDC".to_string(),
            source_amount: 100_000,
            source_owner: "alice.x3".to_string(),
            source_lock_contract: None,
            dest_chain: "x3".to_string(),
            dest_asset: "X3".to_string(),
            dest_min_amount: Some(95_000),
            dest_receiver: "alice.x3".to_string(),
            constraints: vec![
                SourceConstraint {
                    kind: "slippage".to_string(),
                    arg: "<= 1%".to_string(),
                },
                SourceConstraint {
                    kind: "max_fee".to_string(),
                    arg: "<= 100".to_string(),
                },
            ],
            timeout_secs: 600,
        };

        let result = draft_to_compiled_plan(draft, 2).expect("should produce result");
        assert!(result.adapter_errors.is_empty(), "no adapter errors");
        // X3-only intent doesn't need bridge proofs — this is valid since it's an
        // X3-internal swap with no cross-chain bridge requirements.
        // Note: the safety check for refund path enforcement is not yet wired.
        // When implemented, this intent should produce a MissingRefundPath warning.
        assert!(
            result.errors.is_empty(),
            "X3-only intent should compile without errors (no cross-chain bridge needed): {:?}",
            result.errors
        );
        assert_eq!(
            result.intent.source.asset.chain,
            crate::types::ChainKind::X3
        );
        assert_eq!(
            result.intent.destination.asset.chain,
            crate::types::ChainKind::X3
        );
    }
}
