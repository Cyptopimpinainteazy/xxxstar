//! AST -> bytecode lowering pipeline (simplified for v0.1).
//!
//! This module performs:
//! - simple lowering from AST to a linear sequence of bytecode instructions
//! - simple stack/register allocation suitable for naive codegen
//! - deterministic error reporting

use x3_lang_ast::ast::*;
use x3_lang_common::{Span, Symbol};

pub struct LowerCtx {
    pub next_label: u32,
}

impl LowerCtx {
    pub fn new() -> Self {
        LowerCtx { next_label: 0 }
    }
    pub fn fresh_label(&mut self) -> u32 {
        let r = self.next_label;
        self.next_label += 1;
        r
    }
}

/// A lowered instruction suitable for encoding into bytecode.
#[derive(Clone, Debug)]
pub struct LoweredInstr {
    pub opcode: u8,
    pub flags: u8,
    pub operand: u16,
}

pub fn lower_program(_p: &Program) -> Result<Vec<LoweredInstr>, String> {
    // For now: stub - unwrap top-level functions, strategies and produce a NOP sequence
    let mut instrs = Vec::new();
    // For now: stub - lower top-level functions and their bodies into a sequential list of instructions
    for item in &_p.items {
        match &item.node {
            Item::Function(func) => {
                // Visit each statement in function body
                for stmt in &func.body.stmts {
                    match stmt {
                        Statement::Expr(expr) => {
                            if let Expression::Literal(lit) = expr {
                                match lit {
                                    LiteralExpr::Int { value, .. } => {
                                        // encode 16-bit immediate if possible
                                        let imm = (*value as i32) as i16 as u16;
                                        instrs.push(LoweredInstr {
                                            opcode: 0x20,
                                            flags: 0,
                                            operand: imm,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // Add HALT after a function
                instrs.push(LoweredInstr {
                    opcode: 0xFF,
                    flags: 0,
                    operand: 0,
                });
            }
            _ => {}
        }
    }
    Ok(instrs)
}
