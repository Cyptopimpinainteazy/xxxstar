pub mod emitter;
pub mod ir;
pub mod lowering;
pub mod parser;
pub mod regalloc;

use emitter::emit_x3ir;
use lowering::{lower_program, LowerCtx};
use parser::parse_source;
use x3_lang_ast::ast::Program;
use x3_lang_common::X3Error;

// Re-export IR types
pub use ir::{Condition, FailureAction, Operation, ProgramMetadata, RequireKind, X3IR};

/// Compile an X3 AST program to bytecode
///
/// Pipeline: AST → X3IR → Bytecode
pub fn compile_program(program: &Program) -> Result<Vec<u8>, X3Error> {
    compile_program_with_context(program, LowerCtx::new())
}

/// Parse and compile X3 source for the currently supported capability subset.
pub fn compile_source(source: &str) -> Result<Vec<u8>, X3Error> {
    let program = parse_source(source)?;
    compile_program(&program)
}

/// Compile with explicit lowering context (for replay protection, chain_id, etc.)
pub fn compile_program_with_context(program: &Program, ctx: LowerCtx) -> Result<Vec<u8>, X3Error> {
    // AST → X3IR
    let ir = lower_program(program, ctx)?;

    // X3IR → Bytecode
    let bytecode = emit_x3ir(&ir)?;

    // Verify bytecode is valid (basic sanity checks)
    verify_bytecode(&bytecode)?;

    Ok(bytecode)
}

/// Get the IR without emitting to bytecode (useful for analysis/optimization)
pub fn compile_to_ir(program: &Program) -> Result<X3IR, X3Error> {
    lower_program(program, LowerCtx::new())
}

/// Verify bytecode is properly formed (basic checks)
fn verify_bytecode(bytecode: &[u8]) -> Result<(), X3Error> {
    if bytecode.is_empty() {
        return Err(X3Error::CodegenError {
            message: "bytecode is empty".to_string(),
            span: None,
        });
    }

    // Check version byte
    if bytecode[0] != 0x01 {
        return Err(X3Error::CodegenError {
            message: format!("invalid bytecode version: {}", bytecode[0]),
            span: None,
        });
    }

    // Check alignment
    if bytecode.len() % 4 != 0 {
        return Err(X3Error::CodegenError {
            message: "bytecode not 4-byte aligned".to_string(),
            span: None,
        });
    }

    Ok(())
}
