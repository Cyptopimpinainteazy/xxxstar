//! Lowering an `automation` declaration into the chain's own task vocabulary.
//!
//! The parser produces [`AutomationDecl`]; the chain executes
//! `x3_automation::{Condition, Action}` through `pallet-x3-automation`. This
//! module is the only place the two meet, so there is exactly one answer to
//! "what does this declaration become on chain".
//!
//! Two rules, both deliberate:
//!
//! * **Unsupported source fails explicitly.** A trigger or action the pallet
//!   cannot represent is an error naming the construct, never a silent drop or a
//!   defaulted `Manual`/`Noop`. A declaration that compiled must mean what it
//!   says.
//! * **Budgets are numbers or they are refused.** A `max_trade` that is not a
//!   non-negative integer is an error here rather than a value coerced later by
//!   whoever happens to read it.

use x3_automation::{Action, Condition};
use x3_lang_ast::ast::{
    AutomationDecl, AutomationPermissionKind, Block, Expression, LiteralExpr, Statement,
};
use x3_lang_common::X3Error;

/// The budgets a declaration grants, in the units it wrote them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutomationBudgets {
    /// Wallet the automation draws on, as written.
    pub wallet: Option<String>,
    /// Largest single amount it may move.
    pub max_trade: Option<u128>,
    /// Cumulative loss it may incur per period.
    pub daily_loss_limit: Option<u128>,
    /// Chains it may touch. Empty means the declaration did not restrict them.
    pub allowed_chains: Vec<String>,
}

/// A declaration lowered into what the chain can actually execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomationPlan {
    pub name: String,
    pub condition: Condition,
    pub action: Action,
    pub budgets: AutomationBudgets,
}

/// Lower `decl` into a plan.
///
/// Fails with a specific message for anything the pallet cannot represent.
pub fn lower(decl: &AutomationDecl) -> Result<AutomationPlan, X3Error> {
    Ok(AutomationPlan {
        name: decl.name.as_str().to_string(),
        condition: lower_trigger(&decl.trigger)?,
        action: lower_execute(&decl.execute, decl.name.as_str())?,
        budgets: lower_permissions(decl)?,
    })
}

/// `manual()`, `block_number(n)`, `timestamp(ms)`, `price_above(a, p)`,
/// `price_below(a, p)`, `price_band(a, lo, hi)`.
///
/// These are the conditions `pallet-x3-automation` can decide from on-chain
/// state. Anything else — including an off-chain connector — is refused: a
/// trigger a validator could not re-evaluate would make execution unreplayable.
fn lower_trigger(trigger: &Expression) -> Result<Condition, X3Error> {
    let (name, args) = match trigger {
        Expression::Call { callee, args } => {
            let name = callee_name(callee).ok_or_else(|| unsupported_trigger("a non-call callee"))?;
            (name, args.as_slice())
        }
        _ => return Err(unsupported_trigger("a trigger that is not a call")),
    };

    match (name.as_str(), args.len()) {
        ("manual", 0) => Ok(Condition::Manual),
        ("block_number", 1) => Ok(Condition::BlockNumber(int_arg(args, 0)? as u64)),
        ("timestamp", 1) => Ok(Condition::Timestamp(int_arg(args, 0)? as u64)),
        ("price_above", 2) => Ok(Condition::PriceThreshold {
            asset_id: oracle_asset_arg(args, 0)?,
            threshold: int_arg(args, 1)? as u64,
            above: true,
        }),
        ("price_below", 2) => Ok(Condition::PriceThreshold {
            asset_id: oracle_asset_arg(args, 0)?,
            threshold: int_arg(args, 1)? as u64,
            above: false,
        }),
        ("price_band", 3) => {
            let lower = int_arg(args, 1)? as u64;
            let upper = int_arg(args, 2)? as u64;
            if lower > upper {
                return Err(parse_error(format!(
                    "trigger `price_band`: lower bound {lower} is above upper bound {upper}; \
                     an inverted band can never fire"
                )));
            }
            Ok(Condition::PriceBand {
                asset_id: oracle_asset_arg(args, 0)?,
                lower,
                upper,
            })
        }
        _ => Err(unsupported_trigger(&format!(
            "`{name}` with {} argument(s)",
            args.len()
        ))),
    }
}

/// The execute body must be exactly one call to a verb the pallet implements.
///
/// Restricting it to one statement is the point: a body of several statements
/// would need a state machine the pallet does not have, and pretending otherwise
/// would let a declaration look like it did more than it does.
fn lower_execute(block: &Block, automation: &str) -> Result<Action, X3Error> {
    let mut statements = block.stmts.iter();
    let first = statements
        .next()
        .ok_or_else(|| unsupported_action(automation, "an empty `execute` block"))?;
    if statements.next().is_some() {
        return Err(unsupported_action(
            automation,
            "more than one statement in `execute`; the chain executes a single whitelisted action",
        ));
    }

    let call = match first {
        Statement::Expr(Expression::Call { callee, args }) => (callee, args),
        _ => {
            return Err(unsupported_action(
                automation,
                "an `execute` body that is not a single call",
            ))
        }
    };
    let (callee, args) = call;
    let name = callee_name(callee)
        .ok_or_else(|| unsupported_action(automation, "a non-name callee in `execute`"))?;

    match (name.as_str(), args.len()) {
        ("noop", 0) => Ok(Action::Noop),
        ("transfer_native", 2) => Ok(Action::TransferNative {
            recipient: account_arg(args, 0, automation)?,
            amount: int_arg(args, 1)?,
        }),
        ("transfer_asset", 3) => Ok(Action::TransferLedgerAsset {
            asset: ledger_asset_arg(args, 0, automation)?,
            recipient: account_arg(args, 1, automation)?,
            amount: int_arg(args, 2)?,
        }),
        _ => Err(unsupported_action(
            automation,
            &format!("`{name}` with {} argument(s)", args.len()),
        )),
    }
}

fn lower_permissions(decl: &AutomationDecl) -> Result<AutomationBudgets, X3Error> {
    let mut budgets = AutomationBudgets::default();
    for permission in &decl.permissions {
        match permission.kind {
            AutomationPermissionKind::Wallet => {
                let name = match &permission.value {
                    Expression::Literal(LiteralExpr::String(text)) => text.as_str().to_string(),
                    Expression::Ident(symbol) => symbol.as_str().to_string(),
                    _ => {
                        return Err(parse_error(format!(
                            "automation `{}`: `wallet` expects a name or identifier",
                            decl.name.as_str()
                        )))
                    }
                };
                if budgets.wallet.replace(name).is_some() {
                    return Err(parse_error(format!(
                        "automation `{}`: `wallet` declared twice",
                        decl.name.as_str()
                    )));
                }
            }
            AutomationPermissionKind::MaxTrade | AutomationPermissionKind::DailyLossLimit => {
                let value = match &permission.value {
                    Expression::Literal(LiteralExpr::Int { value, .. }) => *value,
                    _ => {
                        return Err(parse_error(format!(
                            "automation `{}`: `{:?}` expects a non-negative integer amount",
                            decl.name.as_str(),
                            permission.kind
                        )))
                    }
                };
                let slot = if permission.kind == AutomationPermissionKind::MaxTrade {
                    &mut budgets.max_trade
                } else {
                    &mut budgets.daily_loss_limit
                };
                if slot.replace(value).is_some() {
                    return Err(parse_error(format!(
                        "automation `{}`: `{:?}` declared twice",
                        decl.name.as_str(),
                        permission.kind
                    )));
                }
            }
            AutomationPermissionKind::AllowedChains => {
                // One chain per `allowed_chain: "Name"`, repeated. The canonical
                // AST has no array literal, and inventing one here would make the
                // permission syntax mean something the parser never produced.
                let Expression::Literal(LiteralExpr::String(text)) = &permission.value else {
                    return Err(parse_error(format!(
                        "automation `{}`: `allowed_chain` expects a chain name",
                        decl.name.as_str()
                    )));
                };
                budgets.allowed_chains.push(text.as_str().to_string());
            }
        }
    }
    Ok(budgets)
}

/// The final segment of a callee path: `x3.transfer_native` yields
/// `transfer_native`, and a bare `manual` yields `manual`.
fn callee_name(callee: &Expression) -> Option<String> {
    match callee {
        Expression::Ident(symbol) => Some(symbol.as_str().to_string()),
        Expression::FieldAccess { field, .. } => Some(field.as_str().to_string()),
        _ => None,
    }
}

fn int_arg(args: &[Expression], index: usize) -> Result<u128, X3Error> {
    match args.get(index) {
        Some(Expression::Literal(LiteralExpr::Int { value, .. })) => Ok(*value),
        _ => Err(parse_error(format!(
            "argument {} must be a non-negative integer literal",
            index + 1
        ))),
    }
}

fn oracle_asset_arg(args: &[Expression], index: usize) -> Result<u32, X3Error> {
    let value = int_arg(args, index)?;
    u32::try_from(value).map_err(|_| {
        parse_error(format!(
            "argument {}: asset id {value} does not fit the oracle's u32 asset space",
            index + 1
        ))
    })
}

fn ledger_asset_arg(
    args: &[Expression],
    index: usize,
    automation: &str,
) -> Result<u32, X3Error> {
    let value = int_arg(args, index)?;
    u32::try_from(value).map_err(|_| {
        unsupported_action(
            automation,
            &format!("asset id {value} outside the ledger's u32 asset space"),
        )
    })
}

/// A 32-byte account key.
///
/// Accepts an integer (a test or a chain-local id) or a string of hex. Anything
/// else is refused rather than hashed into a lookalike identity: an account the
/// author did not name must never receive funds.
fn account_arg(args: &[Expression], index: usize, automation: &str) -> Result<[u8; 32], X3Error> {
    match args.get(index) {
        Some(Expression::Literal(LiteralExpr::Int { value, .. })) => {
            let mut key = [0u8; 32];
            key[..16].copy_from_slice(&value.to_le_bytes());
            Ok(key)
        }
        Some(Expression::Literal(LiteralExpr::String(text))) => {
            let raw = text.as_str();
            let hex = raw.strip_prefix("0x").unwrap_or(raw);
            if hex.len() != 64 {
                return Err(unsupported_action(
                    automation,
                    "a recipient that is not 32 bytes of hex",
                ));
            }
            let mut key = [0u8; 32];
            for (byte, chunk) in key.iter_mut().zip(hex.as_bytes().chunks(2)) {
                let pair = core::str::from_utf8(chunk).map_err(|_| {
                    unsupported_action(automation, "a recipient that is not 32 bytes of hex")
                })?;
                *byte = u8::from_str_radix(pair, 16).map_err(|_| {
                    unsupported_action(automation, "a recipient that is not 32 bytes of hex")
                })?;
            }
            Ok(key)
        }
        _ => Err(unsupported_action(
            automation,
            "a recipient that is neither an integer nor a hex string",
        )),
    }
}

fn parse_error(message: impl Into<String>) -> X3Error {
    X3Error::ParseError {
        message: message.into(),
        span: x3_lang_common::Span::DUMMY,
        expected: vec![],
        found: String::new(),
    }
}

fn unsupported_trigger(construct: &str) -> X3Error {
    parse_error(format!(
        "automation trigger: {construct} cannot be decided on chain. Supported: \
         manual(), block_number(n), timestamp(ms), price_above(asset, price), \
         price_below(asset, price), price_band(asset, low, high)"
    ))
}

fn unsupported_action(automation: &str, construct: &str) -> X3Error {
    parse_error(format!(
        "automation `{automation}`: {construct} is not an executable action. \
         Supported: noop(), transfer_native(recipient, amount), \
         transfer_asset(asset, recipient, amount)"
    ))
}
