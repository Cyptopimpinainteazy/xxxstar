//! Stateful verifier for the canonical trading execution boundary.
use crate::ir::{AssetKey, TradingOperation, ValueRef};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
    vec::Vec,
};
fn push_unsafe(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}
fn require_non_empty(errors: &mut Vec<String>, context: &str, field: &str, value: &str) {
    if value.trim().is_empty() {
        push_unsafe(errors, format!("{context}: {field} must not be empty"));
    }
}
pub fn verify_trading_program(ops: &[TradingOperation]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        verify_trading_operation(op, &format!("program[{i}]"), &mut errors);
    }
    verify_trading_sequences(ops, "program", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
#[derive(Default)]
struct TradingSequenceState {
    began: bool,
    committed: bool,
    aborted: bool,
    receipt_seen: bool,
    all_debts_guard_seen: bool,
    profit_guard_seen: bool,
    open_debts: BTreeMap<String, AssetKey>,
    closed_debts: BTreeSet<String>,
    bindings: BTreeMap<String, AssetKey>,
    invariant_guards_seen: BTreeSet<crate::ir::InvariantKind>,
    /// Set once a Bridge operation has run. Nothing that operates on the
    /// source chain (OpenDebt/ExecuteSwap/CloseDebt) may appear after it —
    /// defense in depth for the same invariant trading_verify.rs already
    /// enforces at the AST level.
    bridged: bool,
}

pub fn verify_trading_sequences(trading: &[TradingOperation], context: &str, diagnostics: &mut Vec<String>) {
    if trading.is_empty() {
        return;
    }

    let mut state = TradingSequenceState::default();

    for (index, op) in trading.iter().enumerate() {
        let op_context = format!("{context}.trading[{index}]");

        if state.committed || state.aborted {
            push_unsafe(
                diagnostics,
                format!("{op_context}: trading operation appears after terminal commit/abort"),
            );
            continue;
        }

        match op {
            TradingOperation::BeginAtomicTrade { .. } => {
                if state.began {
                    push_unsafe(diagnostics, format!("{op_context}: duplicate BeginAtomicTrade"));
                }
                if index != 0 {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: BeginAtomicTrade must be the first trading operation"),
                    );
                }
                state.began = true;
            }
            TradingOperation::OpenDebt { debt_id, asset, .. } => {
                require_trade_started(&state, &op_context, diagnostics);
                if state.bridged {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: open debt appears after the trade bridged to another chain"),
                    );
                }
                if state.open_debts.contains_key(debt_id) || state.closed_debts.contains(debt_id) {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: debt '{debt_id}' is opened more than once"),
                    );
                } else {
                    state.open_debts.insert(debt_id.clone(), asset.clone());
                }
            }
            TradingOperation::ExecuteSwap {
                binding,
                from,
                to,
                input,
                ..
            } => {
                require_trade_started(&state, &op_context, diagnostics);

                if state.bridged {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: swap appears after the trade bridged to another chain"),
                    );
                }

                if state.receipt_seen || state.all_debts_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: swap appears after final trading guards/receipt"),
                    );
                }

                match input {
                    ValueRef::Literal(_) => {}
                    ValueRef::Binding(name) => {
                        if let Some((debt_id, field)) = name.split_once('.') {
                            if field != "amount" {
                                push_unsafe(diagnostics, format!("{op_context}: unsupported debt binding '{name}'"));
                            }
                            match state.open_debts.get(debt_id) {
                                Some(asset) if asset == from => {}
                                Some(asset) => push_unsafe(
                                    diagnostics,
                                    format!(
                                        "{op_context}: debt binding '{name}' has asset {} but swap expects {}",
                                        asset.symbol, from.symbol
                                    ),
                                ),
                                None => push_unsafe(
                                    diagnostics,
                                    format!("{op_context}: debt binding '{name}' used before open debt"),
                                ),
                            }
                        } else {
                            match state.bindings.get(name) {
                                Some(asset) if asset == from => {}
                                Some(asset) => push_unsafe(
                                    diagnostics,
                                    format!(
                                        "{op_context}: binding '{name}' has asset {} but swap expects {}",
                                        asset.symbol, from.symbol
                                    ),
                                ),
                                None => push_unsafe(
                                    diagnostics,
                                    format!("{op_context}: binding '{name}' used before creation"),
                                ),
                            }
                        }
                    }
                }

                if let Some(existing) = state.bindings.get(binding) {
                    if existing != to {
                        push_unsafe(
                            diagnostics,
                            format!(
                                "{op_context}: binding '{binding}' reused with conflicting asset {} -> {}",
                                existing.symbol, to.symbol
                            ),
                        );
                    } else {
                        push_unsafe(
                            diagnostics,
                            format!("{op_context}: binding '{binding}' is assigned more than once"),
                        );
                    }
                } else {
                    state.bindings.insert(binding.clone(), to.clone());
                }
            }
            TradingOperation::Bridge { from, to, input, .. } => {
                require_trade_started(&state, &op_context, diagnostics);

                if state.bridged {
                    push_unsafe(diagnostics, format!("{op_context}: duplicate Bridge in one trade"));
                }
                if state.receipt_seen || state.all_debts_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: bridge appears after final trading guards/receipt"),
                    );
                }
                if from == to {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: bridge from and to assets must differ"),
                    );
                }

                match input {
                    ValueRef::Literal(_) => {}
                    ValueRef::Binding(name) => {
                        if let Some((debt_id, field)) = name.split_once('.') {
                            if field != "amount" {
                                push_unsafe(diagnostics, format!("{op_context}: unsupported debt binding '{name}'"));
                            }
                            match state.open_debts.get(debt_id) {
                                Some(asset) if asset == from => {}
                                Some(asset) => push_unsafe(
                                    diagnostics,
                                    format!(
                                        "{op_context}: debt binding '{name}' has asset {} but bridge expects {}",
                                        asset.symbol, from.symbol
                                    ),
                                ),
                                None => push_unsafe(
                                    diagnostics,
                                    format!("{op_context}: debt binding '{name}' used before open debt"),
                                ),
                            }
                        } else {
                            match state.bindings.get(name) {
                                Some(asset) if asset == from => {}
                                Some(asset) => push_unsafe(
                                    diagnostics,
                                    format!(
                                        "{op_context}: binding '{name}' has asset {} but bridge expects {}",
                                        asset.symbol, from.symbol
                                    ),
                                ),
                                None => push_unsafe(
                                    diagnostics,
                                    format!("{op_context}: binding '{name}' used before creation"),
                                ),
                            }
                        }
                    }
                }

                state.bridged = true;
            }
            TradingOperation::CloseDebt { debt_id } => {
                require_trade_started(&state, &op_context, diagnostics);
                if state.bridged {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: close debt appears after the trade bridged to another chain"),
                    );
                }
                if state.closed_debts.contains(debt_id) {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: debt '{debt_id}' is closed more than once"),
                    );
                } else if state.open_debts.remove(debt_id).is_none() {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: debt '{debt_id}' closed before it was opened"),
                    );
                } else {
                    state.closed_debts.insert(debt_id.clone());
                }
            }
            TradingOperation::AssertMinNetProfit { .. } => {
                require_trade_started(&state, &op_context, diagnostics);
                if state.receipt_seen || state.all_debts_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: minimum-profit guard appears after final guards/receipt"),
                    );
                }
                if state.profit_guard_seen {
                    push_unsafe(diagnostics, format!("{op_context}: duplicate minimum-profit guard"));
                }
                state.profit_guard_seen = true;
            }
            TradingOperation::AssertAllDebtsClosed => {
                require_trade_started(&state, &op_context, diagnostics);
                if !state.open_debts.is_empty() {
                    push_unsafe(
                        diagnostics,
                        format!(
                            "{op_context}: all-debts guard reached with open debts: {}",
                            state.open_debts.keys().cloned().collect::<Vec<_>>().join(", ")
                        ),
                    );
                }
                if !state.profit_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: all-debts guard must follow the minimum-profit guard"),
                    );
                }
                if state.all_debts_guard_seen {
                    push_unsafe(diagnostics, format!("{op_context}: duplicate all-debts guard"));
                }
                state.all_debts_guard_seen = true;
            }
            TradingOperation::AssertInvariant { kind } => {
                require_trade_started(&state, &op_context, diagnostics);
                if state.receipt_seen || state.committed {
                    push_unsafe(
                        diagnostics,
                        format!(
                            "{op_context}: invariant '{}' must precede receipt/commit",
                            kind.as_str()
                        ),
                    );
                }
                if !state.invariant_guards_seen.insert(*kind) {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: duplicate invariant guard '{}'", kind.as_str()),
                    );
                }
            }
            TradingOperation::EmitTradeReceipt => {
                require_trade_started(&state, &op_context, diagnostics);
                if !state.profit_guard_seen || !state.all_debts_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: receipt must follow minimum-profit and all-debts guards"),
                    );
                }
                if state.receipt_seen {
                    push_unsafe(diagnostics, format!("{op_context}: duplicate receipt emission"));
                }
                state.receipt_seen = true;
            }
            TradingOperation::CommitAtomicTrade => {
                require_trade_started(&state, &op_context, diagnostics);
                if !state.open_debts.is_empty() {
                    push_unsafe(diagnostics, format!("{op_context}: commit with open debts"));
                }
                if !state.profit_guard_seen {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: commit missing minimum-profit guard"),
                    );
                }
                if !state.all_debts_guard_seen {
                    push_unsafe(diagnostics, format!("{op_context}: commit missing all-debts guard"));
                }
                if !state.receipt_seen {
                    push_unsafe(diagnostics, format!("{op_context}: commit missing receipt"));
                }
                if index + 1 != trading.len() {
                    push_unsafe(
                        diagnostics,
                        format!("{op_context}: CommitAtomicTrade must be the final trading operation"),
                    );
                }
                state.committed = true;
            }
            TradingOperation::AbortAtomicTrade => {
                require_trade_started(&state, &op_context, diagnostics);
                state.aborted = true;
                if index + 1 != trading.len() {
                    push_unsafe(diagnostics, format!("{op_context}: AbortAtomicTrade must be terminal"));
                }
            }
        }
    }

    if !state.began {
        push_unsafe(
            diagnostics,
            format!("{context}: trading sequence is missing BeginAtomicTrade"),
        );
    }
    if !state.committed && !state.aborted {
        push_unsafe(
            diagnostics,
            format!("{context}: trading sequence has no terminal commit/abort"),
        );
    }
}

fn require_trade_started(state: &TradingSequenceState, context: &str, diagnostics: &mut Vec<String>) {
    if !state.began {
        push_unsafe(
            diagnostics,
            format!("{context}: trading operation appears before BeginAtomicTrade"),
        );
    }
}

pub fn verify_trading_operation(trading: &TradingOperation, context: &str, diagnostics: &mut Vec<String>) {
    match trading {
        TradingOperation::BeginAtomicTrade { trade_id, policy } => {
            require_non_empty(diagnostics, context, "trade_id", trade_id);
            require_non_empty(diagnostics, context, "policy.policy_id", &policy.policy_id);
            require_non_empty(diagnostics, context, "policy.chain", &policy.chain);
            if policy.policy_version == 0 {
                push_unsafe(
                    diagnostics,
                    format!("{context}: policy version must be greater than zero"),
                );
            }
            if policy.max_slippage_bps > 10_000 {
                push_unsafe(diagnostics, format!("{context}: policy max_slippage_bps exceeds 10000"));
            }
            if let Some(deviation_bps) = policy.max_oracle_deviation_bps {
                if deviation_bps > 10_000 {
                    push_unsafe(
                        diagnostics,
                        format!("{context}: policy max_oracle_deviation_bps exceeds 10000"),
                    );
                }
            }
            verify_asset_key(&policy.max_gas_asset, context, "policy.max_gas_asset", diagnostics);
            match (policy.max_cumulative_loss, &policy.max_cumulative_loss_asset) {
                (Some(_), Some(asset)) => {
                    verify_asset_key(asset, context, "policy.max_cumulative_loss_asset", diagnostics);
                }
                (None, None) => {}
                _ => {
                    push_unsafe(
                        diagnostics,
                        format!(
                            "{context}: policy max_cumulative_loss and max_cumulative_loss_asset must both be set or both be absent"
                        ),
                    );
                }
            }
            if policy.max_flash_fee_bps > 10_000 {
                push_unsafe(
                    diagnostics,
                    format!("{context}: policy max_flash_fee_bps exceeds 10000"),
                );
            }
            if policy.deadline_blocks == 0 {
                push_unsafe(
                    diagnostics,
                    format!("{context}: policy deadline_blocks must be greater than zero"),
                );
            }
        }
        TradingOperation::OpenDebt {
            debt_id,
            provider,
            asset,
            principal,
        } => {
            require_non_empty(diagnostics, context, "debt_id", debt_id);
            require_non_empty(diagnostics, context, "provider", provider);
            verify_asset_key(asset, context, "asset", diagnostics);
            if *principal == 0 {
                push_unsafe(
                    diagnostics,
                    format!("{context}: debt principal must be greater than zero"),
                );
            }
        }
        TradingOperation::ExecuteSwap {
            binding,
            venue,
            from,
            to,
            min_output,
            ..
        } => {
            require_non_empty(diagnostics, context, "binding", binding);
            require_non_empty(diagnostics, context, "venue", venue);
            verify_asset_key(from, context, "from", diagnostics);
            verify_asset_key(to, context, "to", diagnostics);
            if *min_output == 0 {
                push_unsafe(diagnostics, format!("{context}: min_output must be greater than zero"));
            }
        }
        TradingOperation::Bridge {
            via,
            from,
            to,
            receiver,
            ..
        } => {
            require_non_empty(diagnostics, context, "via", via);
            verify_asset_key(from, context, "from", diagnostics);
            verify_asset_key(to, context, "to", diagnostics);
            require_non_empty(diagnostics, context, "receiver", receiver);
        }
        TradingOperation::CloseDebt { debt_id } => {
            require_non_empty(diagnostics, context, "debt_id", debt_id);
        }
        TradingOperation::AssertMinNetProfit {
            settlement_asset,
            minimum,
        } => {
            verify_asset_key(settlement_asset, context, "settlement_asset", diagnostics);
            if *minimum == 0 {
                push_unsafe(
                    diagnostics,
                    format!("{context}: minimum net profit must be greater than zero"),
                );
            }
        }
        TradingOperation::AssertAllDebtsClosed
        | TradingOperation::AssertInvariant { .. }
        | TradingOperation::EmitTradeReceipt
        | TradingOperation::CommitAtomicTrade
        | TradingOperation::AbortAtomicTrade => {}
    }
}

fn verify_asset_key(asset: &AssetKey, context: &str, field: &str, diagnostics: &mut Vec<String>) {
    require_non_empty(diagnostics, context, &format!("{field}.vm_family"), &asset.vm_family);
    require_non_empty(diagnostics, context, &format!("{field}.chain"), &asset.chain);
    require_non_empty(
        diagnostics,
        context,
        &format!("{field}.canonical_id"),
        &asset.canonical_id,
    );
    require_non_empty(diagnostics, context, &format!("{field}.symbol"), &asset.symbol);
}
