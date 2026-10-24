#![allow(clippy::result_large_err)]
//! AST to HIR lowering.
//!
//! This module transforms the parser's AST into HIR, performing:
//! - Symbol resolution and allocation
//! - Type annotation (propagating types from type checker)
//! - Desugaring (for→while, loop→while, etc.)
//! - Atomic block structuring (begin/end markers)
//! - Validation of control flow
//!
//! The lowerer assumes type checking has already run and types are available.

use std::collections::HashMap;

use x3_ast::{
    BinaryExpression, Expression, Function, GlobalLet, Item, Module, Statement, UnaryExpression,
};
use x3_common::Span;
use x3_typeck::{FunctionSignature, PrimitiveType, Type, TypeKind};

use crate::error::{HirError, HirErrorKind, HirResult};
use crate::hir::*;

/// Builds HIR from a parser-produced AST.
///
/// This is the main entry point for AST→HIR transformation.
/// Call `HirLowerer::lower(module)` to transform a parsed module.
pub struct HirLowerer {
    /// Next available symbol ID.
    next_symbol_id: usize,
    /// Next available label ID.
    #[allow(dead_code)]
    next_label_id: usize,
    /// Next available atomic block ID.
    next_atomic_id: usize,
    /// All allocated symbols.
    symbols: Vec<Symbol>,
    /// Top-level symbol lookup (name → ID).
    top_level: HashMap<String, SymbolId>,
    /// Current loop nesting depth (for break/continue validation).
    loop_depth: usize,
    /// Current atomic block (None if not in atomic context).
    current_atomic: Option<AtomicBlockId>,
    /// Declared structs, by name. The type checker has already refused unknown, duplicate and
    /// recursive ones; a struct value lowers to a tuple of its fields in declaration order.
    structs: HashMap<String, x3_typeck::StructType>,
}

impl HirLowerer {
    /// Lower an AST module to HIR.
    pub fn lower(module: Module) -> HirResult<HirModule> {
        let mut lowerer = Self {
            next_symbol_id: 0,
            next_label_id: 0,
            next_atomic_id: 0,
            symbols: Vec::new(),
            top_level: HashMap::new(),
            loop_depth: 0,
            current_atomic: None,
            structs: HashMap::new(),
        };
        let span = module.span;
        lowerer.collect_structs(&module.items);

        // First pass: collect top-level declarations
        lowerer.collect_top_level(&module.items)?;

        // Second pass: lower all items
        let mut globals = Vec::new();
        let mut functions = Vec::new();

        for item in module.items {
            match item {
                Item::GlobalLet(global) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&global.name.name)
                        .expect("global symbol was registered");
                    globals.push(lowerer.lower_global(global, symbol)?);
                }
                Item::Function(function) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&function.name.name)
                        .expect("function symbol was registered");
                    functions.push(lowerer.lower_function(function, symbol)?);
                }
                Item::Const(const_item) => {
                    let symbol = *lowerer
                        .top_level
                        .get(&const_item.name.name)
                        .expect("const symbol was registered");
                    globals.push(lowerer.lower_const(const_item, symbol)?);
                }
                Item::Agent(agent) => {
                    // Agent lowering: register agent fields and methods as HIR constructs
                    let _agent_symbol = *lowerer
                        .top_level
                        .get(&agent.name.name)
                        .expect("agent symbol was registered");
                    // Agents are stored but not yet fully lowered to HIR functions
                    // Their fields become storage declarations and methods become functions
                    // Agent lowering deferred to MIR phase
                }
                // Arb program lowering is handled in a dedicated pipeline stage.
                Item::ArbProgram(_) => {}
                // A struct is a type; its values are lowered where they are built.
                Item::Struct(_) => {}
                // An automation is a pallet task, not a HIR function; its body
                // is validated by the parser pass and lowered by the automation
                // pipeline, not here.
                Item::Automation(_) => {}
            }
        }

        Ok(HirModule {
            globals,
            functions,
            agents: Vec::new(), // Agent lowering deferred to MIR phase
            symbols: lowerer.symbols,
            span,
        })
    }

    /// Collect all top-level declarations.
    fn collect_top_level(&mut self, items: &[Item]) -> HirResult<()> {
        for item in items {
            match item {
                Item::GlobalLet(global) => {
                    // Infer type from initializer for now
                    let ty = self.infer_expr_type(&global.initializer);
                    self.register_top_level(
                        &global.name.name,
                        SymbolKind::Global,
                        ty,
                        global.span,
                    )?;
                }
                Item::Function(function) => {
                    let fn_ty = self.build_function_type(function);
                    self.register_top_level(
                        &function.name.name,
                        SymbolKind::Function,
                        fn_ty,
                        function.span,
                    )?;
                }
                Item::Const(const_item) => {
                    let ty = self.infer_expr_type(&const_item.value);
                    self.register_top_level(
                        &const_item.name.name,
                        SymbolKind::Global,
                        ty,
                        const_item.span,
                    )?;
                }
                Item::Agent(agent) => {
                    // Register agent as a type
                    self.register_top_level(
                        &agent.name.name,
                        SymbolKind::Agent,
                        Type::named(&agent.name.name),
                        agent.span,
                    )?;
                }
                Item::ArbProgram(_) => {}
                Item::Struct(_) => {}
                Item::Automation(_) => {}
            }
        }
        Ok(())
    }

    /// Resolve every struct's field types, those that name other structs included.
    fn collect_structs(&mut self, items: &[Item]) {
        let defs: HashMap<String, &x3_ast::StructDef> = items
            .iter()
            .filter_map(|item| match item {
                Item::Struct(def) => Some((def.name.name.clone(), def)),
                _ => None,
            })
            .collect();
        let mut names: Vec<&String> = defs.keys().collect();
        names.sort();
        for name in names {
            self.resolve_struct(name, &defs, 0);
        }
    }

    fn resolve_struct(
        &mut self,
        name: &str,
        defs: &HashMap<String, &x3_ast::StructDef>,
        depth: usize,
    ) -> Type {
        if let Some(done) = self.structs.get(name) {
            return Type::new(TypeKind::Struct(done.clone()));
        }
        // The type checker refuses a recursive struct; the bound only keeps this total.
        let Some(def) = defs.get(name).copied().filter(|_| depth <= defs.len()) else {
            return Type::any();
        };
        let fields = def
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.name.clone(),
                    self.resolve_field_type(&field.ty, defs, depth + 1),
                )
            })
            .collect();
        let resolved = x3_typeck::StructType {
            name: name.to_string(),
            fields,
        };
        self.structs.insert(name.to_string(), resolved.clone());
        Type::new(TypeKind::Struct(resolved))
    }

    fn resolve_field_type(
        &mut self,
        ty: &x3_ast::TypeAnnotation,
        defs: &HashMap<String, &x3_ast::StructDef>,
        depth: usize,
    ) -> Type {
        match &ty.shape {
            x3_ast::TypeShape::Array { element, size } => {
                Type::array(self.resolve_field_type(element, defs, depth), *size)
            }
            x3_ast::TypeShape::Vector { element } => {
                Type::vector(self.resolve_field_type(element, defs, depth))
            }
            x3_ast::TypeShape::Tuple { elements } => Type::tuple(
                elements
                    .iter()
                    .map(|e| self.resolve_field_type(e, defs, depth))
                    .collect(),
            ),
            x3_ast::TypeShape::Named if defs.contains_key(&ty.name.name) => {
                self.resolve_struct(&ty.name.name, defs, depth)
            }
            x3_ast::TypeShape::Named => self.resolve_type_annotation(ty),
        }
    }

    /// The position and type of `field` in a value of type `object`: a struct's named field, or
    /// a tuple's element written `t.0`.
    fn field_position(object: &Type, field: &str) -> Option<(usize, Type)> {
        match &object.kind {
            TypeKind::Struct(def) => def.field(field).map(|(i, ty)| (i, ty.clone())),
            TypeKind::Tuple(elements) => {
                let i: usize = field.parse().ok()?;
                elements.get(i).map(|ty| (i, ty.clone()))
            }
            _ => None,
        }
    }

    fn position_literal(at: usize, span: Span) -> HirExpr {
        HirExpr::new(
            HirExprKind::Literal(x3_common::Literal::Integer(at as i64)),
            Type::i64(),
            span,
        )
    }

    /// Build a function type from AST function.
    fn build_function_type(&self, function: &Function) -> Type {
        let param_types: Vec<Type> = function
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type_annotation(t))
                    .unwrap_or_else(Type::any)
            })
            .collect();

        let return_ty = function
            .ret_ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(Type::unit);

        Type::function(param_types, return_ty)
    }

    /// Resolve a type annotation from AST.
    fn resolve_type_annotation(&self, ty: &x3_ast::TypeAnnotation) -> Type {
        match &ty.shape {
            x3_ast::TypeShape::Array { element, size } => {
                return Type::array(self.resolve_type_annotation(element), *size)
            }
            x3_ast::TypeShape::Vector { element } => {
                return Type::vector(self.resolve_type_annotation(element))
            }
            x3_ast::TypeShape::Tuple { elements } => {
                return Type::tuple(
                    elements
                        .iter()
                        .map(|element| self.resolve_type_annotation(element))
                        .collect(),
                )
            }
            x3_ast::TypeShape::Named => {}
        }
        // Simple type name resolution
        match ty.name.name.as_str() {
            "i8" => Type::i8(),
            "i16" => Type::i16(),
            "i32" => Type::i32(),
            "i64" => Type::i64(),
            "i128" => Type::i128(),
            "u8" => Type::u8(),
            "u16" => Type::u16(),
            "u32" => Type::u32(),
            "u64" => Type::u64(),
            "u128" => Type::u128(),
            "u256" => Type::u256(),
            "bool" => Type::bool(),
            "string" => Type::string(),
            "bytes" => Type::bytes(),
            "address" => Type::address(),
            "pubkey" => Type::pubkey(),
            "()" | "unit" => Type::unit(),
            name => match self.structs.get(name) {
                Some(def) => Type::new(TypeKind::Struct(def.clone())),
                None => Type::named(name),
            },
        }
    }

    /// Infer type from expression (simple inference for globals).
    #[allow(clippy::only_used_in_recursion)]
    fn infer_expr_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => match &lit.literal {
                x3_common::Literal::Integer(_) => Type::i64(),
                x3_common::Literal::Float(_) => Type::u64(), // X3 represents floats as u64 (no native float in VM)
                x3_common::Literal::String(_) => Type::string(),
                x3_common::Literal::Bool(_) => Type::bool(),
                x3_common::Literal::Unit => Type::unit(),
            },
            Expression::Binary(binary) => {
                // For comparisons, result is bool
                match binary.op {
                    x3_ast::BinaryOp::Equal
                    | x3_ast::BinaryOp::NotEqual
                    | x3_ast::BinaryOp::Less
                    | x3_ast::BinaryOp::LessEqual
                    | x3_ast::BinaryOp::Greater
                    | x3_ast::BinaryOp::GreaterEqual => Type::bool(),
                    x3_ast::BinaryOp::LogicalAnd | x3_ast::BinaryOp::LogicalOr => Type::bool(),
                    _ => self.infer_expr_type(&binary.left),
                }
            }
            Expression::Unary(unary) => match unary.op {
                x3_ast::UnaryOp::Not => Type::bool(),
                _ => self.infer_expr_type(&unary.expr),
            },
            _ => Type::any(), // Fallback to any for complex expressions
        }
    }

    /// Register a top-level symbol.
    fn register_top_level(
        &mut self,
        name: &str,
        kind: SymbolKind,
        ty: Type,
        span: Span,
    ) -> HirResult<SymbolId> {
        if self.top_level.contains_key(name) {
            return Err(HirError::duplicate_symbol(name, span));
        }
        let id = self.allocate_symbol(name, kind, ty, span);
        self.top_level.insert(name.to_string(), id);
        Ok(id)
    }

    /// Lower a global declaration.
    fn lower_global(&mut self, global: GlobalLet, symbol: SymbolId) -> HirResult<HirGlobal> {
        if global.mutable {
            return Err(HirError::new(
                HirErrorKind::MutableGlobal(global.name.name.clone()),
                global.span,
            ));
        }
        let scope = ScopeStack::new();
        let initializer = self.lower_expression(&global.initializer, &scope)?;
        let ty = initializer.ty.clone();
        Ok(HirGlobal {
            symbol,
            ty,
            initializer,
            span: global.span,
        })
    }

    /// Lower a const declaration.
    fn lower_const(&mut self, const_item: x3_ast::Const, symbol: SymbolId) -> HirResult<HirGlobal> {
        let scope = ScopeStack::new();
        let initializer = self.lower_expression(&const_item.value, &scope)?;
        let ty = initializer.ty.clone();
        Ok(HirGlobal {
            symbol,
            ty,
            initializer,
            span: const_item.span,
        })
    }

    /// Lower a function declaration.
    fn lower_function(&mut self, function: Function, symbol: SymbolId) -> HirResult<HirFunction> {
        let mut scope = ScopeStack::new();
        let mut params = Vec::new();

        // Lower parameters
        for param in function.params {
            let ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type_annotation(t))
                .unwrap_or_else(Type::any);
            let local_symbol = self.allocate_symbol(
                &param.name.name,
                SymbolKind::Param {
                    mutable: param.mutable,
                },
                ty.clone(),
                param.span,
            );

            if !scope.insert(
                &param.name.name,
                LocalInfo {
                    symbol: local_symbol,
                    ty: ty.clone(),
                    mutable: param.mutable,
                },
            ) {
                return Err(HirError::new(
                    HirErrorKind::DuplicateParam(param.name.name),
                    param.span,
                ));
            }

            params.push(HirParam {
                symbol: local_symbol,
                name: param.name.name,
                ty,
                mutable: param.mutable,
                span: param.span,
            });
        }

        // Lower body
        let body = self.lower_statements(&function.body.statements, &mut scope)?;

        // Return type
        let return_ty = function
            .ret_ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(Type::unit);

        Ok(HirFunction {
            symbol,
            params,
            body,
            return_ty,
            attrs: FunctionAttrs::default(),
            span: function.span,
        })
    }

    /// Lower a list of statements.
    fn lower_statements(
        &mut self,
        statements: &[Statement],
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        let mut lowered = Vec::new();
        for statement in statements {
            lowered.extend(self.lower_statement(statement, scope)?);
        }
        Ok(lowered)
    }

    /// Lower a single statement.
    ///
    /// Returns `Vec<HirStmt>` because some AST statements (notably atomic
    /// blocks) lower to multiple HIR statements (AtomicBegin + body +
    /// AtomicEnd).  Callers should use `extend` to collect results.
    fn lower_statement(
        &mut self,
        statement: &Statement,
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        match statement {
            Statement::Let(binding) => self.lower_let(binding, scope).map(|s| vec![s]),
            Statement::Expr(expr) => self
                .lower_expression_statement(expr, scope)
                .map(|s| vec![s]),
            Statement::Return(value, span) => {
                let expr = value
                    .as_ref()
                    .map(|expr| self.lower_expression(expr, scope))
                    .transpose()?;
                Ok(vec![HirStmt::Return {
                    value: expr,
                    span: *span,
                }])
            }
            Statement::If(branch) => {
                let condition = self.lower_expression(&branch.condition, scope)?;
                let then_block = self.lower_block(&branch.then_block.statements, scope)?;
                let else_block = if let Some(else_block) = &branch.else_block {
                    self.lower_block(&else_block.statements, scope)?
                } else {
                    Vec::new()
                };
                Ok(vec![HirStmt::If {
                    condition,
                    then_block,
                    else_block,
                    span: branch.span,
                }])
            }
            Statement::While(loop_stmt) => {
                self.loop_depth += 1;
                let condition = self.lower_expression(&loop_stmt.condition, scope)?;
                let body = self.lower_block(&loop_stmt.body.statements, scope)?;
                self.loop_depth -= 1;
                Ok(vec![HirStmt::While {
                    label: None,
                    condition,
                    body,
                    span: loop_stmt.span,
                }])
            }
            Statement::Loop(loop_stmt) => {
                // Desugar `loop { ... }` to `while true { ... }`
                self.loop_depth += 1;
                let body = self.lower_block(&loop_stmt.body.statements, scope)?;
                self.loop_depth -= 1;

                let true_lit = HirExpr::new(
                    HirExprKind::Literal(x3_common::Literal::Bool(true)),
                    Type::bool(),
                    loop_stmt.span,
                );

                Ok(vec![HirStmt::While {
                    label: None,
                    condition: true_lit,
                    body,
                    span: loop_stmt.span,
                }])
            }
            Statement::For(for_stmt) => self.lower_for_loop(for_stmt, scope),
            Statement::Break(break_stmt) => {
                if self.loop_depth == 0 {
                    return Err(HirError::break_outside_loop(break_stmt.span));
                }
                Ok(vec![HirStmt::Break {
                    label: None,
                    span: break_stmt.span,
                }])
            }
            Statement::Continue(continue_stmt) => {
                if self.loop_depth == 0 {
                    return Err(HirError::continue_outside_loop(continue_stmt.span));
                }
                Ok(vec![HirStmt::Continue {
                    label: None,
                    span: continue_stmt.span,
                }])
            }
            Statement::Atomic(atomic_block) => {
                // Check for nested atomics
                if self.current_atomic.is_some() {
                    return Err(HirError::atomic_nesting(atomic_block.span));
                }

                let block_id = self.alloc_atomic_id();
                self.current_atomic = Some(block_id);

                // Lower the atomic block contents
                let body = self.lower_block(&atomic_block.body.statements, scope)?;

                self.current_atomic = None;

                // Wrap in begin/end markers — the full sequence is emitted:
                //   AtomicBegin, <body...>, AtomicEnd
                // This ensures the atomic block's body and commit marker are
                // preserved through HIR and not silently dropped.
                let mut stmts = Vec::with_capacity(body.len() + 2);
                stmts.push(HirStmt::AtomicBegin {
                    block_id,
                    span: atomic_block.span,
                });
                stmts.extend(body);
                stmts.push(HirStmt::AtomicEnd {
                    block_id,
                    commit: true,
                    span: atomic_block.span,
                });

                Ok(stmts)
            }
            Statement::Emit(emit_stmt) => {
                // `emit Name(a, b)`: the name and the payload, taken apart. Every event used to be
                // named the literal string "event" — a fabricated name, so two different events
                // were indistinguishable in anything that read them — and the whole call was
                // lowered as the single argument.
                match &emit_stmt.value {
                    Expression::Call(call) => {
                        let Expression::Identifier(name) = &*call.callee else {
                            return Err(HirError::new(
                                HirErrorKind::InvalidAssignTarget,
                                emit_stmt.span,
                            ));
                        };
                        let args = call
                            .args
                            .iter()
                            .map(|arg| self.lower_expression(arg, scope))
                            .collect::<HirResult<Vec<_>>>()?;
                        Ok(vec![HirStmt::Emit {
                            event_name: name.name.clone(),
                            args,
                            span: emit_stmt.span,
                        }])
                    }
                    // `emit x;` names no event. The chain compiler refuses it rather than
                    // inventing a name for it.
                    other => Err(HirError::new(
                        HirErrorKind::UnknownSymbol(
                            "emit needs an event: `emit Name(args)`".into(),
                        ),
                        other.span(),
                    )),
                }
            }
        }
    }

    /// Lower a `for` loop by desugaring it to `while`.
    ///
    /// Both forms used to lose part of the loop. The range form built `let i = start` and then
    /// never emitted it, and appended `i = i + 1` to the end of the body, so a `continue` jumped to
    /// the condition past the increment and spun until it ran out of gas. The C-style form lowered
    /// its initializer and discarded the result, and lowered its update with the expression
    /// lowering, which refuses an assignment. The desugarings below keep every part, and put the
    /// step where `continue` cannot skip it.
    fn lower_for_loop(
        &mut self,
        for_stmt: &x3_ast::ForStatement,
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        let span = for_stmt.span;
        match &for_stmt.kind {
            // for (let i in start..end) { body }
            //
            //   let mut <counter> = start;
            //   let <end> = end;              // evaluated once, as a range is a value
            //   while <counter> < <end> {
            //       let i = <counter>;        // a fresh, immutable binding per iteration
            //       <counter> = <counter> + 1;
            //       body                      // `continue` re-tests an already-stepped counter
            //   }
            x3_ast::ForLoopKind::Range { variable, range } => {
                let iter_ty = Type::i64();
                // The bounds are evaluated in the enclosing scope: `i` is not visible in them.
                let start = self.lower_expression(&range.start, scope)?;
                let end = self.lower_expression(&range.end, scope)?;

                let counter = self.allocate_symbol(
                    "<for counter>",
                    SymbolKind::Local { mutable: true },
                    iter_ty.clone(),
                    span,
                );
                let end_symbol = self.allocate_symbol(
                    "<for end>",
                    SymbolKind::Local { mutable: false },
                    iter_ty.clone(),
                    span,
                );
                let var = |symbol| HirExpr::new(HirExprKind::Var(symbol), iter_ty.clone(), span);

                scope.push_frame();
                let iter_symbol = self.allocate_symbol(
                    &variable.name,
                    SymbolKind::Local { mutable: false },
                    iter_ty.clone(),
                    variable.span,
                );
                scope.insert(
                    &variable.name,
                    LocalInfo {
                        symbol: iter_symbol,
                        ty: iter_ty.clone(),
                        mutable: false,
                    },
                );
                self.loop_depth += 1;
                let user_body = self.lower_block(&for_stmt.body.statements, scope);
                self.loop_depth -= 1;
                scope.pop_frame();
                let user_body = user_body?;

                let step = HirExpr::new(
                    HirExprKind::Binary {
                        op: x3_ast::BinaryOp::Add,
                        left: Box::new(var(counter)),
                        right: Box::new(HirExpr::new(
                            HirExprKind::Literal(x3_common::Literal::Integer(1)),
                            iter_ty.clone(),
                            span,
                        )),
                    },
                    iter_ty.clone(),
                    span,
                );
                let mut body = Vec::with_capacity(user_body.len() + 2);
                body.push(HirStmt::Let {
                    symbol: iter_symbol,
                    ty: iter_ty.clone(),
                    value: var(counter),
                    mutable: false,
                    span: variable.span,
                });
                body.push(HirStmt::Assign {
                    target: AssignTarget::Variable(counter),
                    value: step,
                    span,
                });
                body.extend(user_body);

                let condition = HirExpr::new(
                    HirExprKind::Binary {
                        op: x3_ast::BinaryOp::Less,
                        left: Box::new(var(counter)),
                        right: Box::new(var(end_symbol)),
                    },
                    Type::bool(),
                    span,
                );
                Ok(vec![
                    HirStmt::Let {
                        symbol: counter,
                        ty: iter_ty.clone(),
                        value: start,
                        mutable: true,
                        span,
                    },
                    HirStmt::Let {
                        symbol: end_symbol,
                        ty: iter_ty.clone(),
                        value: end,
                        mutable: false,
                        span,
                    },
                    HirStmt::While {
                        label: None,
                        condition,
                        body,
                        span,
                    },
                ])
            }
            // for (init; cond; update) { body }
            //
            //   init;
            //   let mut <first> = true;
            //   while true {
            //       if <first> { <first> = false; } else { update; }
            //       if !cond { break; }
            //       body                      // `continue` returns to the top, which runs update
            //   }
            x3_ast::ForLoopKind::CStyle {
                init,
                condition,
                update,
            } => {
                scope.push_frame();
                let result = (|| -> HirResult<Vec<HirStmt>> {
                    let mut stmts = Vec::new();
                    if let Some(init_stmt) = init {
                        stmts.extend(self.lower_statement(init_stmt, scope)?);
                    }
                    let cond = condition
                        .as_ref()
                        .map(|c| self.lower_expression(c, scope))
                        .transpose()?;
                    let update = update
                        .as_ref()
                        .map(|u| self.lower_expression_statement(u, scope))
                        .transpose()?;
                    self.loop_depth += 1;
                    let user_body = self.lower_block(&for_stmt.body.statements, scope);
                    self.loop_depth -= 1;
                    let user_body = user_body?;

                    let bool_lit = |value| {
                        HirExpr::new(
                            HirExprKind::Literal(x3_common::Literal::Bool(value)),
                            Type::bool(),
                            span,
                        )
                    };
                    let mut body = Vec::new();
                    if let Some(update) = update {
                        let first = self.allocate_symbol(
                            "<for first>",
                            SymbolKind::Local { mutable: true },
                            Type::bool(),
                            span,
                        );
                        stmts.push(HirStmt::Let {
                            symbol: first,
                            ty: Type::bool(),
                            value: bool_lit(true),
                            mutable: true,
                            span,
                        });
                        body.push(HirStmt::If {
                            condition: HirExpr::new(HirExprKind::Var(first), Type::bool(), span),
                            then_block: vec![HirStmt::Assign {
                                target: AssignTarget::Variable(first),
                                value: bool_lit(false),
                                span,
                            }],
                            else_block: vec![update],
                            span,
                        });
                    }
                    if let Some(cond) = cond {
                        body.push(HirStmt::If {
                            condition: HirExpr::new(
                                HirExprKind::Unary {
                                    op: x3_ast::UnaryOp::Not,
                                    operand: Box::new(cond),
                                },
                                Type::bool(),
                                span,
                            ),
                            then_block: vec![HirStmt::Break { label: None, span }],
                            else_block: vec![],
                            span,
                        });
                    }
                    body.extend(user_body);
                    stmts.push(HirStmt::While {
                        label: None,
                        condition: bool_lit(true),
                        body,
                        span,
                    });
                    Ok(stmts)
                })();
                scope.pop_frame();
                result
            }
        }
    }

    /// Lower a let binding.
    fn lower_let(
        &mut self,
        binding: &x3_ast::LetStatement,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        let value = self.lower_expression(&binding.initializer, scope)?;
        let ty = binding
            .ty
            .as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or_else(|| value.ty.clone());

        let symbol = self.allocate_symbol(
            &binding.name.name,
            SymbolKind::Local {
                mutable: binding.mutable,
            },
            ty.clone(),
            binding.span,
        );

        if !scope.insert(
            &binding.name.name,
            LocalInfo {
                symbol,
                ty: ty.clone(),
                mutable: binding.mutable,
            },
        ) {
            return Err(HirError::duplicate_symbol(&binding.name.name, binding.span));
        }

        Ok(HirStmt::Let {
            symbol,
            ty,
            value,
            mutable: binding.mutable,
            span: binding.span,
        })
    }

    /// Lower an expression statement.
    fn lower_expression_statement(
        &mut self,
        expr: &Expression,
        scope: &mut ScopeStack,
    ) -> HirResult<HirStmt> {
        // Handle assignment specially
        if let Expression::Assign(assign) = expr {
            let target_info = match scope.lookup(&assign.target.name) {
                Some(info) => info,
                // A top-level binding is a constant: assigning to one is an immutable assignment,
                // not a reference to a name that does not exist.
                None if self.top_level.contains_key(&assign.target.name) => {
                    return Err(HirError::immutable_assign(&assign.target.name, assign.span));
                }
                None => {
                    return Err(HirError::unknown_symbol(
                        &assign.target.name,
                        assign.target.span,
                    ))
                }
            };

            if !target_info.mutable {
                return Err(HirError::immutable_assign(&assign.target.name, assign.span));
            }

            let value = self.lower_expression(&assign.value, scope)?;
            return Ok(HirStmt::Assign {
                target: AssignTarget::Variable(target_info.symbol),
                value,
                span: assign.span,
            });
        }

        // `a[i] = v`: rebinds `a`, so the same rules as `a = v` (a mutable local).
        if let Expression::IndexAssign(assign) = expr {
            let target_info = match scope.lookup(&assign.target.name) {
                Some(info) => info,
                None if self.top_level.contains_key(&assign.target.name) => {
                    return Err(HirError::immutable_assign(&assign.target.name, assign.span));
                }
                None => {
                    return Err(HirError::unknown_symbol(
                        &assign.target.name,
                        assign.target.span,
                    ))
                }
            };
            if !target_info.mutable {
                return Err(HirError::immutable_assign(&assign.target.name, assign.span));
            }
            let array = HirExpr::new(
                HirExprKind::Var(target_info.symbol),
                target_info.ty.clone(),
                assign.target.span,
            );
            let element_ty = Self::element_type(&target_info.ty, &assign.index);
            let index = self.lower_expression(&assign.index, scope)?;
            let value = self.lower_expression(&assign.value, scope)?;
            return Ok(HirStmt::Assign {
                target: AssignTarget::Index {
                    array: Box::new(array),
                    index: Box::new(index),
                    element_ty,
                },
                value,
                span: assign.span,
            });
        }

        // `s.f = v`: rebinds `s` to a copy with the field replaced, as `a[i] = v` does.
        if let Expression::FieldAssign(assign) = expr {
            let target_info = match scope.lookup(&assign.target.name) {
                Some(info) => info,
                None if self.top_level.contains_key(&assign.target.name) => {
                    return Err(HirError::immutable_assign(&assign.target.name, assign.span));
                }
                None => {
                    return Err(HirError::unknown_symbol(
                        &assign.target.name,
                        assign.target.span,
                    ))
                }
            };
            if !target_info.mutable {
                return Err(HirError::immutable_assign(&assign.target.name, assign.span));
            }
            let Some((at, element_ty)) = Self::field_position(&target_info.ty, &assign.field.name)
            else {
                return Err(HirError::new(
                    HirErrorKind::InvalidAssignTarget,
                    assign.field.span,
                ));
            };
            let object = HirExpr::new(
                HirExprKind::Var(target_info.symbol),
                target_info.ty.clone(),
                assign.target.span,
            );
            let value = self.lower_expression(&assign.value, scope)?;
            return Ok(HirStmt::Assign {
                target: AssignTarget::Index {
                    array: Box::new(object),
                    index: Box::new(Self::position_literal(at, assign.field.span)),
                    element_ty,
                },
                value,
                span: assign.span,
            });
        }

        let expr = self.lower_expression(expr, scope)?;
        Ok(HirStmt::Expr(expr))
    }

    /// Lower a block (creates new scope frame).
    fn lower_block(
        &mut self,
        statements: &[Statement],
        scope: &mut ScopeStack,
    ) -> HirResult<Vec<HirStmt>> {
        scope.push_frame();
        let result = self.lower_statements(statements, scope);
        scope.pop_frame();
        result
    }

    /// Lower an expression.
    fn lower_expression(&mut self, expr: &Expression, scope: &ScopeStack) -> HirResult<HirExpr> {
        match expr {
            Expression::Literal(literal) => {
                let ty = match &literal.literal {
                    x3_common::Literal::Integer(_) => Type::i64(),
                    x3_common::Literal::Float(_) => Type::u64(), // X3 represents floats as u64 (no native float in VM)
                    x3_common::Literal::String(_) => Type::string(),
                    x3_common::Literal::Bool(_) => Type::bool(),
                    x3_common::Literal::Unit => Type::unit(),
                };
                Ok(HirExpr::new(
                    HirExprKind::Literal(literal.literal.clone()),
                    ty,
                    literal.span,
                ))
            }

            Expression::Identifier(id) => {
                // First check local scope
                if let Some(info) = scope.lookup(&id.name) {
                    return Ok(HirExpr::new(
                        HirExprKind::Var(info.symbol),
                        info.ty.clone(),
                        id.span,
                    ));
                }

                // Then check top-level
                if let Some(&symbol) = self.top_level.get(&id.name) {
                    let ty = self.symbol_type(symbol).unwrap_or_else(Type::any);
                    return Ok(HirExpr::new(HirExprKind::Var(symbol), ty, id.span));
                }

                Err(HirError::unknown_symbol(&id.name, id.span))
            }

            Expression::Binary(binary) => self.lower_binary(binary, scope),
            Expression::Unary(unary) => self.lower_unary(unary, scope),
            Expression::Call(call) => self.lower_call(call, scope),

            Expression::Assign(assign) => {
                // Assignment as expression is invalid in statement context
                Err(HirError::new(
                    HirErrorKind::InvalidAssignTarget,
                    assign.span,
                ))
            }

            Expression::FieldAccess(field_access) => {
                let object = self.lower_expression(&field_access.object, scope)?;
                // A struct's field and a tuple's `t.0` are elements at a position fixed now.
                if let Some((at, field_ty)) =
                    Self::field_position(&object.ty, &field_access.field.name)
                {
                    return Ok(HirExpr::new(
                        HirExprKind::Index {
                            array: Box::new(object),
                            index: Box::new(Self::position_literal(at, field_access.field.span)),
                        },
                        field_ty,
                        field_access.span,
                    ));
                }
                // Field type resolution requires type checker integration; defaults to Any
                let field_ty = Type::any();
                Ok(HirExpr::new(
                    HirExprKind::Field {
                        object: Box::new(object),
                        field: field_access.field.name.clone(),
                    },
                    field_ty,
                    field_access.span,
                ))
            }

            Expression::Array(array) => {
                let elements = array
                    .elements
                    .iter()
                    .map(|element| self.lower_expression(element, scope))
                    .collect::<HirResult<Vec<_>>>()?;
                let element_ty = elements
                    .first()
                    .map(|element| element.ty.clone())
                    .unwrap_or_else(Type::any);
                let ty = Type::array(element_ty, elements.len());
                Ok(HirExpr::new(HirExprKind::Array(elements), ty, array.span))
            }

            Expression::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|element| self.lower_expression(element, scope))
                    .collect::<HirResult<Vec<_>>>()?;
                let ty = Type::tuple(elements.iter().map(|e| e.ty.clone()).collect());
                Ok(HirExpr::new(HirExprKind::Tuple(elements), ty, tuple.span))
            }

            Expression::Index(index) => {
                let base = self.lower_expression(&index.base, scope)?;
                let ty = Self::element_type(&base.ty, &index.index);
                let at = self.lower_expression(&index.index, scope)?;
                Ok(HirExpr::new(
                    HirExprKind::Index {
                        array: Box::new(base),
                        index: Box::new(at),
                    },
                    ty,
                    index.span,
                ))
            }

            // An element write is a statement (see `lower_expression_statement`).
            Expression::IndexAssign(assign) => Err(HirError::new(
                HirErrorKind::InvalidAssignTarget,
                assign.span,
            )),
            Expression::FieldAssign(assign) => Err(HirError::new(
                HirErrorKind::InvalidAssignTarget,
                assign.span,
            )),

            // `Name { f: v, ... }`: a tuple of the fields in declaration order. The values are
            // evaluated in the order the literal writes them, as everywhere else in the language;
            // when that is not declaration order they are bound to temporaries first, so a
            // literal that writes `{ b: f(), a: g() }` calls `f` before `g`.
            Expression::StructLiteral(literal) => {
                let Some(def) = self.structs.get(&literal.name.name).cloned() else {
                    return Err(HirError::unknown_symbol(
                        &literal.name.name,
                        literal.name.span,
                    ));
                };
                let written: Vec<&str> = literal
                    .fields
                    .iter()
                    .map(|f| f.name.name.as_str())
                    .collect();
                let declared: Vec<&str> = def.fields.iter().map(|(n, _)| n.as_str()).collect();
                let ty = Type::new(TypeKind::Struct(def.clone()));
                if written == declared {
                    let elements = literal
                        .fields
                        .iter()
                        .map(|init| self.lower_expression(&init.value, scope))
                        .collect::<HirResult<Vec<_>>>()?;
                    return Ok(HirExpr::new(HirExprKind::Tuple(elements), ty, literal.span));
                }
                let mut stmts = Vec::with_capacity(literal.fields.len());
                let mut temps = HashMap::new();
                for init in &literal.fields {
                    let value = self.lower_expression(&init.value, scope)?;
                    let symbol = self.allocate_symbol(
                        &format!("{}.{}", literal.name.name, init.name.name),
                        SymbolKind::Local { mutable: false },
                        value.ty.clone(),
                        init.span,
                    );
                    temps.insert(
                        init.name.name.clone(),
                        (symbol, value.ty.clone(), init.span),
                    );
                    stmts.push(HirStmt::Let {
                        symbol,
                        ty: value.ty.clone(),
                        value,
                        mutable: false,
                        span: init.span,
                    });
                }
                let mut elements = Vec::with_capacity(def.fields.len());
                for (name, _) in &def.fields {
                    let (symbol, field_ty, span) = temps
                        .remove(name)
                        .ok_or_else(|| HirError::unknown_symbol(name, literal.span))?;
                    elements.push(HirExpr::new(HirExprKind::Var(symbol), field_ty, span));
                }
                let tuple = HirExpr::new(HirExprKind::Tuple(elements), ty.clone(), literal.span);
                Ok(HirExpr::new(
                    HirExprKind::Block {
                        stmts,
                        expr: Some(Box::new(tuple)),
                    },
                    ty,
                    literal.span,
                ))
            }

            Expression::Range(range) => {
                // Range expressions should only appear in for-loop contexts
                // Lower to a tuple for now (start, end)
                let start = self.lower_expression(&range.start, scope)?;
                let end = self.lower_expression(&range.end, scope)?;
                let ty = Type::tuple(vec![start.ty.clone(), end.ty.clone()]);
                Ok(HirExpr::new(
                    HirExprKind::Tuple(vec![start, end]),
                    ty,
                    range.span,
                ))
            }
        }
    }

    /// Lower a binary expression.
    fn lower_binary(
        &mut self,
        binary: &BinaryExpression,
        scope: &ScopeStack,
    ) -> HirResult<HirExpr> {
        let left = self.lower_expression(&binary.left, scope)?;
        let right = self.lower_expression(&binary.right, scope)?;

        // Determine result type
        let ty = match binary.op {
            x3_ast::BinaryOp::Equal
            | x3_ast::BinaryOp::NotEqual
            | x3_ast::BinaryOp::Less
            | x3_ast::BinaryOp::LessEqual
            | x3_ast::BinaryOp::Greater
            | x3_ast::BinaryOp::GreaterEqual
            | x3_ast::BinaryOp::LogicalAnd
            | x3_ast::BinaryOp::LogicalOr => Type::bool(),
            _ => left.ty.clone(), // Arithmetic preserves operand type
        };

        Ok(HirExpr::new(
            HirExprKind::Binary {
                op: binary.op,
                left: Box::new(left),
                right: Box::new(right),
            },
            ty,
            binary.span,
        ))
    }

    /// Lower a unary expression.
    fn lower_unary(&mut self, unary: &UnaryExpression, scope: &ScopeStack) -> HirResult<HirExpr> {
        let operand = self.lower_expression(&unary.expr, scope)?;

        let ty = match unary.op {
            x3_ast::UnaryOp::Not => Type::bool(),
            _ => operand.ty.clone(),
        };

        Ok(HirExpr::new(
            HirExprKind::Unary {
                op: unary.op,
                operand: Box::new(operand),
            },
            ty,
            unary.span,
        ))
    }

    /// The type of `base[index]`: an array's element type, or the tuple element a constant index
    /// names (the type checker has already refused any other tuple index).
    fn element_type(base: &Type, index: &Expression) -> Type {
        match &base.kind {
            TypeKind::Struct(def) => match index {
                Expression::Literal(literal) => match literal.literal {
                    x3_common::Literal::Integer(n) if n >= 0 && (n as usize) < def.fields.len() => {
                        def.fields[n as usize].1.clone()
                    }
                    _ => Type::any(),
                },
                _ => Type::any(),
            },
            TypeKind::Array { element, .. } | TypeKind::Vector(element) => (**element).clone(),
            TypeKind::Tuple(elements) => match index {
                Expression::Literal(literal) => match literal.literal {
                    x3_common::Literal::Integer(n) if n >= 0 && (n as usize) < elements.len() => {
                        elements[n as usize].clone()
                    }
                    _ => Type::any(),
                },
                _ => Type::any(),
            },
            TypeKind::Bytes => Type::i64(),
            _ => Type::any(),
        }
    }

    /// Lower a function call.
    fn lower_call(
        &mut self,
        call: &x3_ast::CallExpression,
        scope: &ScopeStack,
    ) -> HirResult<HirExpr> {
        // `x.len()`: the one method the language has, on arrays, tuples and strings.
        if let Expression::FieldAccess(field) = &*call.callee {
            if field.field.name == "len" && call.args.is_empty() {
                let receiver = self.lower_expression(&field.object, scope)?;
                return Ok(HirExpr::new(
                    HirExprKind::MethodCall {
                        receiver: Box::new(receiver),
                        method: "len".to_string(),
                        args: Vec::new(),
                    },
                    Type::i64(),
                    call.span,
                ));
            }
        }
        // For now, only support direct function calls
        if let Expression::Identifier(callee) = &*call.callee {
            // Resolve callee
            let (callee_symbol, callee_ty) = if let Some(info) = scope.lookup(&callee.name) {
                (info.symbol, info.ty.clone())
            } else if let Some(&symbol) = self.top_level.get(&callee.name) {
                let ty = self.symbol_type(symbol).unwrap_or_else(Type::any);
                (symbol, ty)
            } else if let Some(intrinsic) = x3_common::intrinsics::by_name(&callee.name) {
                // A host call: a call to its reserved symbol (see `x3_common::intrinsics`).
                let i64_ty = || Type::new(TypeKind::Primitive(PrimitiveType::I64));
                let sig = FunctionSignature::new(
                    (0..intrinsic.arity).map(|_| i64_ty()).collect(),
                    if intrinsic.returns_value {
                        i64_ty()
                    } else {
                        Type::unit()
                    },
                );
                (
                    SymbolId(intrinsic.symbol),
                    Type::new(TypeKind::Function(sig)),
                )
            } else {
                return Err(HirError::unknown_symbol(&callee.name, callee.span));
            };

            // Get return type from function type and validate arity
            let return_ty = if let Some(sig) = callee_ty.as_function() {
                if sig.params.len() != call.args.len() {
                    return Err(HirError::new(
                        HirErrorKind::ArgumentCountMismatch {
                            expected: sig.params.len(),
                            found: call.args.len(),
                        },
                        call.span,
                    ));
                }
                (*sig.return_type).clone()
            } else {
                return Err(HirError::not_callable(callee_ty.clone(), call.span));
            };

            // Lower arguments
            let args: Vec<HirExpr> = call
                .args
                .iter()
                .map(|arg| self.lower_expression(arg, scope))
                .collect::<HirResult<Vec<_>>>()?;

            Ok(HirExpr::new(
                HirExprKind::Call {
                    callee: callee_symbol,
                    args,
                },
                return_ty,
                call.span,
            ))
        } else {
            Err(HirError::new(
                HirErrorKind::NotImplemented("indirect calls".to_string()),
                call.span,
            ))
        }
    }

    /// Allocate a new symbol.
    fn allocate_symbol(&mut self, name: &str, kind: SymbolKind, ty: Type, span: Span) -> SymbolId {
        let id = SymbolId(self.next_symbol_id);
        self.next_symbol_id += 1;
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            kind,
            ty,
            span,
        });
        id
    }

    /// Allocate a new atomic block ID.
    fn alloc_atomic_id(&mut self) -> AtomicBlockId {
        let id = AtomicBlockId(self.next_atomic_id);
        self.next_atomic_id += 1;
        id
    }

    /// Get a symbol's type.
    fn symbol_type(&self, symbol: SymbolId) -> Option<Type> {
        self.symbols
            .iter()
            .find(|s| s.id == symbol)
            .map(|s| s.ty.clone())
    }
}

// ============================================================================
// Scope Management
// ============================================================================

#[derive(Clone)]
struct LocalInfo {
    symbol: SymbolId,
    ty: Type,
    mutable: bool,
}

struct ScopeStack {
    frames: Vec<HashMap<String, LocalInfo>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    fn insert(&mut self, name: &str, info: LocalInfo) -> bool {
        // Disallow shadowing: check any existing frame for the name.
        for frame in self.frames.iter() {
            if frame.contains_key(name) {
                return false;
            }
        }
        let frame = self.frames.last_mut().unwrap();
        frame.insert(name.to_string(), info);
        true
    }

    fn lookup(&self, name: &str) -> Option<LocalInfo> {
        for frame in self.frames.iter().rev() {
            if let Some(info) = frame.get(name) {
                return Some(info.clone());
            }
        }
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use x3_parser::Parser;

    fn parse_and_lower(source: &str) -> HirResult<HirModule> {
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("parse");
        HirLowerer::lower(module)
    }

    #[test]
    fn lower_simple_function() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                let result = x + y;
                return result;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        assert_eq!(hir.functions.len(), 1);
        let function = &hir.functions[0];

        // Check params
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].name, "x");
        assert_eq!(function.params[1].name, "y");

        // Check body has let + return
        assert!(function.body.len() >= 2);
        assert!(matches!(function.body[0], HirStmt::Let { .. }));
    }

    #[test]
    fn disallow_shadowing_across_scopes() {
        let source = r#"
            fn shadow_test() -> i32 {
                let x = 10;
                if true {
                    let x = 20; // shadowing should be an error now
                    return x;
                }
            }
        "#;
        let res = parse_and_lower(source);
        assert!(res.is_err(), "expected shadowing to be rejected");
    }

    #[test]
    fn call_argument_count_mismatch() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 { return x + y; }
            fn caller() -> i32 { return add(1); }
        "#;
        let res = parse_and_lower(source);
        assert!(res.is_err(), "expected argument count mismatch to error");
    }

    #[test]
    fn lower_global() {
        let source = "let PI = 3141592;";
        let hir = parse_and_lower(source).expect("lower");

        assert_eq!(hir.globals.len(), 1);
        assert!(hir.symbol_name(hir.globals[0].symbol) == Some("PI"));
    }

    #[test]
    fn lower_while_loop() {
        let source = r#"
            fn count() {
                let mut i = 0;
                while i < 10 {
                    i = i + 1;
                }
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let body = &hir.functions[0].body;
        assert!(body.iter().any(|s| matches!(s, HirStmt::While { .. })));
    }

    #[test]
    fn lower_if_else() {
        let source = r#"
            fn max(a: i32, b: i32) -> i32 {
                if a > b {
                    return a;
                } else {
                    return b;
                }
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let body = &hir.functions[0].body;
        let if_stmt = body.iter().find(|s| matches!(s, HirStmt::If { .. }));
        assert!(if_stmt.is_some());

        if let Some(HirStmt::If {
            then_block,
            else_block,
            ..
        }) = if_stmt
        {
            assert!(!then_block.is_empty());
            assert!(!else_block.is_empty());
        }
    }

    #[test]
    fn rejects_duplicate_params() {
        let source = "fn dup(x: i32, x: i32) {}";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_variable() {
        let source = "fn foo() { return unknown; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_immutable_assign() {
        let source = r#"
            fn foo() {
                let x = 1;
                x = 2;
            }
        "#;
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_break_outside_loop() {
        let source = "fn foo() { break; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_continue_outside_loop() {
        let source = "fn foo() { continue; }";
        let result = parse_and_lower(source);
        assert!(result.is_err());
    }

    #[test]
    fn allows_break_in_loop() {
        let source = r#"
            fn foo() {
                while true {
                    break;
                }
            }
        "#;
        let result = parse_and_lower(source);
        assert!(result.is_ok());
    }

    #[test]
    fn type_annotations_preserved() {
        let source = r#"
            fn typed(x: u64, y: bool) -> i32 {
                return 0;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        let func = &hir.functions[0];
        // Check that params have correct types
        assert!(func.params[0].ty.to_string().contains("u64"));
        assert!(func.params[1].ty.to_string().contains("bool"));
    }

    #[test]
    fn expressions_carry_types() {
        let source = r#"
            fn foo() -> bool {
                return 1 < 2;
            }
        "#;
        let hir = parse_and_lower(source).expect("lower");

        if let HirStmt::Return {
            value: Some(expr), ..
        } = &hir.functions[0].body[0]
        {
            // Comparison should have bool type
            assert!(expr.ty.is_bool());
        } else {
            panic!("expected return statement");
        }
    }
}
