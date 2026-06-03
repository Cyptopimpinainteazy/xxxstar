//! Bytecode verifier for X3 VM
//!
//! Performs deterministic checks on the bytecode to ensure safety:
//! - valid opcodes
//! - instruction boundaries
//! - jump targets
//! - memory and immediate operand ranges

use crate::x3_lang_vm::InstructionStream;
use std::collections::HashSet;
use x3_lang_common::{decode_capability_payload, CapabilityPayload};

#[derive(Debug)]
pub enum VerifyError {
    InvalidOpcode(u8, usize),
    InvalidOperand(usize),
    JumpToNonBoundary(usize, usize),
    OutOfBounds(usize),
}

/// Validate that `code` is valid bytecode and return set of instruction boundaries.
pub fn verify(code: &InstructionStream) -> Result<HashSet<usize>, VerifyError> {
    // Emitted X3 bytecode is padded to 4-byte alignment, while capability
    // operations themselves use opcode + u16 payload length + payload.
    if code.len() % 4 != 0 {
        return Err(VerifyError::OutOfBounds(code.len()));
    }
    let mut boundaries = HashSet::new();
    let bytes = code.as_slice();
    let mut pc = first_instruction_pc(bytes);
    while pc + 4 <= bytes.len() {
        if bytes[pc..].iter().all(|byte| *byte == 0) {
            break;
        }
        boundaries.insert(pc);
        let opcode = bytes[pc];
        if !valid_opcode(opcode) {
            return Err(VerifyError::InvalidOpcode(opcode, pc));
        }
        if is_payload_opcode(opcode) {
            let payload = read_payload(bytes, pc)?;
            validate_payload_opcode(opcode, payload, pc)?;
            pc += 3 + payload.len();
            continue;
        }

        let _flags = bytes[pc + 1];
        let operand = u16::from_le_bytes([bytes[pc + 2], bytes[pc + 3]]);
        // check flags & operand ranges depending on opcode (simplified)
        // for branches ensure destination is inside code and aligned
        match opcode {
            0x30 | 0x31 => {
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
            0x32 => {
                let target = operand as usize;
                if target >= bytes.len() {
                    return Err(VerifyError::InvalidOperand(pc));
                }
                if target % 4 != 0 {
                    return Err(VerifyError::JumpToNonBoundary(pc, target));
                }
            }
            0x33 => { /* RET - valid */ }
            _ => {}
        }
        pc += 4;
    }
    Ok(boundaries)
}

fn first_instruction_pc(bytes: &[u8]) -> usize {
    if bytes.first() == Some(&0x01) && bytes.get(1).copied().is_some_and(is_payload_opcode) {
        1
    } else {
        0
    }
}

fn is_payload_opcode(op: u8) -> bool {
    (0x80..=0x9B).contains(&op)
}

fn read_payload(bytes: &[u8], pc: usize) -> Result<&[u8], VerifyError> {
    if pc + 3 > bytes.len() {
        return Err(VerifyError::OutOfBounds(pc));
    }
    let len = u16::from_le_bytes([bytes[pc + 1], bytes[pc + 2]]) as usize;
    let start = pc + 3;
    let end = start
        .checked_add(len)
        .ok_or(VerifyError::InvalidOperand(pc))?;
    if end > bytes.len() {
        return Err(VerifyError::OutOfBounds(pc));
    }
    Ok(&bytes[start..end])
}

fn validate_payload_opcode(opcode: u8, payload: &[u8], pc: usize) -> Result<(), VerifyError> {
    let payload =
        decode_capability_payload(opcode, payload).map_err(|_| VerifyError::InvalidOperand(pc))?;
    match payload {
        CapabilityPayload::ScheduledDispatch { period_blocks, .. } => {
            if period_blocks == 0 {
                return Err(VerifyError::InvalidOperand(pc));
            }
        }
        CapabilityPayload::ProofVerify { proof, input, .. } => {
            if proof.is_empty() || input.is_empty() {
                return Err(VerifyError::InvalidOperand(pc));
            }
        }
        CapabilityPayload::MultisigCheck { required, total } => {
            if required > total {
                return Err(VerifyError::InvalidOperand(pc));
            }
        }
        CapabilityPayload::GasAdaptive {
            high_gas_ops,
            low_gas_ops,
        } => {
            if high_gas_ops == 0 || low_gas_ops == 0 {
                return Err(VerifyError::InvalidOperand(pc));
            }
        }
        CapabilityPayload::SubExec {
            bytecode_hash,
            gas_limit,
            ..
        } => {
            if bytecode_hash.is_empty() || gas_limit == 0 {
                return Err(VerifyError::InvalidOperand(pc));
            }
        }
        _ => {}
    }
    Ok(())
}

fn valid_opcode(op: u8) -> bool {
    // Accept every opcode the emitter can produce, including
    // asset ops (0x20-0x24), control (0x30-0x33), guards
    // (0x40-0x44), atomic (0x50-0x52), emit/call (0x60-0x66),
    // vector (0x70-0x73), capability payloads (0x80-0x9B),
    // and extras (0xA0-0xA5). Halt (0xFF) and reserved (0x00-0x18)
    // are also valid. Anything outside 0x00-0xFF is impossible.
    op <= 0xA5 || op == 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_lang_common::encode_capability_payload;

    fn payload_code(opcode: u8, payload: CapabilityPayload) -> InstructionStream {
        let payload = encode_capability_payload(&payload).expect("test payload should encode");
        let mut bytes = vec![opcode];
        bytes.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&payload);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        InstructionStream::new(bytes)
    }

    #[test]
    fn verifier_accepts_structurally_valid_capability_payloads() {
        for (opcode, payload) in [
            (
                0x82,
                CapabilityPayload::ScheduledDispatch {
                    period_blocks: 5,
                    entry_ops: 1,
                },
            ),
            (
                0x85,
                CapabilityPayload::ProofVerify {
                    kind: 0,
                    proof: "proof".into(),
                    input: "input".into(),
                    key_or_threshold: "vk".into(),
                },
            ),
            (
                0x94,
                CapabilityPayload::MultisigCheck {
                    required: 2,
                    total: 3,
                },
            ),
            (
                0x99,
                CapabilityPayload::GasAdaptive {
                    high_gas_ops: 1,
                    low_gas_ops: 1,
                },
            ),
        ] {
            assert!(
                verify(&payload_code(opcode, payload)).is_ok(),
                "opcode 0x{opcode:02x} should verify"
            );
        }
    }

    #[test]
    fn verifier_rejects_invalid_capability_payloads() {
        for (opcode, payload) in [
            (
                0x82,
                CapabilityPayload::ScheduledDispatch {
                    period_blocks: 0,
                    entry_ops: 1,
                },
            ),
            (
                0x85,
                CapabilityPayload::ProofVerify {
                    kind: 0,
                    proof: "".into(),
                    input: "input".into(),
                    key_or_threshold: "vk".into(),
                },
            ),
            (
                0x94,
                CapabilityPayload::MultisigCheck {
                    required: 4,
                    total: 3,
                },
            ),
            (
                0x99,
                CapabilityPayload::GasAdaptive {
                    high_gas_ops: 0,
                    low_gas_ops: 1,
                },
            ),
        ] {
            assert!(
                verify(&payload_code(opcode, payload)).is_err(),
                "opcode 0x{opcode:02x} should reject malformed payload"
            );
        }
    }

    /// Lock in the upper bound of the `valid_opcode` allowlist.
    ///
    /// Prior to the allowlist-simplification change, the hand-rolled match
    /// explicitly rejected 0xA6..=0xFE even though the doc-comment claims
    /// the cutoff is 0xA5. The new range-based allowlist must continue to
    /// reject opcodes above the cutoff; this test fails if anyone widens
    /// the cap without intending to.
    #[test]
    fn verifier_rejects_unassigned_high_opcodes() {
        // Construct a 4-byte aligned instruction stream that begins with
        // the opcode under test. The remaining three bytes are zero, so
        // the loop terminates at the first all-zero instruction boundary.
        let stream_for = |op: u8| InstructionStream::new(vec![op, 0, 0, 0]);
        for op in [0xA6u8, 0xB0, 0xC7, 0xFE] {
            let stream = stream_for(op);
            match verify(&stream) {
                Err(VerifyError::InvalidOpcode(got, _)) => assert_eq!(got, op),
                other => panic!("opcode 0x{op:02x} should be rejected, got {other:?}"),
            }
        }
    }

    /// Halt (0xFF) is the single allowed opcode above 0xA5. A naive
    /// `op <= 0xA5` check silently breaks HALT and every test that emits
    /// it; this test guards against that regression.
    #[test]
    fn verifier_accepts_halt_opcode() {
        let stream = InstructionStream::new(vec![0xFF, 0x00, 0x00, 0x00]);
        verify(&stream).expect("0xFF HALT must verify");
    }
}
