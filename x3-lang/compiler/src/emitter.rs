//! Bytecode emitter: converts lowered instructions into aligned binary format.

use crate::lowering::LoweredInstr;

pub fn emit(instrs: &[LoweredInstr]) -> Vec<u8> {
    let mut out = Vec::new();
    for instr in instrs {
        out.push(instr.opcode);
        out.push(instr.flags);
        out.extend_from_slice(&instr.operand.to_le_bytes());
    }
    // pad to 4-bytes already aligned
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}
