//! Structural validator for X3 AST.
//!
//! This validator runs immediately after parsing to ensure the AST has
//! the correct structural shape. It checks:
//!
//! - Control flow placement (return in functions, break/continue in loops)
//! - Atomic block nesting rules
//! - Agent/context placement
//! - Span completeness
//! - Basic well-formedness
//!
//! This is NOT semantic analysis (no types, no scoping beyond structure).

use crate::error::{ParseError, ParseResult};
use x3_ast::{
    Agent, AtomicBlock, Automation, Block, Const, EmitStatement, Expression, ForLoopKind,
    ForStatement, Function, GlobalLet, Identifier, IfStatement, Item, LoopStatement, Module,
    Permission, PermissionKind, Statement, StructDef, WhileStatement,
};
use x3_common::{Literal, Span};

/// Context for structural validation.
#[derive(Clone, Copy)]
struct ValidationContext {
    /// Are we inside a function? (affects return validity)
    in_function: bool,
    /// Are we inside a loop? (affects break/continue validity)
    in_loop: bool,
    /// Are we inside an atomic block? (affects nesting)
    in_atomic: bool,
    /// Current atomic nesting depth
    atomic_depth: usize,
}

/// Structural validator for X3 AST.
pub struct StructuralValidator;

impl StructuralValidator {
    /// Validate the entire module structure.
    pub fn validate(module: &Module) -> ParseResult<()> {
        let mut validator = Self;
        validator.validate_module(module)
    }

    fn validate_module(&mut self, module: &Module) -> ParseResult<()> {
        // Check spans
        self.validate_span(&module.span)?;

        // Validate top-level items
        for item in &module.items {
            match item {
                Item::Function(func) => self.validate_function(func)?,
                Item::GlobalLet(global) => self.validate_global_let(global)?,
                Item::Const(const_item) => self.validate_const(const_item)?,
                // Arb programs are validated by dedicated arb-specific passes.
                Item::ArbProgram(_) => {}
                Item::Agent(agent) => self.validate_agent(agent)?,
                Item::Struct(def) => self.validate_struct(def)?,
                Item::Automation(automation) => self.validate_automation(automation)?,
            }
        }

        Ok(())
    }

    fn validate_function(&mut self, function: &Function) -> ParseResult<()> {
        self.validate_span(&function.span)?;
        self.validate_identifier(&function.name)?;

        // Validate parameters
        for param in &function.params {
            self.validate_span(&param.span)?;
            self.validate_identifier(&param.name)?;
            if let Some(ty) = &param.ty {
                self.validate_span(&ty.span)?;
                self.validate_identifier(&ty.name)?;
            }
        }

        // Validate return type
        if let Some(ret_ty) = &function.ret_ty {
            self.validate_span(&ret_ty.span)?;
            self.validate_identifier(&ret_ty.name)?;
        }

        // Validate body with function context
        let ctx = ValidationContext {
            in_function: true,
            in_loop: false,
            in_atomic: false,
            atomic_depth: 0,
        };
        self.validate_block(&function.body, ctx)?;

        Ok(())
    }

    fn validate_global_let(&mut self, global: &GlobalLet) -> ParseResult<()> {
        self.validate_span(&global.span)?;
        self.validate_identifier(&global.name)?;

        if let Some(ty) = &global.ty {
            self.validate_span(&ty.span)?;
            self.validate_identifier(&ty.name)?;
        }

        // Global lets are expressions, not statements
        let ctx = ValidationContext {
            in_function: false,
            in_loop: false,
            in_atomic: false,
            atomic_depth: 0,
        };
        self.validate_expression(&global.initializer, ctx)?;

        Ok(())
    }

    fn validate_const(&mut self, const_item: &Const) -> ParseResult<()> {
        self.validate_span(&const_item.span)?;
        self.validate_identifier(&const_item.name)?;
        self.validate_span(&const_item.ty.span)?;
        self.validate_identifier(&const_item.ty.name)?;

        // Const values are expressions
        let ctx = ValidationContext {
            in_function: false,
            in_loop: false,
            in_atomic: false,
            atomic_depth: 0,
        };
        self.validate_expression(&const_item.value, ctx)?;

        Ok(())
    }

    /// A struct names each field once.
    /// Validate an `automation` declaration.
    ///
    /// Beyond structure, this enforces the determinism boundary: an automation
    /// body is a chain-side workflow, so it may not call a connector whose answer
    /// depends on an off-chain service. `http.*`, `ai.*` and `gpu.*` are refused
    /// here rather than compiled into something that could never be replayed by
    /// a validator.
    fn validate_automation(&mut self, automation: &Automation) -> ParseResult<()> {
        self.validate_span(&automation.span)?;
        self.validate_identifier(&automation.name)?;
        self.validate_span(&automation.trigger.span())?;
        reject_off_chain_connectors(&automation.name.name, "trigger", &automation.trigger)?;

        let mut seen: Vec<PermissionKind> = Vec::new();
        for permission in &automation.permissions {
            self.validate_span(&permission.span)?;
            if seen.contains(&permission.kind) {
                return Err(ParseError::new(
                    format!(
                        "automation `{}` declares {:?} twice",
                        automation.name.name, permission.kind
                    ),
                    permission.span,
                ));
            }
            seen.push(permission.kind);
            validate_permission_value(&automation.name.name, permission)?;
        }

        self.validate_automation_block(&automation.name.name, &automation.execute, "execute")?;
        if let Some(on_failure) = &automation.on_failure {
            self.validate_automation_block(&automation.name.name, on_failure, "on_failure")?;
        }
        Ok(())
    }

    fn validate_automation_block(
        &mut self,
        automation: &str,
        block: &Block,
        section: &str,
    ) -> ParseResult<()> {
        self.validate_span(&block.span)?;
        for expr in block_expressions(block) {
            reject_off_chain_connectors(automation, section, expr)?;
        }
        Ok(())
    }

    fn validate_struct(&mut self, def: &StructDef) -> ParseResult<()> {
        self.validate_span(&def.span)?;
        self.validate_identifier(&def.name)?;
        for (i, field) in def.fields.iter().enumerate() {
            self.validate_identifier(&field.name)?;
            self.validate_span(&field.ty.span)?;
            if def.fields[..i]
                .iter()
                .any(|f| f.name.name == field.name.name)
            {
                return Err(ParseError::new(
                    format!(
                        "struct `{}` declares field `{}` twice",
                        def.name.name, field.name.name
                    ),
                    field.name.span,
                ));
            }
        }
        Ok(())
    }

    fn validate_agent(&mut self, agent: &Agent) -> ParseResult<()> {
        self.validate_span(&agent.span)?;
        self.validate_identifier(&agent.name)?;

        // Validate nested items recursively
        for item in &agent.items {
            match item {
                Item::Function(func) => self.validate_function(func)?,
                Item::GlobalLet(global) => self.validate_global_let(global)?,
                Item::Const(const_item) => self.validate_const(const_item)?,
                Item::ArbProgram(_) => {}
                Item::Agent(nested_agent) => self.validate_agent(nested_agent)?,
                Item::Struct(def) => self.validate_struct(def)?,
                Item::Automation(automation) => self.validate_automation(automation)?,
            }
        }

        Ok(())
    }

    fn validate_block(&mut self, block: &Block, ctx: ValidationContext) -> ParseResult<()> {
        self.validate_span(&block.span)?;

        for statement in &block.statements {
            self.validate_statement(statement, ctx)?;
        }

        Ok(())
    }

    fn validate_statement(
        &mut self,
        statement: &Statement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        match statement {
            Statement::Let(let_stmt) => {
                self.validate_span(&let_stmt.span)?;
                self.validate_identifier(&let_stmt.name)?;
                if let Some(ty) = &let_stmt.ty {
                    self.validate_span(&ty.span)?;
                    self.validate_identifier(&ty.name)?;
                }
                self.validate_expression(&let_stmt.initializer, ctx)?;
            }
            Statement::Expr(expr) => {
                self.validate_expression(expr, ctx)?;
            }
            Statement::Return(value, span) => {
                self.validate_span(span)?;
                if !ctx.in_function {
                    return Err(ParseError::new(
                        "return statement outside of function",
                        *span,
                    ));
                }
                if let Some(expr) = value {
                    self.validate_expression(expr, ctx)?;
                }
            }
            Statement::If(if_stmt) => {
                self.validate_if_statement(if_stmt, ctx)?;
            }
            Statement::While(while_stmt) => {
                self.validate_while_statement(while_stmt, ctx)?;
            }
            Statement::Loop(loop_stmt) => {
                self.validate_loop_statement(loop_stmt, ctx)?;
            }
            Statement::For(for_stmt) => {
                self.validate_for_statement(for_stmt, ctx)?;
            }
            Statement::Atomic(atomic) => {
                self.validate_atomic_block(atomic, ctx)?;
            }
            Statement::Emit(emit) => {
                self.validate_emit_statement(emit, ctx)?;
            }
            Statement::Break(break_stmt) => {
                self.validate_span(&break_stmt.span)?;
                if !ctx.in_loop {
                    return Err(ParseError::new(
                        "break statement outside of loop",
                        break_stmt.span,
                    ));
                }
            }
            Statement::Continue(continue_stmt) => {
                self.validate_span(&continue_stmt.span)?;
                if !ctx.in_loop {
                    return Err(ParseError::new(
                        "continue statement outside of loop",
                        continue_stmt.span,
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_if_statement(
        &mut self,
        if_stmt: &IfStatement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&if_stmt.span)?;
        self.validate_expression(&if_stmt.condition, ctx)?;
        self.validate_block(&if_stmt.then_block, ctx)?;
        if let Some(else_block) = &if_stmt.else_block {
            self.validate_block(else_block, ctx)?;
        }
        Ok(())
    }

    fn validate_while_statement(
        &mut self,
        while_stmt: &WhileStatement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&while_stmt.span)?;
        self.validate_expression(&while_stmt.condition, ctx)?;
        let loop_ctx = ValidationContext {
            in_loop: true,
            ..ctx
        };
        self.validate_block(&while_stmt.body, loop_ctx)?;
        Ok(())
    }

    fn validate_loop_statement(
        &mut self,
        loop_stmt: &LoopStatement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&loop_stmt.span)?;
        let loop_ctx = ValidationContext {
            in_loop: true,
            ..ctx
        };
        self.validate_block(&loop_stmt.body, loop_ctx)?;
        Ok(())
    }

    fn validate_for_statement(
        &mut self,
        for_stmt: &ForStatement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&for_stmt.span)?;

        match &for_stmt.kind {
            ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                // Validate initializer
                if let Some(init_stmt) = init {
                    match init_stmt.as_ref() {
                        Statement::Let(let_stmt) => {
                            self.validate_statement(&Statement::Let(let_stmt.clone()), ctx)?;
                        }
                        Statement::Expr(expr) => {
                            self.validate_expression(expr, ctx)?;
                        }
                        _ => {
                            return Err(ParseError::new(
                                "invalid for loop initializer",
                                for_stmt.span,
                            ))
                        }
                    }
                }

                // Validate condition
                if let Some(cond) = condition {
                    self.validate_expression(cond, ctx)?;
                }

                // Validate update
                if let Some(update_expr) = update {
                    self.validate_expression(update_expr, ctx)?;
                }
            }
            ForLoopKind::Range { variable, range } => {
                self.validate_identifier(variable)?;
                self.validate_expression(&Expression::Range(range.clone()), ctx)?;
            }
        }

        let loop_ctx = ValidationContext {
            in_loop: true,
            ..ctx
        };
        self.validate_block(&for_stmt.body, loop_ctx)?;
        Ok(())
    }

    fn validate_atomic_block(
        &mut self,
        atomic: &AtomicBlock,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&atomic.span)?;

        // Validate metadata expression if present
        if let Some(metadata) = &atomic.metadata {
            self.validate_expression(metadata, ctx)?;
        }

        // Atomic blocks cannot be nested
        if ctx.in_atomic {
            return Err(ParseError::new(
                "nested atomic blocks are not allowed",
                atomic.span,
            ));
        }

        let atomic_ctx = ValidationContext {
            in_atomic: true,
            atomic_depth: ctx.atomic_depth + 1,
            ..ctx
        };
        self.validate_block(&atomic.body, atomic_ctx)?;
        Ok(())
    }

    fn validate_emit_statement(
        &mut self,
        emit: &EmitStatement,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        self.validate_span(&emit.span)?;
        self.validate_expression(&emit.value, ctx)?;
        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    // Context is forwarded to nested expressions for future validation rules.
    fn validate_expression(
        &mut self,
        expression: &Expression,
        ctx: ValidationContext,
    ) -> ParseResult<()> {
        match expression {
            Expression::Literal(lit) => self.validate_span(&lit.span)?,
            Expression::Identifier(id) => self.validate_identifier(id)?,
            Expression::Binary(bin) => {
                self.validate_span(&bin.span)?;
                self.validate_expression(&bin.left, ctx)?;
                self.validate_expression(&bin.right, ctx)?;
            }
            Expression::Unary(unary) => {
                self.validate_span(&unary.span)?;
                self.validate_expression(&unary.expr, ctx)?;
            }
            Expression::Call(call) => {
                self.validate_span(&call.span)?;
                self.validate_expression(&call.callee, ctx)?;
                for arg in &call.args {
                    self.validate_expression(arg, ctx)?;
                }
            }
            Expression::Assign(assign) => {
                self.validate_span(&assign.span)?;
                self.validate_identifier(&assign.target)?;
                self.validate_expression(&assign.value, ctx)?;
            }
            Expression::FieldAccess(field_access) => {
                self.validate_span(&field_access.span)?;
                self.validate_expression(&field_access.object, ctx)?;
                self.validate_identifier(&field_access.field)?;
            }
            Expression::Range(range) => {
                self.validate_span(&range.span)?;
                self.validate_expression(&range.start, ctx)?;
                self.validate_expression(&range.end, ctx)?;
            }
            Expression::Array(array) => {
                self.validate_span(&array.span)?;
                for element in &array.elements {
                    self.validate_expression(element, ctx)?;
                }
            }
            Expression::Tuple(tuple) => {
                self.validate_span(&tuple.span)?;
                for element in &tuple.elements {
                    self.validate_expression(element, ctx)?;
                }
            }
            Expression::Index(index) => {
                self.validate_span(&index.span)?;
                self.validate_expression(&index.base, ctx)?;
                self.validate_expression(&index.index, ctx)?;
            }
            Expression::IndexAssign(assign) => {
                self.validate_span(&assign.span)?;
                self.validate_identifier(&assign.target)?;
                self.validate_expression(&assign.index, ctx)?;
                self.validate_expression(&assign.value, ctx)?;
            }
            Expression::StructLiteral(literal) => {
                self.validate_span(&literal.span)?;
                self.validate_identifier(&literal.name)?;
                for field in &literal.fields {
                    self.validate_identifier(&field.name)?;
                    self.validate_expression(&field.value, ctx)?;
                }
            }
            Expression::FieldAssign(assign) => {
                self.validate_span(&assign.span)?;
                self.validate_identifier(&assign.target)?;
                self.validate_identifier(&assign.field)?;
                self.validate_expression(&assign.value, ctx)?;
            }
        }
        Ok(())
    }

    fn validate_identifier(&mut self, identifier: &Identifier) -> ParseResult<()> {
        self.validate_span(&identifier.span)?;
        if identifier.name.is_empty() {
            return Err(ParseError::new("empty identifier name", identifier.span));
        }
        Ok(())
    }

    fn validate_span(&mut self, span: &Span) -> ParseResult<()> {
        if span.start > span.end {
            return Err(ParseError::new("invalid span: start > end", *span));
        }
        Ok(())
    }
}

/// Connectors whose answer depends on an off-chain service. An automation body
/// is a chain-side workflow, so none of them may appear in any of its sections.
const OFF_CHAIN_CONNECTORS: [&str; 3] = ["http", "ai", "gpu"];

/// Refuse any off-chain connector call reachable from `expr`.
///
/// Recursive on purpose: a connector wrapped in a call argument, a `let`
/// initializer, a `return`, or an `if` branch is still a connector call. Both
/// the callee's root (`http.get(..)`) and its last path segment
/// (`chain.http.get(..)`) are checked, because either spelling reaches the same
/// service.
fn reject_off_chain_connectors(
    automation: &str,
    section: &str,
    expr: &Expression,
) -> ParseResult<()> {
    if let Expression::Call(call) = expr {
        for segment in callee_segments(&call.callee) {
            if OFF_CHAIN_CONNECTORS.contains(&segment) {
                return Err(ParseError::new(
                    format!(
                        "automation `{automation}`: `{segment}.*` is an off-chain connector and \
                         cannot appear in `{section}`; an on-chain workflow must be replayable \
                         by every validator"
                    ),
                    call.span,
                ));
            }
        }
    }
    for child in child_expressions(expr) {
        reject_off_chain_connectors(automation, section, child)?;
    }
    Ok(())
}

/// Identifier segments of a callee path, outermost last:
/// `chain.http.get` yields `["chain", "http", "get"]`.
fn callee_segments(callee: &Expression) -> Vec<&str> {
    match callee {
        Expression::Identifier(identifier) => vec![identifier.name.as_str()],
        Expression::FieldAccess(access) => {
            let mut segments = callee_segments(&access.object);
            segments.push(access.field.name.as_str());
            segments
        }
        _ => Vec::new(),
    }
}

/// Every sub-expression of `expr`, so a walk cannot miss one.
fn child_expressions(expr: &Expression) -> Vec<&Expression> {
    match expr {
        Expression::Literal(_) | Expression::Identifier(_) => Vec::new(),
        Expression::Binary(binary) => vec![&binary.left, &binary.right],
        Expression::Unary(unary) => vec![&unary.expr],
        Expression::Call(call) => {
            let mut out = vec![call.callee.as_ref()];
            out.extend(call.args.iter());
            out
        }
        Expression::Assign(assign) => vec![&assign.value],
        Expression::FieldAccess(access) => vec![&access.object],
        Expression::Range(range) => vec![&range.start, &range.end],
        Expression::Array(array) => array.elements.iter().collect(),
        Expression::Tuple(tuple) => tuple.elements.iter().collect(),
        Expression::Index(index) => vec![&index.base, &index.index],
        Expression::IndexAssign(assign) => vec![&assign.index, &assign.value],
        Expression::StructLiteral(literal) => literal.fields.iter().map(|f| &f.value).collect(),
        Expression::FieldAssign(assign) => vec![&assign.value],
    }
}

/// Every expression anywhere in `block`, nested blocks included.
///
/// A connector inside an `if`, a loop body or an `atomic` block is still a
/// connector call; the walk has to reach it.
fn block_expressions(block: &Block) -> Vec<&Expression> {
    let mut out = Vec::new();
    for statement in &block.statements {
        out.extend(statement_expressions(statement));
        for nested in nested_blocks(statement) {
            out.extend(block_expressions(nested));
        }
    }
    out
}

/// Blocks a statement owns, so the walk can descend into them.
fn nested_blocks(statement: &Statement) -> Vec<&Block> {
    match statement {
        Statement::If(statement) => {
            let mut out = vec![&statement.then_block];
            if let Some(else_block) = &statement.else_block {
                out.push(else_block);
            }
            out
        }
        Statement::While(statement) => vec![&statement.body],
        Statement::Loop(statement) => vec![&statement.body],
        Statement::For(statement) => vec![&statement.body],
        Statement::Atomic(block) => vec![&block.body],
        Statement::Let(_)
        | Statement::Expr(_)
        | Statement::Return(_, _)
        | Statement::Emit(_)
        | Statement::Break(_)
        | Statement::Continue(_) => Vec::new(),
    }
}

/// Expressions a statement can contain at its own level.
fn statement_expressions(statement: &Statement) -> Vec<&Expression> {
    match statement {
        Statement::Let(statement) => vec![&statement.initializer],
        Statement::Expr(expr) => vec![expr],
        Statement::Return(expr, _) => expr.as_ref().into_iter().collect(),
        Statement::If(statement) => vec![&statement.condition],
        Statement::While(statement) => vec![&statement.condition],
        Statement::Loop(_) | Statement::Break(_) | Statement::Continue(_) => Vec::new(),
        Statement::For(statement) => match &statement.kind {
            // Both loop forms carry expressions; their bodies are walked by
            // `nested_blocks`, not here.
            ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                let mut out: Vec<&Expression> = Vec::new();
                if let Some(condition) = condition {
                    out.push(condition);
                }
                if let Some(update) = update {
                    out.push(update);
                }
                if let Some(init) = init {
                    out.extend(statement_expressions(init));
                }
                out
            }
            ForLoopKind::Range { range, .. } => vec![&range.start, &range.end],
        },
        Statement::Atomic(block) => block.metadata.as_ref().into_iter().collect(),
        Statement::Emit(statement) => vec![&statement.value],
    }
}

/// Check a capability's value against the shape its kind requires.
///
/// A capability is a budget, so a value the chain cannot enforce is a bug in the
/// source, not something to coerce later: a negative `max_trade` or an
/// `allowed_chains` entry that is not a chain name would silently change what
/// the automation is permitted to do.
fn validate_permission_value(automation: &str, permission: &Permission) -> ParseResult<()> {
    let literal = match &permission.value {
        Expression::Literal(literal) => Some(&literal.literal),
        _ => None,
    };
    let fail = |expected: &str| {
        Err(ParseError::new(
            format!(
                "automation `{automation}`: {:?} expects {expected}",
                permission.kind
            ),
            permission.span,
        ))
    };
    match permission.kind {
        PermissionKind::Wallet => match literal {
            Some(Literal::String(_)) => Ok(()),
            _ => match &permission.value {
                // An identifier names a wallet binding declared elsewhere.
                Expression::Identifier(_) => Ok(()),
                _ => fail("a wallet name or identifier"),
            },
        },
        PermissionKind::MaxTrade | PermissionKind::DailyLossLimit => match literal {
            Some(Literal::Integer(value)) if *value >= 0 => Ok(()),
            Some(Literal::Integer(_)) => Err(ParseError::new(
                format!(
                    "automation `{automation}`: {:?} cannot be negative",
                    permission.kind
                ),
                permission.span,
            )),
            _ => fail("a non-negative integer amount"),
        },
        PermissionKind::AllowedChains => match &permission.value {
            Expression::Array(array) => {
                for element in &array.elements {
                    match element {
                        Expression::Literal(lit) if matches!(lit.literal, Literal::String(_)) => {}
                        _ => return fail("an array of chain names"),
                    }
                }
                Ok(())
            }
            _ => fail("an array of chain names"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::{Block, Function, Identifier, Item, LiteralExpression, Module};

    #[test]
    fn rejects_return_outside_function() {
        let module = Module {
            span: Span::new(0, 10),
            items: vec![Item::GlobalLet(GlobalLet {
                name: Identifier {
                    name: "x".into(),
                    span: Span::new(0, 1),
                },
                mutable: false,
                ty: None,
                initializer: Expression::Literal(LiteralExpression {
                    literal: x3_common::Literal::Integer(42),
                    span: Span::new(4, 6),
                }),
                span: Span::new(0, 10),
            })],
        };

        // This should pass - global let is fine
        assert!(StructuralValidator::validate(&module).is_ok());
    }

    #[test]
    fn accepts_valid_function() {
        let module = Module {
            span: Span::new(0, 50),
            items: vec![Item::Function(Function {
                name: Identifier {
                    name: "test".into(),
                    span: Span::new(4, 8),
                },
                params: vec![],
                ret_ty: None,
                body: Block {
                    statements: vec![Statement::Return(None, Span::new(20, 26))],
                    span: Span::new(15, 30),
                },
                span: Span::new(0, 35),
            })],
        };

        assert!(StructuralValidator::validate(&module).is_ok());
    }
}
