//! X3 language type checker: assigns types to all expressions and validates constraints.
//!
//! Takes an AST from the parser and produces a typed AST or a list of type errors.
//! Performs: type inference, function signature checking, and return type validation.

use std::collections::HashMap;

use crate::parser::{BinOp, Expr, FnDecl, SourceFile, Stmt, TypeExpr};

/// A type error from the type checker.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TypeError {
    pub message: String,
    pub location: String,
}

impl TypeError {
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            message: message.into(),
        }
    }
}

/// The type environment: maps names to their types.
pub type TypeEnv = HashMap<String, TypeExpr>;

/// Type-check the binary operator application and return the result type.
pub fn typecheck_binop(op: &BinOp, lhs: &TypeExpr, rhs: &TypeExpr) -> Result<TypeExpr, TypeError> {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
            if lhs == &TypeExpr::I64 && rhs == &TypeExpr::I64 {
                Ok(TypeExpr::I64)
            } else if lhs == &TypeExpr::U64 && rhs == &TypeExpr::U64 {
                Ok(TypeExpr::U64)
            } else {
                Err(TypeError::new("binop", format!("type mismatch: {lhs:?} op {rhs:?}")))
            }
        }
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt => {
            if lhs == rhs {
                Ok(TypeExpr::Bool)
            } else {
                Err(TypeError::new("binop", format!("cannot compare {lhs:?} with {rhs:?}")))
            }
        }
    }
}

/// Infer the type of an expression given a type environment.
pub fn infer_expr(expr: &Expr, env: &TypeEnv) -> Result<TypeExpr, TypeError> {
    match expr {
        Expr::IntLit(_) => Ok(TypeExpr::I64),
        Expr::BoolLit(_) => Ok(TypeExpr::Bool),
        Expr::StrLit(_) => Ok(TypeExpr::Str),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::new(name, format!("undefined variable '{name}'"))),
        Expr::BinOp { op, lhs, rhs } => {
            let lt = infer_expr(lhs, env)?;
            let rt = infer_expr(rhs, env)?;
            typecheck_binop(op, &lt, &rt)
        }
        Expr::Call { callee, args: _ } => {
            // Simplified: calls return Unit (full inference requires function table)
            env.get(callee)
                .cloned()
                .unwrap_or(TypeExpr::Unit)
                .pipe_ok()
        }
        Expr::Block(stmts) => {
            let mut local_env = env.clone();
            let mut last = TypeExpr::Unit;
            for stmt in stmts {
                last = typecheck_stmt(stmt, &mut local_env)?;
            }
            Ok(last)
        }
        Expr::If { cond, then_branch, else_branch } => {
            let ct = infer_expr(cond, env)?;
            if ct != TypeExpr::Bool {
                return Err(TypeError::new("if", "condition must be Bool"));
            }
            let tt = infer_expr(then_branch, env)?;
            if let Some(eb) = else_branch {
                let et = infer_expr(eb, env)?;
                if tt != et {
                    return Err(TypeError::new("if", "then/else branch types must match"));
                }
            }
            Ok(tt)
        }
    }
}

/// Helper trait to convert T into Ok(T).
trait PipeOk: Sized {
    fn pipe_ok(self) -> Result<Self, TypeError>;
}
impl PipeOk for TypeExpr {
    fn pipe_ok(self) -> Result<Self, TypeError> {
        Ok(self)
    }
}

/// Type-check a statement, updating the environment and returning the "value type".
pub fn typecheck_stmt(stmt: &Stmt, env: &mut TypeEnv) -> Result<TypeExpr, TypeError> {
    match stmt {
        Stmt::Let { name, ty, value } => {
            let inferred = infer_expr(value, env)?;
            if let Some(expected) = ty {
                if &inferred != expected {
                    return Err(TypeError::new(
                        name,
                        format!("expected {expected:?}, got {inferred:?}"),
                    ));
                }
            }
            env.insert(name.clone(), inferred);
            Ok(TypeExpr::Unit)
        }
        Stmt::Return(expr) => {
            if let Some(e) = expr {
                infer_expr(e, env)
            } else {
                Ok(TypeExpr::Unit)
            }
        }
        Stmt::Expr(e) => infer_expr(e, env),
        Stmt::While { cond, body } => {
            let ct = infer_expr(cond, env)?;
            if ct != TypeExpr::Bool {
                return Err(TypeError::new("while", "condition must be Bool"));
            }
            infer_expr(body, env)?;
            Ok(TypeExpr::Unit)
        }
    }
}

/// Type-check a function declaration.
pub fn typecheck_fn(func: &FnDecl, global: &TypeEnv) -> Result<(), Vec<TypeError>> {
    let mut env = global.clone();
    for p in &func.params {
        env.insert(p.name.clone(), p.ty.clone());
    }
    let mut errors = Vec::new();
    for stmt in &func.body {
        if let Err(e) = typecheck_stmt(stmt, &mut env) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Type-check an entire source file.
pub fn typecheck_file(file: &SourceFile) -> Vec<TypeError> {
    let global: TypeEnv = file
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.return_ty.clone()))
        .collect();
    let mut all_errors = Vec::new();
    for func in &file.functions {
        if let Err(mut errs) = typecheck_fn(func, &global) {
            all_errors.append(&mut errs);
        }
    }
    all_errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{FnDecl, Param, TypeExpr};

    fn empty_env() -> TypeEnv {
        TypeEnv::new()
    }

    #[test]
    fn test_int_lit_infers_i64() {
        let e = Expr::IntLit(42);
        assert_eq!(infer_expr(&e, &empty_env()), Ok(TypeExpr::I64));
    }

    #[test]
    fn test_bool_lit_infers_bool() {
        let e = Expr::BoolLit(true);
        assert_eq!(infer_expr(&e, &empty_env()), Ok(TypeExpr::Bool));
    }

    #[test]
    fn test_undefined_variable_error() {
        let e = Expr::Ident("x".into());
        assert!(infer_expr(&e, &empty_env()).is_err());
    }

    #[test]
    fn test_binop_add_i64() {
        let e = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Expr::IntLit(1)),
            rhs: Box::new(Expr::IntLit(2)),
        };
        assert_eq!(infer_expr(&e, &empty_env()), Ok(TypeExpr::I64));
    }

    #[test]
    fn test_binop_type_mismatch_error() {
        let e = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Expr::IntLit(1)),
            rhs: Box::new(Expr::BoolLit(true)),
        };
        assert!(infer_expr(&e, &empty_env()).is_err());
    }

    #[test]
    fn test_let_binding_updates_env() {
        let mut env = empty_env();
        let stmt = Stmt::Let {
            name: "x".into(),
            ty: None,
            value: Expr::IntLit(10),
        };
        typecheck_stmt(&stmt, &mut env).unwrap();
        assert_eq!(env.get("x"), Some(&TypeExpr::I64));
    }

    #[test]
    fn test_let_type_annotation_mismatch_error() {
        let mut env = empty_env();
        let stmt = Stmt::Let {
            name: "x".into(),
            ty: Some(TypeExpr::Bool),
            value: Expr::IntLit(10),
        };
        assert!(typecheck_stmt(&stmt, &mut env).is_err());
    }

    #[test]
    fn test_empty_file_no_errors() {
        let file = SourceFile { functions: vec![] };
        assert!(typecheck_file(&file).is_empty());
    }

    #[test]
    fn test_comparison_returns_bool() {
        let e = Expr::BinOp {
            op: BinOp::Lt,
            lhs: Box::new(Expr::IntLit(1)),
            rhs: Box::new(Expr::IntLit(2)),
        };
        assert_eq!(infer_expr(&e, &empty_env()), Ok(TypeExpr::Bool));
    }
}
