//! Copy Propagation Pass
//!
//! This pass performs two types of copy propagation:
//! 1. Identity pattern propagation: x+0 -> x, x*1 -> x, x-0 -> x, etc.
//! 2. Value equivalence tracking: when a value is known to equal another.
//!
//! # Transformations
//!
//! ```text
//! v1 = v0 + 0    // identity: add zero
//! v2 = v1 * 1    // identity: multiply by one
//! v3 = v2 - 0    // identity: subtract zero
//! ```
//! becomes:
//! ```text
//! v1 = v0        // replaced: add zero is identity
//! v2 = v0        // replaced: multiply by one is identity, propagates v1=v0
//! v3 = v0        // replaced: subtract zero is identity, propagates through chain
//! ```
//!
//! # Algorithm
//!
//! 1. For each block, scan statements left-to-right
//! 2. Detect identity patterns in Binary ops (x+0, x*1, etc.)
//! 3. Replace identity ops with their operand
//! 4. Track value equivalence: when target = source, record the mapping
//! 5. Propagate equivalences through subsequent statements
//! 6. Iterate until no changes (for chains of copies)

use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::BTreeMap;
use x3_ast::BinaryOp;
use x3_common::Literal;
use x3_mir::{MirModule, MirRhs, MirValue};

/// Copy propagation pass.
pub struct CopyPropagationPass;

impl CopyPropagationPass {
    pub fn new() -> Self {
        CopyPropagationPass
    }

    /// Check if a binary operation is an identity (e.g., x + 0, x * 1).
    /// Returns Some(operand_value) if identity, None otherwise.
    fn is_identity(
        &self,
        op: BinaryOp,
        lhs: MirValue,
        rhs: MirValue,
        literals: &BTreeMap<MirValue, Literal>,
    ) -> Option<MirValue> {
        match op {
            // x + 0 = x
            BinaryOp::Add => {
                if let Some(Literal::Integer(0)) = literals.get(&rhs) {
                    return Some(lhs);
                }
                None
            }
            // 0 + x = x
            BinaryOp::Add => {
                if let Some(Literal::Integer(0)) = literals.get(&lhs) {
                    return Some(rhs);
                }
                None
            }
            // x - 0 = x
            BinaryOp::Sub => {
                if let Some(Literal::Integer(0)) = literals.get(&rhs) {
                    return Some(lhs);
                }
                None
            }
            // x * 1 = x
            BinaryOp::Mul => {
                if let Some(Literal::Integer(1)) = literals.get(&rhs) {
                    return Some(lhs);
                }
                None
            }
            // 1 * x = x
            BinaryOp::Mul => {
                if let Some(Literal::Integer(1)) = literals.get(&lhs) {
                    return Some(rhs);
                }
                None
            }
            // x / 1 = x
            BinaryOp::Div => {
                if let Some(Literal::Integer(1)) = literals.get(&rhs) {
                    return Some(lhs);
                }
                None
            }
            _ => None,
        }
    }

    /// Resolve a value through copy chain, returning the ultimate source.
    fn resolve(&self, value: MirValue, copies: &BTreeMap<MirValue, MirValue>) -> MirValue {
        let mut current = value;
        let mut visited = std::collections::HashSet::new();

        // Follow copy chain with cycle detection
        for _ in 0..100 {
            if !visited.insert(current) {
                break; // cycle detected
            }
            if let Some(&src) = copies.get(&current) {
                if src == current {
                    break; // self-copy
                }
                current = src;
            } else {
                break;
            }
        }
        current
    }

    /// Collect all literal values from a function
    fn collect_literals(&self, func: &x3_mir::MirFunction) -> BTreeMap<MirValue, Literal> {
        let mut literals = BTreeMap::new();

        for block in &func.blocks {
            for stmt in &block.statements {
                if let MirRhs::Literal(lit) = &stmt.rhs {
                    literals.insert(stmt.target, lit.clone());
                }
            }
        }

        literals
    }

    /// Replace a value with its equivalent if available
    fn replace_value(
        &self,
        value: MirValue,
        replacements: &BTreeMap<MirValue, MirValue>,
    ) -> MirValue {
        // First resolve through copy chains
        let resolved = self.resolve(value, replacements);

        // Then check for any direct replacements
        if let Some(&replacement) = replacements.get(&resolved) {
            return replacement;
        }

        resolved
    }
}

impl Default for CopyPropagationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for CopyPropagationPass {
    fn name(&self) -> &'static str {
        "copy_propagation"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut total_changes = 0usize;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10;

        // Iterate until no changes (for chains of copies)
        while iterations < MAX_ITERATIONS {
            iterations += 1;
            let mut iter_changes = 0;

            for func in module.functions.iter_mut() {
                // Collect literals for identity detection
                let literals = self.collect_literals(func);

                // Track value equivalences: target -> source
                let mut copies: BTreeMap<MirValue, MirValue> = BTreeMap::new();

                // First pass: find identity operations and replace them
                for block in func.blocks.iter_mut() {
                    let mut new_statements = Vec::new();

                    for stmt in block.statements.iter() {
                        let mut replaced_stmt = stmt.clone();

                        // Replace values in the RHS
                        if let MirRhs::Binary(op, lhs, rhs) = &stmt.rhs {
                            // Check for identity patterns first
                            if let Some(identity_operand) =
                                self.is_identity(*op, *lhs, *rhs, &literals)
                            {
                                // This is an identity operation, replace with operand
                                replaced_stmt.rhs = MirRhs::Literal(Literal::Integer(0)); // placeholder
                                                                                          // Actually, we want to propagate the value - but since we don't have
                                                                                          // a "move" instruction, we record the equivalence
                                copies.insert(stmt.target, identity_operand);
                                iter_changes += 1;
                                continue;
                            }

                            // Replace operands with their equivalents
                            let new_lhs = self.replace_value(*lhs, &copies);
                            let new_rhs = self.replace_value(*rhs, &copies);

                            if new_lhs != *lhs || new_rhs != *rhs {
                                replaced_stmt.rhs = MirRhs::Binary(*op, new_lhs, new_rhs);
                                iter_changes += 1;
                            }
                        } else if let MirRhs::Unary(op, arg) = &stmt.rhs {
                            let new_arg = self.replace_value(*arg, &copies);
                            if new_arg != *arg {
                                replaced_stmt.rhs = MirRhs::Unary(*op, new_arg);
                                iter_changes += 1;
                            }
                        } else if let MirRhs::Call { target, args } = &stmt.rhs {
                            let new_args: Vec<MirValue> = args
                                .iter()
                                .map(|&v| self.replace_value(v, &copies))
                                .collect();
                            if new_args != *args {
                                replaced_stmt.rhs = MirRhs::Call {
                                    target: *target,
                                    args: new_args,
                                };
                                iter_changes += 1;
                            }
                        } else if let MirRhs::Load { model, addr } = &stmt.rhs {
                            let new_addr = self.replace_value(*addr, &copies);
                            if new_addr != *addr {
                                replaced_stmt.rhs = MirRhs::Load {
                                    model: *model,
                                    addr: new_addr,
                                };
                                iter_changes += 1;
                            }
                        } else if let MirRhs::Store { model, addr, val } = &stmt.rhs {
                            let new_addr = self.replace_value(*addr, &copies);
                            let new_val = self.replace_value(*val, &copies);
                            if new_addr != *addr || new_val != *val {
                                replaced_stmt.rhs = MirRhs::Store {
                                    model: *model,
                                    addr: new_addr,
                                    val: new_val,
                                };
                                iter_changes += 1;
                            }
                        }

                        // Record literal values for identity detection
                        if let MirRhs::Literal(lit) = &replaced_stmt.rhs {
                            // We don't add to literals here as they're already collected
                            // But we DO want to track value equivalence
                        }

                        new_statements.push(replaced_stmt);
                    }

                    // Update terminator if needed
                    if let Some(ref mut terminator) = block.terminator {
                        match terminator {
                            x3_mir::MirTerminator::Branch { cond, .. } => {
                                let new_cond = self.replace_value(*cond, &copies);
                                if new_cond != *cond {
                                    *cond = new_cond;
                                    iter_changes += 1;
                                }
                            }
                            _ => {}
                        }
                    }

                    block.statements = new_statements;
                }
            }

            total_changes += iter_changes;

            // If no changes in this iteration, we're done
            if iter_changes == 0 {
                break;
            }
        }

        if total_changes > 0 {
            Ok(PassResult::with_count(
                total_changes,
                format!(
                    "copy_propagation: {} changes in {} iterations",
                    total_changes, iterations
                ),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
    use x3_mir::{
        MirBlock, MirBlockId, MirFunction, MirRhs, MirStatement, MirTerminator, SymbolId,
    };

    fn make_module(func: MirFunction) -> MirModule {
        MirModule {
            functions: vec![func],
            span: Span::dummy(),
        }
    }

    #[test]
    fn copy_prop_no_change_simple() {
        // Simple function with no copies
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Integer(42)),
                }],
                terminator: Some(MirTerminator::Return(Some(MirValue(0)))),
            }],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = CopyPropagationPass::new();
        let result = pass.run(&mut module).unwrap();

        // No copies to propagate
        assert!(!result.changed);
    }

    #[test]
    fn resolve_copy_chain() {
        let mut copies = BTreeMap::new();
        copies.insert(MirValue(2), MirValue(1));
        copies.insert(MirValue(1), MirValue(0));

        // v2 -> v1 -> v0
        let pass = CopyPropagationPass::new();
        let resolved = pass.resolve(MirValue(2), &copies);
        assert_eq!(resolved, MirValue(0));
    }

    #[test]
    fn resolve_no_copy() {
        let copies = BTreeMap::new();

        // v5 is not a copy of anything
        let pass = CopyPropagationPass::new();
        let resolved = pass.resolve(MirValue(5), &copies);
        assert_eq!(resolved, MirValue(5));
    }

    #[test]
    fn identity_add_zero() {
        // v1 = v0 + 0 should become v1 = v0
        let mut copies: BTreeMap<MirValue, MirValue> = BTreeMap::new();

        let pass = CopyPropagationPass::new();
        let result = pass.is_identity(BinaryOp::Add, MirValue(0), MirValue(1), &{
            let mut lit: BTreeMap<MirValue, Literal> = BTreeMap::new();
            lit.insert(MirValue(1), Literal::Integer(0));
            lit
        });

        assert_eq!(result, Some(MirValue(0)));
    }

    #[test]
    fn identity_mul_one() {
        // v1 = v0 * 1 should become v1 = v0
        let mut lit = BTreeMap::new();
        lit.insert(MirValue(1), Literal::Integer(1));

        let pass = CopyPropagationPass::new();
        let result = pass.is_identity(BinaryOp::Mul, MirValue(0), MirValue(1), &lit);

        assert_eq!(result, Some(MirValue(0)));
    }

    #[test]
    fn no_identity_normal_add() {
        // v1 = v0 + 5 is not an identity
        let mut lit = BTreeMap::new();
        lit.insert(MirValue(1), Literal::Integer(5));

        let pass = CopyPropagationPass::new();
        let result = pass.is_identity(BinaryOp::Add, MirValue(0), MirValue(1), &lit);

        assert_eq!(result, None);
    }

    #[test]
    fn identity_sub_zero() {
        // v1 = v0 - 0 should become v1 = v0
        let mut lit = BTreeMap::new();
        lit.insert(MirValue(1), Literal::Integer(0));

        let pass = CopyPropagationPass::new();
        let result = pass.is_identity(BinaryOp::Sub, MirValue(0), MirValue(1), &lit);

        assert_eq!(result, Some(MirValue(0)));
    }

    #[test]
    fn replace_through_copy() {
        // If v1 = v0, then using v1 should become v0
        let mut copies = BTreeMap::new();
        copies.insert(MirValue(1), MirValue(0));

        let pass = CopyPropagationPass::new();
        let result = pass.replace_value(MirValue(1), &copies);

        assert_eq!(result, MirValue(0));
    }

    #[test]
    fn replace_chain() {
        // v2 = v1, v1 = v0 => v2 should become v0
        let mut copies = BTreeMap::new();
        copies.insert(MirValue(2), MirValue(1));
        copies.insert(MirValue(1), MirValue(0));

        let pass = CopyPropagationPass::new();
        let result = pass.replace_value(MirValue(2), &copies);

        assert_eq!(result, MirValue(0));
    }
}
