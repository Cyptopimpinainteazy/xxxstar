pub mod emitter;
pub mod intent_emit;
pub mod ir;
pub mod lowering;
pub mod parser;
pub mod regalloc;
pub mod semantic;
pub mod spec {
    pub mod opcodes {
        include!("../../spec/opcodes.rs");
    }
}

use emitter::emit_x3ir;
use lowering::{lower_program, LowerCtx};
use parser::parse_source;
use semantic::verify_with_defaults as verify_semantics;
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

/// Parse, lower, and run the semantic verifier.
///
/// Returns the IR plus the list of semantic errors (empty list = clean).
/// Use this for the `check` CLI command: it stops at the semantic pass
/// without emitting bytecode, exposing every production-safety problem
/// in a single shot.
pub fn check_source(source: &str) -> Result<(Program, crate::ir::X3IR, Vec<X3Error>), X3Error> {
    let program = parse_source(source)?;
    let ir = compile_to_ir(&program)?;
    match verify_semantics(&ir) {
        Ok(()) => Ok((program, ir, Vec::new())),
        Err(errs) => Ok((program, ir, errs)),
    }
}

/// Run the semantic verifier against an X3IR program.
pub fn check_ir(ir: &crate::ir::X3IR) -> Result<(), Vec<X3Error>> {
    verify_semantics(ir)
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

/// Re-export the cross-chain intent adapter boundary.
///
/// `x3-lang` does not depend on the cross-chain intent crate (no
/// dependency cycle). Instead the language compiler builds an
/// `IntentSpecDraft` (a JSON-serializable value) at the compiler
/// boundary, and the main workspace's
/// `x3-crosschain-intent::adapter::intent_spec_to_crosschain_intent`
/// is the single canonical consumer that converts the draft into
/// a fully-validated `CrossChainIntent` and stamps the canonical
/// hash.
pub use intent_emit::{IntentSpecDraft, SourceConstraint};

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
