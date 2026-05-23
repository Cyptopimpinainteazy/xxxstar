pub mod lowering;
pub mod emitter;
pub mod regalloc;

use lowering::lower_program;
use emitter::emit;
use x3_lang_vm::verifier;
use x3_lang_vm::InstructionStream;

use x3_lang_ast::ast::Program;

pub fn compile_program(program: &Program) -> Result<Vec<u8>, String> {
    let lowered = lower_program(program)?;
    let lowered = regalloc::allocate(&lowered);
    let bytes = emit(&lowered);
    // verify bytes
    let stream = InstructionStream::new(bytes.clone());
    verifier::verify(&stream).map_err(|e| format!("verify failed: {:?}", e))?;
    Ok(bytes)
}
