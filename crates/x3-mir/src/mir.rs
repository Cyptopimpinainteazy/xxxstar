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

/// Atomic block identifier used in MIR.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MirAtomicBlockId(pub u16);

/// A statement in a basic block.
///
/// Most statements are SSA assignments (`target = rhs`), but atomic
/// block markers are also statements so they appear in instruction
/// order and act as optimization barriers.
#[derive(Clone, Debug, PartialEq)]
pub enum MirStatement {
    /// SSA binding: target = rhs
    Assign { target: MirValue, rhs: MirRhs },
    /// Begin an atomic transaction block.
    AtomicBegin { block_id: MirAtomicBlockId },
    /// End an atomic transaction block (commit or rollback).
    AtomicEnd {
        block_id: MirAtomicBlockId,
        commit: bool,
    },
}

impl MirStatement {
    /// Returns the target value for assignment statements, or `None` for
    /// atomic markers (which produce no SSA value).
    pub fn target(&self) -> Option<MirValue> {
        match self {
            MirStatement::Assign { target, .. } => Some(*target),
            _ => None,
        }
    }

    /// Returns the RHS for assignment statements.
    pub fn rhs(&self) -> Option<&MirRhs> {
        match self {
            MirStatement::Assign { rhs, .. } => Some(rhs),
            _ => None,
        }
    }

    /// Convenience: returns `(target, rhs)` for assignment statements.
    pub fn as_assign(&self) -> Option<(MirValue, &MirRhs)> {
        match self {
            MirStatement::Assign { target, rhs } => Some((*target, rhs)),
            _ => None,
        }
    }

    /// Whether this statement is an atomic marker (begin/end).
    /// Atomic markers are optimization barriers — no reordering across them.
    pub fn is_atomic_marker(&self) -> bool {
        matches!(
            self,
            MirStatement::AtomicBegin { .. } | MirStatement::AtomicEnd { .. }
        )
    }
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
