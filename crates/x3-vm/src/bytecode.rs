//! X3VM bytecode format and validation.
//!
//! Defines the canonical bytecode structure and provides a validator that checks
//! structural integrity before the interpreter is allowed to execute the program.

/// Magic bytes identifying X3 bytecode files.
pub const X3BC_MAGIC: &[u8; 4] = b"X3BC";

/// Current bytecode format version.
pub const X3BC_VERSION: u16 = 1;

/// Maximum number of instructions in a single function.
pub const MAX_INSTRUCTIONS: usize = 65_536;

/// Maximum number of functions per module.
pub const MAX_FUNCTIONS: usize = 1_024;

/// Maximum size of the constant pool.
pub const MAX_CONST_POOL: usize = 4_096;

/// Bytecode opcodes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0x00,
    /// Load constant from pool: operand = pool index.
    LdConst = 0x01,
    /// Add top two stack values.
    Add = 0x10,
    /// Subtract.
    Sub = 0x11,
    /// Multiply.
    Mul = 0x12,
    /// Integer divide (traps on zero).
    Div = 0x13,
    /// Unconditional jump: operand = target PC.
    Jump = 0x20,
    /// Jump-if-zero: operand = target PC.
    JumpZ = 0x21,
    /// Call function by index.
    Call = 0x30,
    /// Return from current function.
    Ret = 0x31,
    /// Begin atomic window.
    AtomicBegin = 0x40,
    /// End atomic window (commit).
    AtomicEnd = 0x41,
    /// Abort current atomic window.
    AtomicAbort = 0x42,
    /// Halt execution.
    Halt = 0xFF,
}

impl TryFrom<u8> for Opcode {
    type Error = BytecodeError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Opcode::Nop),
            0x01 => Ok(Opcode::LdConst),
            0x10 => Ok(Opcode::Add),
            0x11 => Ok(Opcode::Sub),
            0x12 => Ok(Opcode::Mul),
            0x13 => Ok(Opcode::Div),
            0x20 => Ok(Opcode::Jump),
            0x21 => Ok(Opcode::JumpZ),
            0x30 => Ok(Opcode::Call),
            0x31 => Ok(Opcode::Ret),
            0x40 => Ok(Opcode::AtomicBegin),
            0x41 => Ok(Opcode::AtomicEnd),
            0x42 => Ok(Opcode::AtomicAbort),
            0xFF => Ok(Opcode::Halt),
            _ => Err(BytecodeError::UnknownOpcode(v)),
        }
    }
}

/// A single instruction: opcode + optional 32-bit operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub operand: u32,
}

/// A single function in the module.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Function {
    pub instructions: Vec<Instruction>,
    pub max_stack_depth: u16,
}

/// A parsed bytecode module.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BytecodeModule {
    pub version: u16,
    pub functions: Vec<Function>,
    pub const_pool: Vec<u64>,
}

/// Errors produced by the bytecode parser/validator.
#[derive(Debug, PartialEq, Eq)]
pub enum BytecodeError {
    /// Wrong magic bytes.
    BadMagic,
    /// Unsupported version.
    UnsupportedVersion(u16),
    /// Truncated bytecode.
    UnexpectedEof,
    /// Unknown opcode encountered.
    UnknownOpcode(u8),
    /// Jump target out of bounds.
    InvalidJumpTarget { target: u32, bound: u32 },
    /// Function index out of bounds.
    InvalidFunctionIndex(u32),
    /// Constant pool index out of bounds.
    InvalidConstIndex(u32),
    /// Unbalanced atomic blocks.
    UnbalancedAtomicBlocks,
    /// Module exceeds size limits.
    ModuleTooLarge,
}

/// Parse and validate raw bytes into a `BytecodeModule`.
pub fn parse(bytes: &[u8]) -> Result<BytecodeModule, BytecodeError> {
    if bytes.len() < 6 {
        return Err(BytecodeError::UnexpectedEof);
    }
    if &bytes[0..4] != X3BC_MAGIC {
        return Err(BytecodeError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != X3BC_VERSION {
        return Err(BytecodeError::UnsupportedVersion(version));
    }
    // Minimal stub: for now return an empty module (full parser in vm.rs)
    Ok(BytecodeModule {
        version,
        functions: Vec::new(),
        const_pool: Vec::new(),
    })
}

/// Validate that all jump targets and call indices are in bounds.
pub fn validate(module: &BytecodeModule) -> Result<(), BytecodeError> {
    if module.functions.len() > MAX_FUNCTIONS {
        return Err(BytecodeError::ModuleTooLarge);
    }
    if module.const_pool.len() > MAX_CONST_POOL {
        return Err(BytecodeError::ModuleTooLarge);
    }
    for func in &module.functions {
        if func.instructions.len() > MAX_INSTRUCTIONS {
            return Err(BytecodeError::ModuleTooLarge);
        }
        let n = func.instructions.len() as u32;
        let mut atomic_depth: u32 = 0;
        for instr in &func.instructions {
            match instr.opcode {
                Opcode::Jump | Opcode::JumpZ => {
                    if instr.operand >= n {
                        return Err(BytecodeError::InvalidJumpTarget {
                            target: instr.operand,
                            bound: n,
                        });
                    }
                }
                Opcode::Call => {
                    if instr.operand as usize >= module.functions.len() {
                        return Err(BytecodeError::InvalidFunctionIndex(instr.operand));
                    }
                }
                Opcode::LdConst => {
                    if instr.operand as usize >= module.const_pool.len()
                        && !module.const_pool.is_empty()
                    {
                        return Err(BytecodeError::InvalidConstIndex(instr.operand));
                    }
                }
                Opcode::AtomicBegin => atomic_depth += 1,
                Opcode::AtomicEnd | Opcode::AtomicAbort => {
                    if atomic_depth == 0 {
                        return Err(BytecodeError::UnbalancedAtomicBlocks);
                    }
                    atomic_depth -= 1;
                }
                _ => {}
            }
        }
        if atomic_depth != 0 {
            return Err(BytecodeError::UnbalancedAtomicBlocks);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_module(funcs: Vec<Function>, pool: Vec<u64>) -> BytecodeModule {
        BytecodeModule {
            version: X3BC_VERSION,
            functions: funcs,
            const_pool: pool,
        }
    }

    fn instr(opcode: Opcode, operand: u32) -> Instruction {
        Instruction { opcode, operand }
    }

    #[test]
    fn test_parse_bad_magic() {
        let bytes = b"XXXC\x01\x00";
        assert_eq!(parse(bytes), Err(BytecodeError::BadMagic));
    }

    #[test]
    fn test_parse_good_magic() {
        let mut bytes = b"X3BC\x01\x00".to_vec();
        bytes.extend_from_slice(&[0u8; 10]);
        assert!(parse(&bytes).is_ok());
    }

    #[test]
    fn test_validate_empty_module() {
        let m = make_module(vec![], vec![]);
        assert!(validate(&m).is_ok());
    }

    #[test]
    fn test_invalid_jump_target() {
        let func = Function {
            instructions: vec![instr(Opcode::Jump, 999)],
            max_stack_depth: 0,
        };
        let m = make_module(vec![func], vec![]);
        assert!(matches!(
            validate(&m),
            Err(BytecodeError::InvalidJumpTarget { .. })
        ));
    }

    #[test]
    fn test_balanced_atomic_blocks() {
        let func = Function {
            instructions: vec![
                instr(Opcode::AtomicBegin, 0),
                instr(Opcode::Nop, 0),
                instr(Opcode::AtomicEnd, 0),
                instr(Opcode::Ret, 0),
            ],
            max_stack_depth: 0,
        };
        let m = make_module(vec![func], vec![]);
        assert!(validate(&m).is_ok());
    }

    #[test]
    fn test_unbalanced_atomic_rejected() {
        let func = Function {
            instructions: vec![instr(Opcode::AtomicBegin, 0), instr(Opcode::Ret, 0)],
            max_stack_depth: 0,
        };
        let m = make_module(vec![func], vec![]);
        assert_eq!(validate(&m), Err(BytecodeError::UnbalancedAtomicBlocks));
    }

    #[test]
    fn test_extra_atomic_end_rejected() {
        let func = Function {
            instructions: vec![instr(Opcode::AtomicEnd, 0)],
            max_stack_depth: 0,
        };
        let m = make_module(vec![func], vec![]);
        assert_eq!(validate(&m), Err(BytecodeError::UnbalancedAtomicBlocks));
    }
}
