use std::collections::HashMap;

use x3_common::Span;
use x3_hir::hir::{AssignTarget, HirExpr, HirExprKind, HirFunction, HirModule, HirStmt, SymbolId};

use crate::error::MirError;
use crate::memory::MemoryModel;
use crate::mir::*;

/// Converts HIR into deterministic SSA-form MIR.
pub struct MirLowerer;

impl MirLowerer {
    pub fn lower(module: &HirModule) -> Result<MirModule, MirError> {
        let constants = std::rc::Rc::new(constants_of(module)?);
        let mut functions = Vec::new();
        for function in &module.functions {
            let mut builder = MirFunctionBuilder::new(function.symbol, function.span);
            builder.constants = constants.clone();
            functions.push(builder.lower_function(function)?);
        }
        Ok(MirModule {
            functions,
            span: module.span,
        })
    }
}

/// The module's top-level bindings (`const` and immutable `let`), by symbol.
///
/// They are compile-time constants: a use of one is lowered as its initializer, at the use site,
/// through the same `lower_expr` every other expression goes through, so a constant computes
/// exactly what the same expression written inline would, with no second evaluator to disagree
/// with the VM. That only preserves meaning if evaluating the initializer has no effects and
/// terminates, so an initializer may use literals, operators and other constants only, and the
/// references between constants must not form a cycle.
fn constants_of(module: &HirModule) -> Result<HashMap<SymbolId, HirExpr>, MirError> {
    let constants: HashMap<SymbolId, HirExpr> = module
        .globals
        .iter()
        .map(|global| (global.symbol, global.initializer.clone()))
        .collect();

    fn check_pure(
        expr: &HirExpr,
        constants: &HashMap<SymbolId, HirExpr>,
        refs: &mut Vec<SymbolId>,
    ) -> Result<(), MirError> {
        match &expr.kind {
            HirExprKind::Literal(_) => Ok(()),
            HirExprKind::Var(symbol) if constants.contains_key(symbol) => {
                refs.push(*symbol);
                Ok(())
            }
            HirExprKind::Binary { left, right, .. } => {
                check_pure(left, constants, refs)?;
                check_pure(right, constants, refs)
            }
            HirExprKind::Unary { operand, .. } => check_pure(operand, constants, refs),
            _ => Err(MirError::new(
                "a top-level binding must be initialized by a constant expression (literals, \
                 operators and other top-level constants); calls and other effects are refused",
            )),
        }
    }

    let mut edges: HashMap<SymbolId, Vec<SymbolId>> = HashMap::new();
    for (symbol, initializer) in &constants {
        let mut refs = Vec::new();
        check_pure(initializer, &constants, &mut refs)?;
        edges.insert(*symbol, refs);
    }

    // Depth-first search for a cycle: `const A = B; const B = A;` has no value.
    fn visit(
        symbol: SymbolId,
        edges: &HashMap<SymbolId, Vec<SymbolId>>,
        state: &mut HashMap<SymbolId, bool>,
    ) -> Result<(), MirError> {
        match state.get(&symbol) {
            Some(true) => return Ok(()),
            Some(false) => {
                return Err(MirError::new(
                    "top-level constants refer to each other in a cycle, so none of them has a \
                     value",
                ))
            }
            None => {}
        }
        state.insert(symbol, false);
        for next in edges.get(&symbol).into_iter().flatten() {
            visit(*next, edges, state)?;
        }
        state.insert(symbol, true);
        Ok(())
    }
    let mut state = HashMap::new();
    for symbol in edges.keys() {
        visit(*symbol, &edges, &mut state)?;
    }
    Ok(constants)
}

/// The error for a construct the chain compiler has no meaning-preserving lowering for.
fn unsupported(what: &str) -> MirError {
    MirError::new(format!(
        "{what} is not supported by the chain compiler: it has no lowering that preserves its \
         meaning, so it is refused rather than compiled to a program that computes something else"
    ))
}

/// Add every symbol `stmts` assigns to, recursing into branches and loop bodies.
///
/// A name assigned anywhere is a cell for the whole function (see `MirFunctionBuilder::mutated`).
fn collect_mutated(stmts: &[HirStmt], out: &mut std::collections::HashSet<SymbolId>) {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Variable(symbol),
                ..
            } => {
                out.insert(*symbol);
            }
            // `a[i] = v` rebinds `a` (value semantics), so `a` is assigned too. Missing this made
            // a loop that fills an array keep only its last write: without a cell, each iteration
            // read the array's value from before the loop.
            HirStmt::Assign {
                target: AssignTarget::Index { array, .. },
                ..
            } => {
                if let HirExprKind::Var(symbol) = &array.kind {
                    out.insert(*symbol);
                }
            }
            HirStmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_mutated(then_block, out);
                collect_mutated(else_block, out);
            }
            HirStmt::While { body, .. } => collect_mutated(body, out),
            _ => {}
        }
    }
}

struct MirFunctionBuilder {
    symbol: SymbolId,
    span: Span,
    blocks: Vec<MirBlock>,
    current_block: MirBlockId,
    params: Vec<MirValue>,
    value_map: HashMap<SymbolId, MirValue>,
    /// Symbols the function assigns to, and the cell each one lives in.
    ///
    /// A name bound by `let` and never assigned is a value: the map says which, and SSA needs no
    /// more. A name that *is* assigned cannot be one — the loop that assigns it jumps back to a
    /// condition that was already lowered, and that condition reads a register, so the assignment
    /// has to reach that register. Lowering a mutated name as a register-model cell does exactly
    /// that: reads load from the cell and writes store into it, so the back-edge sees the new value.
    /// Measured before this: `while (i <= n) { total = total + i; i = i + 1; }` compiled and then
    /// looped forever, because the body wrote fresh temporaries the condition never read
    /// (TICKET-132).
    mutated: std::collections::HashSet<SymbolId>,
    slots: HashMap<SymbolId, MirValue>,
    /// Values this lowering knows hold a float.
    ///
    /// The type checker has no float primitive, so the flag cannot come from `HirExpr::ty`; it comes
    /// from the expression that produced the value (a float literal, or an operation on one) and is
    /// carried forward here. A float that arrives from somewhere this cannot see — a call's result —
    /// is treated as an integer, which fails loudly at the opcode (`TypeMismatch`) rather than
    /// silently computing on the wrong representation (TICKET-133).
    float_values: std::collections::HashSet<MirValue>,
    /// Whether each mutated name's cell holds a float, so a read of it keeps the flag.
    slot_float: HashMap<SymbolId, bool>,
    /// The loops this builder is inside, innermost last: where `break` and `continue` go.
    ///
    /// `break` and `continue` used to be matched to an empty arm — the comment said label
    /// resolution was missing — so a program that wrote `break` jumped nowhere and looped until it
    /// ran out of gas. Measured on `loop_break.x3`, which computes 21: `GasExhausted { used:
    /// 1_000_000 }` (TICKET-132). An unlabelled break/continue targets the innermost loop; a
    /// labelled one is refused rather than approximated.
    loops: Vec<LoopTargets>,
    next_value: usize,
    /// The module's constants (see [`constants_of`]): a `Var` that is not a local is one of these.
    constants: std::rc::Rc<HashMap<SymbolId, HirExpr>>,
}

/// Where the innermost loop's `break` and `continue` go.
#[derive(Clone, Copy)]
struct LoopTargets {
    break_to: MirBlockId,
    continue_to: MirBlockId,
}

impl MirFunctionBuilder {
    fn new(symbol: SymbolId, span: Span) -> Self {
        let mut builder = Self {
            symbol,
            span,
            blocks: Vec::new(),
            current_block: MirBlockId(0),
            params: Vec::new(),
            value_map: HashMap::new(),
            mutated: std::collections::HashSet::new(),
            slots: HashMap::new(),
            float_values: std::collections::HashSet::new(),
            slot_float: HashMap::new(),
            loops: Vec::new(),
            next_value: 0,
            constants: std::rc::Rc::new(HashMap::new()),
        };
        let entry = builder.create_block();
        builder.current_block = entry;
        builder
    }

    fn lower_function(mut self, function: &HirFunction) -> Result<MirFunction, MirError> {
        for param in &function.params {
            let value = self.allocate_value();
            self.value_map.insert(param.symbol, value);
            self.params.push(value);
        }
        collect_mutated(&function.body, &mut self.mutated);
        // A mutated parameter gets a cell of its own, copied from the incoming value, for the same
        // reason a mutated `let` does (see `new_cell`).
        for param in &function.params {
            if self.mutated.contains(&param.symbol) {
                if let Some(&value) = self.value_map.get(&param.symbol) {
                    let cell = self.new_cell(value);
                    self.slots.insert(param.symbol, cell);
                    self.value_map.insert(param.symbol, cell);
                }
            }
        }
        self.lower_statements(&function.body)?;
        self.ensure_current_block_has_terminator();
        Ok(MirFunction {
            symbol: self.symbol,
            params: self.params,
            entry: MirBlockId(0),
            blocks: self.blocks,
            span: self.span,
        })
    }

    fn lower_statements(&mut self, statements: &[HirStmt]) -> Result<(), MirError> {
        for statement in statements {
            self.lower_statement(statement)?;
        }
        Ok(())
    }

    fn lower_statement(&mut self, statement: &HirStmt) -> Result<(), MirError> {
        match statement {
            HirStmt::Let { symbol, value, .. } => {
                let evaluated = self.lower_expr(value)?;
                if self.mutated.contains(symbol) {
                    // Writes store into the cell and reads load from it, so the loop's back-edge
                    // sees the current value. The cell is a *copy* of the initial value (see
                    // `new_cell`), never the initial value's own register.
                    let cell = self.new_cell(evaluated);
                    self.slots.insert(*symbol, cell);
                    let is_float = self.float_values.contains(&evaluated);
                    self.slot_float.insert(*symbol, is_float);
                    self.value_map.insert(*symbol, cell);
                } else {
                    self.value_map.insert(*symbol, evaluated);
                }
            }
            HirStmt::Assign {
                target,
                value,
                span: _,
            } => {
                match target {
                    AssignTarget::Variable(symbol) => {
                        if !self.value_map.contains_key(symbol) {
                            return Err(MirError::new(format!(
                                "assignment target not found for symbol {symbol:?}"
                            )));
                        }
                        let evaluated = self.lower_expr(value)?;
                        if self.slots.contains_key(symbol) {
                            let is_float = self.float_values.contains(&evaluated);
                            self.slot_float.insert(*symbol, is_float);
                        }
                        if let Some(&cell) = self.slots.get(symbol) {
                            // Store into the cell, do not just rename the value: the loop's
                            // condition was lowered already and reads this register.
                            let stored = self.emit_assignment(MirRhs::Store {
                                model: MemoryModel::Register,
                                addr: cell,
                                val: evaluated,
                            });
                            self.value_map.insert(*symbol, stored);
                        } else {
                            self.value_map.insert(*symbol, evaluated);
                        }
                    }
                    AssignTarget::Index { array, index, .. } => {
                        // `a[i] = v` rebinds `a` to a copy with element `i` replaced (value
                        // semantics; see `x3_common::intrinsics::INDEX_SET_SYMBOL`). Only a named
                        // array can be rebound, so `f()[i] = v` or `a[i][j] = v` is refused rather
                        // than writing into a temporary nobody reads.
                        let HirExprKind::Var(symbol) = &array.kind else {
                            return Err(unsupported(
                                "assignment to an element of something other than a named array",
                            ));
                        };
                        let base = self.lower_expr(array)?;
                        let at = self.lower_expr(index)?;
                        let item = self.lower_expr(value)?;
                        let updated = self.emit_assignment(MirRhs::Call {
                            target: SymbolId(x3_common::intrinsics::INDEX_SET_SYMBOL),
                            args: vec![base, at, item],
                        });
                        if let Some(&cell) = self.slots.get(symbol) {
                            let stored = self.emit_assignment(MirRhs::Store {
                                model: MemoryModel::Register,
                                addr: cell,
                                val: updated,
                            });
                            self.value_map.insert(*symbol, stored);
                        } else {
                            self.value_map.insert(*symbol, updated);
                        }
                    }
                    AssignTarget::Field { .. } => {
                        // A struct field's index needs the struct's layout, which HIR does not
                        // carry. This arm used to evaluate the value and drop the write.
                        return Err(unsupported("assignment to a field"));
                    }
                }
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr)?;
            }
            HirStmt::Return { value, span: _ } => {
                let payload = value
                    .as_ref()
                    .map(|expr| self.lower_expr(expr))
                    .transpose()?;
                self.set_terminator(MirTerminator::Return(payload));
                let next = self.create_block();
                self.current_block = next;
            }
            HirStmt::If {
                condition,
                then_block,
                else_block,
                span: _,
            } => {
                self.lower_if(condition, then_block, else_block)?;
            }
            HirStmt::While {
                condition,
                body,
                label: _,
                span: _,
            } => {
                self.lower_while(condition, body)?;
            }
            HirStmt::Break { label, .. } => {
                if label.is_some() {
                    return Err(MirError::new(
                        "a labelled `break` is not lowered yet: only the innermost loop can be left,                          so this is refused rather than sent to the wrong loop",
                    ));
                }
                let target = self
                    .loops
                    .last()
                    .ok_or_else(|| MirError::new("`break` outside a loop"))?
                    .break_to;
                self.set_terminator(MirTerminator::Goto(target));
                // Anything after the break is unreachable; it gets its own block, which the
                // reachability pass then drops.
                let next = self.create_block();
                self.current_block = next;
            }
            HirStmt::Continue { label, .. } => {
                if label.is_some() {
                    return Err(MirError::new(
                        "a labelled `continue` is not lowered yet: only the innermost loop can be                          continued, so this is refused rather than sent to the wrong loop",
                    ));
                }
                let target = self
                    .loops
                    .last()
                    .ok_or_else(|| MirError::new("`continue` outside a loop"))?
                    .continue_to;
                self.set_terminator(MirTerminator::Goto(target));
                let next = self.create_block();
                self.current_block = next;
            }
            HirStmt::AtomicBegin { block_id, .. } => {
                self.push_atomic_begin(MirAtomicBlockId(block_id.0 as u16));
            }
            HirStmt::AtomicEnd {
                block_id, commit, ..
            } => {
                self.push_atomic_end(MirAtomicBlockId(block_id.0 as u16), *commit);
            }
            // Both were matched to an empty arm, so an `emit` compiled and emitted nothing: a
            // program whose receipt is supposed to carry an event produced a receipt without one.
            // `emit Name(a, b)` lowers to a call to the reserved `emit` symbol whose first
            // argument is the event's name. It was refused here — "compiled and emitted nothing"
            // — because the opcode existed and no engine implemented it; both do now, and the
            // receipt carries what they journal.
            //
            // A call is the right shape for the same reason a host call is: every optimizer pass
            // already treats one as an effect, so an `emit` whose result nobody reads is not
            // deleted and is not hoisted across a store.
            HirStmt::Emit {
                event_name, args, ..
            } => {
                if event_name.trim().is_empty() {
                    return Err(MirError::new(
                        "an event must be named: `emit` with an empty name would journal an entry \
                         no reader could identify",
                    ));
                }
                let name = self.emit_literal(x3_common::Literal::String(event_name.clone()));
                let mut call_args = vec![name];
                for arg in args {
                    call_args.push(self.lower_expr(arg)?);
                }
                self.emit_assignment(MirRhs::Call {
                    target: SymbolId(x3_common::intrinsics::EMIT_SYMBOL),
                    args: call_args,
                });
            }
            HirStmt::AgentInit { .. } => return Err(unsupported("agent initialisation")),
        }
        Ok(())
    }

    fn lower_if(
        &mut self,
        condition: &HirExpr,
        then_block: &[HirStmt],
        else_block: &[HirStmt],
    ) -> Result<(), MirError> {
        let cond_value = self.lower_expr(condition)?;
        let then_id = self.create_block();
        let else_id = self.create_block();
        let merge_id = self.create_block();
        self.set_terminator(MirTerminator::Branch {
            cond: cond_value,
            then_block: then_id,
            else_block: else_id,
        });
        self.current_block = then_id;
        self.lower_statements(then_block)?;
        self.ensure_goto(merge_id);
        self.current_block = else_id;
        self.lower_statements(else_block)?;
        self.ensure_goto(merge_id);
        self.current_block = merge_id;
        Ok(())
    }

    fn lower_while(&mut self, condition: &HirExpr, body: &[HirStmt]) -> Result<(), MirError> {
        let cond_id = self.create_block();
        let body_id = self.create_block();
        let merge_id = self.create_block();
        self.set_terminator(MirTerminator::Goto(cond_id));
        self.current_block = cond_id;
        let cond_value = self.lower_expr(condition)?;
        self.set_terminator(MirTerminator::Branch {
            cond: cond_value,
            then_block: body_id,
            else_block: merge_id,
        });
        self.current_block = body_id;
        self.loops.push(LoopTargets {
            break_to: merge_id,
            continue_to: cond_id,
        });
        self.lower_statements(body)?;
        self.loops.pop();
        self.ensure_goto(cond_id);
        self.current_block = merge_id;
        Ok(())
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Result<MirValue, MirError> {
        match &expr.kind {
            HirExprKind::Literal(literal) => {
                let value = self.emit_literal(literal.clone());
                if matches!(literal, x3_common::Literal::Float(_)) {
                    self.float_values.insert(value);
                }
                Ok(value)
            }
            HirExprKind::Var(symbol) => {
                // A mutated name is read from its cell, not from the value map: the map holds the
                // value of the last assignment *as this lowering walked it*, and a loop body's
                // reader may execute before an assignment the walk has already passed. Loading from
                // the cell is what makes the loop see the current value (TICKET-132).
                if let Some(&cell) = self.slots.get(symbol) {
                    let loaded = self.emit_assignment(MirRhs::Load {
                        model: MemoryModel::Register,
                        addr: cell,
                    });
                    if *self.slot_float.get(symbol).unwrap_or(&false) {
                        self.float_values.insert(loaded);
                    }
                    return Ok(loaded);
                }
                if let Some(&value) = self.value_map.get(symbol) {
                    return Ok(value);
                }
                if let Some(initializer) = self.constants.get(symbol).cloned() {
                    return self.lower_expr(&initializer);
                }
                Err(MirError::new(format!(
                    "value for symbol {symbol:?} missing"
                )))
            }
            HirExprKind::Binary { op, left, right } => {
                // The chain's value model has integers, bools and (in simulation) floats. A string
                // or byte string reaches the VM as a constant-pool reference, and an operator on
                // one ran as an integer operation that failed at run time with `TypeMismatch` on
                // both engines — a program the type checker accepted and no chain could run. It is
                // refused here instead, where the source position is still known.
                for operand in [left, right] {
                    if matches!(
                        operand.ty.kind,
                        x3_typeck::TypeKind::String | x3_typeck::TypeKind::Bytes
                    ) {
                        return Err(unsupported(&format!(
                            "`{op:?}` on a string or byte-string value"
                        )));
                    }
                }
                let left_val = self.lower_expr(left)?;
                let right_val = self.lower_expr(right)?;
                let float =
                    self.float_values.contains(&left_val) || self.float_values.contains(&right_val);
                let value = self.emit_assignment(MirRhs::Binary {
                    op: *op,
                    left: left_val,
                    right: right_val,
                    float,
                });
                if float {
                    self.float_values.insert(value);
                }
                Ok(value)
            }
            HirExprKind::Unary { op, operand } => {
                let val = self.lower_expr(operand)?;
                Ok(self.emit_assignment(MirRhs::Unary(*op, val)))
            }
            HirExprKind::Call { callee, args } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expr(arg)?);
                }
                Ok(self.emit_assignment(MirRhs::Call {
                    target: *callee,
                    args: mir_args,
                }))
            }
            // Every kind below used to lower to *something*: a `Unit` literal (method calls,
            // indexing, arrays, tuples and ranges, `if` expressions, context access, VM intrinsics,
            // `self`) or its operand unchanged (a field access returned the whole object, a cast
            // returned the uncast value). Each such program compiled, ran and returned a value the
            // source did not compute. A construct with no lowering that keeps its meaning is refused.
            HirExprKind::MethodCall {
                receiver,
                method,
                args,
            } if method == "len" && args.is_empty() => {
                let base = self.lower_expr(receiver)?;
                Ok(self.emit_assignment(MirRhs::Call {
                    target: SymbolId(x3_common::intrinsics::LEN_SYMBOL),
                    args: vec![base],
                }))
            }
            HirExprKind::MethodCall { .. } => Err(unsupported("a method call")),
            HirExprKind::Field { .. } => Err(unsupported("a field access")),
            HirExprKind::Index { array, index } => {
                let base = self.lower_expr(array)?;
                let at = self.lower_expr(index)?;
                Ok(self.emit_assignment(MirRhs::Call {
                    target: SymbolId(x3_common::intrinsics::INDEX_GET_SYMBOL),
                    args: vec![base, at],
                }))
            }
            HirExprKind::Array(elements) | HirExprKind::Tuple(elements) => {
                let target = if matches!(expr.kind, HirExprKind::Array(_)) {
                    x3_common::intrinsics::ARRAY_NEW_SYMBOL
                } else {
                    x3_common::intrinsics::TUPLE_NEW_SYMBOL
                };
                let mut items = Vec::with_capacity(elements.len());
                for element in elements {
                    items.push(self.lower_expr(element)?);
                }
                Ok(self.emit_assignment(MirRhs::Call {
                    target: SymbolId(target),
                    args: items,
                }))
            }
            HirExprKind::Block { stmts, expr } => {
                self.lower_statements(stmts)?;
                if let Some(e) = expr {
                    self.lower_expr(e)
                } else {
                    Ok(self.emit_assignment(MirRhs::Literal(x3_common::Literal::Unit)))
                }
            }
            HirExprKind::IfExpr { .. } => Err(unsupported("an `if` expression")),
            HirExprKind::Cast { .. } => Err(unsupported("a cast")),
            HirExprKind::ContextAccess(_) => Err(unsupported("a context field")),
            HirExprKind::VmIntrinsic { .. } => Err(unsupported("a VM intrinsic")),
            HirExprKind::SelfRef => Err(unsupported("`self`")),
        }
    }

    /// A fresh register holding a copy of `initial`, to be a mutable variable's storage.
    ///
    /// The cell used to *be* the initial value: `let mut x = a` made `x`'s storage `a`'s register,
    /// so `x = 7` overwrote `a`, and `let mut x = p` overwrote the parameter. Measured at O0:
    /// `let a = 5; let mut x = a; x = 7; return a * 10 + x` returned 77, not 57. It also broke
    /// every optimization level above O1, because the passes read the cell's defining statement as
    /// an immutable SSA definition: an inner loop's `let mut j = 0` is a literal, a literal is
    /// loop-invariant, and hoisting it out of the outer loop meant `j` was reset once instead of
    /// per iteration (`nested_while` returned 6 instead of 120). A copy defined by a register load
    /// is neither: loads are not hoisted or folded, and the copy aliases nothing.
    fn new_cell(&mut self, initial: MirValue) -> MirValue {
        let cell = self.emit_assignment(MirRhs::Load {
            model: MemoryModel::Register,
            addr: initial,
        });
        if self.float_values.contains(&initial) {
            self.float_values.insert(cell);
        }
        cell
    }

    fn emit_literal(&mut self, literal: x3_common::Literal) -> MirValue {
        self.emit_assignment(MirRhs::Literal(literal))
    }

    fn emit_assignment(&mut self, rhs: MirRhs) -> MirValue {
        let target = self.allocate_value();
        let block = self.current_block_mut();
        block.statements.push(MirStatement::Assign { target, rhs });
        target
    }

    fn push_atomic_begin(&mut self, block_id: MirAtomicBlockId) {
        let block = self.current_block_mut();
        block
            .statements
            .push(MirStatement::AtomicBegin { block_id });
    }

    fn push_atomic_end(&mut self, block_id: MirAtomicBlockId, commit: bool) {
        let block = self.current_block_mut();
        block
            .statements
            .push(MirStatement::AtomicEnd { block_id, commit });
    }

    fn ensure_goto(&mut self, target: MirBlockId) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(MirTerminator::Goto(target));
        }
    }

    fn ensure_current_block_has_terminator(&mut self) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(MirTerminator::Return(None));
        }
    }

    fn set_terminator(&mut self, terminator: MirTerminator) {
        let block = self.current_block_mut();
        if block.terminator.is_none() {
            block.terminator = Some(terminator);
        }
    }

    fn create_block(&mut self) -> MirBlockId {
        let id = MirBlockId(self.blocks.len());
        self.blocks.push(MirBlock {
            id,
            statements: Vec::new(),
            terminator: None,
        });
        id
    }

    fn current_block_mut(&mut self) -> &mut MirBlock {
        &mut self.blocks[self.current_block.0]
    }

    fn allocate_value(&mut self) -> MirValue {
        let value = MirValue(self.next_value);
        self.next_value += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_hir::lower::HirLowerer;
    use x3_parser::Parser;

    #[test]
    fn lower_function_with_return() {
        let source = "fn add(x: i32, y: i32) -> i32 { return x + y; }";
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("parse");
        let hir = HirLowerer::lower(module).expect("hir");
        let mir = MirLowerer::lower(&hir).expect("mir");
        assert_eq!(mir.functions.len(), 1);
        let function = &mir.functions[0];
        assert!(function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(MirTerminator::Return(Some(_))))));
    }
}
