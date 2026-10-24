//! Lower x3-lang gateway intents into typed runtime call specs.
//!
//! This is intentionally small and deterministic: the gateway only accepts
//! literal router intents until the broader typed intent IR is ready.

use x3_ast::{Expression, Item, LiteralExpression, Module, Statement};
use x3_common::Literal;

/// Domain accepted by gateway lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayDomain {
    X3Native,
    X3Evm,
    X3Svm,
}

/// Account literal accepted by gateway lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayAccount {
    X3Native(String),
    Evm(String),
    Svm(String),
}

impl GatewayAccount {
    pub fn domain(&self) -> GatewayDomain {
        match self {
            Self::X3Native(_) => GatewayDomain::X3Native,
            Self::Evm(_) => GatewayDomain::X3Evm,
            Self::Svm(_) => GatewayDomain::X3Svm,
        }
    }
}

/// VM target accepted by atomic bundle gateway lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayVm {
    X3,
    Evm,
    Svm,
    Cross,
}

/// Atomic-kernel leg emitted by x3-lang gateway lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayAtomicLeg {
    pub vm: GatewayVm,
    pub token_in: [u8; 32],
    pub token_out: [u8; 32],
    pub amount_in: u128,
    pub min_amount_out: u128,
    pub deadline: u64,
}

/// Runtime call shape emitted by the x3-lang gateway lowering pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayRuntimeCall {
    RouterXvmTransfer {
        destination: GatewayDomain,
        recipient: GatewayAccount,
        amount: u128,
        expires_in: u64,
    },
    AtomicSubmitBundle {
        legs: Vec<GatewayAtomicLeg>,
        deadline_blocks: u64,
        chain_id: u32,
        nonce: u64,
    },
}

/// Error returned when x3-lang source cannot be lowered into a gateway call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayLoweringError {
    Parse(String),
    MissingEntrypoint,
    MissingGatewayCall,
    UnsupportedCall(String),
    InvalidArgumentCount {
        call: String,
        expected: usize,
        got: usize,
    },
    InvalidStringArgument {
        call: String,
        index: usize,
    },
    InvalidIntegerArgument {
        call: String,
        index: usize,
    },
    InvalidDomain(String),
    InvalidAccount {
        value: String,
        expected_domain: GatewayDomain,
    },
    InvalidVm(String),
    InvalidHashArgument {
        call: String,
        index: usize,
    },
    EmptyBundle,
    /// A bundle's first argument is not a list of legs, or one leg is not a six-field tuple.
    InvalidBundleLeg {
        call: String,
        /// The leg's position, or `None` when the argument is not a list of legs at all.
        leg: Option<usize>,
    },
}

impl core::fmt::Display for GatewayLoweringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "failed to parse x3-lang gateway source: {err}"),
            Self::MissingEntrypoint => f.write_str("missing main() gateway entrypoint"),
            Self::MissingGatewayCall => f.write_str("main() does not contain a gateway call"),
            Self::UnsupportedCall(call) => write!(f, "unsupported gateway call: {call}"),
            Self::InvalidArgumentCount {
                call,
                expected,
                got,
            } => {
                write!(f, "{call} expects {expected} arguments, got {got}")
            }
            Self::InvalidStringArgument { call, index } => {
                write!(f, "{call} argument {index} must be a string literal")
            }
            Self::InvalidIntegerArgument { call, index } => {
                write!(
                    f,
                    "{call} argument {index} must be a non-negative integer literal"
                )
            }
            Self::InvalidDomain(value) => write!(f, "unsupported gateway domain: {value}"),
            Self::InvalidAccount {
                value,
                expected_domain,
            } => write!(
                f,
                "account literal {value} is not compatible with {expected_domain:?}"
            ),
            Self::InvalidVm(value) => write!(f, "unsupported atomic VM target: {value}"),
            Self::InvalidHashArgument { call, index } => {
                write!(f, "{call} argument {index} must be a 32-byte hex hash")
            }
            Self::EmptyBundle => f.write_str("atomic bundle must contain at least one leg"),
            Self::InvalidBundleLeg { call, leg: None } => write!(
                f,
                "{call} takes a list of legs as its first argument: \
                 [(vm, token_in, token_out, amount_in, min_amount_out, deadline), ...]"
            ),
            Self::InvalidBundleLeg {
                call,
                leg: Some(leg),
            } => write!(
                f,
                "{call} leg {leg} must be a six-field tuple \
                 (vm, token_in, token_out, amount_in, min_amount_out, deadline)"
            ),
        }
    }
}

impl std::error::Error for GatewayLoweringError {}

/// Parse x3-lang source and emit the first supported gateway runtime call from `main()`.
pub fn lower_gateway_call(source: &str) -> Result<GatewayRuntimeCall, GatewayLoweringError> {
    let module = x3_parser::parse_program(source)
        .map_err(|err| GatewayLoweringError::Parse(format!("{err:?}")))?;
    lower_gateway_call_from_module(&module)
}

pub fn lower_gateway_call_from_module(
    module: &Module,
) -> Result<GatewayRuntimeCall, GatewayLoweringError> {
    let main = module
        .items
        .iter()
        .find_map(|item| match item {
            Item::Function(function) if function.name.name == "main" => Some(function),
            _ => None,
        })
        .ok_or(GatewayLoweringError::MissingEntrypoint)?;

    main.body
        .statements
        .iter()
        .find_map(lower_statement)
        .transpose()?
        .ok_or(GatewayLoweringError::MissingGatewayCall)
}

fn lower_statement(stmt: &Statement) -> Option<Result<GatewayRuntimeCall, GatewayLoweringError>> {
    match stmt {
        Statement::Expr(expr) => Some(lower_expression(expr)),
        Statement::Return(Some(expr), _) => Some(lower_expression(expr)),
        _ => None,
    }
}

fn lower_expression(expr: &Expression) -> Result<GatewayRuntimeCall, GatewayLoweringError> {
    let Expression::Call(call) = expr else {
        return Err(GatewayLoweringError::MissingGatewayCall);
    };

    let call_name = callee_name(&call.callee)
        .ok_or_else(|| GatewayLoweringError::UnsupportedCall("<expression>".to_string()))?;

    match call_name.as_str() {
        "xvm_transfer" | "router.xvm_transfer" => lower_xvm_transfer(&call_name, &call.args),
        "submit_atomic_bundle" | "atomic.submit_bundle" => {
            lower_submit_atomic_bundle(&call_name, &call.args)
        }
        other => Err(GatewayLoweringError::UnsupportedCall(other.to_string())),
    }
}

fn lower_xvm_transfer(
    call: &str,
    args: &[Expression],
) -> Result<GatewayRuntimeCall, GatewayLoweringError> {
    const EXPECTED_ARGS: usize = 4;
    if args.len() != EXPECTED_ARGS {
        return Err(GatewayLoweringError::InvalidArgumentCount {
            call: call.to_string(),
            expected: EXPECTED_ARGS,
            got: args.len(),
        });
    }

    let destination = domain_arg(call, args, 0)?;
    let recipient = account_arg(call, args, 1, &destination)?;

    Ok(GatewayRuntimeCall::RouterXvmTransfer {
        destination,
        recipient,
        amount: integer_arg(call, args, 2)? as u128,
        expires_in: integer_arg(call, args, 3)?,
    })
}

/// Extract a VM type from an expression at the given argument index.
fn vm_arg(
    call: &str,
    args: &[Expression],
    index: usize,
) -> Result<GatewayVm, GatewayLoweringError> {
    let value = string_arg(call, args, index)?;
    parse_vm(&value).ok_or(GatewayLoweringError::InvalidVm(value))
}

fn lower_submit_atomic_bundle(
    call: &str,
    args: &[Expression],
) -> Result<GatewayRuntimeCall, GatewayLoweringError> {
    // Two calling conventions:
    //
    // - multi-leg: `submit_atomic_bundle([leg, ...], deadline_blocks, chain_id, nonce)`, each leg a
    //   tuple `(vm, token_in, token_out, amount_in, min_amount_out, deadline)` of literals. This was
    //   the intended form; it was unreachable while the parser had no array or tuple literals.
    // - single-leg: the same six fields and the three trailing ones, as nine flat arguments.
    //
    // How many legs a chain admits is the chain's bound (`MaxLegsPerBundle`), enforced where the
    // bundle is submitted; here a bundle must have at least one.
    const MULTI_LEG_ARGS: usize = 4;
    const SINGLE_LEG_ARGS: usize = 9;
    const LEG_FIELDS: usize = 6;

    let (legs, trailing) = match args.len() {
        MULTI_LEG_ARGS => {
            let Expression::Array(list) = &args[0] else {
                return Err(GatewayLoweringError::InvalidBundleLeg {
                    call: call.to_string(),
                    leg: None,
                });
            };
            if list.elements.is_empty() {
                return Err(GatewayLoweringError::EmptyBundle);
            }
            let mut legs = Vec::with_capacity(list.elements.len());
            for (position, element) in list.elements.iter().enumerate() {
                let Expression::Tuple(fields) = element else {
                    return Err(GatewayLoweringError::InvalidBundleLeg {
                        call: call.to_string(),
                        leg: Some(position),
                    });
                };
                if fields.elements.len() != LEG_FIELDS {
                    return Err(GatewayLoweringError::InvalidBundleLeg {
                        call: call.to_string(),
                        leg: Some(position),
                    });
                }
                legs.push(leg_from(call, &fields.elements)?);
            }
            (legs, &args[1..])
        }
        SINGLE_LEG_ARGS => (
            vec![leg_from(call, &args[..LEG_FIELDS])?],
            &args[LEG_FIELDS..],
        ),
        got => {
            return Err(GatewayLoweringError::InvalidArgumentCount {
                call: call.to_string(),
                expected: if matches!(args.first(), Some(Expression::Array(_))) {
                    MULTI_LEG_ARGS
                } else {
                    SINGLE_LEG_ARGS
                },
                got,
            })
        }
    };

    Ok(GatewayRuntimeCall::AtomicSubmitBundle {
        legs,
        deadline_blocks: integer_arg(call, trailing, 0)?,
        chain_id: integer_arg(call, trailing, 1)? as u32,
        nonce: integer_arg(call, trailing, 2)?,
    })
}

/// One leg from its six fields, in order.
fn leg_from(call: &str, fields: &[Expression]) -> Result<GatewayAtomicLeg, GatewayLoweringError> {
    Ok(GatewayAtomicLeg {
        vm: vm_arg(call, fields, 0)?,
        token_in: hash_arg(call, fields, 1)?,
        token_out: hash_arg(call, fields, 2)?,
        amount_in: integer_arg(call, fields, 3)? as u128,
        min_amount_out: integer_arg(call, fields, 4)? as u128,
        deadline: integer_arg(call, fields, 5)?,
    })
}

fn callee_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Identifier(id) => Some(id.name.clone()),
        Expression::FieldAccess(field) => {
            let base = callee_name(&field.object)?;
            Some(format!("{}.{}", base, field.field.name))
        }
        _ => None,
    }
}

fn domain_arg(
    call: &str,
    args: &[Expression],
    index: usize,
) -> Result<GatewayDomain, GatewayLoweringError> {
    let value = string_arg(call, args, index)?;
    parse_domain(&value).ok_or(GatewayLoweringError::InvalidDomain(value))
}

fn account_arg(
    call: &str,
    args: &[Expression],
    index: usize,
    expected_domain: &GatewayDomain,
) -> Result<GatewayAccount, GatewayLoweringError> {
    let value = string_arg(call, args, index)?;
    let account = parse_account(&value).ok_or_else(|| GatewayLoweringError::InvalidAccount {
        value: value.clone(),
        expected_domain: expected_domain.clone(),
    })?;

    if account.domain() != *expected_domain {
        return Err(GatewayLoweringError::InvalidAccount {
            value,
            expected_domain: expected_domain.clone(),
        });
    }

    Ok(account)
}

fn string_arg(
    call: &str,
    args: &[Expression],
    index: usize,
) -> Result<String, GatewayLoweringError> {
    match literal_arg(args, index) {
        Some(Literal::String(value)) => Ok(value.clone()),
        _ => Err(GatewayLoweringError::InvalidStringArgument {
            call: call.to_string(),
            index,
        }),
    }
}

fn integer_arg(call: &str, args: &[Expression], index: usize) -> Result<u64, GatewayLoweringError> {
    match literal_arg(args, index) {
        Some(Literal::Integer(value)) if *value >= 0 => Ok(*value as u64),
        _ => Err(GatewayLoweringError::InvalidIntegerArgument {
            call: call.to_string(),
            index,
        }),
    }
}

fn hash_arg(
    call: &str,
    args: &[Expression],
    index: usize,
) -> Result<[u8; 32], GatewayLoweringError> {
    let value = string_arg(call, args, index)?;
    parse_hex_32(&value).ok_or_else(|| GatewayLoweringError::InvalidHashArgument {
        call: call.to_string(),
        index,
    })
}

fn literal_arg(args: &[Expression], index: usize) -> Option<&Literal> {
    match args.get(index) {
        Some(Expression::Literal(LiteralExpression { literal, .. })) => Some(literal),
        _ => None,
    }
}

fn parse_domain(value: &str) -> Option<GatewayDomain> {
    match value {
        "x3native" => Some(GatewayDomain::X3Native),
        "x3evm" => Some(GatewayDomain::X3Evm),
        "x3svm" => Some(GatewayDomain::X3Svm),
        _ => None,
    }
}

fn parse_account(value: &str) -> Option<GatewayAccount> {
    match value {
        value if value.ends_with("_native") => Some(GatewayAccount::X3Native(value.to_string())),
        value if value.ends_with("_evm") => Some(GatewayAccount::Evm(value.to_string())),
        value if value.ends_with("_svm") => Some(GatewayAccount::Svm(value.to_string())),
        _ => None,
    }
}

fn parse_vm(value: &str) -> Option<GatewayVm> {
    match value {
        "x3" => Some(GatewayVm::X3),
        "evm" => Some(GatewayVm::Evm),
        "svm" => Some(GatewayVm::Svm),
        "cross" => Some(GatewayVm::Cross),
        _ => None,
    }
}

fn parse_hex_32(value: &str) -> Option<[u8; 32]> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    if hex.len() != 64 {
        return None;
    }

    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&hex[start..start + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_router_xvm_transfer_from_main() {
        let source = r#"
            fn main() {
                xvm_transfer("x3evm", "alice_evm", 10, 50);
            }
        "#;

        assert_eq!(
            lower_gateway_call(source).unwrap(),
            GatewayRuntimeCall::RouterXvmTransfer {
                destination: GatewayDomain::X3Evm,
                recipient: GatewayAccount::Evm("alice_evm".to_string()),
                amount: 10,
                expires_in: 50,
            }
        );
    }

    #[test]
    fn rejects_router_account_domain_mismatch() {
        let source = r#"
            fn main() {
                xvm_transfer("x3evm", "alice_svm", 10, 50);
            }
        "#;

        assert!(matches!(
            lower_gateway_call(source),
            Err(GatewayLoweringError::InvalidAccount { .. })
        ));
    }

    #[test]
    fn lowers_atomic_submit_bundle_from_main() {
        let source = r#"
            fn main() {
                submit_atomic_bundle(
                    "cross",
                    "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    100,
                    90,
                    1800000000,
                    50,
                    1,
                    1
                );
            }
        "#;

        let lowered = lower_gateway_call(source).unwrap();
        let GatewayRuntimeCall::AtomicSubmitBundle {
            legs,
            deadline_blocks,
            chain_id,
            nonce,
        } = lowered
        else {
            panic!("expected atomic bundle");
        };

        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].vm, GatewayVm::Cross);
        assert_eq!(legs[0].token_in, [0xaa; 32]);
        assert_eq!(legs[0].token_out, [0xbb; 32]);
        assert_eq!(legs[0].amount_in, 100);
        assert_eq!(legs[0].min_amount_out, 90);
        assert_eq!(deadline_blocks, 50);
        assert_eq!(chain_id, 1);
        assert_eq!(nonce, 1);
    }

    #[test]
    fn rejects_non_literal_amount() {
        let source = r#"
            fn main() {
                let amount = 10;
                xvm_transfer("x3evm", "alice_evm", amount, 50);
            }
        "#;

        assert!(matches!(
            lower_gateway_call(source),
            Err(GatewayLoweringError::InvalidIntegerArgument { index: 2, .. })
        ));
    }

    // ── Multi-leg atomic bundle tests ─────────────────────────────────
    //
    // The multi-leg calling convention takes a list of leg tuples. It was unreachable while the
    // parser had no array or tuple literals; these tests used to pin that as a parse failure. Now
    // the list lowers to one leg per tuple, and the malformed shapes are refused by the gateway,
    // each with its own reason.

    #[test]
    fn lowers_a_multi_leg_bundle_one_leg_per_tuple() {
        let source = r#"
            fn main() {
                submit_atomic_bundle(
                    [
                        ("cross", "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", 100, 90, 1800000000),
                        ("evm", "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", 200, 180, 1800000001),
                    ],
                    50,
                    1,
                    1
                );
            }
        "#;

        let GatewayRuntimeCall::AtomicSubmitBundle {
            legs,
            deadline_blocks,
            chain_id,
            nonce,
        } = lower_gateway_call(source).expect("a well-formed bundle lowers")
        else {
            panic!("expected atomic bundle");
        };
        assert_eq!(legs.len(), 2);
        assert_eq!(legs[0].vm, GatewayVm::Cross);
        assert_eq!(legs[0].token_in, [0xaa; 32]);
        assert_eq!(legs[0].amount_in, 100);
        assert_eq!(legs[0].deadline, 1_800_000_000);
        assert_eq!(legs[1].vm, GatewayVm::Evm);
        assert_eq!(legs[1].token_out, [0xdd; 32]);
        assert_eq!(legs[1].min_amount_out, 180);
        assert_eq!(legs[1].deadline, 1_800_000_001);
        assert_eq!((deadline_blocks, chain_id, nonce), (50, 1, 1));
    }

    #[test]
    fn rejects_a_bundle_whose_legs_are_not_in_a_list() {
        let source = r#"
            fn main() {
                submit_atomic_bundle(
                    ("cross", "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", 100, 90, 1800000000),
                    50,
                    1,
                    1
                );
            }
        "#;

        assert!(matches!(
            lower_gateway_call(source),
            Err(GatewayLoweringError::InvalidBundleLeg { leg: None, .. })
        ));
    }

    #[test]
    fn rejects_an_empty_bundle() {
        let source = r#"
            fn main() {
                submit_atomic_bundle([], 50, 1, 1);
            }
        "#;

        assert!(matches!(
            lower_gateway_call(source),
            Err(GatewayLoweringError::EmptyBundle)
        ));
    }

    #[test]
    fn rejects_a_leg_that_is_not_a_six_field_tuple() {
        let short = r#"
            fn main() {
                submit_atomic_bundle([("cross", 1, 2)], 50, 1, 1);
            }
        "#;
        assert!(matches!(
            lower_gateway_call(short),
            Err(GatewayLoweringError::InvalidBundleLeg { leg: Some(0), .. })
        ));

        let not_a_tuple = r#"
            fn main() {
                submit_atomic_bundle([("cross", "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", 1, 1, 1), 7], 50, 1, 1);
            }
        "#;
        assert!(matches!(
            lower_gateway_call(not_a_tuple),
            Err(GatewayLoweringError::InvalidBundleLeg { leg: Some(1), .. })
        ));
    }

    #[test]
    fn a_leg_field_is_checked_like_a_flat_argument() {
        // A bad hash inside a leg is refused by the same check the single-leg form uses.
        let source = r#"
            fn main() {
                submit_atomic_bundle([("cross", "0xnot-a-hash", "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", 1, 1, 1)], 50, 1, 1);
            }
        "#;
        assert!(matches!(
            lower_gateway_call(source),
            Err(GatewayLoweringError::InvalidHashArgument { index: 1, .. })
        ));
    }

    // ── Cross-VM router pallet regression test ───────────────────────
    //
    // `pallets/x3-cross-vm-router/src/tests.rs` calls
    // `x3_compiler::lower_gateway_call` on the source string below.
    // This test pins the same contract inside the compiler crate so a
    // regression is caught at the source.

    #[test]
    fn lowers_router_xvm_transfer_for_cross_vm_router_pallet() {
        let source = r#"
            fn main() {
                xvm_transfer("x3evm", "alice_evm", 10, 50);
            }
        "#;

        let lowered = lower_gateway_call(source)
            .expect("the cross-vm-router pallet test feeds this source to lower_gateway_call");
        let GatewayRuntimeCall::RouterXvmTransfer {
            destination,
            recipient,
            amount,
            expires_in,
        } = lowered
        else {
            panic!("expected RouterXvmTransfer");
        };

        assert_eq!(destination, GatewayDomain::X3Evm);
        assert_eq!(recipient, GatewayAccount::Evm("alice_evm".to_string()));
        assert_eq!(amount, 10);
        assert_eq!(expires_in, 50);
    }
}
