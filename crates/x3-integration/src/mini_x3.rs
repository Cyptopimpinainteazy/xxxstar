//! Minimal no-std X3BC bytecode interpreter
//!
//! Provides real X3 bytecode execution in no-std (WASM) builds.
//! Mirrors the logic in `x3-vm` but without std/alloc-heavy dependencies.
//!
//! # Binary Format (X3BC v1)
//! ```text
//! Header (24 bytes):
//!   [0..4]   Magic "X3BC"
//!   [4..8]   Version u32 LE
//!   [8..12]  Flags   u32 LE
//!   [12..16] Checksum u32 LE
//!   [16..20] MinVersion u32 LE
//!   [20..24] FeatureFlags u32 LE
//! Const pool: count:u32 + entries (tag:u8 + data)
//!   tag 0 = Integer (i64)  tag 1 = Float (f64)
//!   tag 2 = String (len:u32 + utf8)
//!   tag 3 = Bool (u8)      tag 4 = Bytes (len:u32 + bytes)
//! Function table: count:u32 + entries
//!   name_len:u16 + name + entry:u32 + params:u8 + locals:u16 + stack:u16 + ret:u8
//! Globals table: count:u32 + entries
//!   name_len:u16 + name + type_tag:u8 + mutable:u8 + init_const:u32
//! Code section: len:u32 + bytes
//! ```
//!
//! # Register encoding
//! All register operands are u8 (max 256 registers per frame).
//! The call convention uses a sliding register window per frame.

use sp_std::vec;
use sp_std::vec::Vec;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X3Error {
    InvalidMagic,
    UnexpectedEof,
    InvalidOpcode(u8),
    DivisionByZero,
    GasExhausted,
    StackOverflow,
    StackUnderflow,
    TypeMismatch,
    ConstPoolOutOfBounds,
    FunctionNotFound,
    GlobalOutOfBounds,
    RegisterOutOfBounds,
    UserPanic,
}

pub type X3Result<T> = Result<T, X3Error>;

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Default)]
pub enum MiniValue {
    I64(i64),
    F64(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    #[default]
    Unit,
}

impl MiniValue {
    fn as_i64(&self) -> X3Result<i64> {
        match self {
            MiniValue::I64(v) => Ok(*v),
            MiniValue::Bool(b) => Ok(*b as i64),
            _ => Err(X3Error::TypeMismatch),
        }
    }
    fn as_f64(&self) -> X3Result<f64> {
        match self {
            MiniValue::F64(v) => Ok(*v),
            MiniValue::I64(v) => Ok(*v as f64),
            _ => Err(X3Error::TypeMismatch),
        }
    }
    fn as_bool(&self) -> X3Result<bool> {
        match self {
            MiniValue::Bool(b) => Ok(*b),
            MiniValue::I64(v) => Ok(*v != 0),
            _ => Err(X3Error::TypeMismatch),
        }
    }
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MiniValue::I64(v) => v.to_le_bytes().to_vec(),
            MiniValue::F64(v) => v.to_bits().to_le_bytes().to_vec(),
            MiniValue::Bool(v) => vec![*v as u8],
            MiniValue::Bytes(b) => b.clone(),
            MiniValue::Unit => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum MiniConst {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
struct MiniFunc {
    entry: u32,
    #[allow(dead_code)]
    param_count: u8,
    local_count: u16,
}

#[derive(Debug, Clone)]
struct MiniGlobal {
    mutable: bool,
    init_const: u32,
}

#[derive(Debug)]
struct MiniModule {
    const_pool: Vec<MiniConst>,
    functions: Vec<MiniFunc>,
    globals: Vec<MiniGlobal>,
    code: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Binary parser
// ---------------------------------------------------------------------------

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
    fn read_u8(&mut self) -> X3Result<u8> {
        if self.pos >= self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn read_u16(&mut self) -> X3Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }
    fn read_u32(&mut self) -> X3Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }
    fn read_i64(&mut self) -> X3Result<i64> {
        if self.pos + 8 > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        let v = i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        self.pos += 8;
        Ok(v)
    }
    fn read_f64(&mut self) -> X3Result<f64> {
        if self.pos + 8 > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        let v = f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        self.pos += 8;
        Ok(v)
    }
    fn read_bytes(&mut self, n: usize) -> X3Result<Vec<u8>> {
        if self.pos + n > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
    fn skip(&mut self, n: usize) -> X3Result<()> {
        if self.pos + n > self.data.len() {
            return Err(X3Error::UnexpectedEof);
        }
        self.pos += n;
        Ok(())
    }
}

fn parse_module(bytes: &[u8]) -> X3Result<MiniModule> {
    let mut r = Reader::new(bytes);

    // Header (24 bytes)
    if r.remaining() < 24 {
        return Err(X3Error::UnexpectedEof);
    }
    let magic = r.read_bytes(4)?;
    if magic != b"X3BC" {
        return Err(X3Error::InvalidMagic);
    }
    r.skip(20)?; // version, flags, checksum, minversion, features

    // Const pool
    let const_count = r.read_u32()? as usize;
    let mut const_pool = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        let tag = r.read_u8()?;
        let c = match tag {
            0 => MiniConst::Integer(r.read_i64()?),
            1 => MiniConst::Float(r.read_f64()?),
            2 => {
                let len = r.read_u32()? as usize;
                r.skip(len)?;
                MiniConst::Bytes(vec![])
            }
            3 => MiniConst::Bool(r.read_u8()? != 0),
            4 => {
                let len = r.read_u32()? as usize;
                let b = r.read_bytes(len)?;
                MiniConst::Bytes(b)
            }
            _ => return Err(X3Error::UnexpectedEof),
        };
        const_pool.push(c);
    }

    // Function table
    let func_count = r.read_u32()? as usize;
    let mut functions = Vec::with_capacity(func_count);
    for _ in 0..func_count {
        let name_len = r.read_u16()? as usize;
        r.skip(name_len)?; // skip name
        let entry = r.read_u32()?;
        let param_count = r.read_u8()?;
        let local_count = r.read_u16()?;
        r.skip(2)?; // max_stack u16
        r.skip(1)?; // return_type_tag u8
        functions.push(MiniFunc {
            entry,
            param_count,
            local_count,
        });
    }

    // Global table
    let global_count = r.read_u32()? as usize;
    let mut globals = Vec::with_capacity(global_count);
    for _ in 0..global_count {
        let name_len = r.read_u16()? as usize;
        r.skip(name_len)?;
        r.skip(1)?; // type_tag
        let mutable = r.read_u8()? != 0;
        let init_const = r.read_u32()?;
        globals.push(MiniGlobal {
            mutable,
            init_const,
        });
    }

    // Code section
    let code_len = r.read_u32()? as usize;
    let code = r.read_bytes(code_len)?;

    Ok(MiniModule {
        const_pool,
        functions,
        globals,
        code,
    })
}

// ---------------------------------------------------------------------------
// Execution result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct X3ExecResult {
    pub return_val: MiniValue,
    pub gas_used: u64,
}

// ---------------------------------------------------------------------------
// VM
// ---------------------------------------------------------------------------

struct CallFrame {
    ip: usize,
    base: usize,
    ret_addr: usize,
    func_idx: usize,
}

const MAX_REGS: usize = 256;
const MAX_DEPTH: usize = 64;

struct Vm<'m> {
    module: &'m MiniModule,
    regs: Vec<MiniValue>,
    call_stack: Vec<CallFrame>,
    globals: Vec<MiniValue>,
    gas_used: u64,
    gas_limit: u64,
}

enum Step {
    Continue(usize),
    Return(Option<MiniValue>),
    Halt,
}

impl<'m> Vm<'m> {
    fn new(module: &'m MiniModule, gas_limit: u64) -> Self {
        // Initialise globals from const pool
        let globals: Vec<MiniValue> = module
            .globals
            .iter()
            .map(|g| {
                module
                    .const_pool
                    .get(g.init_const as usize)
                    .map(mini_value_from_const)
                    .unwrap_or(MiniValue::Unit)
            })
            .collect();
        Vm {
            module,
            regs: vec![MiniValue::Unit; MAX_REGS],
            call_stack: Vec::with_capacity(MAX_DEPTH),
            globals,
            gas_used: 0,
            gas_limit,
        }
    }

    fn resolve(&self, reg: usize) -> usize {
        self.call_stack.last().map(|f| f.base).unwrap_or(0) + reg
    }

    fn r8(&self, ip: usize) -> X3Result<u8> {
        self.module
            .code
            .get(ip)
            .copied()
            .ok_or(X3Error::UnexpectedEof)
    }
    fn r32(&self, ip: usize) -> X3Result<u32> {
        let c = &self.module.code;
        if ip + 4 > c.len() {
            return Err(X3Error::UnexpectedEof);
        }
        Ok(u32::from_le_bytes([c[ip], c[ip + 1], c[ip + 2], c[ip + 3]]))
    }
    fn r16(&self, ip: usize) -> X3Result<u16> {
        let c = &self.module.code;
        if ip + 2 > c.len() {
            return Err(X3Error::UnexpectedEof);
        }
        Ok(u16::from_le_bytes([c[ip], c[ip + 1]]))
    }
    fn ri8(&self, ip: usize) -> X3Result<i8> {
        Ok(self.r8(ip)? as i8)
    }

    fn run(&mut self) -> X3Result<Option<MiniValue>> {
        loop {
            if self.gas_used >= self.gas_limit {
                return Err(X3Error::GasExhausted);
            }

            let frame = self.call_stack.last().ok_or(X3Error::StackUnderflow)?;
            let ip = frame.ip;

            if ip >= self.module.code.len() {
                return Err(X3Error::UnexpectedEof);
            }

            let op = self.module.code[ip];
            self.gas_used += 1;

            let step = self.exec(op, ip)?;

            match step {
                Step::Continue(next) => {
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = next;
                    }
                }
                Step::Return(val) => {
                    let frame = self.call_stack.pop().ok_or(X3Error::StackUnderflow)?;
                    if frame.ret_addr == usize::MAX {
                        return Ok(val); // top-level return
                    }
                    if let Some(v) = val {
                        // Return value goes into r0 of the restored caller frame
                        let base = self.call_stack.last().map(|f| f.base).unwrap_or(0);
                        self.regs[base] = v;
                    }
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = frame.ret_addr;
                    }
                }
                Step::Halt => return Ok(None),
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn exec(&mut self, op: u8, ip: usize) -> X3Result<Step> {
        // Capture base FIRST — this ends the immutable borrow before any mutable ops.
        let base = self.call_stack.last().map(|f| f.base).unwrap_or(0);

        // Helper: absolute register index from relative reg operand
        macro_rules! reg {
            ($r:expr) => {
                base + $r as usize
            };
        }
        macro_rules! rv {
            ($r:expr) => {
                &self.regs[reg!($r)]
            };
        }
        macro_rules! set {
            ($r:expr, $v:expr) => {
                self.regs[reg!($r)] = $v;
            };
        }

        match op {
            // -------- Control Flow --------
            0x00 => Ok(Step::Continue(ip + 1)), // Nop
            0x01 => Ok(Step::Continue(self.r32(ip + 1)? as usize)), // Jump
            0x02 => {
                // JumpIf
                let cond = self.r8(ip + 1)? as usize;
                let tgt = self.r32(ip + 2)? as usize;
                if rv!(cond).as_bool()? {
                    Ok(Step::Continue(tgt))
                } else {
                    Ok(Step::Continue(ip + 6))
                }
            }
            0x03 => {
                // JumpUnless
                let cond = self.r8(ip + 1)? as usize;
                let tgt = self.r32(ip + 2)? as usize;
                if !rv!(cond).as_bool()? {
                    Ok(Step::Continue(tgt))
                } else {
                    Ok(Step::Continue(ip + 6))
                }
            }
            0x04 => {
                // Call
                let _dst = self.r8(ip + 1)? as usize;
                let func_idx = self.r32(ip + 2)? as usize;
                let argc = self.r16(ip + 6)? as usize;
                let func = self
                    .module
                    .functions
                    .get(func_idx)
                    .ok_or(X3Error::FunctionNotFound)?
                    .clone();
                if self.call_stack.len() >= MAX_DEPTH {
                    return Err(X3Error::StackOverflow);
                }
                // collect args (from caller base)
                let mut args = Vec::with_capacity(argc);
                for i in 0..argc {
                    let ar = self.r8(ip + 8 + i)? as usize;
                    args.push(self.regs[self.resolve(ar)].clone());
                }
                let caller_base = self.call_stack.last().map(|f| f.base).unwrap_or(0);
                let caller_locals = self
                    .call_stack
                    .last()
                    .map(|f| self.module.functions[f.func_idx].local_count as usize)
                    .unwrap_or(0);
                let callee_base = caller_base + caller_locals;
                if callee_base + func.local_count as usize >= MAX_REGS {
                    return Err(X3Error::RegisterOutOfBounds);
                }
                for (i, a) in args.into_iter().enumerate() {
                    self.regs[callee_base + i] = a;
                }
                let ret_addr = ip + 8 + argc;
                self.call_stack.push(CallFrame {
                    ip: func.entry as usize,
                    base: callee_base,
                    ret_addr,
                    func_idx,
                });
                Ok(Step::Continue(func.entry as usize))
            }
            0x05 => {
                // Ret
                let s = self.r8(ip + 1)? as usize;
                let v = self.regs[self.resolve(s)].clone();
                Ok(Step::Return(Some(v)))
            }
            0x06 => Ok(Step::Return(None)), // RetVoid
            0x07 => Ok(Step::Halt),         // Halt

            // -------- Load/Store --------
            0x10 => {
                // LoadConst
                let d = self.r8(ip + 1)? as usize;
                let idx = self.r32(ip + 2)? as usize;
                let cv = self
                    .module
                    .const_pool
                    .get(idx)
                    .ok_or(X3Error::ConstPoolOutOfBounds)?;
                let v = mini_value_from_const(cv);
                set!(d, v);
                Ok(Step::Continue(ip + 6))
            }
            0x11 => {
                // Mov
                let d = self.r8(ip + 1)? as usize;
                let s = self.r8(ip + 2)? as usize;
                let v = rv!(s).clone();
                set!(d, v);
                Ok(Step::Continue(ip + 3))
            }
            0x12 => {
                // LoadGlobal
                let d = self.r8(ip + 1)? as usize;
                let idx = self.r32(ip + 2)? as usize;
                let v = self
                    .globals
                    .get(idx)
                    .ok_or(X3Error::GlobalOutOfBounds)?
                    .clone();
                set!(d, v);
                Ok(Step::Continue(ip + 6))
            }
            0x13 => {
                // StoreGlobal
                let idx = self.r32(ip + 1)? as usize;
                let s = self.r8(ip + 5)? as usize;
                if !self
                    .module
                    .globals
                    .get(idx)
                    .map(|g| g.mutable)
                    .unwrap_or(false)
                {
                    return Err(X3Error::UserPanic);
                }
                let v = rv!(s).clone();
                if idx < self.globals.len() {
                    self.globals[idx] = v;
                }
                Ok(Step::Continue(ip + 6))
            }
            0x18 => {
                // LoadImm
                let d = self.r8(ip + 1)? as usize;
                let v = self.ri8(ip + 2)? as i64;
                set!(d, MiniValue::I64(v));
                Ok(Step::Continue(ip + 3))
            }
            0x19 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 2))
            } // LoadZero
            0x1A => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Bool(true));
                Ok(Step::Continue(ip + 2))
            } // LoadTrue
            0x1B => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Bool(false));
                Ok(Step::Continue(ip + 2))
            } // LoadFalse

            // -------- Integer Arithmetic --------
            0x20 => {
                // AddI
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)]
                    .as_i64()?
                    .wrapping_add(self.regs[reg!(b)].as_i64()?);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x21 => {
                // SubI
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)]
                    .as_i64()?
                    .wrapping_sub(self.regs[reg!(b)].as_i64()?);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x22 => {
                // MulI
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)]
                    .as_i64()?
                    .wrapping_mul(self.regs[reg!(b)].as_i64()?);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x23 => {
                // DivI
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let vb = self.regs[reg!(b)].as_i64()?;
                if vb == 0 {
                    return Err(X3Error::DivisionByZero);
                }
                let v = self.regs[reg!(a)].as_i64()? / vb;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x24 => {
                // ModI
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let vb = self.regs[reg!(b)].as_i64()?;
                if vb == 0 {
                    return Err(X3Error::DivisionByZero);
                }
                let v = self.regs[reg!(a)].as_i64()? % vb;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x25 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_i64()?.wrapping_neg();
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            } // NegI
            0x26 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_i64()? + 1;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            } // Inc
            0x27 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_i64()? - 1;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            } // Dec

            // -------- Float Arithmetic --------
            0x30 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? + self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 4))
            } // AddF
            0x31 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? - self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 4))
            } // SubF
            0x32 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? * self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 4))
            } // MulF
            0x33 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let vb = self.regs[reg!(b)].as_f64()?;
                let v = self.regs[reg!(a)].as_f64()? / vb;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 4))
            } // DivF
            0x35 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = -self.regs[reg!(s)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 3))
            } // NegF

            // -------- Comparisons --------
            0x40 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? == self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x41 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? != self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x42 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? < self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x43 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? <= self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x44 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? > self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x45 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? >= self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x46 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? == self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x47 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? != self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x48 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? < self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x49 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? <= self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x4A => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? > self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x4B => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_f64()? >= self.regs[reg!(b)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }

            // -------- Bitwise --------
            0x50 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? & self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x51 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? | self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x52 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? ^ self.regs[reg!(b)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x53 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = !self.regs[reg!(s)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            }
            0x54 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? << (self.regs[reg!(b)].as_i64()? & 63);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x55 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_i64()? >> (self.regs[reg!(b)].as_i64()? & 63);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x56 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v =
                    (self.regs[reg!(a)].as_i64()? as u64) >> (self.regs[reg!(b)].as_i64()? & 63);
                self.regs[reg!(d)] = MiniValue::I64(v as i64);
                Ok(Step::Continue(ip + 4))
            }
            0x58 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_bool()? && self.regs[reg!(b)].as_bool()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x59 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let v = self.regs[reg!(a)].as_bool()? || self.regs[reg!(b)].as_bool()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 4))
            }
            0x5A => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = !self.regs[reg!(s)].as_bool()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 3))
            }

            // -------- Type Conversions --------
            0x60 | 0x61 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_i64()?;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            }
            0x62 | 0x63 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 3))
            }
            0x64 | 0x65 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_f64()? as i64;
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            }
            0x66 | 0x67 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_f64()?;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 3))
            }
            0x68 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_bool()?;
                self.regs[reg!(d)] = MiniValue::Bool(v);
                Ok(Step::Continue(ip + 3))
            }

            // -------- Array / Context / Atomic / Agent / Intrinsics (return defaults) --------
            // These opcodes decode their operands and skip the right number of bytes.
            // In the WASM no-std context, cross-VM intrinsics and GPU ops are no-ops.
            0x70 => {
                self.r8(ip + 1)?;
                self.r16(ip + 2)?;
                Ok(Step::Continue(ip + 4))
            } // NewArray
            0x71 => {
                let (d, _) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)?);
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 3))
            }
            0x72 | 0x73 => {
                self.r8(ip + 1)?;
                self.r8(ip + 2)?;
                Ok(Step::Continue(ip + 3))
            }
            0x74 => {
                let n = self.r16(ip + 2)? as usize;
                Ok(Step::Continue(ip + 4 + n))
            } // NewTuple: [op][dst][count][regs...]
            0x75 => {
                let d = self.r8(ip + 1)? as usize;
                self.r8(ip + 2)?;
                self.r16(ip + 3)?;
                set!(d, MiniValue::Unit);
                Ok(Step::Continue(ip + 5))
            }

            // Context ops — return zero / unit
            0x80 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Bytes(sp_std::vec![0u8;20]));
                Ok(Step::Continue(ip + 2))
            } // ctx_sender -> zero addr
            0x81 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 2))
            } // ctx_block_height
            0x82 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 2))
            } // ctx_timestamp
            0x83 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 2))
            } // ctx_value
            0x84 => {
                let d = self.r8(ip + 1)? as usize;
                set!(
                    d,
                    MiniValue::I64(self.gas_limit as i64 - self.gas_used as i64)
                );
                Ok(Step::Continue(ip + 2))
            } // ctx_gas
            0x85 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(3375));
                Ok(Step::Continue(ip + 2))
            } // chain_id

            // Atomic ops — tracked but no real isolation here (single-threaded WASM)
            0x90..=0x92 => {
                self.r16(ip + 1)?;
                Ok(Step::Continue(ip + 3))
            }
            0x93 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Bool(false));
                Ok(Step::Continue(ip + 2))
            }

            // Agent / emit — skipped
            0xA0 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Unit);
                Ok(Step::Continue(ip + 2))
            }
            0xA1 => {
                // agent_init: [op][agent:u8][field_count:u16][...]
                let _a = self.r8(ip + 1)?;
                let n = self.r16(ip + 2)? as usize;
                Ok(Step::Continue(ip + 4 + n * 3))
            }
            0xA2 => {
                // emit: [op][event_id:u32][argc:u16][args...]
                let argc = self.r16(ip + 5)? as usize;
                Ok(Step::Continue(ip + 7 + argc))
            }

            // VM intrinsics — all return zero/unit
            0xB0..=0xB9 | 0xC0..=0xC7 | 0xD0..=0xD7 => {
                // All EVM / SVM / GPU ops take dst+src operands; return zero and skip sensibly.
                // Simplification: read dst, skip 4 more bytes, return I64(0).
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 6))
            }

            // Debug ops
            0xF0 | 0xF1 => {
                self.r8(ip + 1)?;
                Ok(Step::Continue(ip + 2))
            } // DebugPrint, Breakpoint
            0xF2 => {
                // Assert
                let cond = self.r8(ip + 1)? as usize;
                let _msg = self.r32(ip + 2)?;
                if !rv!(cond).as_bool()? {
                    return Err(X3Error::UserPanic);
                }
                Ok(Step::Continue(ip + 6))
            }
            0xF3 => Err(X3Error::UserPanic), // Panic

            // LoadIndex / StoreIndex / LoadField / StoreField — arrays not implemented in mini
            0x14 | 0x15 => {
                let d = self.r8(ip + 1)? as usize;
                self.r8(ip + 2)?;
                self.r8(ip + 3)?;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 4))
            }
            0x16 | 0x17 => {
                let d = self.r8(ip + 1)? as usize;
                self.r8(ip + 2)?;
                self.r16(ip + 3)?;
                set!(d, MiniValue::I64(0));
                Ok(Step::Continue(ip + 5))
            }

            // ModF (0x34)
            0x34 => {
                let (d, a, b) = (
                    self.r8(ip + 1)? as usize,
                    self.r8(ip + 2)? as usize,
                    self.r8(ip + 3)? as usize,
                );
                let vb = self.regs[reg!(b)].as_f64()?;
                let v = self.regs[reg!(a)].as_f64()? % vb;
                self.regs[reg!(d)] = MiniValue::F64(v);
                Ok(Step::Continue(ip + 4))
            }

            _ => Err(X3Error::InvalidOpcode(op)),
        }
    }
}

fn mini_value_from_const(c: &MiniConst) -> MiniValue {
    match c {
        MiniConst::Integer(v) => MiniValue::I64(*v),
        MiniConst::Float(v) => MiniValue::F64(*v),
        MiniConst::Bool(v) => MiniValue::Bool(*v),
        MiniConst::Bytes(v) => MiniValue::Bytes(v.clone()),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute the first (entry) function in an X3BC module.
///
/// `payload` must be a valid X3BC binary.
/// `gas_limit` caps execution; `GasExhausted` is returned if exceeded.
pub fn execute_x3bc(payload: &[u8], gas_limit: u64) -> Result<X3ExecResult, X3Error> {
    let module = parse_module(payload)?;
    if module.functions.is_empty() {
        return Err(X3Error::FunctionNotFound);
    }
    let mut vm = Vm::new(&module, gas_limit);
    let func_entry = module.functions[0].entry as usize;
    vm.call_stack.push(CallFrame {
        ip: func_entry,
        base: 0,
        ret_addr: usize::MAX,
        func_idx: 0,
    });
    let ret = vm.run()?;
    Ok(X3ExecResult {
        return_val: ret.unwrap_or(MiniValue::Unit),
        gas_used: vm.gas_used,
    })
}

/// Validate the X3BC binary format without executing.
pub fn validate_x3bc(payload: &[u8]) -> Result<(), X3Error> {
    let _ = parse_module(payload)?;
    Ok(())
}

/// Conservative gas estimate based on code section size (EIP-2028-style formula).
pub fn estimate_gas_x3bc(payload: &[u8]) -> u64 {
    // Each instruction costs ~1 gas; code section ≈ bytecount / 4 instructions
    let base: u64 = 21_000;
    base + (payload.len() as u64) * 10
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal X3BC payload:
    ///   LoadImm r0, 42 → Ret r0
    fn make_simple_module() -> Vec<u8> {
        let mut b = Vec::new();
        // Header
        b.extend_from_slice(b"X3BC");
        b.extend_from_slice(&1u32.to_le_bytes()); // version 1.0.0
        b.extend_from_slice(&0u32.to_le_bytes()); // flags
        b.extend_from_slice(&0u32.to_le_bytes()); // checksum (unused in mini)
        b.extend_from_slice(&1u32.to_le_bytes()); // min_version
        b.extend_from_slice(&0u32.to_le_bytes()); // features
                                                  // Const pool (empty)
        b.extend_from_slice(&0u32.to_le_bytes());
        // Function table: 1 function, entry=0, params=0, locals=16
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes()); // name_len = 0 (no name)
        b.extend_from_slice(&0u32.to_le_bytes()); // entry = 0
        b.push(0u8); // param_count = 0
        b.extend_from_slice(&16u16.to_le_bytes()); // local_count = 16
        b.extend_from_slice(&16u16.to_le_bytes()); // max_stack = 16
        b.push(1u8); // return_type_tag = 1 (int)
                     // Global table (empty)
        b.extend_from_slice(&0u32.to_le_bytes());
        // Code: LoadImm r0, 42 (0x18, 0x00, 42) + Ret r0 (0x05, 0x00)
        let code: &[u8] = &[0x18, 0x00, 42, 0x05, 0x00];
        b.extend_from_slice(&(code.len() as u32).to_le_bytes());
        b.extend_from_slice(code);
        // no debug, no metadata
        b.push(0u8);
        b.push(0u8);
        b
    }

    #[test]
    fn test_parse_minimal_module() {
        let payload = make_simple_module();
        assert!(parse_module(&payload).is_ok());
    }

    #[test]
    fn test_execute_returns_42() {
        let payload = make_simple_module();
        let result = execute_x3bc(&payload, 10_000).unwrap();
        assert_eq!(result.return_val, MiniValue::I64(42));
    }

    #[test]
    fn test_gas_exhausted() {
        let payload = make_simple_module();
        // Gas limit of 1 should be exhausted
        let err = execute_x3bc(&payload, 1).unwrap_err();
        assert_eq!(err, X3Error::GasExhausted);
    }

    #[test]
    fn test_invalid_magic() {
        let mut payload = make_simple_module();
        payload[0] = 0xFF;
        assert_eq!(parse_module(&payload).unwrap_err(), X3Error::InvalidMagic);
    }

    #[test]
    fn test_add_operation() {
        // LoadImm r0, 5; LoadImm r1, 3; AddI r2, r0, r1; Ret r2
        // Replace code section
        let code: &[u8] = &[
            0x18, 0x00, 5, // LoadImm r0, 5
            0x18, 0x01, 3, // LoadImm r1, 3
            0x20, 0x02, 0x00, 0x01, // AddI r2, r0, r1
            0x05, 0x02, // Ret r2
        ];
        // Patch code section in the serialized payload
        // Code starts after header(24) + const_pool(4) + func_table(12+11) + globals(4)
        let payload = rebuild_with_code(code);
        let result = execute_x3bc(&payload, 10_000).unwrap();
        assert_eq!(result.return_val, MiniValue::I64(8));
    }

    fn rebuild_with_code(code: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"X3BC");
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // Const pool (empty)
        b.extend_from_slice(&0u32.to_le_bytes());
        // Function table: 1 function, entry=0, params=0, locals=16
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0u8);
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.push(1u8);
        // Global table (empty)
        b.extend_from_slice(&0u32.to_le_bytes());
        // Code
        b.extend_from_slice(&(code.len() as u32).to_le_bytes());
        b.extend_from_slice(code);
        b.push(0u8);
        b.push(0u8);
        b
    }
}
