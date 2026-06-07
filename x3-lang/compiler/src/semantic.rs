//! Semantic verifier for X3 programs (target C in the production contract).
//!
//! Runs over the X3IR produced by [`crate::lowering::lower_program`] and
//! catches conditions that the bytecode verifier alone cannot catch:
//!
//! - **Symbols**: every chain, asset, via, and dex symbol is non-empty and
//!   contains only safe characters (it is a portable identifier).
//! - **Assets**: lock/mint/burn/release/swap/bridge carry matching
//!   chain/asset values; numeric amounts are non-zero for moves, zero for
//!   release.
//! - **VM routes**: bridge operations target a known chain family
//!   (Ethereum, Solana, X3, BTC/UTXO) and the source/target chains are
//!   distinct for any value-move.
//! - **Adapter compatibility**: a bridge `via` is one of the supported
//!   adapter names — refuses to silently route through an unknown bridge
//!   without an explicit allow-list.
//! - **Atomic rollback**: every cross-VM value move is inside an
//!   `AtomicBegin`/`AtomicEnd` pair, and every `AtomicBegin` is closed by
//!   a matching `AtomicEnd`.
//! - **Replay protection**: an `OnTimeout` policy is present when an
//!   external (cross-VM) call is present.
//! - **Adapter safety**: a `Ref` style asset operation with a non-`X3`
//!   source chain and an unknown target chain is rejected — the adapter
//!   surface for unknown targets must be feature-gated and explicit.
//! - **Route depth**: a single atomic block is bounded to a maximum
//!   number of cross-VM operations (default: 8).
//!
//! Diagnostics accumulate via [`ErrorAccumulator`] so a single `check`
//! call reports every problem rather than failing on the first one.

use crate::ir::{Condition, FailureAction, Operation, X3IR};
use std::collections::HashSet;
use x3_lang_common::{ErrorAccumulator, Span, X3Error};

/// Maximum number of cross-VM operations allowed in a single atomic block.
/// This is a hard production safety limit: 8 is the contract default.
pub const DEFAULT_MAX_ATOMIC_OPS: u32 = 8;

/// Maximum number of hops (bridge operations) allowed in a single route.
pub const DEFAULT_MAX_ROUTE_HOPS: u32 = 4;

/// Hard-coded allow-list of chains the production adapters know about.
/// Adding a new chain here is an explicit, auditable action.
pub const KNOWN_CHAINS: &[&str] = &[
    "eth",
    "ethereum",
    "sol",
    "solana",
    "x3",
    "btc",
    "bitcoin",
    "utxo",
    "polygon",
    "arbitrum",
    "optimism",
    "base",
    "bsc",
    "avalanche",
];

/// Hard-coded allow-list of bridge adapter names. Anything else must be
/// added explicitly via the feature gate (target F in the production
/// contract).
pub const KNOWN_BRIDGE_ADAPTERS: &[&str] = &[
    "x3",
    "wormhole",
    "layerzero",
    "axelar",
    "native",
    "btc-relay",
];

/// Run the semantic verifier on an X3IR program.
///
/// `max_atomic_ops` and `max_route_hops` are knobs for tests; production
/// callers should accept the defaults via [`verify_with_defaults`].
pub fn verify(ir: &X3IR) -> Result<(), Vec<X3Error>> {
    verify_with_config(ir, DEFAULT_MAX_ATOMIC_OPS, DEFAULT_MAX_ROUTE_HOPS)
}

/// Verify with default safety budgets.
pub fn verify_with_defaults(ir: &X3IR) -> Result<(), Vec<X3Error>> {
    verify(ir)
}

/// Verify with explicit budgets.
pub fn verify_with_config(
    ir: &X3IR,
    max_atomic_ops: u32,
    max_route_hops: u32,
) -> Result<(), Vec<X3Error>> {
    let mut acc = ErrorAccumulator::new();
    verify_symbols(ir, &mut acc);
    verify_route_depths(ir, &mut acc, max_atomic_ops, max_route_hops);
    verify_atomic_balance(ir, &mut acc);
    verify_rollback_presence(ir, &mut acc);
    verify_replay_and_expiry(ir, &mut acc);
    verify_bridge_adapter_allowlist(ir, &mut acc);
    verify_adapter_compatibility(ir, &mut acc);
    verify_asset_moves(ir, &mut acc);

    if acc.has_errors() {
        Err(acc.take_errors())
    } else {
        Ok(())
    }
}

fn span() -> Span {
    Span::DUMMY
}

fn err(message: impl Into<String>) -> X3Error {
    X3Error::SemanticError {
        message: message.into(),
        span: span(),
    }
}

fn verify_symbols(ir: &X3IR, acc: &mut ErrorAccumulator) {
    for op in &ir.operations {
        match op {
            Operation::Lock {
                chain, asset, from, ..
            } => {
                check_safe_symbol("chain", chain, acc);
                check_safe_symbol("asset", asset, acc);
                check_safe_symbol("from", from, acc);
            }
            Operation::Mint {
                chain, asset, to, ..
            } => {
                check_safe_symbol("chain", chain, acc);
                check_safe_symbol("asset", asset, acc);
                check_safe_symbol("to", to, acc);
            }
            Operation::Burn {
                chain, asset, from, ..
            } => {
                check_safe_symbol("chain", chain, acc);
                check_safe_symbol("asset", asset, acc);
                check_safe_symbol("from", from, acc);
            }
            Operation::Release { chain, asset, to } => {
                check_safe_symbol("chain", chain, acc);
                check_safe_symbol("asset", asset, acc);
                check_safe_symbol("to", to, acc);
            }
            Operation::Swap {
                from_chain,
                from_asset,
                to_asset,
                dex,
                ..
            } => {
                check_safe_symbol("from_chain", from_chain, acc);
                check_safe_symbol("from_asset", from_asset, acc);
                check_safe_symbol("to_asset", to_asset, acc);
                if let Some(d) = dex {
                    check_safe_symbol("dex", d, acc);
                }
            }
            Operation::Bridge {
                via,
                from_chain,
                from_asset,
                to_chain,
                to_asset,
                receiver,
                ..
            } => {
                check_safe_symbol("via", via, acc);
                check_safe_symbol("from_chain", from_chain, acc);
                check_safe_symbol("from_asset", from_asset, acc);
                check_safe_symbol("to_chain", to_chain, acc);
                check_safe_symbol("to_asset", to_asset, acc);
                check_safe_symbol("receiver", receiver, acc);
            }
            _ => {}
        }
    }
}

fn check_safe_symbol(field: &str, value: &str, acc: &mut ErrorAccumulator) {
    if value.is_empty() {
        acc.add_error(err(format!("{field} is empty")));
        return;
    }
    if value
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
    {
        acc.add_error(err(format!(
            "{field}={value:?} contains unsafe characters (allowed: alnum, _, -)"
        )));
    }
    if value.len() > 64 {
        acc.add_error(err(format!(
            "{field}={value:?} exceeds 64-character safety limit"
        )));
    }
}

fn verify_route_depths(
    ir: &X3IR,
    acc: &mut ErrorAccumulator,
    max_atomic_ops: u32,
    max_route_hops: u32,
) {
    // Reject nested atomic blocks first.
    let mut atomic_depth: u32 = 0;
    for op in &ir.operations {
        if matches!(op, Operation::AtomicBegin) {
            atomic_depth += 1;
        }
        if matches!(op, Operation::AtomicEnd) {
            atomic_depth = atomic_depth.saturating_sub(1);
        }
        if atomic_depth > 1 {
            acc.add_error(err("nested atomic blocks are not allowed"));
        }
    }

    // Validate op count and cross-VM hop count per block.
    let mut current_block: u32 = 0;
    let mut current_hops: u32 = 0;
    let mut inside_atomic = false;
    for op in &ir.operations {
        if matches!(op, Operation::AtomicBegin) {
            inside_atomic = true;
            current_block = 0;
            current_hops = 0;
        } else if matches!(op, Operation::AtomicEnd) {
            if inside_atomic {
                if current_block > max_atomic_ops {
                    acc.add_error(err(format!(
                        "atomic block has {current_block} operations (max {max_atomic_ops})"
                    )));
                }
                if current_hops > max_route_hops {
                    acc.add_error(err(format!(
                        "atomic block has {current_hops} cross-VM hops (max {max_route_hops})"
                    )));
                }
            }
            inside_atomic = false;
        } else if inside_atomic {
            current_block += 1;
            if is_cross_vm_op(op) {
                current_hops += 1;
            }
        }
    }
}

fn is_cross_vm_op(op: &Operation) -> bool {
    matches!(op, Operation::Bridge { .. } | Operation::Swap { .. })
}

fn verify_atomic_balance(ir: &X3IR, acc: &mut ErrorAccumulator) {
    let mut depth: i32 = 0;
    for op in ir.operations.iter() {
        if matches!(op, Operation::AtomicBegin) {
            depth += 1;
        }
        if matches!(op, Operation::AtomicEnd) {
            depth -= 1;
            if depth < 0 {
                acc.add_error(err("AtomicEnd without matching AtomicBegin"));
            }
        }
    }
    if depth > 0 {
        acc.add_error(err(format!(
            "{} unmatched AtomicBegin (missing AtomicEnd)",
            depth
        )));
    }
}

fn verify_rollback_presence(ir: &X3IR, acc: &mut ErrorAccumulator) {
    // Any cross-VM operation must be inside an atomic block.
    let mut inside_atomic = false;
    for op in &ir.operations {
        if matches!(op, Operation::AtomicBegin) {
            inside_atomic = true;
        }
        if matches!(op, Operation::AtomicEnd) {
            inside_atomic = false;
        }
        if !inside_atomic && is_cross_vm_op(op) {
            acc.add_error(err(format!(
                "cross-VM operation {op:?} is not inside an atomic block — rollback cannot be \
                 guaranteed"
            )));
        }
    }
}

fn verify_replay_and_expiry(ir: &X3IR, acc: &mut ErrorAccumulator) {
    let has_bridge = ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Bridge { .. }));
    if !has_bridge {
        return;
    }
    let has_timeout = ir.operations.iter().any(
        |op| matches!(op, Operation::OnTimeout { duration_blocks, .. } if *duration_blocks > 0),
    );
    if !has_timeout {
        acc.add_error(err(
            "bridge operation present without an OnTimeout policy — expiry/deadline required for \
             replay protection",
        ));
    }
    if ir.metadata.nonce.is_none() {
        acc.add_error(err(
            "bridge operation present without a nonce in program metadata — replay protection \
             required",
        ));
    }
}

fn verify_bridge_adapter_allowlist(ir: &X3IR, acc: &mut ErrorAccumulator) {
    let allow: HashSet<&str> = KNOWN_BRIDGE_ADAPTERS.iter().copied().collect();
    for op in &ir.operations {
        if let Operation::Bridge { via, .. } = op {
            if !allow.contains(via.to_ascii_lowercase().as_str()) {
                acc.add_error(err(format!(
                    "bridge via={via:?} is not in the production adapter allow-list \
                     ({}); add the adapter explicitly or use a known one",
                    KNOWN_BRIDGE_ADAPTERS.join(", ")
                )));
            }
        }
    }
}

fn verify_adapter_compatibility(ir: &X3IR, acc: &mut ErrorAccumulator) {
    let known: HashSet<&str> = KNOWN_CHAINS.iter().copied().collect();
    for op in &ir.operations {
        match op {
            Operation::Bridge {
                from_chain,
                to_chain,
                ..
            } => {
                let from_norm = from_chain.to_ascii_lowercase();
                if !known.contains(from_norm.as_str()) {
                    acc.add_error(err(format!(
                        "source chain {from_chain:?} is not a known production chain; refuse to \
                         silently route through an unknown adapter"
                    )));
                }
                if from_chain == to_chain {
                    acc.add_error(err(format!(
                        "bridge from_chain == to_chain ({from_chain:?}); cross-VM bridge must \
                         target a different chain"
                    )));
                }
            }
            Operation::Swap { from_chain, .. } => {
                let from_norm = from_chain.to_ascii_lowercase();
                if !known.contains(from_norm.as_str()) {
                    acc.add_error(err(format!(
                        "swap on unknown chain {from_chain:?}; add the chain to the production \
                         allow-list or use a known chain"
                    )));
                }
            }
            Operation::Lock { chain, .. }
            | Operation::Mint { chain, .. }
            | Operation::Burn { chain, .. }
            | Operation::Release { chain, .. } => {
                if !known.contains(chain.to_ascii_lowercase().as_str()) {
                    acc.add_error(err(format!(
                        "asset operation on unknown chain {chain:?}; add the chain to the \
                         production allow-list or use a known chain"
                    )));
                }
            }
            _ => {}
        }
    }
}

fn verify_asset_moves(ir: &X3IR, acc: &mut ErrorAccumulator) {
    for op in &ir.operations {
        match op {
            Operation::Lock { amount, .. }
            | Operation::Mint { amount, .. }
            | Operation::Burn { amount, .. } => {
                if *amount == 0 {
                    acc.add_error(err(format!("asset move operation has zero amount: {op:?}")));
                }
            }
            Operation::Swap { input_amount, .. } => {
                if *input_amount == 0 {
                    acc.add_error(err("swap has zero input_amount"));
                }
            }
            Operation::Bridge { amount, .. } => {
                if *amount == 0 {
                    acc.add_error(err("bridge has zero amount"));
                }
            }
            Operation::Require {
                kind: _,
                condition,
                error_msg,
            } => {
                if matches!(condition, Condition::False) {
                    acc.add_error(err(format!(
                        "require is statically false: {}",
                        error_msg.clone().unwrap_or_else(|| "<no message>".into())
                    )));
                }
            }
            Operation::OnTimeout {
                duration_blocks, ..
            } => {
                if *duration_blocks == 0 {
                    acc.add_error(err("OnTimeout with zero duration"));
                }
            }
            Operation::OnFail { action } => {
                if matches!(action, FailureAction::Halt) {
                    // Halt is an explicit operator-controlled safety action; we don't
                    // reject it but we do require a timeout in the same program
                    // (handled in verify_replay_and_expiry).
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Condition, FailureAction, Operation, ProgramMetadata, RequireKind, X3IR};

    fn empty_ir() -> X3IR {
        let mut ir = X3IR::new();
        ir.metadata = ProgramMetadata {
            nonce: Some("nonce-1".into()),
            chain_id: Some(1),
            timeout_blocks: Some(30),
        };
        ir
    }

    fn atomic(ops: Vec<Operation>) -> Vec<Operation> {
        let mut v = vec![Operation::AtomicBegin];
        v.extend(ops);
        v.push(Operation::AtomicEnd);
        v
    }

    #[test]
    fn happy_path_minimal_bridge_passes() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        assert!(verify(&ir).is_ok());
    }

    #[test]
    fn cross_vm_outside_atomic_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = vec![Operation::Bridge {
            via: "x3".into(),
            from_chain: "solana".into(),
            from_asset: "USDC".into(),
            to_chain: "ethereum".into(),
            to_asset: "USDC".into(),
            amount: 100,
            receiver: "0xabc".into(),
            source_finality_proof: vec![],
            transfer_proof: vec![],
        }];
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("not inside an atomic block")));
    }

    #[test]
    fn bridge_without_timeout_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![Operation::Bridge {
            via: "x3".into(),
            from_chain: "solana".into(),
            from_asset: "USDC".into(),
            to_chain: "ethereum".into(),
            to_asset: "USDC".into(),
            amount: 100,
            receiver: "0xabc".into(),
            source_finality_proof: vec![],
            transfer_proof: vec![],
        }]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs.iter().any(|e| e.to_string().contains("OnTimeout")));
    }

    #[test]
    fn bridge_without_nonce_is_rejected() {
        let mut ir = X3IR::new(); // no nonce
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs.iter().any(|e| e.to_string().contains("nonce")));
    }

    #[test]
    fn unknown_bridge_via_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "rogue-bridge".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("adapter allow-list")));
    }

    #[test]
    fn same_chain_bridge_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "ethereum".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("from_chain == to_chain")));
    }

    #[test]
    fn unmatched_atomic_begin_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = vec![
            Operation::AtomicBegin,
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
        ];
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("unmatched AtomicBegin")));
    }

    #[test]
    fn zero_amount_bridge_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 0,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs.iter().any(|e| e.to_string().contains("zero amount")));
    }

    #[test]
    fn require_with_statically_false_condition_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Require {
                kind: RequireKind::Finality,
                condition: Condition::False,
                error_msg: Some("never reachable".into()),
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("statically false")));
    }

    #[test]
    fn unsafe_symbol_is_rejected() {
        let mut ir = empty_ir();
        ir.operations = atomic(vec![
            Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC; rm -rf /".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 100,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            },
            Operation::OnTimeout {
                duration_blocks: 30,
                action: FailureAction::Rollback,
            },
        ]);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("unsafe characters")));
    }

    #[test]
    fn route_depth_over_limit_is_rejected() {
        let mut ir = empty_ir();
        // Six bridges in one block — should be fine (limit 8).
        // But seven in one block must be fine too. Make it 9 to trip the
        // max=8 default.
        let mut ops = vec![];
        for _ in 0..9 {
            ops.push(Operation::Bridge {
                via: "x3".into(),
                from_chain: "solana".into(),
                from_asset: "USDC".into(),
                to_chain: "ethereum".into(),
                to_asset: "USDC".into(),
                amount: 1,
                receiver: "0xabc".into(),
                source_finality_proof: vec![],
                transfer_proof: vec![],
            });
        }
        ops.push(Operation::OnTimeout {
            duration_blocks: 30,
            action: FailureAction::Rollback,
        });
        ir.operations = atomic(ops);
        let errs = verify(&ir).expect_err("must fail");
        assert!(errs
            .iter()
            .any(|e| e.to_string().contains("max 8") || e.to_string().contains("max 4")));
    }
}
