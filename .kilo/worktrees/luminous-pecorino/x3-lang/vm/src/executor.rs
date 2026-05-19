//! X3 VM executor - fetch-decode-execute loop and handlers for opcodes.

use crate::x3_lang_vm::{InstructionStream, Register, VM};
use x3_lang_common::BinOp;

pub type ExecResult<T> = Result<T, ExecError>;

#[derive(Debug)]
pub enum ExecError {
    OutOfGas,
    InvalidOpcode(u8),
    InvalidOperand,
    MemoryOutOfBounds,
    Panic(String),
}

pub type GasCost = u128;

/// Execute the VM until halt or out of gas.
pub fn execute(vm: &mut VM) -> ExecResult<()> {
    loop {
        if vm.state.pc >= vm.code.len() {
            return Ok(());
        }
        // Fetch instruction
        let opcode = vm.code.as_slice()[vm.state.pc];
        let flags = vm
            .code
            .as_slice()
            .get(vm.state.pc + 1)
            .copied()
            .unwrap_or(0);
        let operand = read_u16_le(vm.code.as_slice(), vm.state.pc + 2).unwrap_or(0);
        let pc_next = vm.state.pc + 4;

        // Gas accounting (simplified: 1 unit per instruction)
        let cost = gas_cost_for_opcode(opcode);
        if vm.state.gas < cost {
            return Err(ExecError::OutOfGas);
        }
        vm.state.gas -= cost;

        match opcode {
            0x01 => {
                // ADD_RRR - REG-REG-REG: operand encodes registers
                // flags: REG3
                let (ra, rb, rc) = decode_regtriplet(operand);
                vm.state.registers[ra as usize] =
                    vm.state.registers[rb as usize].wrapping_add(vm.state.registers[rc as usize]);
            }
            0x02 => {
                // SUB_RRR
                let (ra, rb, rc) = decode_regtriplet(operand);
                vm.state.registers[ra as usize] =
                    vm.state.registers[rb as usize].wrapping_sub(vm.state.registers[rc as usize]);
            }
            0x10 => {
                // LOAD_RAI: R[a] = mem[R[b] + imm16]
                let (ra, rb, imm) = decode_reg_reg_imm(operand);
                let addr = (vm.state.registers[rb as usize] as usize).wrapping_add(imm as usize);
                if addr + 16 > vm.state.memory.len() {
                    return Err(ExecError::MemoryOutOfBounds);
                }
                // Read 16 bytes and produce u128 (little endian)
                let mut val = 0u128;
                for i in 0..16 {
                    val |= (vm.state.memory[addr + i] as u128) << (i * 8);
                }
                vm.state.registers[ra as usize] = val;
            }
            0x11 => {
                // STORE_RAI
                let (ra, rb, imm) = decode_reg_reg_imm(operand);
                let addr = (vm.state.registers[rb as usize] as usize).wrapping_add(imm as usize);
                if addr + 16 > vm.state.memory.len() {
                    return Err(ExecError::MemoryOutOfBounds);
                }
                let val = vm.state.registers[ra as usize];
                for i in 0..16 {
                    vm.state.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                }
            }
            0x20 => {
                // PUSH_IMM - operand is immediate 16-bit sign-extended
                let imm16 = operand as i16 as i128 as u128;
                vm.state.registers[0] = imm16; // use R0 as push target then increment SP
                vm.state.sp = vm.state.sp.wrapping_add(1);
            }
            0x21 => {
                // POP dest
                // operand holds dest register
                let dest = (operand & 0xFF) as usize;
                if vm.state.sp == 0 {
                    return Err(ExecError::Panic("stack underflow".to_string()));
                }
                vm.state.sp -= 1;
                vm.state.registers[dest] = vm.state.registers[0];
            }
            0x30 => {
                // JMP offset
                let rel = operand as i16;
                let dst = (pc_next as i32).wrapping_add(rel as i32) as usize;
                vm.state.pc = dst;
                continue;
            }
            0x31 => {
                // JZ - jump if top-of-stack zero
                let dest = operand as i16;
                let top = vm.state.registers[0];
                if top == 0 {
                    let dst = (pc_next as i32).wrapping_add(dest as i32) as usize;
                    vm.state.pc = dst;
                    continue;
                }
            }
            0x32 => {
                // CALL - push return and jump
                let addr = operand as usize;
                // Use memory as call stack for return addresses, simplify
                let retpc = pc_next as u128;
                // push
                vm.state.registers[1] = retpc; // R1 used as return register
                vm.state.pc = addr;
                continue;
            }
            0x33 => {
                // RET
                let retpc = vm.state.registers[1] as usize;
                vm.state.pc = retpc;
                continue;
            }
            0x40 => {
                // CRYPTO_SHA256 - read 16 bytes from R[b]
                let (ra, rb, _) = decode_reg_reg_imm(operand);
                // Simplified sha by summing bytes (placeholder); replace with real crypto op in production
                let val = vm.state.registers[rb as usize];
                // sum bytes
                let mut sum = 0u128;
                for i in 0..16 {
                    sum = sum.wrapping_add((val >> (i * 8)) & 0xFF);
                }
                vm.state.registers[ra as usize] = sum; // placeholder deterministic op
            }
            0x50 => { // ATOMIC_BEGIN - begin atomic window
                 // placeholder no-op; adapter will manage provisional states
            }
            0x51 => { // ATOMIC_COMMIT - commit
                 // placeholder
            }
            0x52 => { // ATOMIC_ROLLBACK - rollback
                 // placeholder
            }
            0x60 => { // EVM_CALL - Bridge to EVM
                 // placeholder deterministic call semantics: no external network executed here
            }
            0x61 => { // SVM_CALL - Bridge to SVM
            }
            0x70 => {
                // SIMD_ADD_VV - Vector Add placeholder
                let (va, vb, vc) = decode_regtriplet(operand);
                let mut out = [0u8; 16];
                for i in 0..16 {
                    out[i] = vm.state.vector_registers[vb as usize][i]
                        .wrapping_add(vm.state.vector_registers[vc as usize][i]);
                }
                vm.state.vector_registers[va as usize] = out;
            }
            0x00 => { // NOP
            }
            0xFF => {
                // HALT
                return Ok(());
            }
            other => return Err(ExecError::InvalidOpcode(other)),
        }

        vm.state.pc = pc_next;
    }
}

fn gas_cost_for_opcode(opcode: u8) -> u128 {
    match opcode {
        0x01 | 0x02 => 1,
        0x10 | 0x11 => 5,
        0x20 | 0x21 => 1,
        0x30 | 0x31 => 2,
        0x32 | 0x33 => 5,
        0x40 => 10,
        0x50 | 0x51 => 250,
        0x60 | 0x61 => 100,
        0x70 => 2,
        0xFF => 0,
        _ => 1,
    }
}

fn read_u16_le(bytes: &[u8], idx: usize) -> Option<u16> {
    if idx + 1 >= bytes.len() {
        return None;
    }
    Some((bytes[idx] as u16) | ((bytes[idx + 1] as u16) << 8))
}

fn decode_regtriplet(operand: u16) -> (u8, u8, u8) {
    // operand packs three 5-bit registers: r0[0..4], r1[5..9], r2[10..14]
    let ra = (operand & 0x1F) as u8;
    let rb = ((operand >> 5) & 0x1F) as u8;
    let rc = ((operand >> 10) & 0x1F) as u8;
    (ra, rb, rc)
}

fn decode_reg_reg_imm(operand: u16) -> (u8, u8, u16) {
    // operand: low 5 bits ra, next 5 bits rb, top 6 bits imm6 - extend
    let ra = (operand & 0x1F) as u8;
    let rb = ((operand >> 5) & 0x1F) as u8;
    let imm = (operand >> 10) & 0x3F;
    (ra, rb, imm)
}
