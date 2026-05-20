//! Intent compiler: CrossChainIntent → Vec<X3Instruction>
//!
//! The compiler transforms a validated [`CrossChainIntent`] into an ordered
//! execution plan of [`X3Instruction`]s. Before emitting any instructions,
//! it runs **13 safety checks** and returns all errors found — not just the
//! first — so the user can fix everything at once.
//!
//! ## Safety Check Order
//!
//! The compiler processes checks in this order. All checks are run regardless
//! of earlier failures (collect-all-errors approach):
//!
//! 1. Unknown/unsupported chains (X3-INTENT-008)
//! 2. Unknown/ambiguous assets (X3-INTENT-009)
//! 3. Missing timeout (X3-INTENT-003)
//! 4. Missing refund path (X3-INTENT-004)
//! 5. Unbounded execution — no fee cap AND no timeout (X3-INTENT-013)
//! 6. Missing fee cap (X3-INTENT-007)
//! 7. Missing finality for bridge (X3-INTENT-001)
//! 8. Insufficient finality level (below chain safe minimum)
//! 9. Missing proof for bridge/mint/release (X3-INTENT-002)
//! 10. Missing canonical supply check for mints (X3-INTENT-011)
//! 11. Missing slippage guard for swaps (X3-INTENT-006)
//! 12. Missing receiver validation (X3-INTENT-005)
//! 13. Unsafe bridge venues in route (X3-INTENT-010)
//!
//! ## Instruction Emission
//!
//! After all checks pass, the compiler emits instructions in this order:
//!
//! 1. RegisterTimeoutWatchdog
//! 2. ValidateWalletOwner
//! 3. CheckBalance
//! 4. SimulateExecution (if require_route_simulated)
//! 5. QuoteBestRoute
//! 6. LockAsset
//! 7. WaitFinality (source)
//! 8. VerifyProof (source lock)
//! 9. CheckCanonicalSupply (if bridging to X3)
//! 10. MintCanonical (if bridging to X3)
//! 11. ExecuteSwap (if swap required)
//! 12. BridgeToDestination / ReleaseDestination (if dest ≠ X3)
//! 13. WaitFinality (destination, if cross-chain)
//! 14. VerifyProof (destination receipt, if required)
//! 15. VerifyFinalReceipt
//! 16. EmitIntentReceipt

use crate::error::IntentCompileError;
use crate::instructions::{TimeoutAction, X3Instruction};
use crate::intent::CrossChainIntent;
use crate::types::{
    AssetRef, ChainKind, FailureAction, FinalityLevel, ProofKind,
};

/// The result of intent compilation.
///
/// `errors` is always populated if compilation failed.
/// `plan` is populated only when `errors` is empty.
#[derive(Debug)]
pub struct CompileResult {
    /// Ordered list of instructions to execute, or empty if errors exist.
    pub plan: Vec<X3Instruction>,
    /// All compile errors found. Non-empty means compilation failed.
    pub errors: Vec<IntentCompileError>,
}

impl CompileResult {
    pub fn ok(plan: Vec<X3Instruction>) -> Self {
        Self {
            plan,
            errors: Vec::new(),
        }
    }

    pub fn failed(errors: Vec<IntentCompileError>) -> Self {
        Self {
            plan: Vec::new(),
            errors,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Cross-chain intent compiler.
///
/// Stateless — all state comes from the [`CrossChainIntent`] being compiled.
/// Construct with [`IntentCompiler::new`] and call [`IntentCompiler::compile`].
pub struct IntentCompiler;

impl IntentCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a [`CrossChainIntent`] into an execution plan.
    ///
    /// Runs all 13 safety checks. Returns all errors found, not just the first.
    /// Returns an instruction plan only if all checks pass.
    pub fn compile(&self, intent: &CrossChainIntent) -> CompileResult {
        let mut errors: Vec<IntentCompileError> = Vec::new();

        // ── Safety check 1: Unknown chains ───────────────────────────────────
        let source_chain = intent.source.asset.chain;
        let dest_chain = intent.destination.asset.chain;

        // (ChainKind is an enum — only known chains can be constructed, so
        //  this check applies to parsed / externally-supplied string data.
        //  Here we verify they are X3-known chains for adapter availability.)
        if !Self::is_supported_chain(source_chain) {
            errors.push(IntentCompileError::UnknownChain {
                chain: source_chain.as_str().to_string(),
            });
        }
        if !Self::is_supported_chain(dest_chain) {
            errors.push(IntentCompileError::UnknownChain {
                chain: dest_chain.as_str().to_string(),
            });
        }

        // ── Safety check 2: Unknown/ambiguous assets ─────────────────────────
        if intent.source.asset.symbol.is_empty() {
            errors.push(IntentCompileError::UnknownAsset {
                asset: intent.source.asset.display(),
                chain: source_chain.as_str().to_string(),
            });
        }
        if intent.destination.asset.symbol.is_empty() {
            errors.push(IntentCompileError::UnknownAsset {
                asset: intent.destination.asset.display(),
                chain: dest_chain.as_str().to_string(),
            });
        }

        // ── Safety check 3: Missing timeout ──────────────────────────────────
        if intent.timeout.timeout_secs == 0 {
            errors.push(IntentCompileError::MissingTimeout);
        }

        // ── Safety check 4: Missing refund path ──────────────────────────────
        if intent.timeout.on_fail.is_empty() {
            errors.push(IntentCompileError::MissingRefundPath);
        }

        // ── Safety check 5: Unbounded execution ──────────────────────────────
        let has_fee_cap = intent.requirements.max_total_fee.is_some();
        let has_timeout = intent.timeout.timeout_secs > 0;
        if !has_fee_cap && !has_timeout {
            errors.push(IntentCompileError::UnboundedExecution);
        }

        // ── Safety check 6: Missing fee cap ──────────────────────────────────
        if !has_fee_cap {
            errors.push(IntentCompileError::MissingFeeCap {
                asset: intent.source.asset.symbol.clone(),
            });
        }

        // ── Safety check 7 & 8: Finality for bridge operations ───────────────
        if intent.requires_bridge() {
            let finality_for_source = intent
                .requirements
                .finality
                .iter()
                .find(|f| f.chain == source_chain);

            match finality_for_source {
                None => {
                    errors.push(IntentCompileError::MissingFinality {
                        chain: source_chain.as_str().to_string(),
                        asset: intent.source.asset.display(),
                        min_confirmations: source_chain.default_safe_confirmations(),
                    });
                }
                Some(req) => {
                    // Check 8: is finality level sufficient?
                    if !req.is_safe() {
                        if let FinalityLevel::Confirmations(n) = &req.level {
                            errors.push(IntentCompileError::InsufficientFinality {
                                chain: source_chain.as_str().to_string(),
                                specified: *n,
                                minimum: source_chain.default_safe_confirmations(),
                            });
                        }
                    }
                }
            }
        }

        // ── Safety check 9: Proof for bridge/mint/release ────────────────────
        if intent.requires_bridge() {
            // Must have a proof for the source lock event.
            let has_source_lock_proof = intent
                .requirements
                .proofs
                .iter()
                .any(|p| p.chain == source_chain);

            if !has_source_lock_proof {
                errors.push(IntentCompileError::MissingProof {
                    step: "LockAsset".to_string(),
                    chain: source_chain.as_str().to_string(),
                });
            }

            // If destination is a non-X3 chain, also need destination receipt proof.
            if dest_chain != ChainKind::X3 {
                let has_dest_proof = intent
                    .requirements
                    .proofs
                    .iter()
                    .any(|p| p.chain == dest_chain);

                if !has_dest_proof {
                    errors.push(IntentCompileError::MissingProof {
                        step: "ReleaseDestination".to_string(),
                        chain: dest_chain.as_str().to_string(),
                    });
                }
            }
        }

        // ── Safety check 10: Canonical supply check ───────────────────────────
        if intent.requires_bridge() && dest_chain == ChainKind::X3 {
            if !intent.requirements.require_canonical_supply_valid {
                errors.push(IntentCompileError::MissingCanonicalSupplyCheck {
                    asset: intent.destination.asset.display(),
                });
            }
        }
        // For full bridge-and-swap (source external → dest external via X3):
        if intent.requires_bridge()
            && source_chain != ChainKind::X3
            && !intent.requirements.require_canonical_supply_valid
        {
            // Will mint x3-canonical in the middle — still need supply check.
            errors.push(IntentCompileError::MissingCanonicalSupplyCheck {
                asset: format!(
                    "x3.{}.e (canonical wrap of {})",
                    intent.source.asset.symbol,
                    intent.source.asset.display()
                ),
            });
        }

        // ── Safety check 11: Slippage guard for swaps ────────────────────────
        if intent.requires_swap() && intent.requirements.max_slippage_bps.is_none() {
            errors.push(IntentCompileError::MissingSlippageGuard {
                from: intent.source.asset.display(),
                to: intent.destination.asset.display(),
            });
        }

        // ── Safety check 12: Receiver validation ─────────────────────────────
        if !intent.requirements.require_receiver_is_owner {
            errors.push(IntentCompileError::MissingReceiverValidation);
        }

        // ── Safety check 13: Unsafe bridge venues ────────────────────────────
        for venue in &intent.route.allow {
            if venue.starts_with("bridge.") && venue == "bridge.unknown" {
                errors.push(IntentCompileError::UnsafeRoute {
                    venue: venue.clone(),
                });
            }
        }
        // Explicitly denied venues should not appear in allow list.
        for denied in &intent.route.deny {
            if intent.route.allow.contains(denied) {
                errors.push(IntentCompileError::UnsafeRoute {
                    venue: denied.clone(),
                });
            }
        }

        // ── Abort if any errors ───────────────────────────────────────────────
        if !errors.is_empty() {
            return CompileResult::failed(errors);
        }

        // ── Emit execution plan ───────────────────────────────────────────────
        let plan = self.emit_plan(intent);
        CompileResult::ok(plan)
    }

    // ── Instruction emission ─────────────────────────────────────────────────

    fn emit_plan(&self, intent: &CrossChainIntent) -> Vec<X3Instruction> {
        let mut plan: Vec<X3Instruction> = Vec::new();
        let source_chain = intent.source.asset.chain;
        let dest_chain = intent.destination.asset.chain;
        let slippage_bps = intent.requirements.max_slippage_bps.unwrap_or(50);

        // Step 0: Register timeout watchdog (first — protects against partial execution)
        plan.push(X3Instruction::RegisterTimeoutWatchdog {
            intent_id: intent.id,
            timeout_secs: intent.timeout.timeout_secs,
            on_timeout_action: Self::map_first_on_fail_to_timeout_action(intent),
        });

        // Step 1: Validate owner controls the source address
        plan.push(X3Instruction::ValidateWalletOwner {
            owner: intent.source.owner.clone(),
            chain: source_chain,
        });

        // Step 2: Check source balance
        plan.push(X3Instruction::CheckBalance {
            asset: intent.source.asset.clone(),
            owner: intent.source.owner.clone(),
            required: intent.source.amount,
        });

        // Step 3: Simulate if required
        if intent.requirements.require_route_simulated {
            plan.push(X3Instruction::SimulateExecution {
                intent_id: intent.id,
            });
        }

        // Step 4: Quote the best route
        plan.push(X3Instruction::QuoteBestRoute {
            from: intent.source.asset.clone(),
            to: intent.destination.asset.clone(),
            amount: intent.source.amount,
            objective: format!("{:?}", intent.route.objective),
        });

        // Step 5: Lock source asset (first irreversible step)
        let lock_contract = intent
            .source
            .lock_contract
            .clone()
            .unwrap_or_else(|| format!("{}.bridge.{}", source_chain.as_str(), intent.source.asset.symbol));

        plan.push(X3Instruction::LockAsset {
            asset: intent.source.asset.clone(),
            amount: intent.source.amount,
            from_address: intent.source.owner.clone(),
            contract: lock_contract,
        });

        // Step 6: Wait for source finality
        if let Some(fin_req) = intent
            .requirements
            .finality
            .iter()
            .find(|f| f.chain == source_chain)
        {
            plan.push(X3Instruction::WaitFinality {
                chain: source_chain,
                level: fin_req.level.clone(),
                block_or_slot: None,
            });
        }

        // Step 7: Verify source lock proof
        for proof_req in intent
            .requirements
            .proofs
            .iter()
            .filter(|p| p.chain == source_chain)
        {
            plan.push(X3Instruction::VerifyProof {
                label: proof_req.label.clone(),
                chain: proof_req.chain,
                kind_tag: Self::proof_kind_tag(&proof_req.kind),
            });
        }

        // Step 8: Check canonical supply (before any mint)
        if source_chain != ChainKind::X3 {
            let canonical_asset = AssetRef::new(
                ChainKind::X3,
                format!("{}.e", intent.source.asset.symbol),
            );
            plan.push(X3Instruction::CheckCanonicalSupply {
                wrapped_asset: canonical_asset.clone(),
            });

            // Step 9: Mint canonical wrapped asset on X3
            let source_lock_label = intent
                .requirements
                .proofs
                .iter()
                .find(|p| p.chain == source_chain)
                .map(|p| p.label.clone())
                .unwrap_or_else(|| format!("{}.lock_event", source_chain.as_str()));

            plan.push(X3Instruction::MintCanonical {
                canonical_asset: canonical_asset.clone(),
                amount: intent.source.amount,
                to: format!("{}.x3", Self::strip_chain_prefix(&intent.source.owner)),
                proof_label: source_lock_label,
            });

            // Step 10: If swap required (canonical → destination asset)
            if intent.requires_swap() {
                let swap_dest = if dest_chain == ChainKind::X3 {
                    intent.destination.asset.clone()
                } else {
                    // Intermediate: x3.{dest_symbol} before releasing to dest chain
                    AssetRef::new(
                        ChainKind::X3,
                        intent.destination.asset.symbol.clone(),
                    )
                };

                let min_out = intent
                    .destination
                    .min_amount
                    .unwrap_or(intent.source.amount * 95 / 100); // 5% floor if unspecified

                plan.push(X3Instruction::ExecuteSwap {
                    from: canonical_asset,
                    to: swap_dest.clone(),
                    amount_in: intent.source.amount,
                    min_amount_out: min_out,
                    slippage_bps,
                    venue: intent
                        .route
                        .allow
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "x3.dex".to_string()),
                });
            }
        } else if intent.requires_swap() {
            // X3-native swap (no bridge)
            let min_out = intent
                .destination
                .min_amount
                .unwrap_or(intent.source.amount * 95 / 100);

            plan.push(X3Instruction::ExecuteSwap {
                from: intent.source.asset.clone(),
                to: intent.destination.asset.clone(),
                amount_in: intent.source.amount,
                min_amount_out: min_out,
                slippage_bps,
                venue: intent
                    .route
                    .allow
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "x3.dex".to_string()),
            });
        }

        // Step 11: Bridge to destination chain (if dest ≠ X3)
        if dest_chain != ChainKind::X3 {
            let amount = intent.destination.min_amount.unwrap_or(intent.source.amount);
            plan.push(X3Instruction::BridgeToDestination {
                asset: intent.destination.asset.clone(),
                amount,
                to_address: intent.destination.receiver.clone(),
                dest_chain,
            });

            // Step 12: Wait for destination finality
            if let Some(fin_req) = intent
                .requirements
                .finality
                .iter()
                .find(|f| f.chain == dest_chain)
            {
                plan.push(X3Instruction::WaitFinality {
                    chain: dest_chain,
                    level: fin_req.level.clone(),
                    block_or_slot: None,
                });
            }

            // Step 13: Verify destination receipt proof
            for proof_req in intent
                .requirements
                .proofs
                .iter()
                .filter(|p| p.chain == dest_chain)
            {
                plan.push(X3Instruction::VerifyProof {
                    label: proof_req.label.clone(),
                    chain: proof_req.chain,
                    kind_tag: Self::proof_kind_tag(&proof_req.kind),
                });
            }
        }

        // Step 14: Verify final receipt
        let min_out = intent
            .destination
            .min_amount
            .unwrap_or(intent.source.amount * 95 / 100);
        plan.push(X3Instruction::VerifyFinalReceipt {
            chain: dest_chain,
            expected_asset: intent.destination.asset.clone(),
            expected_min_amount: min_out,
            to_address: intent.destination.receiver.clone(),
        });

        // Step 15: Emit receipt
        plan.push(X3Instruction::EmitIntentReceipt {
            intent_id: intent.id,
            verbose: intent.receipt.include_state_transitions,
        });

        plan
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Currently all ChainKind variants are supported. This check is here
    /// to catch future cases where a chain might be deprecated or gated.
    fn is_supported_chain(_chain: ChainKind) -> bool {
        true
    }

    fn proof_kind_tag(kind: &ProofKind) -> String {
        match kind {
            ProofKind::EventProof { .. } => "EventProof".to_string(),
            ProofKind::MerkleProof { .. } => "MerkleProof".to_string(),
            ProofKind::LightClientProof { .. } => "LightClientProof".to_string(),
            ProofKind::ValidatorQuorum { .. } => "ValidatorQuorum".to_string(),
            ProofKind::ZkProof { .. } => "ZkProof".to_string(),
            ProofKind::SpvProof { .. } => "SpvProof".to_string(),
            ProofKind::GpuBatchReceipt => "GpuBatchReceipt".to_string(),
        }
    }

    fn map_first_on_fail_to_timeout_action(intent: &CrossChainIntent) -> TimeoutAction {
        match intent.timeout.on_fail.first() {
            Some(FailureAction::RefundSource) | None => TimeoutAction::RefundSource,
            Some(FailureAction::RefundX3 { asset, to }) => TimeoutAction::RefundX3 {
                asset: asset.clone(),
                to: to.clone(),
            },
            Some(FailureAction::Quarantine) | Some(FailureAction::InsuranceClaim) => {
                TimeoutAction::Quarantine
            }
            Some(FailureAction::RollbackIfPossible) => TimeoutAction::RefundSource,
            Some(FailureAction::RefundDestinationStable { .. }) => TimeoutAction::RefundSource,
        }
    }

    fn strip_chain_prefix(owner: &str) -> &str {
        // e.g. "alice.eth" → "alice"
        owner.split('.').next().unwrap_or(owner)
    }
}

impl Default for IntentCompiler {
    fn default() -> Self {
        Self::new()
    }
}
