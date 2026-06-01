//! Lower x3-lang gateway intents into typed runtime call specs.
//!
//! This is intentionally small and deterministic: the gateway only accepts
//! literal router intents until the broader typed intent IR is ready.

use x3_ast::{Expression, Item, LiteralExpression, Module, Statement};
use x3_common::Literal;

/// Runtime call shape emitted by the x3-lang gateway lowering pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GatewayRuntimeCall {
    RouterXvmTransfer {
        destination: String,
        recipient: String,
        amount: u128,
        expires_in: u64,
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

    Ok(GatewayRuntimeCall::RouterXvmTransfer {
        destination: string_arg(call, args, 0)?,
        recipient: string_arg(call, args, 1)?,
        amount: integer_arg(call, args, 2)? as u128,
        expires_in: integer_arg(call, args, 3)? as u64,
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

fn literal_arg(args: &[Expression], index: usize) -> Option<&Literal> {
    match args.get(index) {
        Some(Expression::Literal(LiteralExpression { literal, .. })) => Some(literal),
        _ => None,
    }
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
                destination: "x3evm".to_string(),
                recipient: "alice_evm".to_string(),
                amount: 10,
                expires_in: 50,
            }
        );
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
}
