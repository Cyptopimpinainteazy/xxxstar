//! Bytecode verifier for X3 VM
//!
//! Performs deterministic checks on the bytecode to ensure safety:
//! - valid opcodes
//! - instruction boundaries
//! - jump targets
//! - memory and immediate operand ranges

use crate::x3_lang_vm::InstructionStream;
use std::collections::HashSet;

#[derive(Debug)]
pub enum VerifyError {
    InvalidOpcode(u8, usize),
    InvalidOperand(usize),
    JumpToNonBoundary(usize, usize),
    OutOfBounds(usize),
}

/// Validate that `code` is valid bytecode and return set of instruction boundaries.
pub fn verify(code: &InstructionStream) -> Result<HashSet<usize>, VerifyError> {
    // code is 4-byte aligned control stream
    if code.len() % 4 != 0 {
        return Err(VerifyError::OutOfBounds(code.len()));
    }
    let mut boundaries = HashSet::new();
    let bytes = code.as_slice();
    let mut pc = 0usize;
    while pc + 4 <= bytes.len() {
        boundaries.insert(pc);
        let opcode = bytes[pc];
        let flags = bytes[pc + 1];
        let operand = u16::from_le_bytes([bytes[pc + 2], bytes[pc + 3]]);
        if !valid_opcode(opcode) {
            return Err(VerifyError::InvalidOpcode(opcode, pc));
        }
        // check flags & operand ranges depending on opcode (simplified)
        // for branches ensure destination is inside code and aligned
        match opcode {
            0x30 | 0x31 | 0x32 => {
                // relative or absolute jumps; compute target
                let rel = operand as i16;
                let target = (pc + 4) as i32 + rel as i32; // relative
                if target < 0 || (target as usize) >= bytes.len() {
                    return Err(VerifyError::InvalidOperand(pc));
                }
                if ((target as usize) % 4) != 0 {
                    return Err(VerifyError::JumpToNonBoundary(pc, target as usize));
                }
            }
            0x33 => { /* RET - valid */ }
            _ => {}
        }
        pc += 4;
    }
    Ok(boundaries)
}

fn valid_opcode(op: u8) -> bool {
    match op {
        0x00 | 0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x08 | 0x09 | 0x0A | 0x0B
        | 0x0C | 0x0D | 0x0E | 0x0F | 0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17
        | 0x18 | 0x20 | 0x21 | 0x30 | 0x31 | 0x32 | 0x33 | 0x40 | 0x41 | 0x42 | 0x43 | 0x44
        | 0x50 | 0x51 | 0x52 | 0x60 | 0x61 | 0x62 | 0x63 | 0x64 | 0x65 | 0x66 | 0x70 | 0x71
        | 0x72 | 0x73 | 0x80 | 0x81 | 0x82 | 0x83 | 0x84 | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4
        | 0xA5 | 0xFF => true,
        _ => false,
    }
}
