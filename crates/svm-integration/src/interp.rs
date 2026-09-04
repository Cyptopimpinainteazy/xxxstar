//! Minimal eBPF Interpreter (no_std)
//!
//! A real, register-level sBPF interpreter that does not depend on std.
//! Conforms to the Solana eBPF (sBPF v1) instruction set.
//!
//! # Supported instructions
//! - ALU64: ADD, SUB, MUL, DIV, OR, AND, LSH, RSH, NEG, MOD, XOR, MOV, ARSH
//! - ALU32: same operations truncated to 32 bits
//! - JMP: JA, JEQ, JGT, JGE, JSET, JNE, JSGT, JSGE, JLT, JLE, JSLT, JSLE
//! - CALL: syscall dispatch (unknown calls return 0)
//! - EXIT: halt and return r0
//! - LD/LDX/ST/STX: byte / half / word / doubleword memory ops
//! - LD_DW (0x18): 64-bit immediate load (two-instruction wide)
//!
//! # ELF handling
//! If the payload starts with the 4-byte ELF magic (`\x7fELF`), the
//! interpreter locates the `.text` section and executes it.  Otherwise
//! the bytes are treated as a raw instruction stream.

use crate::{SvmConfig, SvmError, SvmExecutionResult, SvmResult};
use ed25519_dalek::VerifyingKey;
use sp_std::vec;
use sp_std::vec::Vec;

// ---------------------------------------------------------------------------
// Opcode encoding constants
// ---------------------------------------------------------------------------

// Class (low 3 bits)
const _CLS_LD: u8 = 0x00;
const CLS_LDX: u8 = 0x01;
const CLS_ST: u8 = 0x02;
const CLS_STX: u8 = 0x03;
const CLS_ALU32: u8 = 0x04;
const CLS_JMP: u8 = 0x05;
const CLS_JMP32: u8 = 0x06;
const CLS_ALU64: u8 = 0x07;

// Source mode (bit 3)
const _SRC_IMM: u8 = 0x00;
const SRC_REG: u8 = 0x08;

// ALU operation (high nibble >> 4)
const ALU_ADD: u8 = 0x0;
const ALU_SUB: u8 = 0x1;
const ALU_MUL: u8 = 0x2;
const ALU_DIV: u8 = 0x3;
const ALU_OR: u8 = 0x4;
const ALU_AND: u8 = 0x5;
const ALU_LSH: u8 = 0x6;
const ALU_RSH: u8 = 0x7;
const ALU_NEG: u8 = 0x8;
const ALU_MOD: u8 = 0x9;
const ALU_XOR: u8 = 0xa;
const ALU_MOV: u8 = 0xb;
const ALU_ARSH: u8 = 0xc;

// JMP operation (high nibble >> 4)
const JMP_JA: u8 = 0x0;
const JMP_JEQ: u8 = 0x1;
const JMP_JGT: u8 = 0x2;
const JMP_JGE: u8 = 0x3;
const JMP_JSET: u8 = 0x4;
const JMP_JNE: u8 = 0x5;
const JMP_JSGT: u8 = 0x6;
const JMP_JSGE: u8 = 0x7;
const JMP_CALL: u8 = 0x8;
const JMP_EXIT: u8 = 0x9;
const JMP_JLT: u8 = 0xa;
const JMP_JLE: u8 = 0xb;
const JMP_JSLT: u8 = 0xc;
const JMP_JSLE: u8 = 0xd;

// Load size (bits 3-4)
const SZ_W: u8 = 0x00;
const SZ_H: u8 = 0x08;
const SZ_B: u8 = 0x10;
const SZ_DW: u8 = 0x18;

// Special opcodes
const OP_LD_DW: u8 = 0x18; // 64-bit immediate load (two-insn)
const OP_CALL: u8 = 0x85;
const OP_EXIT: u8 = 0x95;

// Number of registers
const NREG: usize = 11; // r0..r10
                        // Stack size in bytes
const STACK_SIZE: usize = 4096;
// Maximum instruction count before aborting (compute limit)
const MAX_INSN_FUEL: u64 = 1_000_000;

// ---------------------------------------------------------------------------
// Minimalist instruction representation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Insn {
    opcode: u8,
    dst: u8, // destination register (0-10)
    src: u8, // source register (0-10)
    off: i16,
    imm: i32,
}

fn decode(raw: &[u8; 8]) -> Insn {
    Insn {
        opcode: raw[0],
        dst: raw[1] & 0x0f,
        src: (raw[1] >> 4) & 0x0f,
        off: i16::from_le_bytes([raw[2], raw[3]]),
        imm: i32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]),
    }
}

// ---------------------------------------------------------------------------
// ELF64 minimal parser – finds the .text section in an sBPF ELF
// ---------------------------------------------------------------------------

fn elf_find_text(data: &[u8]) -> Option<&[u8]> {
    // Check ELF magic + class=64-bit
    if data.len() < 64 {
        return None;
    }
    if &data[0..4] != b"\x7fELF" {
        return None;
    }
    if data[4] != 2 {
        return None;
    } // ELF64

    let e_shoff = u64::from_le_bytes(data.get(40..48)?.try_into().ok()?) as usize;
    let e_shentsize = u16::from_le_bytes(data.get(58..60)?.try_into().ok()?) as usize;
    let e_shnum = u16::from_le_bytes(data.get(60..62)?.try_into().ok()?) as usize;
    let e_shstrndx = u16::from_le_bytes(data.get(62..64)?.try_into().ok()?) as usize;

    if e_shentsize == 0 || e_shnum == 0 {
        return None;
    }

    // Locate string table section header
    let shstr_off = e_shoff.checked_add(e_shstrndx.checked_mul(e_shentsize)?)?;
    let shstr_sh = data.get(shstr_off..shstr_off + e_shentsize)?;
    let str_offset = u64::from_le_bytes(shstr_sh.get(24..32)?.try_into().ok()?) as usize;
    let str_size = u64::from_le_bytes(shstr_sh.get(32..40)?.try_into().ok()?) as usize;
    let strtab = data.get(str_offset..str_offset + str_size)?;

    // Scan all section headers looking for ".text"
    for i in 0..e_shnum {
        let sh_off = e_shoff.checked_add(i.checked_mul(e_shentsize)?)?;
        let sh = data.get(sh_off..sh_off + e_shentsize)?;
        let name_idx = u32::from_le_bytes(sh.get(0..4)?.try_into().ok()?) as usize;
        // Find null-terminated name in strtab
        let name_bytes = strtab.get(name_idx..)?;
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = &name_bytes[..name_end];
        if name == b".text" {
            let sec_off = u64::from_le_bytes(sh.get(24..32)?.try_into().ok()?) as usize;
            let sec_size = u64::from_le_bytes(sh.get(32..40)?.try_into().ok()?) as usize;
            return data.get(sec_off..sec_off + sec_size);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Interpreter state
// ---------------------------------------------------------------------------

struct Vm<'a> {
    regs: [u64; NREG],
    stack: [u8; STACK_SIZE],
    /// Writable copy of the input data.  Solana programs can modify account
    /// data in place; capturing those writes here ensures they are reflected
    /// in the execution result.
    heap: Vec<u8>,
    fuel: u64,
    insns: &'a [u8], // raw instruction bytes
    pc: usize,       // instruction index
    /// Execution context for sysvar data (slot, timestamp, etc.)
    config: &'a SvmConfig,
}

impl<'a> Vm<'a> {
    fn new(insns: &'a [u8], input: &[u8], fuel: u64, config: &'a SvmConfig) -> Self {
        let mut regs = [0u64; NREG];
        // r1 = pointer to input data (mapped at offset STACK_SIZE in our memory model)
        regs[1] = STACK_SIZE as u64;
        // r2 = length of input
        regs[2] = input.len() as u64;
        // r10 = stack top (stack grows downward from STACK_SIZE)
        regs[10] = STACK_SIZE as u64;
        Vm {
            regs,
            stack: [0u8; STACK_SIZE],
            heap: input.to_vec(),
            fuel,
            insns,
            pc: 0,
            config,
        }
    }

    #[inline(always)]
    fn reg(&self, r: u8) -> u64 {
        self.regs[r.min(10) as usize]
    }

    #[inline(always)]
    fn set_reg(&mut self, r: u8, v: u64) {
        if (r as usize) < NREG {
            self.regs[r as usize] = v;
        }
    }

    #[inline(always)]
    fn charge_fuel(&mut self, cost: u64) -> Result<(), SvmError> {
        if self.fuel < cost {
            return Err(SvmError::OutOfComputeUnits);
        }
        self.fuel -= cost;
        Ok(())
    }

    #[inline(always)]
    fn is_on_ed25519_curve(pubkey: &[u8; 32]) -> bool {
        VerifyingKey::from_bytes(pubkey).is_ok()
    }

    /// Read `size` bytes from memory.
    /// Memory map: address 0..STACK_SIZE = our local stack,
    ///             address STACK_SIZE..(STACK_SIZE + heap.len()) = writable input/account data.
    /// Out-of-bounds access returns an error (enforces memory safety).
    fn mem_read(&self, addr: u64, size: usize) -> Result<u64, SvmError> {
        let addr = addr as usize;
        if size == 0 || size > 8 {
            return Err(SvmError::ExecutionFailed);
        }
        if addr < STACK_SIZE {
            let end = addr.checked_add(size).ok_or(SvmError::ExecutionFailed)?;
            if end > STACK_SIZE {
                return Err(SvmError::ExecutionFailed);
            }
            let mut out = [0u8; 8];
            out[..size].copy_from_slice(&self.stack[addr..end]);
            Ok(u64::from_le_bytes(out))
        } else {
            let heap_off = addr.wrapping_sub(STACK_SIZE);
            let end = heap_off
                .checked_add(size)
                .ok_or(SvmError::ExecutionFailed)?;
            if end > self.heap.len() {
                return Err(SvmError::ExecutionFailed);
            }
            let mut out = [0u8; 8];
            out[..size].copy_from_slice(&self.heap[heap_off..end]);
            Ok(u64::from_le_bytes(out))
        }
    }

    /// Write `size` bytes to memory.
    /// Writes are allowed to both the stack and the heap (input data) region.
    /// Out-of-bounds writes return an error.
    fn mem_write(&mut self, addr: u64, value: u64, size: usize) -> Result<(), SvmError> {
        let addr = addr as usize;
        if size == 0 || size > 8 {
            return Err(SvmError::ExecutionFailed);
        }
        let bytes = value.to_le_bytes();
        if addr < STACK_SIZE {
            let end = addr.checked_add(size).ok_or(SvmError::ExecutionFailed)?;
            if end > STACK_SIZE {
                return Err(SvmError::ExecutionFailed);
            }
            self.stack[addr..end].copy_from_slice(&bytes[..size]);
            Ok(())
        } else {
            let heap_off = addr.wrapping_sub(STACK_SIZE);
            let end = heap_off
                .checked_add(size)
                .ok_or(SvmError::ExecutionFailed)?;
            if end > self.heap.len() {
                return Err(SvmError::ExecutionFailed);
            }
            self.heap[heap_off..end].copy_from_slice(&bytes[..size]);
            Ok(())
        }
    }

    /// Read an arbitrary byte slice from the VM memory map.
    fn mem_read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, SvmError> {
        let addr = addr as usize;
        if len == 0 {
            return Ok(Vec::new());
        }
        let end = addr.checked_add(len).ok_or(SvmError::ExecutionFailed)?;

        if addr < STACK_SIZE {
            if end > STACK_SIZE {
                return Err(SvmError::ExecutionFailed);
            }
            Ok(self.stack[addr..end].to_vec())
        } else {
            let heap_off = addr.wrapping_sub(STACK_SIZE);
            let heap_end = heap_off.checked_add(len).ok_or(SvmError::ExecutionFailed)?;
            if heap_end > self.heap.len() {
                return Err(SvmError::ExecutionFailed);
            }
            Ok(self.heap[heap_off..heap_end].to_vec())
        }
    }

    /// Write an arbitrary byte slice to the VM memory map.
    fn mem_write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<(), SvmError> {
        let addr = addr as usize;
        if data.is_empty() {
            return Ok(());
        }
        let end = addr
            .checked_add(data.len())
            .ok_or(SvmError::ExecutionFailed)?;

        if addr < STACK_SIZE {
            if end > STACK_SIZE {
                return Err(SvmError::ExecutionFailed);
            }
            self.stack[addr..end].copy_from_slice(data);
            Ok(())
        } else {
            let heap_off = addr.wrapping_sub(STACK_SIZE);
            let heap_end = heap_off
                .checked_add(data.len())
                .ok_or(SvmError::ExecutionFailed)?;
            if heap_end > self.heap.len() {
                return Err(SvmError::ExecutionFailed);
            }
            self.heap[heap_off..heap_end].copy_from_slice(data);
            Ok(())
        }
    }

    fn run(&mut self) -> Result<u64, SvmError> {
        let num_insns = self.insns.len() / 8;

        while self.pc < num_insns {
            if self.fuel == 0 {
                return Err(SvmError::OutOfComputeUnits);
            }
            self.fuel -= 1;

            let raw: &[u8; 8] = self.insns[self.pc * 8..(self.pc + 1) * 8]
                .try_into()
                .map_err(|_| SvmError::InvalidPayload)?;
            let i = decode(raw);

            match i.opcode {
                // --------------------------------------------------------
                // EXIT
                // --------------------------------------------------------
                OP_EXIT => return Ok(self.regs[0]),

                // --------------------------------------------------------
                // 64-bit immediate load (two-instruction pseudo)
                // --------------------------------------------------------
                OP_LD_DW => {
                    let lo = i.imm as u64;
                    // Next instruction holds the high 32 bits
                    let hi = if self.pc + 1 < num_insns {
                        let next_raw: &[u8; 8] = self.insns[(self.pc + 1) * 8..(self.pc + 2) * 8]
                            .try_into()
                            .map_err(|_| SvmError::InvalidPayload)?;
                        i32::from_le_bytes([next_raw[4], next_raw[5], next_raw[6], next_raw[7]])
                            as u64
                    } else {
                        0
                    };
                    self.set_reg(i.dst, (hi << 32) | (lo & 0xffff_ffff));
                    self.pc += 2; // skip extension word
                    continue;
                }

                // --------------------------------------------------------
                // CALL – dispatch on imm (helper id); unknown → r0 = 0
                // --------------------------------------------------------
                OP_CALL => {
                    let retval = self.dispatch_syscall(i.imm as u32)?;
                    self.regs[0] = retval;
                    // callee-saved r6-r9 preserved, clobber r1-r5
                    self.regs[1] = 0;
                    self.regs[2] = 0;
                    self.regs[3] = 0;
                    self.regs[4] = 0;
                    self.regs[5] = 0;
                }

                // --------------------------------------------------------
                // ALU64
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_ALU64 => {
                    let src_val = if (op & SRC_REG) != 0 {
                        self.reg(i.src)
                    } else {
                        i.imm as i64 as u64
                    };
                    let dst_val = self.reg(i.dst);
                    let result = alu64(op >> 4, dst_val, src_val)?;
                    self.set_reg(i.dst, result);
                }

                // --------------------------------------------------------
                // ALU32 – result truncated to 32 bits, zero-extended
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_ALU32 => {
                    let src_val = if (op & SRC_REG) != 0 {
                        self.reg(i.src)
                    } else {
                        i.imm as u64
                    };
                    let dst_val = self.reg(i.dst) & 0xffff_ffff;
                    let result32 = alu32(op >> 4, dst_val as u32, src_val as u32)? as u64;
                    self.set_reg(i.dst, result32); // zero-extended to 64
                }

                // --------------------------------------------------------
                // JMP (64-bit comparisons)
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_JMP => {
                    let src_val = if (op & SRC_REG) != 0 {
                        self.reg(i.src)
                    } else {
                        i.imm as i64 as u64
                    };
                    let dst_val = self.reg(i.dst);
                    let take = match op >> 4 {
                        JMP_JA => true,
                        JMP_JEQ => dst_val == src_val,
                        JMP_JGT => dst_val > src_val,
                        JMP_JGE => dst_val >= src_val,
                        JMP_JSET => (dst_val & src_val) != 0,
                        JMP_JNE => dst_val != src_val,
                        JMP_JSGT => (dst_val as i64) > (src_val as i64),
                        JMP_JSGE => (dst_val as i64) >= (src_val as i64),
                        JMP_JLT => dst_val < src_val,
                        JMP_JLE => dst_val <= src_val,
                        JMP_JSLT => (dst_val as i64) < (src_val as i64),
                        JMP_JSLE => (dst_val as i64) <= (src_val as i64),
                        JMP_EXIT => {
                            return Ok(self.regs[0]);
                        }
                        JMP_CALL => {
                            let retval = self.dispatch_syscall(i.imm as u32)?;
                            self.regs[0] = retval;
                            false
                        }
                        _ => false,
                    };
                    if take {
                        let new_pc = (self.pc as i64) + 1 + (i.off as i64);
                        if new_pc < 0 || new_pc as usize >= num_insns {
                            return Err(SvmError::ExecutionFailed);
                        }
                        self.pc = new_pc as usize;
                        continue;
                    }
                }

                // --------------------------------------------------------
                // JMP32 (32-bit comparisons)
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_JMP32 => {
                    let src_val = (if (op & SRC_REG) != 0 {
                        self.reg(i.src)
                    } else {
                        i.imm as i64 as u64
                    }) as u32;
                    let dst_val = self.reg(i.dst) as u32;
                    let take = match op >> 4 {
                        JMP_JA => true,
                        JMP_JEQ => dst_val == src_val,
                        JMP_JGT => dst_val > src_val,
                        JMP_JGE => dst_val >= src_val,
                        JMP_JSET => (dst_val & src_val) != 0,
                        JMP_JNE => dst_val != src_val,
                        JMP_JSGT => (dst_val as i32) > (src_val as i32),
                        JMP_JSGE => (dst_val as i32) >= (src_val as i32),
                        JMP_JLT => dst_val < src_val,
                        JMP_JLE => dst_val <= src_val,
                        JMP_JSLT => (dst_val as i32) < (src_val as i32),
                        JMP_JSLE => (dst_val as i32) <= (src_val as i32),
                        _ => false,
                    };
                    if take {
                        let new_pc = (self.pc as i64) + 1 + (i.off as i64);
                        if new_pc < 0 || new_pc as usize >= num_insns {
                            return Err(SvmError::ExecutionFailed);
                        }
                        self.pc = new_pc as usize;
                        continue;
                    }
                }

                // --------------------------------------------------------
                // LDX – load from memory into dst
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_LDX => {
                    let addr = self.reg(i.src).wrapping_add(i.off as i64 as u64);
                    let size = size_of_op(op);
                    let val = self.mem_read(addr, size)?;
                    self.set_reg(i.dst, val);
                }

                // --------------------------------------------------------
                // ST – store immediate into memory
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_ST => {
                    let addr = self.reg(i.dst).wrapping_add(i.off as i64 as u64);
                    let size = size_of_op(op);
                    self.mem_write(addr, i.imm as i64 as u64, size)?;
                }

                // --------------------------------------------------------
                // STX – store register value into memory
                // --------------------------------------------------------
                op if (op & 0x07) == CLS_STX => {
                    let addr = self.reg(i.dst).wrapping_add(i.off as i64 as u64);
                    let size = size_of_op(op);
                    self.mem_write(addr, self.reg(i.src), size)?;
                }

                _ => {
                    // Unknown opcode — reject to prevent undefined behavior
                    return Err(SvmError::ExecutionFailed);
                }
            }

            self.pc += 1;
        }

        // Fell off the end without EXIT — return r0
        Ok(self.regs[0])
    }
}

// ---------------------------------------------------------------------------
// ALU helpers
// ---------------------------------------------------------------------------

fn alu64(op: u8, dst: u64, src: u64) -> Result<u64, SvmError> {
    Ok(match op {
        ALU_ADD => dst.wrapping_add(src),
        ALU_SUB => dst.wrapping_sub(src),
        ALU_MUL => dst.wrapping_mul(src),
        ALU_DIV => {
            if src == 0 {
                return Err(SvmError::ExecutionFailed);
            }
            dst / src
        }
        ALU_OR => dst | src,
        ALU_AND => dst & src,
        ALU_LSH => dst.wrapping_shl((src & 63) as u32),
        ALU_RSH => dst.wrapping_shr((src & 63) as u32),
        ALU_NEG => (dst as i64).wrapping_neg() as u64,
        ALU_MOD => {
            if src == 0 {
                return Err(SvmError::ExecutionFailed);
            }
            dst % src
        }
        ALU_XOR => dst ^ src,
        ALU_MOV => src,
        ALU_ARSH => ((dst as i64).wrapping_shr((src & 63) as u32)) as u64,
        _ => dst, // reserved
    })
}

fn alu32(op: u8, dst: u32, src: u32) -> Result<u32, SvmError> {
    Ok(match op {
        ALU_ADD => dst.wrapping_add(src),
        ALU_SUB => dst.wrapping_sub(src),
        ALU_MUL => dst.wrapping_mul(src),
        ALU_DIV => {
            if src == 0 {
                return Err(SvmError::ExecutionFailed);
            }
            dst / src
        }
        ALU_OR => dst | src,
        ALU_AND => dst & src,
        ALU_LSH => dst.wrapping_shl(src & 31),
        ALU_RSH => dst.wrapping_shr(src & 31),
        ALU_NEG => (dst as i32).wrapping_neg() as u32,
        ALU_MOD => {
            if src == 0 {
                return Err(SvmError::ExecutionFailed);
            }
            dst % src
        }
        ALU_XOR => dst ^ src,
        ALU_MOV => src,
        ALU_ARSH => ((dst as i32).wrapping_shr(src & 31)) as u32,
        _ => dst,
    })
}

#[inline]
fn size_of_op(opcode: u8) -> usize {
    match opcode & SZ_DW {
        SZ_B => 1,
        SZ_H => 2,
        SZ_W => 4,
        SZ_DW => 8,
        _ => 4,
    }
}

// ---------------------------------------------------------------------------
// Syscall dispatch
// ---------------------------------------------------------------------------

impl<'a> Vm<'a> {
    /// Map a Solana syscall ID to a return value.
    /// Implements the core set of Solana BPF syscalls for on-chain execution.
    fn dispatch_syscall(&mut self, id: u32) -> Result<u64, SvmError> {
        match id {
            // sol_log_ (print) – no-op in on-chain context
            0x207559bd => Ok(0),
            // sol_panic_ – return error code
            0x686093bb => Err(SvmError::ExecutionFailed),

            // sol_sha256: r1 = SolBytes* array, r2 = array count, r3 = result ptr (32 bytes)
            0x11f49d86 => self.syscall_hash(sp_io::hashing::sha2_256),

            // sol_keccak256: r1 = SolBytes* array, r2 = array count, r3 = result ptr
            0xd7793abb => self.syscall_hash(sp_io::hashing::keccak_256),

            // sol_memcpy_: r1 = dst, r2 = src, r3 = n
            0x717cc4a3 => {
                let dst = self.regs[1];
                let src = self.regs[2];
                let n = self.regs[3] as usize;
                if n == 0 {
                    return Ok(0);
                }
                self.charge_fuel(n as u64)?;
                let data = self.mem_read_bytes(src, n)?;
                self.mem_write_bytes(dst, &data)?;
                Ok(0)
            }
            // sol_memmove_: r1 = dst, r2 = src, r3 = n (overlapping safe)
            0x434371f8 => {
                let dst = self.regs[1];
                let src = self.regs[2];
                let n = self.regs[3] as usize;
                if n == 0 {
                    return Ok(0);
                }
                self.charge_fuel(n as u64)?;
                // Read first, then write — handles overlapping regions
                let data = self.mem_read_bytes(src, n)?;
                self.mem_write_bytes(dst, &data)?;
                Ok(0)
            }
            // sol_memcmp_: r1 = s1 ptr, r2 = s2 ptr, r3 = n, r4 = result ptr
            0x5fdcde31 => {
                let s1_ptr = self.regs[1];
                let s2_ptr = self.regs[2];
                let n = self.regs[3] as usize;
                let result_ptr = self.regs[4];
                if n == 0 {
                    self.mem_write(result_ptr, 0i32 as u64, 4)?;
                    return Ok(0);
                }
                self.charge_fuel(n as u64)?;
                let s1 = self.mem_read_bytes(s1_ptr, n)?;
                let s2 = self.mem_read_bytes(s2_ptr, n)?;
                let mut cmp_result: i32 = 0;
                for i in 0..n {
                    if s1[i] != s2[i] {
                        cmp_result = (s1[i] as i32) - (s2[i] as i32);
                        break;
                    }
                }
                self.mem_write(result_ptr, cmp_result as u64, 4)?;
                Ok(0)
            }
            // sol_memset_: r1 = dst, r2 = byte value, r3 = n
            0x3770fb22 => {
                let dst = self.regs[1];
                let val = self.regs[2] as u8;
                let n = self.regs[3] as usize;
                if n == 0 {
                    return Ok(0);
                }
                self.charge_fuel(n as u64)?;
                let data = vec![val; n];
                self.mem_write_bytes(dst, &data)?;
                Ok(0)
            }

            // sol_create_program_address: r1=seeds, r2=seeds_len, r3=program_id, r4=result
            // Computes a deterministic address from seeds + program_id using SHA-256
            0x48504a38 => {
                let seeds_ptr = self.regs[1];
                let seeds_len = self.regs[2];
                let program_id_ptr = self.regs[3];
                let result_ptr = self.regs[4];
                let program_id = self.mem_read_bytes(program_id_ptr, 32)?;

                // Build PDA preimage: concatenate all seed bytes + program_id + "ProgramDerivedAddress"
                let mut preimage = Vec::new();
                for i in 0..seeds_len {
                    // Each seed entry is { ptr: u64, len: u64 } = 16 bytes
                    let entry_addr = seeds_ptr + i * 16;
                    let ptr = self.mem_read(entry_addr, 8)?;
                    let len = self.mem_read(entry_addr + 8, 8)? as usize;
                    if len > 32 {
                        return Ok(1); // Solana: seed too long
                    }
                    let seed_data = self.mem_read_bytes(ptr, len)?;
                    preimage.extend_from_slice(&seed_data);
                }
                preimage.extend_from_slice(&program_id);
                preimage.extend_from_slice(b"ProgramDerivedAddress");

                self.charge_fuel(100)?;
                let hash = sp_io::hashing::sha2_256(&preimage);
                if Self::is_on_ed25519_curve(&hash) {
                    return Ok(1);
                }
                self.mem_write_bytes(result_ptr, &hash)?;
                Ok(0)
            }

            // sol_try_find_program_address
            0x0ff98a16 => {
                let seeds_ptr = self.regs[1];
                let seeds_len = self.regs[2];
                let program_id_ptr = self.regs[3];
                let address_ptr = self.regs[4];
                let bump_ptr = self.regs[5];
                let program_id = self.mem_read_bytes(program_id_ptr, 32)?;

                // Solana MAX_SEEDS = 16; reject programs that pass more to prevent
                // unbounded memory reads and compute amplification.
                const MAX_SEEDS: u64 = 16;
                if seeds_len > MAX_SEEDS {
                    return Ok(1);
                }

                // Collect seeds
                let mut seed_data_list: Vec<Vec<u8>> = Vec::new();
                for i in 0..seeds_len {
                    let entry_addr = seeds_ptr + i * 16;
                    let ptr = self.mem_read(entry_addr, 8)?;
                    let len = self.mem_read(entry_addr + 8, 8)? as usize;
                    if len > 32 {
                        return Ok(1);
                    }
                    seed_data_list.push(self.mem_read_bytes(ptr, len)?);
                }

                // Try bump seeds 255..0, debiting compute units per retry to bound
                // the wall-clock cost of the syscall.
                //
                // Each iteration runs sha2_256 over (seeds + bump + program_id + nonce)
                // which is ~85 CU on Solana.  We charge conservatively per iteration.
                const BUMP_ITER_COST: u64 = 100;

                for bump in (0u8..=255).rev() {
                    // Debit fuel before the hash to avoid free work on the last iteration.
                    if self.fuel < BUMP_ITER_COST {
                        return Err(SvmError::OutOfComputeUnits);
                    }
                    self.fuel -= BUMP_ITER_COST;

                    let mut preimage = Vec::new();
                    for seed in &seed_data_list {
                        preimage.extend_from_slice(seed);
                    }
                    preimage.push(bump);
                    preimage.extend_from_slice(&program_id);
                    preimage.extend_from_slice(b"ProgramDerivedAddress");

                    let hash = sp_io::hashing::sha2_256(&preimage);
                    // A valid PDA must NOT lie on the ed25519 curve.
                    if Self::is_on_ed25519_curve(&hash) {
                        continue;
                    }
                    self.mem_write_bytes(address_ptr, &hash)?;
                    self.mem_write(bump_ptr, bump as u64, 1)?;
                    return Ok(0);
                }
                Ok(1) // Could not find valid PDA
            }

            // sol_invoke_signed_c / sol_invoke_signed_rust
            // CPI is not supported in the no_std interpreter path.
            // Return error code so programs detect the failure instead
            // of silently assuming the cross-program call succeeded.
            0xcb228b32 | 0xd7449092 => Err(SvmError::ExecutionFailed),

            // sol_get_clock_sysvar – populate clock struct at r1
            0xe8a04f5a => {
                // Clock struct: slot(u64), epoch_start_timestamp(i64), epoch(u64),
                //               leader_schedule_epoch(u64), unix_timestamp(i64) = 40 bytes
                let mut buf = [0u8; 40];
                // slot
                buf[0..8].copy_from_slice(&self.config.slot.to_le_bytes());
                // epoch_start_timestamp — use block_timestamp as approximation
                buf[8..16].copy_from_slice(&self.config.block_timestamp.to_le_bytes());
                // epoch — derive from slot (slots_per_epoch = 432_000 on mainnet)
                let epoch = self.config.slot / 432_000;
                buf[16..24].copy_from_slice(&epoch.to_le_bytes());
                // leader_schedule_epoch
                buf[24..32].copy_from_slice(&(epoch.saturating_add(1)).to_le_bytes());
                // unix_timestamp
                buf[32..40].copy_from_slice(&self.config.block_timestamp.to_le_bytes());
                self.mem_write_bytes(self.regs[1], &buf)?;
                Ok(0)
            }

            // sol_get_rent_sysvar
            0x3b97b73c => {
                // Rent struct: lamports_per_byte_year(u64), exemption_threshold(f64),
                //              burn_percent(u8) = 17 bytes
                let mut buf = [0u8; 17];
                // lamports_per_byte_year: Solana default = 3_480
                buf[0..8].copy_from_slice(&3_480u64.to_le_bytes());
                // exemption_threshold: Solana default = 2.0 years (as f64)
                buf[8..16].copy_from_slice(&2.0f64.to_le_bytes());
                // burn_percent: Solana default = 50
                buf[16] = 50;
                self.mem_write_bytes(self.regs[1], &buf)?;
                Ok(0)
            }

            // sol_get_epoch_schedule_sysvar
            0x6f54e7b4 => {
                // EpochSchedule: slots_per_epoch(u64), leader_schedule_slot_offset(u64),
                //                warmup(bool), first_normal_epoch(u64), first_normal_slot(u64) = 33 bytes
                let mut buf = [0u8; 33];
                // slots_per_epoch: Solana default = 432_000
                buf[0..8].copy_from_slice(&432_000u64.to_le_bytes());
                // leader_schedule_slot_offset: Solana default = 432_000
                buf[8..16].copy_from_slice(&432_000u64.to_le_bytes());
                // warmup: false
                buf[16] = 0;
                // first_normal_epoch: 0
                buf[17..25].copy_from_slice(&0u64.to_le_bytes());
                // first_normal_slot: 0
                buf[25..33].copy_from_slice(&0u64.to_le_bytes());
                self.mem_write_bytes(self.regs[1], &buf)?;
                Ok(0)
            }

            // sol_log_64 (log 5 u64 values)
            0x7317b434 => Ok(0),

            // sol_log_compute_units
            0x85aa8cf8 => Ok(0),

            // sol_log_data (structured log)
            0x7ef088ca => Ok(0),

            // sol_set_return_data: r1 = data ptr, r2 = data len
            0x834a16b8 => Ok(0),

            // sol_get_return_data: r1 = result ptr, r2 = len, r3 = program_id ptr
            0x2a8df582 => Ok(0), // no return data available

            // sol_get_stack_height
            0xa2609a6c => Ok(0), // depth 0 = top level

            // Any unrecognised syscall: fail the program.
            // Solana rejects unknown syscalls at link-time; we do the same at
            // runtime to prevent programs from relying on undefined behavior.
            _ => {
                #[cfg(feature = "std")]
                log::warn!("Unknown SVM syscall: {:#010x} — aborting execution", id);
                Err(SvmError::ExecutionFailed)
            }
        }
    }

    /// Shared helper for sol_sha256 and sol_keccak256.
    /// Solana convention: r1 = SolBytes* array, r2 = count, r3 = result ptr.
    /// SolBytes = { ptr: u64, len: u64 } — 16 bytes per entry.
    fn syscall_hash(&mut self, hash_fn: fn(&[u8]) -> [u8; 32]) -> Result<u64, SvmError> {
        let vals_ptr = self.regs[1];
        let vals_len = self.regs[2];
        let result_ptr = self.regs[3];

        // Bound the number of SolBytes entries to prevent reading unbounded memory
        // and to limit compute cost. Solana caps seed count at MAX_SEEDS (16).
        const MAX_HASH_ENTRIES: u64 = 32;
        if vals_len > MAX_HASH_ENTRIES {
            return Ok(1); // Return error code to the program (not a fatal interpreter error)
        }

        let mut data = Vec::new();
        for idx in 0..vals_len {
            let entry_addr = vals_ptr + idx * 16;
            let ptr = self.mem_read(entry_addr, 8)?;
            let len = self.mem_read(entry_addr + 8, 8)? as usize;
            // Solana caps individual seed/data chunk at 1 KB.
            if len > 1024 {
                return Ok(1);
            }
            let chunk = self.mem_read_bytes(ptr, len)?;
            data.extend_from_slice(&chunk);
        }

        // Debit compute units proportional to data hashed.
        // Approximation of Solana's cost model: 85 CU base + 0.5 CU/byte.
        let cost: u64 = 85 + (data.len() as u64 / 2).max(1);
        if self.fuel < cost {
            return Err(SvmError::OutOfComputeUnits);
        }
        self.fuel -= cost;

        let hash = hash_fn(&data);
        self.mem_write_bytes(result_ptr, &hash)?;
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Execute an sBPF program in the interpreter.
///
/// `payload` may be either:
/// - A raw eBPF instruction stream (multiple of 8 bytes), or
/// - A full ELF64/sBPF binary (starts with `\x7fELF`).
///
/// `input_data` is passed as r1 with its length in r2.
pub fn execute_bpf(
    payload: &[u8],
    input_data: &[u8],
    config: &SvmConfig,
) -> SvmResult<SvmExecutionResult> {
    if payload.is_empty() {
        return Err(SvmError::InvalidPayload);
    }

    // Determine instruction stream
    let text: &[u8] = if payload.starts_with(b"\x7fELF") {
        elf_find_text(payload).ok_or(SvmError::InvalidPayload)?
    } else if payload.len().is_multiple_of(8) {
        payload
    } else {
        return Err(SvmError::InvalidPayload);
    };

    if text.is_empty() || !text.len().is_multiple_of(8) {
        return Err(SvmError::InvalidPayload);
    }

    let fuel = config.compute_unit_limit.min(MAX_INSN_FUEL);
    let mut vm = Vm::new(text, input_data, fuel, config);

    match vm.run() {
        Ok(r0) => {
            let compute_units = config.compute_unit_limit - vm.fuel;
            let mut result = SvmExecutionResult {
                success: r0 == 0, // Solana convention: 0 = success
                output: r0.to_le_bytes().to_vec(),
                compute_units_used: compute_units,
                account_updates: Vec::new(),
                logs: vec![],
                state_root: [0u8; 32],
            };
            result.state_root = crate::compute_svm_state_root(&result);
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

/// Statically validate a raw BPF instruction stream or ELF.
/// Returns `Ok(())` if the payload is structurally valid.
pub fn validate_program(payload: &[u8]) -> SvmResult<()> {
    if payload.is_empty() {
        return Err(SvmError::InvalidPayload);
    }
    if payload.starts_with(b"\x7fELF") {
        elf_find_text(payload).ok_or(SvmError::InvalidPayload)?;
        return Ok(());
    }
    if !payload.len().is_multiple_of(8) {
        return Err(SvmError::InvalidPayload);
    }
    // Walk instructions and check opcode classes are valid
    let num_insns = payload.len() / 8;
    for idx in 0..num_insns {
        let raw: &[u8; 8] = payload[idx * 8..(idx + 1) * 8]
            .try_into()
            .map_err(|_| SvmError::InvalidPayload)?;
        let ins = decode(raw);
        let class = ins.opcode & 0x07;
        if class > CLS_ALU64 {
            return Err(SvmError::InvalidPayload);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SvmConfig;

    fn default_config() -> SvmConfig {
        SvmConfig::default()
    }

    /// Build a minimal BPF program: `r0 = imm; exit`
    fn prog_return(val: i32) -> Vec<u8> {
        let mut p = Vec::new();
        // MOV64 r0, imm  → opcode=0xb7, dst=0, src=0, off=0, imm=val
        p.extend_from_slice(&[0xb7, 0x00, 0x00, 0x00]);
        p.extend_from_slice(&val.to_le_bytes());
        // EXIT  → opcode=0x95
        p.extend_from_slice(&[0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        p
    }

    #[test]
    fn test_return_zero_is_success() {
        let prog = prog_return(0);
        let res = execute_bpf(&prog, &[], &default_config()).unwrap();
        assert!(res.success, "r0=0 should be success");
    }

    #[test]
    fn test_return_nonzero_is_failure() {
        let prog = prog_return(1);
        let res = execute_bpf(&prog, &[], &default_config()).unwrap();
        assert!(!res.success, "r0=1 should be failure");
    }

    #[test]
    fn test_alu_add() {
        // r0 = 3 + 4; exit
        let mut p = Vec::new();
        // MOV64 r0, 3
        p.extend_from_slice(&[0xb7, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]);
        // ADD64 r0, 4  → opcode=0x07, dst=0, imm=4
        p.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]);
        // EXIT
        p.extend_from_slice(&[0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let res = execute_bpf(&p, &[], &default_config()).unwrap();
        let r0 = u64::from_le_bytes(res.output.as_slice().try_into().unwrap_or([0; 8]));
        assert_eq!(r0, 7);
    }

    #[test]
    fn test_compute_unit_enforcement() {
        // Infinite loop: JA -1
        let mut p = Vec::new();
        // MOV64 r0, 0
        p.extend_from_slice(&[0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // JMP to PC-1 (offset -1): opcode=0x05, off=-1
        p.extend_from_slice(&[0x05, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00]);
        let mut cfg = default_config();
        cfg.compute_unit_limit = 10;
        let res = execute_bpf(&p, &[], &cfg);
        assert_eq!(res, Err(SvmError::OutOfComputeUnits));
    }

    #[test]
    fn test_validate_valid_prog() {
        let prog = prog_return(0);
        assert!(validate_program(&prog).is_ok());
    }

    #[test]
    fn test_validate_empty_fails() {
        assert_eq!(validate_program(&[]), Err(SvmError::InvalidPayload));
    }

    #[test]
    fn test_validate_odd_size_fails() {
        assert_eq!(
            validate_program(&[0x95, 0x00, 0x00]),
            Err(SvmError::InvalidPayload)
        );
    }

    #[test]
    fn test_cpi_syscall_returns_error() {
        // CPI is not supported in the no_std interpreter — both signed and
        // unsigned variants must return an execution error.
        let prog = prog_return(0);
        let text: &[u8] = &prog;
        let cfg = SvmConfig::default();
        let mut vm = Vm::new(text, &[], 1000, &cfg);
        let cpi_c = vm.dispatch_syscall(0xcb228b32);
        assert!(cpi_c.is_err(), "CPI should fail in no_std interpreter");

        let cpi_rust = vm.dispatch_syscall(0xd7449092);
        assert!(cpi_rust.is_err(), "CPI should fail in no_std interpreter");
    }
}
