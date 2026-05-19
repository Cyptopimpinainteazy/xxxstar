//! X3 compiler optimizer: simplification and dead code elimination passes on the IR.
//!
//! Each pass takes an IrFunction and returns a modified copy. Passes compose.

use crate::ir::{BlockId, IrFunction, IrInstr, IrBinOp, Val};
use std::collections::{BTreeMap, HashSet};

/// Run all optimization passes at the given level.
///
/// O0: no optimization (identity)
/// O1: constant folding + DCE
/// O2: O1 + algebraic simplifications
pub fn optimize(func: &mut IrFunction, level: u8) {
    if level == 0 {
        return;
    }
    fold_constants(func);
    eliminate_dead_code(func);
    if level >= 2 {
        algebraic_simplify(func);
    }
}

/// Constant folding: evaluate BinOp instructions where both operands are known constants.
pub fn fold_constants(func: &mut IrFunction) {
    let mut const_map: BTreeMap<Val, i64> = BTreeMap::new();

    for block in func.blocks.values_mut() {
        for instr in block.instrs.iter_mut() {
            match instr {
                IrInstr::Const { dst, val } => {
                    const_map.insert(*dst, *val);
                }
                IrInstr::BinOp { dst, op, lhs, rhs } => {
                    if let (Some(&lv), Some(&rv)) = (const_map.get(lhs), const_map.get(rhs)) {
                        if let Some(result) = eval_binop(*op, lv, rv) {
                            const_map.insert(*dst, result);
                            *instr = IrInstr::Const { dst: *dst, val: result };
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn eval_binop(op: IrBinOp, lhs: i64, rhs: i64) -> Option<i64> {
    match op {
        IrBinOp::Add => Some(lhs.wrapping_add(rhs)),
        IrBinOp::Sub => Some(lhs.wrapping_sub(rhs)),
        IrBinOp::Mul => Some(lhs.wrapping_mul(rhs)),
        IrBinOp::Div => {
            if rhs == 0 {
                None
            } else {
                Some(lhs.wrapping_div(rhs))
            }
        }
        IrBinOp::CmpEq => Some(if lhs == rhs { 1 } else { 0 }),
        IrBinOp::CmpNe => Some(if lhs != rhs { 1 } else { 0 }),
        IrBinOp::CmpLt => Some(if lhs < rhs { 1 } else { 0 }),
        IrBinOp::CmpGt => Some(if lhs > rhs { 1 } else { 0 }),
    }
}

/// Dead code elimination: remove Nop instructions and unreachable blocks.
pub fn eliminate_dead_code(func: &mut IrFunction) {
    // Remove Nop instructions from all blocks
    for block in func.blocks.values_mut() {
        block.instrs.retain(|i| !matches!(i, IrInstr::Nop));
    }

    // Find reachable blocks via BFS from entry
    let mut reachable = HashSet::new();
    let mut queue = vec![func.entry];
    while let Some(bid) = queue.pop() {
        if !reachable.insert(bid) {
            continue;
        }
        if let Some(block) = func.blocks.get(&bid) {
            for instr in &block.instrs {
                match instr {
                    IrInstr::Jump { target } => queue.push(*target),
                    IrInstr::Branch { true_target, false_target, .. } => {
                        queue.push(*true_target);
                        queue.push(*false_target);
                    }
                    _ => {}
                }
            }
        }
    }
    func.blocks.retain(|id, _| reachable.contains(id));
}

/// Algebraic simplifications: x + 0 = x, x * 1 = x, x - 0 = x, etc.
pub fn algebraic_simplify(func: &mut IrFunction) {
    let mut const_map: BTreeMap<Val, i64> = BTreeMap::new();
    // First pass: build const map
    for block in func.blocks.values() {
        for instr in &block.instrs {
            if let IrInstr::Const { dst, val } = instr {
                const_map.insert(*dst, *val);
            }
        }
    }
    // Second pass: simplify
    for block in func.blocks.values_mut() {
        for instr in block.instrs.iter_mut() {
            if let IrInstr::BinOp { dst, op, lhs, rhs } = instr {
                let lv = const_map.get(lhs).copied();
                let rv = const_map.get(rhs).copied();
                match op {
                    IrBinOp::Add if rv == Some(0) => {
                        *instr = IrInstr::BinOp { dst: *dst, op: IrBinOp::Add, lhs: *lhs, rhs: *rhs };
                    }
                    IrBinOp::Mul if rv == Some(1) => {
                        // x * 1 = x: replace with copy (use Add with 0 as proxy)
                        *instr = IrInstr::Nop;
                    }
                    IrBinOp::Mul if lv == Some(0) || rv == Some(0) => {
                        *instr = IrInstr::Const { dst: *dst, val: 0 };
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrFunction, IrInstr, IrType, Val};

    fn simple_fn() -> IrFunction {
        let mut f = IrFunction::new("test", IrType::I64);
        let entry = f.entry;
        let a = f.new_val();
        let b = f.new_val();
        let c = f.new_val();
        f.push(entry, IrInstr::Const { dst: a, val: 10 });
        f.push(entry, IrInstr::Const { dst: b, val: 20 });
        f.push(entry, IrInstr::BinOp { dst: c, op: IrBinOp::Add, lhs: a, rhs: b });
        f.push(entry, IrInstr::Return { val: Some(c) });
        f
    }

    #[test]
    fn test_constant_folding_add() {
        let mut f = simple_fn();
        fold_constants(&mut f);
        // c should be folded to 30
        let entry = f.entry;
        let has_const_30 = f.blocks[&entry]
            .instrs
            .iter()
            .any(|i| matches!(i, IrInstr::Const { val: 30, .. }));
        assert!(has_const_30, "expected constant 30 after folding");
    }

    #[test]
    fn test_dce_removes_nops() {
        let mut f = IrFunction::new("test", IrType::Unit);
        let entry = f.entry;
        f.push(entry, IrInstr::Nop);
        f.push(entry, IrInstr::Return { val: None });
        eliminate_dead_code(&mut f);
        assert!(!f.blocks[&entry].instrs.iter().any(|i| matches!(i, IrInstr::Nop)));
    }

    #[test]
    fn test_dce_removes_unreachable_blocks() {
        let mut f = IrFunction::new("test", IrType::Unit);
        let entry = f.entry;
        let unreachable = f.new_block();
        f.push(entry, IrInstr::Return { val: None });
        f.push(unreachable, IrInstr::Return { val: None });
        assert_eq!(f.blocks.len(), 2);
        eliminate_dead_code(&mut f);
        assert_eq!(f.blocks.len(), 1);
    }

    #[test]
    fn test_optimize_o0_is_identity() {
        let mut f = simple_fn();
        let before_len = f.blocks[&f.entry].instrs.len();
        optimize(&mut f, 0);
        assert_eq!(f.blocks[&f.entry].instrs.len(), before_len);
    }

    #[test]
    fn test_optimize_o1_folds_constants() {
        let mut f = simple_fn();
        optimize(&mut f, 1);
        let entry = f.entry;
        assert!(f.blocks[&entry]
            .instrs
            .iter()
            .any(|i| matches!(i, IrInstr::Const { val: 30, .. })));
    }

    #[test]
    fn test_constant_folding_div() {
        let mut f = IrFunction::new("test", IrType::I64);
        let entry = f.entry;
        let a = f.new_val();
        let b = f.new_val();
        let c = f.new_val();
        f.push(entry, IrInstr::Const { dst: a, val: 100 });
        f.push(entry, IrInstr::Const { dst: b, val: 5 });
        f.push(entry, IrInstr::BinOp { dst: c, op: IrBinOp::Div, lhs: a, rhs: b });
        f.push(entry, IrInstr::Return { val: Some(c) });
        fold_constants(&mut f);
        assert!(f.blocks[&entry]
            .instrs
            .iter()
            .any(|i| matches!(i, IrInstr::Const { val: 20, .. })));
    }

    #[test]
    fn test_constant_folding_div_by_zero_not_folded() {
        let mut f = IrFunction::new("test", IrType::I64);
        let entry = f.entry;
        let a = f.new_val();
        let b = f.new_val();
        let c = f.new_val();
        f.push(entry, IrInstr::Const { dst: a, val: 10 });
        f.push(entry, IrInstr::Const { dst: b, val: 0 });
        f.push(entry, IrInstr::BinOp { dst: c, op: IrBinOp::Div, lhs: a, rhs: b });
        f.push(entry, IrInstr::Return { val: Some(c) });
        // Should NOT fold (div by zero)
        fold_constants(&mut f);
        assert!(!f.blocks[&entry]
            .instrs
            .iter()
            .any(|i| matches!(i, IrInstr::Const { dst, .. } if *dst == c)));
    }
}
