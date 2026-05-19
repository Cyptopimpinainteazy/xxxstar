use x3_ast::{BinaryOp, UnaryOp};
use x3_common::Span;
pub use x3_hir::hir::SymbolId;

use crate::memory::MemoryModel;

/// SSA value produced inside the MIR module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirValue(pub usize);

/// Basic block identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirBlockId(pub usize);

/// Lowered MIR module.
#[derive(Clone, Debug, PartialEq)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub span: Span,
}

/// MIR-describing function body.
#[derive(Clone, Debug, PartialEq)]
pub struct MirFunction {
    pub symbol: SymbolId,
    pub params: Vec<MirValue>,
    pub entry: MirBlockId,
    pub blocks: Vec<MirBlock>,
    pub span: Span,
}

/// A basic block with statements and a terminator.
#[derive(Clone, Debug, PartialEq)]
pub struct MirBlock {
    pub id: MirBlockId,
    pub statements: Vec<MirStatement>,
    pub terminator: Option<MirTerminator>,
}

/// Side-effecting assignment (SSA binding) in a block.
#[derive(Clone, Debug, PartialEq)]
pub struct MirStatement {
    pub target: MirValue,
    pub rhs: MirRhs,
}

/// Right-hand sides for MIR assignments.
#[derive(Clone, Debug, PartialEq)]
pub enum MirRhs {
    Literal(x3_common::Literal),
    Unary(UnaryOp, MirValue),
    Binary(BinaryOp, MirValue, MirValue),
    Call {
        target: SymbolId,
        args: Vec<MirValue>,
    },
    /// Load from memory using the specified model.
    /// `addr` is the address/slot to load from.
    Load {
        model: MemoryModel,
        addr: MirValue,
    },
    /// Store to memory using the specified model.
    /// `addr` is the destination address/slot, `val` is the value to store.
    Store {
        model: MemoryModel,
        addr: MirValue,
        val: MirValue,
    },
}

/// Terminators that control the flow between blocks.
#[derive(Clone, Debug, PartialEq)]
pub enum MirTerminator {
    Return(Option<MirValue>),
    Goto(MirBlockId),
    Branch {
        cond: MirValue,
        then_block: MirBlockId,
        else_block: MirBlockId,
    },
}
