//! Type checker - walks the AST and performs type checking.
//!
//! This is the main entry point for type checking. It combines:
//! - Type environment from name resolution
//! - Type inference for expressions
//! - Constraint solving via unification
//! - Error collection and reporting

use serde::{Deserialize, Serialize};

use x3_ast::{
    Agent, AssignExpression, AtomicBlock, BinaryExpression, Block, CallExpression, Const,
    Expression, FieldAccessExpression, ForLoopKind, ForStatement, Function, GlobalLet, Identifier,
    IfStatement, Item, LetStatement, LiteralExpression, LoopStatement, Module, RangeExpression,
    Statement, UnaryExpression, WhileStatement,
};
use x3_common::{Literal, Span};
use x3_semantics::{ResolvedModule, ScopeId, SymbolId};

use crate::env::TypeEnv;
use crate::error::{TypeError, TypeErrorKind, TypeResult};
use crate::infer::TypeInference;
use crate::types::{AgentType, FunctionSignature, PrimitiveType, Type, TypeKind};

/// The result of type checking a module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypedModule {
    /// Type environment with all type bindings.
    #[serde(skip)]
    pub env: TypeEnv,
    /// Expression types indexed by span start position.
    pub expr_types: Vec<(Span, Type)>,
}

impl TypedModule {
    /// Get the type of an expression at the given span.
    pub fn type_at(&self, span: Span) -> Option<&Type> {
        self.expr_types
            .iter()
            .find(|(s, _)| s.start == span.start)
            .map(|(_, t)| t)
    }
}

/// The type checker.
pub struct TypeChecker {
    /// Type environment.
    env: TypeEnv,
    /// Collected errors.
    errors: Vec<TypeError>,
    /// Expression types.
    expr_types: Vec<(Span, Type)>,
    /// Current function's return type (for checking returns).
    current_return_type: Option<Type>,
    /// Whether we're in an atomic block.
    in_atomic: bool,
    /// Integer-literal type variables and the literal values each one stands for.
    ///
    /// An unsuffixed integer literal takes the integer type its use requires (RFC t5-6, amended
    /// 2026-09-26): it is a type variable here until the first concrete integer type it meets binds
    /// it, at which point every value it stands for must fit that type. A variable never bound
    /// defaults to `i64`, the X3VM's integer.
    int_literals: std::collections::BTreeMap<u32, Vec<i128>>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Create a new type checker.
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
            expr_types: Vec::new(),
            current_return_type: None,
            in_atomic: false,
            int_literals: std::collections::BTreeMap::new(),
        }
    }

    /// Type check a module given the resolved symbol information.
    pub fn check(mut self, module: &Module, resolved: &ResolvedModule) -> TypeResult<TypedModule> {
        // First pass: collect all type declarations
        self.collect_declarations(module, resolved);

        // Second pass: type check all items
        for item in &module.items {
            self.check_item(item, resolved);
        }

        // Return result
        if self.errors.is_empty() {
            Ok(TypedModule {
                env: self.env,
                expr_types: self.expr_types,
            })
        } else {
            Err(self.errors)
        }
    }

    /// Collect type declarations from all items.
    fn collect_declarations(&mut self, module: &Module, resolved: &ResolvedModule) {
        for item in &module.items {
            match item {
                Item::Function(func) => self.collect_function_type(func, resolved),
                Item::Agent(agent) => self.collect_agent_type(agent, resolved),
                Item::GlobalLet(global) => self.collect_global_type(global, resolved),
                Item::Const(const_item) => self.collect_const_type(const_item, resolved),
                // Arb programs are lowered by later compilation phases.
                Item::ArbProgram(_) => {}
            }
        }
    }

    /// Collect a function's type signature.
    fn collect_function_type(&mut self, func: &Function, resolved: &ResolvedModule) {
        // Parse parameter types
        let params: Vec<Type> = func
            .params
            .iter()
            .map(|p| {
                if let Some(ref ty) = p.ty {
                    self.resolve_type_annotation(ty)
                } else {
                    // No type annotation - create a type variable
                    self.env.fresh_type_var()
                }
            })
            .collect();

        // Parse return type
        let return_type = if let Some(ref ret) = func.ret_ty {
            self.resolve_type_annotation(ret)
        } else {
            // Default to unit for no return type
            Type::unit()
        };

        let sig = FunctionSignature::new(params, return_type);

        // Look up the function's symbol ID
        if let Some(symbol_id) = self.symbol_defined_at(func.name.span, resolved) {
            self.env.register_function(symbol_id, sig.clone());
            self.env.bind(
                ScopeId(0), // Global scope
                symbol_id,
                Type::new(TypeKind::Function(sig)),
            );
        }
    }

    /// Collect an agent's type definition.
    fn collect_agent_type(&mut self, agent: &Agent, resolved: &ResolvedModule) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        for item in &agent.items {
            match item {
                Item::GlobalLet(global) => {
                    let ty = if let Some(ref ann) = global.ty {
                        self.resolve_type_annotation(ann)
                    } else {
                        self.infer_expression_type(&global.initializer, resolved)
                    };
                    fields.push((global.name.name.clone(), ty));
                }
                Item::Function(func) => {
                    let params: Vec<Type> = func
                        .params
                        .iter()
                        .map(|p| {
                            if let Some(ref ty) = p.ty {
                                self.resolve_type_annotation(ty)
                            } else {
                                self.env.fresh_type_var()
                            }
                        })
                        .collect();
                    let return_type = if let Some(ref ret) = func.ret_ty {
                        self.resolve_type_annotation(ret)
                    } else {
                        Type::unit()
                    };
                    methods.push((
                        func.name.name.clone(),
                        FunctionSignature::method(params, return_type),
                    ));
                }
                Item::Const(const_item) => {
                    let ty = self.resolve_type_annotation(&const_item.ty);
                    fields.push((const_item.name.name.clone(), ty));
                }
                Item::ArbProgram(_) => {
                    // Not supported as an inner agent item.
                }
                Item::Agent(_) => {
                    // Nested agents are handled by semantics - skip here
                }
            }
        }

        let agent_type = AgentType {
            name: agent.name.name.clone(),
            fields,
            methods,
        };

        self.env.register_agent(agent.name.name.clone(), agent_type);
    }

    /// Collect a global variable's type.
    fn collect_global_type(&mut self, global: &GlobalLet, resolved: &ResolvedModule) {
        let ty = if let Some(ref ann) = global.ty {
            self.resolve_type_annotation(ann)
        } else {
            self.infer_expression_type(&global.initializer, resolved)
        };

        if let Some(symbol_id) = self.symbol_defined_at(global.name.span, resolved) {
            self.env.bind(ScopeId(0), symbol_id, ty);
        }
    }

    /// Collect a constant's type.
    fn collect_const_type(&mut self, const_item: &Const, resolved: &ResolvedModule) {
        let ty = self.resolve_type_annotation(&const_item.ty);

        if let Some(symbol_id) = self.symbol_defined_at(const_item.name.span, resolved) {
            self.env.bind(ScopeId(0), symbol_id, ty);
        }
    }

    /// Resolve a type annotation to a Type.
    fn resolve_type_annotation(&mut self, annotation: &x3_ast::TypeAnnotation) -> Type {
        // For now, simple name-based lookup
        if let Some(ty) = self.env.lookup_type(&annotation.name.name) {
            return ty.clone();
        }

        // Handle common generic type patterns by parsing the name
        // e.g., "vec<u64>" would be parsed if TypeAnnotation supports it
        // For now, just return named type since AST doesn't have type_args
        Type::named(&annotation.name.name)
    }

    /// Type check an item.
    fn check_item(&mut self, item: &Item, resolved: &ResolvedModule) {
        match item {
            Item::Function(func) => self.check_function(func, resolved),
            Item::Agent(agent) => self.check_agent(agent, resolved),
            Item::GlobalLet(global) => self.check_global_let(global, resolved),
            Item::Const(const_item) => self.check_const(const_item, resolved),
            Item::ArbProgram(_) => {}
        }
    }

    /// Type check a function.
    fn check_function(&mut self, func: &Function, resolved: &ResolvedModule) {
        // Get the function's signature
        let sig = if let Some(symbol_id) = self.symbol_defined_at(func.name.span, resolved) {
            self.env.get_function_sig(symbol_id).cloned()
        } else {
            None
        };

        let return_type = sig
            .as_ref()
            .map(|s| s.return_type.as_ref().clone())
            .unwrap_or_else(Type::unit);

        // Bind parameter types to the environment
        if let Some(ref sig) = sig {
            for (param, param_ty) in func.params.iter().zip(sig.params.iter()) {
                if let Some(symbol_id) = self.symbol_defined_at(param.name.span, resolved) {
                    self.env.bind(ScopeId(0), symbol_id, param_ty.clone());
                }
            }
        }

        // Set current return type for checking return statements
        let prev_return_type = self.current_return_type.take();
        self.current_return_type = Some(return_type.clone());

        // Type check the body
        self.check_block(&func.body, resolved);

        // A function with a return type has to return on every path. One that fell off its end
        // compiled to a `Ret` of nothing, so `fn f() -> i64 { let x = 1; }` "returned" a unit
        // the caller then used as an integer.
        let returns_a_value = !matches!(return_type.kind, TypeKind::Unit | TypeKind::Never);
        if returns_a_value && !block_always_returns(&func.body.statements) {
            self.errors.push(TypeError::new(
                TypeErrorKind::MissingReturn {
                    expected: return_type.clone(),
                },
                func.span,
            ));
        }

        // Restore previous return type
        self.current_return_type = prev_return_type;
    }

    /// Type check an agent.
    fn check_agent(&mut self, agent: &Agent, resolved: &ResolvedModule) {
        for item in &agent.items {
            self.check_item(item, resolved);
        }
    }

    /// Type check a global let.
    fn check_global_let(&mut self, global: &GlobalLet, resolved: &ResolvedModule) {
        let init_type = self.infer_expression_type(&global.initializer, resolved);

        if let Some(ref ann) = global.ty {
            let declared_type = self.resolve_type_annotation(ann);
            self.check_type_compatibility(&declared_type, &init_type, global.span);
        }
    }

    /// Type check a const.
    fn check_const(&mut self, const_item: &Const, resolved: &ResolvedModule) {
        let declared_type = self.resolve_type_annotation(&const_item.ty);
        let init_type = self.infer_expression_type(&const_item.value, resolved);
        self.check_type_compatibility(&declared_type, &init_type, const_item.span);
    }

    /// Type check a block.
    fn check_block(&mut self, block: &Block, resolved: &ResolvedModule) {
        for stmt in &block.statements {
            self.check_statement(stmt, resolved);
        }
    }

    /// Type check a statement.
    fn check_statement(&mut self, stmt: &Statement, resolved: &ResolvedModule) {
        match stmt {
            Statement::Let(let_stmt) => self.check_let_statement(let_stmt, resolved),
            Statement::Expr(expr) => {
                self.infer_expression_type(expr, resolved);
            }
            Statement::Return(value, span) => self.check_return(value.as_ref(), *span, resolved),
            Statement::If(if_stmt) => self.check_if_statement(if_stmt, resolved),
            Statement::While(while_stmt) => self.check_while_statement(while_stmt, resolved),
            Statement::Loop(loop_stmt) => self.check_loop_statement(loop_stmt, resolved),
            Statement::For(for_stmt) => self.check_for_statement(for_stmt, resolved),
            Statement::Atomic(atomic) => self.check_atomic_block(atomic, resolved),
            Statement::Emit(emit) => self.check_emit_statement(emit, resolved),
            Statement::Break(_) | Statement::Continue(_) => {
                // These are validated by semantics, no type checking needed
            }
        }
    }

    /// Type check a let statement.
    fn check_let_statement(&mut self, let_stmt: &LetStatement, resolved: &ResolvedModule) {
        let init_type = self.infer_expression_type(&let_stmt.initializer, resolved);

        // An annotated binding has its declared type, not its initialiser's.
        let binding_type = if let Some(ref ann) = let_stmt.ty {
            let declared_type = self.resolve_type_annotation(ann);
            self.check_type_compatibility(&declared_type, &init_type, let_stmt.span);
            declared_type
        } else {
            init_type
        };

        // Bind the variable's type
        if let Some(symbol_id) = self.symbol_defined_at(let_stmt.name.span, resolved) {
            self.env.bind(ScopeId(0), symbol_id, binding_type);
        }
    }

    /// Type check a return statement.
    fn check_return(&mut self, value: Option<&Expression>, span: Span, resolved: &ResolvedModule) {
        let expected = self.current_return_type.clone().unwrap_or_else(Type::unit);

        let actual = if let Some(expr) = value {
            self.infer_expression_type(expr, resolved)
        } else {
            Type::unit()
        };

        self.check_type_compatibility(&expected, &actual, span);
    }

    /// Type check an if statement.
    fn check_if_statement(&mut self, if_stmt: &IfStatement, resolved: &ResolvedModule) {
        // Check condition is bool
        let cond_type = self.infer_expression_type(&if_stmt.condition, resolved);
        let cond_type = self.env.apply_substitutions(&cond_type);
        if !cond_type.is_bool() && !cond_type.is_error() {
            self.errors.push(TypeError::condition_not_bool(
                cond_type,
                if_stmt.condition.span(),
            ));
        }

        // Check branches
        self.check_block(&if_stmt.then_block, resolved);
        if let Some(ref else_block) = if_stmt.else_block {
            self.check_block(else_block, resolved);
        }
    }

    /// Type check a while statement.
    fn check_while_statement(&mut self, while_stmt: &WhileStatement, resolved: &ResolvedModule) {
        let cond_type = self.infer_expression_type(&while_stmt.condition, resolved);
        let cond_type = self.env.apply_substitutions(&cond_type);
        if !cond_type.is_bool() && !cond_type.is_error() {
            self.errors.push(TypeError::condition_not_bool(
                cond_type,
                while_stmt.condition.span(),
            ));
        }

        self.check_block(&while_stmt.body, resolved);
    }

    /// Type check a loop statement.
    fn check_loop_statement(&mut self, loop_stmt: &LoopStatement, resolved: &ResolvedModule) {
        self.check_block(&loop_stmt.body, resolved);
    }

    /// Type check a for statement.
    fn check_for_statement(&mut self, for_stmt: &ForStatement, resolved: &ResolvedModule) {
        match &for_stmt.kind {
            ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                if let Some(init) = init {
                    self.check_statement(init, resolved);
                }
                if let Some(cond) = condition {
                    let cond_type = self.infer_expression_type(cond, resolved);
                    let cond_type = self.env.apply_substitutions(&cond_type);
                    if !cond_type.is_bool() && !cond_type.is_error() {
                        self.errors
                            .push(TypeError::condition_not_bool(cond_type, cond.span()));
                    }
                }
                if let Some(update) = update {
                    self.infer_expression_type(update, resolved);
                }
            }
            ForLoopKind::Range { variable, range } => {
                // The bounds are integers and the loop variable is one: it was never bound, so
                // every use of it in the body was an unknown identifier.
                for bound in [&range.start, &range.end] {
                    let ty = self.infer_expression_type(bound, resolved);
                    self.check_type_compatibility(&Type::i64(), &ty, bound.span());
                }
                if let Some(symbol_id) = self.symbol_defined_at(variable.span, resolved) {
                    self.env.bind(ScopeId(0), symbol_id, Type::i64());
                }
            }
        }

        self.check_block(&for_stmt.body, resolved);
    }

    /// Type check an atomic block.
    fn check_atomic_block(&mut self, atomic: &AtomicBlock, resolved: &ResolvedModule) {
        let prev_in_atomic = self.in_atomic;
        self.in_atomic = true;

        self.check_block(&atomic.body, resolved);

        self.in_atomic = prev_in_atomic;
    }

    /// Type check an emit statement.
    /// `emit Name(args)`: the arguments have types, the event's name does not.
    ///
    /// Inferring the call as a whole looked the name up as a value and reported it not callable,
    /// which is the resolver's story repeated a phase later. An event's payload is checked; its
    /// name is a name.
    fn check_emit_statement(&mut self, emit: &x3_ast::EmitStatement, resolved: &ResolvedModule) {
        match &emit.value {
            Expression::Call(call) if matches!(&*call.callee, Expression::Identifier(_)) => {
                for arg in &call.args {
                    self.infer_expression_type(arg, resolved);
                }
            }
            other => {
                self.infer_expression_type(other, resolved);
            }
        }
    }

    /// Infer the type of an expression.
    fn infer_expression_type(&mut self, expr: &Expression, resolved: &ResolvedModule) -> Type {
        let ty = match expr {
            Expression::Literal(lit) => self.infer_literal_type(lit),
            // `-42` is negation applied to the literal `42` in the AST; as a *type* it is one
            // literal whose value is -42, so it cannot take an unsigned type.
            Expression::Unary(unary) if matches!(unary.op, x3_ast::UnaryOp::Negate) => {
                match &*unary.expr {
                    Expression::Literal(LiteralExpression {
                        literal: Literal::Integer(n),
                        ..
                    }) => self.int_literal(-(*n as i128)),
                    // Anything else keeps the ordinary unary inference, which is what the guard in
                    // front of this arm used to fall through to.
                    _ => self.infer_unary_type(unary, resolved),
                }
            }
            Expression::Identifier(ident) => self.infer_identifier_type(ident, resolved),
            Expression::Binary(bin) => self.infer_binary_type(bin, resolved),
            Expression::Unary(unary) => self.infer_unary_type(unary, resolved),
            Expression::Call(call) => self.infer_call_type(call, resolved),
            Expression::Assign(assign) => self.infer_assign_type(assign, resolved),
            Expression::FieldAccess(field) => self.infer_field_access_type(field, resolved),
            Expression::Range(range) => self.infer_range_type(range, resolved),
        };

        // Record expression type
        self.expr_types.push((expr.span(), ty.clone()));

        ty
    }

    /// Infer type of a literal.
    fn infer_literal_type(&mut self, lit: &LiteralExpression) -> Type {
        match &lit.literal {
            Literal::Integer(n) => self.int_literal(*n as i128),
            Literal::Float(_) => Type::new(TypeKind::Primitive(PrimitiveType::U64)), // Float uses U64 until proper float type is added
            Literal::String(_) => Type::string(),
            Literal::Bool(_) => Type::bool(),
            Literal::Unit => Type::unit(),
        }
    }

    /// Infer type of an identifier.
    fn infer_identifier_type(&mut self, ident: &Identifier, resolved: &ResolvedModule) -> Type {
        // Look up the symbol the resolver bound this use to. This was a lookup by *name* over
        // the whole module, so a local `x` in one function took the type of the first `x`
        // anywhere — another function's parameter, a global, a later shadowed binding.
        if let Some(symbol_id) = self.symbol_used_at(ident.span, resolved) {
            if let Some(ty) = self.env.get(symbol_id) {
                return ty.clone();
            }
        }

        // Not found - return error type and record error
        self.errors
            .push(TypeError::unknown_type(&ident.name, ident.span));
        Type::error()
    }

    /// Infer type of a binary expression.
    fn infer_binary_type(&mut self, bin: &BinaryExpression, resolved: &ResolvedModule) -> Type {
        let left_type = self.infer_expression_type(&bin.left, resolved);
        let right_type = self.infer_expression_type(&bin.right, resolved);
        let op = format!("{:?}", bin.op);

        // An integer literal on either side takes the other side's type, and two literals stay one
        // literal until something binds them. Logical operators take no integers at all, so a
        // literal there falls through to the inference below and is refused.
        let logical = matches!(
            bin.op,
            x3_ast::BinaryOp::LogicalAnd | x3_ast::BinaryOp::LogicalOr
        );
        let (left_var, right_var) = (
            self.unbound_int_literal(&left_type),
            self.unbound_int_literal(&right_type),
        );
        if !logical && (left_var.is_some() || right_var.is_some()) {
            let other = if left_var.is_some() {
                &right_type
            } else {
                &left_type
            };
            let literal = if left_var.is_some() {
                &left_type
            } else {
                &right_type
            };
            if !self.accept(other, literal) {
                self.errors.push(TypeError::invalid_binary_op(
                    &op,
                    self.env.apply_substitutions(&left_type),
                    self.env.apply_substitutions(&right_type),
                    bin.span,
                ));
                return Type::error();
            }
            let operand = self.env.apply_substitutions(&left_type);
            return match bin.op {
                x3_ast::BinaryOp::Equal
                | x3_ast::BinaryOp::NotEqual
                | x3_ast::BinaryOp::Less
                | x3_ast::BinaryOp::LessEqual
                | x3_ast::BinaryOp::Greater
                | x3_ast::BinaryOp::GreaterEqual => Type::bool(),
                _ => operand,
            };
        }
        let left_type = self.env.apply_substitutions(&left_type);
        let right_type = self.env.apply_substitutions(&right_type);

        let mut infer = TypeInference::new(&mut self.env);

        match infer.infer_binary_op(&op, &left_type, &right_type, bin.span) {
            Ok(ty) => ty,
            Err(err) => {
                self.errors.push(*err);
                Type::error()
            }
        }
    }

    /// Infer type of a unary expression.
    fn infer_unary_type(&mut self, unary: &UnaryExpression, resolved: &ResolvedModule) -> Type {
        let operand_type = self.infer_expression_type(&unary.expr, resolved);
        // Negating a still-unbound literal is still that literal's integer type.
        if matches!(unary.op, x3_ast::UnaryOp::Negate)
            && self.unbound_int_literal(&operand_type).is_some()
        {
            return operand_type;
        }
        let operand_type = self.env.apply_substitutions(&operand_type);

        let op = format!("{:?}", unary.op);
        let mut infer = TypeInference::new(&mut self.env);

        match infer.infer_unary_op(&op, &operand_type, unary.span) {
            Ok(ty) => ty,
            Err(err) => {
                self.errors.push(*err);
                Type::error()
            }
        }
    }

    /// Infer type of a function call.
    fn infer_call_type(&mut self, call: &CallExpression, resolved: &ResolvedModule) -> Type {
        // A host call the resolver left unbound (the program declares no such name) takes its
        // signature from `x3_common::intrinsics`: every parameter an `i64`, and an `i64` or unit
        // result.
        let host_call = match &*call.callee {
            Expression::Identifier(ident)
                if self.symbol_used_at(ident.span, resolved).is_none() =>
            {
                x3_common::intrinsics::by_name(&ident.name)
            }
            _ => None,
        };
        // Get the callee type
        let callee_type = match host_call {
            Some(intrinsic) => {
                let i64_ty = || Type::new(TypeKind::Primitive(PrimitiveType::I64));
                Type::new(TypeKind::Function(FunctionSignature::new(
                    (0..intrinsic.arity).map(|_| i64_ty()).collect(),
                    if intrinsic.returns_value {
                        i64_ty()
                    } else {
                        Type::unit()
                    },
                )))
            }
            None => self.infer_expression_type(&call.callee, resolved),
        };

        match &callee_type.kind {
            TypeKind::Function(sig) => {
                // Check argument count
                if call.args.len() != sig.params.len() {
                    self.errors.push(TypeError::wrong_argument_count(
                        sig.params.len(),
                        call.args.len(),
                        call.span,
                    ));
                    return Type::error();
                }

                // Check argument types
                for (i, (arg, param_ty)) in call.args.iter().zip(sig.params.iter()).enumerate() {
                    let arg_type = self.infer_expression_type(arg, resolved);
                    if !self.accept(param_ty, &arg_type) {
                        self.errors.push(TypeError::argument_type_mismatch(
                            i,
                            param_ty.clone(),
                            arg_type,
                            arg.span(),
                        ));
                    }
                }

                sig.return_type.as_ref().clone()
            }
            TypeKind::Error => Type::error(),
            _ => {
                self.errors
                    .push(TypeError::not_callable(callee_type, call.span));
                Type::error()
            }
        }
    }

    /// Infer type of an assignment.
    fn infer_assign_type(&mut self, assign: &AssignExpression, resolved: &ResolvedModule) -> Type {
        let target_type = self.infer_identifier_type(&assign.target, resolved);
        let value_type = self.infer_expression_type(&assign.value, resolved);

        self.check_type_compatibility(&target_type, &value_type, assign.span);

        target_type
    }

    /// Infer type of a field access.
    fn infer_field_access_type(
        &mut self,
        field: &FieldAccessExpression,
        resolved: &ResolvedModule,
    ) -> Type {
        let object_type = self.infer_expression_type(&field.object, resolved);

        // Look up field in agent type
        match &object_type.kind {
            TypeKind::Agent(agent) => {
                if let Some((_, ty)) = agent
                    .fields
                    .iter()
                    .find(|(name, _)| *name == field.field.name)
                {
                    return ty.clone();
                }
                self.errors.push(TypeError::no_field(
                    object_type,
                    &field.field.name,
                    field.span,
                ));
                Type::error()
            }
            TypeKind::Error => Type::error(),
            _ => {
                self.errors.push(TypeError::no_field(
                    object_type,
                    &field.field.name,
                    field.span,
                ));
                Type::error()
            }
        }
    }

    /// Infer type of a range expression.
    fn infer_range_type(&mut self, range: &RangeExpression, resolved: &ResolvedModule) -> Type {
        let start_type = self.infer_expression_type(&range.start, resolved);
        let end_type = self.infer_expression_type(&range.end, resolved);

        if !start_type.is_numeric() || !end_type.is_numeric() {
            self.errors.push(TypeError::new(
                TypeErrorKind::InvalidRangeBounds,
                range.span,
            ));
        }

        // Range type is Range<T> where T is the bound type
        // For now, just return the start type wrapped in a "range"
        Type::named("Range") // Range type as named type; type params resolved by type inference
    }

    /// Check if two types are compatible.
    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        // Error type is compatible with everything (for error recovery)
        if expected.is_error() || found.is_error() {
            return true;
        }

        // Never type is compatible with everything
        if expected.is_never() || found.is_never() {
            return true;
        }

        // Any type is compatible with everything
        if matches!(expected.kind, TypeKind::Any) || matches!(found.kind, TypeKind::Any) {
            return true;
        }

        // Type variables need unification
        if expected.is_type_var() || found.is_type_var() {
            return true; // Will be resolved by inference
        }

        // Direct comparison
        expected.kind == found.kind
    }

    /// A fresh integer-literal type variable standing for `value`.
    fn int_literal(&mut self, value: i128) -> Type {
        let ty = self.env.fresh_type_var();
        if let TypeKind::TypeVar(id) = ty.kind {
            self.int_literals.insert(id, vec![value]);
        }
        ty
    }

    /// The integer-literal variable `ty` is, if it is one no concrete type has bound yet.
    fn unbound_int_literal(&self, ty: &Type) -> Option<u32> {
        match self.env.apply_substitutions(ty).kind {
            TypeKind::TypeVar(id) if self.int_literals.contains_key(&id) => Some(id),
            _ => None,
        }
    }

    /// Whether a value of type `found` may be used where `expected` is required, binding any
    /// integer-literal variable on either side to the concrete integer type on the other.
    ///
    /// A literal binds only to an integer type every value it stands for fits in (`-1` is not a
    /// `u64`, `300` is not a `u8`); two literals merge into one. Typed values are compared exactly:
    /// there is no implicit conversion between integer types (RFC t5-6).
    fn accept(&mut self, expected: &Type, found: &Type) -> bool {
        let expected = self.env.apply_substitutions(expected);
        let found = self.env.apply_substitutions(found);
        match (
            self.unbound_int_literal(&expected),
            self.unbound_int_literal(&found),
        ) {
            (Some(a), Some(b)) => {
                if a != b {
                    let values = self.int_literals.remove(&b).unwrap_or_default();
                    self.int_literals.entry(a).or_default().extend(values);
                    self.env.substitute(b, expected);
                }
                true
            }
            (None, Some(var)) => self.bind_int_literal(var, &expected),
            (Some(var), None) => self.bind_int_literal(var, &found),
            (None, None) => self.types_compatible(&expected, &found),
        }
    }

    fn bind_int_literal(&mut self, var: u32, target: &Type) -> bool {
        match &target.kind {
            TypeKind::Primitive(p) if p.is_integer() => {
                let infer = TypeInference::new(&mut self.env);
                let fits = self
                    .int_literals
                    .get(&var)
                    .map(|values| {
                        values
                            .iter()
                            .all(|v| infer.check_integer_bounds(*v, target))
                    })
                    .unwrap_or(true);
                if fits {
                    self.env.substitute(var, target.clone());
                }
                fits
            }
            // Error recovery, and the unconstrained variables of unannotated parameters.
            TypeKind::Error | TypeKind::Any | TypeKind::TypeVar(_) => true,
            _ => false,
        }
    }

    /// Check type compatibility and report error if incompatible.
    fn check_type_compatibility(&mut self, expected: &Type, found: &Type, span: Span) {
        if !self.accept(expected, found) {
            self.errors.push(TypeError::type_mismatch(
                expected.clone(),
                found.clone(),
                span,
            ));
        }
    }

    /// The symbol whose definition is the identifier at `span`.
    fn symbol_defined_at(&self, span: Span, resolved: &ResolvedModule) -> Option<SymbolId> {
        resolved
            .symbols
            .iter()
            .find(|s| s.def_span == span)
            .map(|s| s.id)
    }

    /// The symbol a use of an identifier at `span` refers to, as the resolver bound it.
    fn symbol_used_at(&self, span: Span, resolved: &ResolvedModule) -> Option<SymbolId> {
        resolved
            .resolve_span(span)
            .or_else(|| self.symbol_defined_at(span, resolved))
    }
}

/// Whether every path through `statements` ends in a `return` (or never ends).
fn block_always_returns(statements: &[Statement]) -> bool {
    statements.iter().any(statement_always_returns)
}

fn statement_always_returns(statement: &Statement) -> bool {
    match statement {
        Statement::Return(..) => true,
        Statement::If(if_stmt) => match &if_stmt.else_block {
            Some(else_block) => {
                block_always_returns(&if_stmt.then_block.statements)
                    && block_always_returns(&else_block.statements)
            }
            None => false,
        },
        // `loop` without a `break` of its own never falls through.
        Statement::Loop(loop_stmt) => !breaks_out(&loop_stmt.body.statements),
        Statement::Atomic(atomic) => block_always_returns(&atomic.body.statements),
        _ => false,
    }
}

/// Whether a `break` in `statements` leaves the loop these statements are the body of (a `break`
/// inside a nested loop leaves that loop instead).
fn breaks_out(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Break(_) => true,
        Statement::If(if_stmt) => {
            breaks_out(&if_stmt.then_block.statements)
                || if_stmt
                    .else_block
                    .as_ref()
                    .is_some_and(|b| breaks_out(&b.statements))
        }
        Statement::Atomic(atomic) => breaks_out(&atomic.body.statements),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_checker_creation() {
        let checker = TypeChecker::new();
        assert!(checker.errors.is_empty());
    }
}
