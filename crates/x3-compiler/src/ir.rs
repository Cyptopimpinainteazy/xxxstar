//! X3 Intermediate Representation (IR): a typed, register-based IR between AST and bytecode.
//!
//! The IR is designed for optimization passes. Each function is represented as a control-flow
//! graph of basic blocks containing SSA-style three-address instructions.

use std::collections::BTreeMap;

/// An SSA value (virtual register).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Val(pub u32);

/// IR types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrType {
    I64,
    U64,
    Bool,
    Ptr,
    Unit,
}

/// Three-address IR instruction.
#[derive(Clone, Debug)]
pub enum IrInstr {
    /// `dst = lhs op rhs`
    BinOp {
        dst: Val,
        op: IrBinOp,
        lhs: Val,
        rhs: Val,
    },
    /// `dst = constant`
    Const { dst: Val, val: i64 },
    /// `dst = call(func, args...)`
    Call {
        dst: Option<Val>,
        func: String,
        args: Vec<Val>,
    },
    /// Unconditional jump to block.
    Jump { target: BlockId },
    /// Conditional branch.
    Branch {
        cond: Val,
        true_target: BlockId,
        false_target: BlockId,
    },
    /// Return a value.
    Return { val: Option<Val> },
    /// Store `val` at address `ptr`.
    Store { ptr: Val, val: Val },
    /// `dst = load from ptr`.
    Load { dst: Val, ptr: Val },
    /// No-op (placeholder after dead code elimination).
    Nop,
}

/// IR binary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
    CmpEq,
    CmpNe,
    CmpLt,
    CmpGt,
}

/// A basic block identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct BlockId(pub u32);

/// A basic block: a sequence of instructions ending in a terminator.
#[derive(Clone, Debug, Default)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instrs: Vec<IrInstr>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            instrs: Vec::new(),
        }
    }

    /// True if the block ends with a terminator (Jump/Branch/Return).
    pub fn is_terminated(&self) -> bool {
        self.instrs.last().map_or(false, |i| {
            matches!(
                i,
                IrInstr::Jump { .. } | IrInstr::Branch { .. } | IrInstr::Return { .. }
            )
        })
    }
}

/// A single IR function.
#[derive(Clone, Debug)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(Val, IrType)>,
    pub return_type: IrType,
    pub blocks: BTreeMap<BlockId, BasicBlock>,
    pub entry: BlockId,
    /// Next SSA value counter.
    next_val: u32,
    /// Next block ID counter.
    next_block: u32,
}

impl IrFunction {
    pub fn new(name: impl Into<String>, return_type: IrType) -> Self {
        let entry = BlockId(0);
        let mut blocks = BTreeMap::new();
        blocks.insert(entry, BasicBlock::new(entry));
        Self {
            name: name.into(),
            params: Vec::new(),
            return_type,
            blocks,
            entry,
            next_val: 0,
            next_block: 1,
        }
    }

    /// Allocate a new SSA value.
    pub fn new_val(&mut self) -> Val {
        let v = Val(self.next_val);
        self.next_val += 1;
        v
    }

    /// Allocate a new basic block.
    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        self.blocks.insert(id, BasicBlock::new(id));
        id
    }

    /// Append an instruction to a block.
    pub fn push(&mut self, block: BlockId, instr: IrInstr) {
        if let Some(b) = self.blocks.get_mut(&block) {
            b.instrs.push(instr);
        }
    }

    /// Validate: every block must be terminated.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (id, block) in &self.blocks {
            if !block.is_terminated() {
                errors.push(format!("block {:?} is not terminated", id));
            }
        }
        errors
    }
}

/// The complete IR module.
#[derive(Clone, Debug, Default)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_val_increments() {
        let mut f = IrFunction::new("test", IrType::Unit);
        let v1 = f.new_val();
        let v2 = f.new_val();
        assert_ne!(v1, v2);
        assert_eq!(v1.0 + 1, v2.0);
    }

    #[test]
    fn test_new_block() {
        let mut f = IrFunction::new("test", IrType::Unit);
        let b1 = f.new_block();
        let b2 = f.new_block();
        assert_ne!(b1, b2);
    }

    #[test]
    fn test_push_and_terminated() {
        let mut f = IrFunction::new("test", IrType::I64);
        let entry = f.entry;
        assert!(!f.blocks[&entry].is_terminated());
        f.push(entry, IrInstr::Return { val: None });
        assert!(f.blocks[&entry].is_terminated());
    }

    #[test]
    fn test_validate_unterminated_block_reports_error() {
        let f = IrFunction::new("test", IrType::Unit);
        let errors = f.validate();
        assert!(
            !errors.is_empty(),
            "expected validation error for unterminated block"
        );
    }

    #[test]
    fn test_validate_terminated_block_no_error() {
        let mut f = IrFunction::new("test", IrType::Unit);
        let entry = f.entry;
        f.push(entry, IrInstr::Return { val: None });
        assert!(f.validate().is_empty());
    }

    #[test]
    fn test_const_instr() {
        let mut f = IrFunction::new("test", IrType::I64);
        let entry = f.entry;
        let dst = f.new_val();
        f.push(entry, IrInstr::Const { dst, val: 42 });
        f.push(entry, IrInstr::Return { val: Some(dst) });
        assert!(f.validate().is_empty());
        assert_eq!(f.blocks[&entry].instrs.len(), 2);
    }

    #[test]
    fn test_binop_instr() {
        let mut f = IrFunction::new("add", IrType::I64);
        let entry = f.entry;
        let a = f.new_val();
        let b = f.new_val();
        let dst = f.new_val();
        f.push(entry, IrInstr::Const { dst: a, val: 10 });
        f.push(entry, IrInstr::Const { dst: b, val: 20 });
        f.push(
            entry,
            IrInstr::BinOp {
                dst,
                op: IrBinOp::Add,
                lhs: a,
                rhs: b,
            },
        );
        f.push(entry, IrInstr::Return { val: Some(dst) });
        assert!(f.validate().is_empty());
    }
}
