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

use sp_std::collections::btree_map::BTreeMap;

use crate::types::{X3ExecutionLog, X3StorageWrite};
use sp_core::H256;

// ---------------------------------------------------------------------------
// Contract slot storage
// ---------------------------------------------------------------------------

/// Domain tag that keeps the storage-slot keyspace disjoint from the global keyspace.
///
/// `LoadGlobal`/`StoreGlobal` read their index as a `u32`, so the largest key they can name is
/// `u32::MAX`. This tag is larger, which is what makes the two keyspaces provably disjoint. The
/// value is the same one `crates/x3-vm/src/vm.rs` uses, so both interpreters name the same slot for
/// the same index; `tests/interpreter_agreement.rs` holds them to it.
const EVM_SLOT_DOMAIN: u64 = 0x5833_4556_4D5F_534C; // "X3EVM_SL" little-endian

/// Bytes of data a slot can carry: the 32-byte word minus the tag and length bytes.
const SLOT_PAYLOAD_MAX: usize = 30;

/// Slot payload tags, matching `crates/x3-vm/src/vm.rs`.
///
/// A tag is what makes a store/load round trip exact. Without it a payload cannot tell
/// `Bytes([1, 2])` from `Bytes([1, 2, 0])`, or either from `I64(197_121)`.
/// Bytes of payload one event may carry, and events one execution may journal. A receipt is
/// consensus state, so both are bounded here rather than by whatever gas the program happens to
/// have; the `std` engine's `EventBuffer` applies the same data bound.
const MAX_EVENT_DATA: usize = 4_096;
const MAX_EVENTS: usize = 64;

use x3_common::value_tags::{
    BOOL as SLOT_TAG_BOOL, BYTES as SLOT_TAG_BYTES, F64 as SLOT_TAG_F64, INT as SLOT_TAG_INT,
};

fn evm_slot_key(slot: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&EVM_SLOT_DOMAIN.to_le_bytes());
    key[8..16].copy_from_slice(&slot.to_le_bytes());
    key
}

/// A slot index is a non-negative integer. Refusing anything else by name keeps a negative or
/// fractional "slot" from being reinterpreted as some other key.
fn slot_number(value: &MiniValue) -> X3Result<u64> {
    match value {
        MiniValue::I64(n) if *n >= 0 => Ok(*n as u64),
        MiniValue::I64(_) => Err(X3Error::InvalidStorageSlot("negative slot index")),
        _ => Err(X3Error::InvalidStorageSlot("slot index is not an integer")),
    }
}

/// Encode a value into a 32-byte slot payload, refusing anything that does not fit.
///
/// `crates/x3-vm` records why this is a refusal and not a truncation: a byte-string longer than the
/// slot is a value the program asked to persist and cannot, so reporting success would persist
/// something else. `Unit` is refused for the same reason (`StoreGlobal` ignores it, which is a
/// different, also-wrong behaviour).
/// A value as `(tag, data)`, the form both a storage slot and an event payload are built from.
///
/// The `std` engine has the same function over its own value type; `interpreter_agreement.rs`
/// holds the two to the same answers, as it already does for the slot journals.
fn tagged_value(value: &MiniValue) -> X3Result<(u8, Vec<u8>)> {
    let (tag, data): (u8, Vec<u8>) = match value {
        MiniValue::I64(n) => (SLOT_TAG_INT, n.to_le_bytes().to_vec()),
        MiniValue::Bool(b) => (SLOT_TAG_BOOL, vec![u8::from(*b)]),
        MiniValue::F64(f) => (SLOT_TAG_F64, f.to_bits().to_le_bytes().to_vec()),
        MiniValue::Bytes(bytes) => (SLOT_TAG_BYTES, bytes.clone()),
        MiniValue::Unit => {
            return Err(X3Error::UnencodableStorageValue(
                "unit is not a storable value",
            ))
        }
    };
    if data.len() > u8::MAX as usize {
        return Err(X3Error::UnencodableStorageValue(
            "value is longer than a tagged value carries",
        ));
    }
    Ok((tag, data))
}

/// Encode a value into a 32-byte slot payload, refusing anything that does not fit.
///
/// `crates/x3-vm` records why this is a refusal and not a truncation: a byte-string longer than the
/// slot is a value the program asked to persist and cannot, so reporting success would persist
/// something else. `Unit` is refused for the same reason (`StoreGlobal` ignores it, which is a
/// different, also-wrong behaviour).
fn encode_slot_payload(value: &MiniValue) -> X3Result<[u8; 32]> {
    let (tag, data) = tagged_value(value)?;
    if data.len() > SLOT_PAYLOAD_MAX {
        return Err(X3Error::UnencodableStorageValue(
            "byte-string is longer than a slot holds",
        ));
    }
    let mut out = [0u8; 32];
    out[0] = tag;
    out[1] = data.len() as u8;
    out[2..2 + data.len()].copy_from_slice(&data);
    Ok(out)
}

/// Decode a slot payload back into the exact value that was stored.
///
/// An unknown tag, a length past the slot's capacity or a fixed-width kind with the wrong width is
/// refused rather than guessed at: a payload this ISA did not write is not evidence of anything, so
/// the read fails closed instead of handing the program a fabricated value.
fn decode_slot_payload(payload: &[u8; 32]) -> X3Result<MiniValue> {
    let len = payload[1] as usize;
    if len > SLOT_PAYLOAD_MAX {
        return Err(X3Error::CorruptStorageSlot);
    }
    let body = &payload[2..2 + len];
    let width = |w: usize| -> X3Result<&[u8]> {
        if body.len() == w {
            Ok(body)
        } else {
            Err(X3Error::CorruptStorageSlot)
        }
    };
    match payload[0] {
        SLOT_TAG_INT => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(width(8)?);
            Ok(MiniValue::I64(i64::from_le_bytes(bytes)))
        }
        SLOT_TAG_BOOL => match width(1)?[0] {
            0 => Ok(MiniValue::Bool(false)),
            1 => Ok(MiniValue::Bool(true)),
            _ => Err(X3Error::CorruptStorageSlot),
        },
        SLOT_TAG_F64 => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(width(8)?);
            Ok(MiniValue::F64(f64::from_bits(u64::from_le_bytes(bytes))))
        }
        SLOT_TAG_BYTES => Ok(MiniValue::Bytes(body.to_vec())),
        _ => Err(X3Error::CorruptStorageSlot),
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X3Error {
    InvalidMagic,
    /// The envelope declares a format version this loader cannot read, or requires a newer loader.
    UnsupportedVersion(u32),
    /// The header's checksum does not match the body — corrupted in transit or edited.
    ChecksumMismatch {
        expected: u32,
        found: u32,
    },
    UnexpectedEof,
    /// A constant-pool entry declares a tag this loader does not know. The body is
    /// malformed rather than truncated, and the module must not be executed.
    InvalidConstTag(u8),
    InvalidOpcode(u8),
    /// A defined opcode that this interpreter has no implementation for.
    ///
    /// The distinction from [`X3Error::InvalidOpcode`] is the point: `InvalidOpcode` means the byte
    /// is not an opcode at all, while this means the byte *is* one — the parser, the verifier and
    /// the compiler all agree on its encoding — and the runtime still cannot execute it. Refusing
    /// by name is the only honest answer; answering a placeholder would put a fabricated value into
    /// consensus state. The `std` VM refuses the same bytes with
    /// `x3_vm::VMErrorKind::UnimplementedOpcode` (`crates/x3-vm/src/vm.rs`).
    UnsupportedOpcode(u8),
    /// `AtomicCommit` with no window open.
    AtomicEndWithoutBegin,
    /// `AtomicRollback` with no window open.
    AtomicRollbackWithoutBegin,
    /// An atomic window rolled back: the window's writes are undone and the execution stops, which
    /// is what `crates/x3-vm` does (`VMErrorKind::AtomicAborted`).
    AtomicAborted,
    /// More atomic windows were opened than [`MAX_ATOMIC_DEPTH`] allows. Each open window holds a
    /// copy of the module's globals, so an unbounded nesting in a loop is a memory amplification a
    /// hostile program could ask for; the limit refuses it instead.
    AtomicDepthExceeded,
    /// `evm_sstore`/`evm_sload` with a slot register that does not hold a non-negative integer.
    InvalidStorageSlot(&'static str),
    /// A value that cannot be carried by a 32-byte slot, refused rather than truncated.
    UnencodableStorageValue(&'static str),
    /// An `emit` the engine refused rather than journalling something the program did not say.
    UnjournalableEvent(&'static str),
    /// A slot payload whose tag, length or width this ISA did not write, so it carries no value.
    CorruptStorageSlot,
    DivisionByZero,
    GasExhausted,
    StackOverflow,
    StackUnderflow,
    TypeMismatch,
    ConstPoolOutOfBounds,
    FunctionNotFound,
    GlobalOutOfBounds,
    RegisterOutOfBounds,
    /// A call passed a different number of arguments than the callee declares parameters.
    ArgumentCountMismatch,
    /// The module is well-formed but not admissible on chain (see `validate_x3bc`).
    ForbiddenOnChain(u8),
    /// A jump or call target, or a function entry, that is not the start of an instruction.
    InvalidJumpTarget(u32),
    UserPanic,
    /// The artifact's compiled policy requires private submission and this execution context has no
    /// private channel to offer it.
    ///
    /// Refused rather than run: the alternative is executing a program that asked not to be public
    /// in the clear, which is the one direction a compiled privacy policy must never take. It is the
    /// engine's own copy of the check `pallet-x3-kernel` performs at intake, so an artifact that
    /// reaches this interpreter by any route is still refused.
    PrivateSubmissionRequired,
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
    /// The compiled policy's demands, read from the header rather than skipped.
    ///
    /// This field is here because the reader used to read the feature word and *discard* it
    /// (`let _features = ...`), which made the artifact's policy a value nobody in the runtime
    /// consulted. A demand the engine cannot see is a demand the engine cannot honour.
    requires_private_submission: bool,
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
        // `n` comes from the input, and the runtime is wasm32: `pos + n` can wrap a 32-bit
        // `usize`, pass a `> len` check, and then panic in the slice. Compare against what is left.
        if n > self.data.len().saturating_sub(self.pos) {
            return Err(X3Error::UnexpectedEof);
        }
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }
    fn peek_u8(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }
    fn skip(&mut self, n: usize) -> X3Result<()> {
        if n > self.data.len().saturating_sub(self.pos) {
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
    if magic != x3_common::bytecode::MAGIC {
        return Err(X3Error::InvalidMagic);
    }

    // The rest of the header used to be skipped (`r.skip(20)`): version, flags, checksum,
    // min-version and feature flags all read past and never checked, so a module with a future
    // version or a body corrupted after compilation was accepted as long as it parsed (TICKET-108).
    // The writer emits all of it, and the definitions live in `x3-common` so this reader and
    // `x3-backend` cannot disagree about them.
    let version = r.read_u32()?;
    let _flags = r.read_u32()?;
    let checksum = r.read_u32()?;
    let min_version = r.read_u32()?;
    let features = r.read_u32()?;
    let requires_private_submission =
        features & x3_common::bytecode::FEATURE_PRIVATE_SUBMISSION_REQUIRED != 0;

    if !x3_common::bytecode::version_is_readable(version) {
        return Err(X3Error::UnsupportedVersion(version));
    }
    if !x3_common::bytecode::loader_satisfies(min_version) {
        // The module says it needs a loader at least this new; this one is older.
        return Err(X3Error::UnsupportedVersion(min_version));
    }
    // This reader predates the trading extension (format 1.1) and has no implementation of it:
    // it refuses a module that declares it, by the format version that introduced it, rather than
    // reading past a section it cannot interpret.
    if features & x3_common::bytecode::FEATURE_TRADING_EXTENSION != 0 {
        return Err(X3Error::UnsupportedVersion(
            x3_common::bytecode::TRADING_VERSION,
        ));
    }
    let expected = x3_common::bytecode::checksum(&bytes[x3_common::bytecode::HEADER_LEN..]);
    if checksum != expected {
        return Err(X3Error::ChecksumMismatch {
            expected,
            found: checksum,
        });
    }

    // Const pool
    let const_count = r.read_u32()? as usize;
    // `with_capacity` on a count read straight out of the module lets a four-byte
    // field ask for a tens-of-gigabytes allocation: `const_count = 0xFF00_0001` aborted
    // the on-chain reader with "memory allocation of 102676561944 bytes failed"
    // (crates/x3-integration/tests/bytecode_robustness.rs). No entry is smaller than a
    // byte, so the bytes that are left bound the count; the loop below still reports
    // EOF on its own terms if the module is truncated.
    let mut const_pool = Vec::with_capacity(const_count.min(r.remaining()));
    for _ in 0..const_count {
        let tag = r.read_u8()?;
        let c = match tag {
            0 => MiniConst::Integer(r.read_i64()?),
            1 => MiniConst::Float(r.read_f64()?),
            2 => {
                // A string and a byte blob are both a length-prefixed byte run; tag 2 only
                // promises the payload is UTF-8 text. This arm used to `skip` the payload and
                // push an empty `Vec`, so every string constant executed as the empty value
                // while `x3-backend` (std) handed the same module the real string. The payload
                // is kept as bytes: `MiniValue` has no separate string case, and keeping it
                // lossless costs nothing here.
                let len = r.read_u32()? as usize;
                let text = r.read_bytes(len)?;
                MiniConst::Bytes(text)
            }
            3 => MiniConst::Bool(r.read_u8()? != 0),
            4 => {
                let len = r.read_u32()? as usize;
                let b = r.read_bytes(len)?;
                MiniConst::Bytes(b)
            }
            // An unknown tag is a malformed body, not a truncated one; reporting EOF here sent
            // callers looking for a byte that was never missing.
            other => return Err(X3Error::InvalidConstTag(other)),
        };
        const_pool.push(c);
    }

    // Function table
    let func_count = r.read_u32()? as usize;
    // `with_capacity` on a count read straight out of the module lets a four-byte
    // field ask for a tens-of-gigabytes allocation: `const_count = 0xFF00_0001` aborted
    // the on-chain reader with "memory allocation of 102676561944 bytes failed"
    // (crates/x3-integration/tests/bytecode_robustness.rs). No entry is smaller than a
    // byte, so the bytes that are left bound the count; the loop below still reports
    // EOF on its own terms if the module is truncated.
    let mut functions = Vec::with_capacity(func_count.min(r.remaining()));
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
    // `with_capacity` on a count read straight out of the module lets a four-byte
    // field ask for a tens-of-gigabytes allocation: `const_count = 0xFF00_0001` aborted
    // the on-chain reader with "memory allocation of 102676561944 bytes failed"
    // (crates/x3-integration/tests/bytecode_robustness.rs). No entry is smaller than a
    // byte, so the bytes that are left bound the count; the loop below still reports
    // EOF on its own terms if the module is truncated.
    let mut globals = Vec::with_capacity(global_count.min(r.remaining()));
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

    // Trailing sections. The runtime does not use them, but it has to accept exactly the modules
    // `x3-backend` accepts: this reader used to stop after the code, so a module whose debug
    // section was malformed was refused by the std reader and run by the chain.
    skip_trailer(&mut r)?;

    Ok(MiniModule {
        const_pool,
        functions,
        globals,
        code,
        requires_private_submission,
    })
}

/// Read past the optional debug and metadata sections with `x3-backend`'s acceptance rules
/// (`BytecodeModule::from_bytes`): a section is present when its flag byte is 1, any other flag
/// byte means absent, and a present section must be complete.
fn skip_trailer(r: &mut Reader<'_>) -> X3Result<()> {
    fn skip_u16_prefixed(r: &mut Reader<'_>) -> X3Result<()> {
        let len = r.read_u16()? as usize;
        r.skip(len)
    }
    fn skip_optional_string(r: &mut Reader<'_>) -> X3Result<()> {
        if r.read_u8()? == 1 {
            skip_u16_prefixed(r)?;
        }
        Ok(())
    }

    // Debug info.
    if r.remaining() > 0 && r.read_u8()? == 1 {
        let map_count = r.read_u32()? as usize;
        for _ in 0..map_count {
            r.skip(8)?; // code offset u32, line u16, column u16
        }
        let name_count = r.read_u32()? as usize;
        for _ in 0..name_count {
            r.skip(4)?; // symbol index
            skip_u16_prefixed(r)?;
        }
    }
    // Metadata.
    if r.remaining() > 0 && r.peek_u8() == Some(1) {
        r.skip(1)?;
        skip_u16_prefixed(r)?; // compiler
        skip_u16_prefixed(r)?; // compiler version
        r.skip(8)?; // compiled at
        skip_optional_string(r)?; // source file
        skip_optional_string(r)?; // source hash
        r.skip(1)?; // opt level
        let annotations = r.read_u32()? as usize;
        for _ in 0..annotations {
            skip_u16_prefixed(r)?;
            skip_u16_prefixed(r)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Execution result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct X3ExecResult {
    pub return_val: MiniValue,
    pub gas_used: u64,
    /// Instructions executed.
    ///
    /// This interpreter charges one gas per instruction, so a receipt built from `gas_used` looked
    /// plausible while reporting a gas figure under an instruction name; the two are counted
    /// separately now (TICKET-130).
    pub instructions_executed: u64,
    /// Slot writes the execution left behind, in order.
    ///
    /// A write abandoned by a rolled-back atomic window is not here at all, and a failed execution
    /// returns `Err` rather than a partial journal, so every entry is a write that survived. Slot
    /// keys are the 32-byte EVM-domain keys (`evm_slot_key`), disjoint from the global keyspace, so
    /// the kernel can apply them to its contract-storage map instead of the balance ledger.
    pub storage_writes: Vec<X3StorageWrite>,
    /// Events the execution journalled, in order.
    ///
    /// Bounded like the slot journal: an execution that fails returns `Err` rather than a partial
    /// list, so every entry is an event that survived.
    pub events: Vec<X3ExecutionLog>,
}

// ---------------------------------------------------------------------------
// VM
// ---------------------------------------------------------------------------

struct CallFrame {
    ip: usize,
    base: usize,
    ret_addr: usize,
    func_idx: usize,
    /// Register in the caller's frame the result is written to — the `Call`'s `dst` operand. Both
    /// interpreters used to drop it and write the caller's `r0` instead; see the note on the std
    /// VM's `Frame::ret_dst` (TICKET-131).
    ret_dst: usize,
}

const MAX_REGS: usize = 256;
const MAX_DEPTH: usize = 64;

/// Deepest nesting of atomic windows the runtime interpreter will track.
///
/// Every open window holds a copy of the module's globals, so this bounds the memory a program can
/// make the interpreter hold on the chain's behalf.
const MAX_ATOMIC_DEPTH: usize = 32;

/// One open atomic window: what it has to undo if it rolls back.
struct AtomicFrame {
    /// The module's globals when the window opened.
    globals: Vec<MiniValue>,
    /// Index into `writes` of this window's first write.
    first_write: usize,
    /// Index into `undo_log` of this window's first undo entry.
    first_undo: usize,
}

struct Vm<'m> {
    module: &'m MiniModule,
    regs: Vec<MiniValue>,
    call_stack: Vec<CallFrame>,
    globals: Vec<MiniValue>,
    /// This execution's view of the contract slots, keyed by the same 32-byte key `crates/x3-vm`
    /// uses. It starts as the chain's `X3ContractStorage` ([`execute_x3bc_with_slots`] seeds it),
    /// so a slot a *previous* comit wrote reads back as that value rather than EVM's zero, and an
    /// entry's `old_value` is the value the execution started from. Seeded entries are not
    /// journaled: they are the state this execution inherited, not a change it made.
    slots: BTreeMap<[u8; 32], [u8; 32]>,
    /// Events this execution journalled, in order (see opcode `0xA2`).
    events: Vec<X3ExecutionLog>,
    /// Every slot write this execution has made, in order.
    writes: Vec<X3StorageWrite>,
    /// `(key, previous)` for every write, so a rolled-back window is undone in reverse without
    /// copying the whole slot map for each window.
    undo_log: Vec<([u8; 32], Option<[u8; 32]>)>,
    /// Open atomic windows, innermost last.
    atomic_frames: Vec<AtomicFrame>,
    gas_used: u64,
    /// Instructions executed, counted next to gas rather than inferred from it.
    instructions_executed: u64,
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
            slots: BTreeMap::new(),
            events: Vec::new(),
            writes: Vec::new(),
            undo_log: Vec::new(),
            atomic_frames: Vec::new(),
            gas_used: 0,
            instructions_executed: 0,
            gas_limit,
        }
    }

    /// Load the chain's contract slots into this execution's view.
    ///
    /// Deliberately not a write: nothing is appended to `writes` or `undo_log`, so a receipt never
    /// reports the chain's own state as a change this program made, and an atomic window that rolls
    /// back restores the seeded value rather than deleting the slot. A payload the chain holds that
    /// this ISA did not write is *not* rejected here — it is rejected when a program reads it
    /// (`CorruptStorageSlot`), so an execution that never touches a corrupt slot still succeeds
    /// while an execution that reads one fails closed.
    fn seed_slots(&mut self, seeds: &[([u8; 32], [u8; 32])]) {
        for (key, payload) in seeds {
            self.slots.insert(*key, *payload);
        }
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
            self.instructions_executed += 1;

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
                        // The caller named the destination register in the `Call`'s `dst` operand;
                        // it is frame-relative, like every other register operand.
                        match self.call_stack.last() {
                            Some(caller) => {
                                let idx = caller.base + frame.ret_dst;
                                if idx >= self.regs.len() {
                                    return Err(X3Error::RegisterOutOfBounds);
                                }
                                self.regs[idx] = v;
                            }
                            None => self.regs[0] = v,
                        }
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

        // Helper: absolute register index from relative reg operand, refused when it falls outside
        // the register file. Operands are a `u8` added to the frame's base, so a callee frame could
        // name a register past the end of `regs`, and indexing it panicked — inside the runtime,
        // on bytes an extrinsic supplied. This is the rule `x3-vm` applies (`resolve_reg_checked`).
        macro_rules! reg {
            ($r:expr) => {{
                let index = base + $r as usize;
                if index >= MAX_REGS {
                    return Err(X3Error::RegisterOutOfBounds);
                }
                index
            }};
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
                let dst = self.r8(ip + 1)? as usize;
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
                // collect args (from caller base). `argc` is a u16 read out of the code
                // stream, so this cannot ask for gigabytes the way the table counts above
                // could, but it is still an allocation sized by input: the register file is
                // the real bound, and the loop below refuses an index outside it.
                let mut args = Vec::with_capacity(argc.min(MAX_REGS));
                for i in 0..argc {
                    let ar = self.r8(ip + 8 + i)? as usize;
                    args.push(self.regs[reg!(ar)].clone());
                }
                // The callee's window starts after the caller's whole frame (params + locals), not
                // after its locals alone: see the note in the std VM's `Call` arm (TICKET-131).
                let (caller_base, caller_footprint) = self
                    .call_stack
                    .last()
                    .map(|f| {
                        let entry = &self.module.functions[f.func_idx];
                        (
                            f.base,
                            entry.param_count as usize + entry.local_count as usize,
                        )
                    })
                    .unwrap_or((0, 0));
                let callee_base = caller_base + caller_footprint;
                // A call passes exactly the callee's parameters. The arguments are written into the
                // callee's window below, and `argc` is a `u16` from the code stream, so a larger
                // count wrote past the register file (a panic) and a smaller one left parameters
                // holding whatever the previous frame left there.
                if argc != func.param_count as usize {
                    return Err(X3Error::ArgumentCountMismatch);
                }
                // The whole callee window — parameters and locals — has to fit. When it does not,
                // the call stack's register budget is spent: that is a stack overflow, reported as
                // one, not a bad register operand (which would be a compiler fault).
                if callee_base + func.param_count as usize + func.local_count as usize > MAX_REGS {
                    return Err(X3Error::StackOverflow);
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
                    ret_dst: dst,
                });
                Ok(Step::Continue(func.entry as usize))
            }
            0x05 => {
                // Ret
                let s = self.r8(ip + 1)? as usize;
                let v = self.regs[reg!(s)].clone();
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
                // `i64::MIN / -1` panics in Rust, release builds included, and this engine runs
                // inside the runtime: one extrinsic dividing those two values would abort block
                // execution. Wrapping matches `x3-vm` and the other integer opcodes.
                let v = self.regs[reg!(a)].as_i64()?.wrapping_div(vb);
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
                // `i64::MIN % -1` panics likewise; it wraps to 0.
                let v = self.regs[reg!(a)].as_i64()?.wrapping_rem(vb);
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 4))
            }
            0x25 => {
                let (d, s) = (self.r8(ip + 1)? as usize, self.r8(ip + 2)? as usize);
                let v = self.regs[reg!(s)].as_i64()?.wrapping_neg();
                self.regs[reg!(d)] = MiniValue::I64(v);
                Ok(Step::Continue(ip + 3))
            } // NegI

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

            // -------- Type Conversions: refused (see `verify_code`, TICKET-149) --------
            0x60..=0x68 => Err(X3Error::UnsupportedOpcode(op)),

            // -------- Aggregates: refused --------
            //
            // `LoadIndex`/`StoreIndex`/`LoadField`/`StoreField` and the array/tuple opcodes used to
            // answer `I64(0)`/`Unit` and skip their operands. That is a fabricated value in
            // consensus state: a program reading `array[0]` got `0`, one storing into an array
            // changed nothing and reported success, and `ArrayLen` answered `0` for any array. The
            // interpreter has no aggregate representation, so the only honest answer is a refusal
            // that names the opcode.
            0x14 | 0x15 | 0x16 | 0x17 | 0x70..=0x75 => Err(X3Error::UnsupportedOpcode(op)),

            // -------- Context reads --------
            // None has a source of truth in the runtime: nothing hands `mini_x3` a sender, a block
            // height, a timestamp, a value or the chain id, so answering one invents it (`0x85`
            // used to answer a hard-coded `3375`). `ctx_gas` (0x84) was answered from this
            // engine's own budget; it is refused now because the other engine may count gas
            // differently (see `verify_code`, TICKET-149).
            0x80..=0x85 => Err(X3Error::UnsupportedOpcode(op)),

            // -------- Atomic windows --------
            //
            // These were skipped while claiming to be tracked, so an atomic block that failed still
            // committed whatever it had written. The only state this interpreter can revert is the
            // module's globals, and it now does exactly that, mirroring `crates/x3-vm/src/vm.rs`:
            // begin snapshots, commit discards, rollback restores and aborts the execution.
            0x90 => {
                self.r16(ip + 1)?; // the window id, validated like the `std` VM validates it
                if self.atomic_frames.len() >= MAX_ATOMIC_DEPTH {
                    return Err(X3Error::AtomicDepthExceeded);
                }
                let frame = AtomicFrame {
                    globals: self.globals.clone(),
                    first_write: self.writes.len(),
                    first_undo: self.undo_log.len(),
                };
                self.atomic_frames.push(frame);
                Ok(Step::Continue(ip + 3))
            }
            0x91 => {
                self.r16(ip + 1)?;
                if self.atomic_frames.pop().is_none() {
                    return Err(X3Error::AtomicEndWithoutBegin);
                }
                Ok(Step::Continue(ip + 3))
            }
            0x92 => {
                self.r16(ip + 1)?;
                let frame = match self.atomic_frames.pop() {
                    Some(frame) => frame,
                    None => return Err(X3Error::AtomicRollbackWithoutBegin),
                };
                // A write the window rolled back must never reach the journal a receipt is built
                // from, so the slots it touched are restored in reverse order and the entries it
                // added are dropped. Undoing and forgetting are one step on purpose: a caller that
                // only reset the map would still report the writes.
                while self.undo_log.len() > frame.first_undo {
                    if let Some((key, previous)) = self.undo_log.pop() {
                        match previous {
                            Some(value) => {
                                self.slots.insert(key, value);
                            }
                            None => {
                                self.slots.remove(&key);
                            }
                        }
                    }
                }
                self.writes.truncate(frame.first_write);
                self.globals = frame.globals;
                Err(X3Error::AtomicAborted)
            }
            0x93 => {
                let d = self.r8(ip + 1)? as usize;
                set!(d, MiniValue::Bool(!self.atomic_frames.is_empty()));
                Ok(Step::Continue(ip + 2))
            }

            // -------- Agents and events: refused --------
            // There is no agent registry in the runtime interpreter, and no event sink: `0xA0` used
            // to answer `Unit` (an invented identity) and `0xA2` dropped the event while reporting
            // success. A dropped event is lost evidence, so it is refused instead.
            0xA0 | 0xA1 => Err(X3Error::UnsupportedOpcode(op)),

            // `emit name:reg argc:u16 args:reg*`. Implemented in both engines now: this arm and
            // the `std` VM's build the same journal entry — the topic is SHA-256 of the event's
            // name, the payload is each argument as `[tag][len][data]`.
            0xA2 => {
                let name_reg = self.r8(ip + 1)? as usize;
                let argc = self.r16(ip + 2)? as usize;
                let name = match &self.regs[reg!(name_reg)] {
                    MiniValue::Bytes(bytes) => bytes.clone(),
                    _ => {
                        return Err(X3Error::UnjournalableEvent(
                            "an event's name must be a byte string",
                        ))
                    }
                };
                if name.is_empty() {
                    return Err(X3Error::UnjournalableEvent("an event must be named"));
                }
                let mut data = Vec::new();
                for index in 0..argc {
                    let arg = self.r8(ip + 4 + index)? as usize;
                    let (tag, bytes) = tagged_value(&self.regs[reg!(arg)])?;
                    data.push(tag);
                    data.push(bytes.len() as u8);
                    data.extend_from_slice(&bytes);
                }
                if data.len() > MAX_EVENT_DATA {
                    return Err(X3Error::UnjournalableEvent("event payload is too large"));
                }
                if self.events.len() >= MAX_EVENTS {
                    return Err(X3Error::UnjournalableEvent(
                        "too many events in one execution",
                    ));
                }
                self.events.push(X3ExecutionLog {
                    topic: H256::from(sp_io::hashing::sha2_256(&name)),
                    data,
                });
                Ok(Step::Continue(ip + 4 + argc))
            }

            // -------- Contract slot storage --------
            //
            // `evm_sstore` / `evm_sload` are the only way an `.x3` program carries a value from one
            // execution to the next, and this interpreter used to refuse both — so no program
            // running on a block could change chain state at all, and the kernel's storage-write
            // channel had no producer on the canonical path. They are implemented here against the
            // same key derivation and the same tagged payload layout `crates/x3-vm` uses, so a slot
            // written off chain and one written on chain name the same 32 bytes.

            // EvmSstore: [op][slot_reg:u8][value_reg:u8]
            0xB4 => {
                let slot_reg = self.r8(ip + 1)? as usize;
                let value_reg = self.r8(ip + 2)? as usize;
                let slot = slot_number(&self.regs[reg!(slot_reg)])?;
                let payload = encode_slot_payload(&self.regs[reg!(value_reg)])?;
                let key = evm_slot_key(slot);
                let previous = self.slots.insert(key, payload);
                self.undo_log.push((key, previous));
                self.writes.push(X3StorageWrite {
                    key: key.into(),
                    old_value: previous,
                    new_value: Some(payload),
                });
                Ok(Step::Continue(ip + 3))
            }

            // EvmSload: [op][dst_reg:u8][slot_reg:u8]
            0xB3 => {
                let dst_reg = self.r8(ip + 1)? as usize;
                let slot_reg = self.r8(ip + 2)? as usize;
                let slot = slot_number(&self.regs[reg!(slot_reg)])?;
                let key = evm_slot_key(slot);
                // EVM's rule for a slot that was never written: it reads as zero. The interpreter
                // starts with no state, so "never written" is the only empty case it can have, and
                // a payload it did not write is refused rather than decoded loosely.
                let value = match self.slots.get(&key) {
                    Some(payload) => decode_slot_payload(payload)?,
                    None => MiniValue::I64(0),
                };
                set!(dst_reg, value);
                Ok(Step::Continue(ip + 3))
            }

            // -------- Cross-VM and GPU intrinsics: refused --------
            //
            // `AGENTS.md` §18 — an accelerator must never be accepted merely because it returned —
            // and §5 — uncertainty must not become success. These arms answered `I64(0)` and
            // skipped six bytes, which was also the wrong width for most of them
            // (`SvmCreateAccount` is seven), so a program that contained one could have the *next*
            // instruction decoded from the middle of its operands. No host for any of these exists
            // in the runtime build; each is refused by name.
            0xB0..=0xB2 | 0xB5..=0xB9 | 0xC0..=0xC7 | 0xD0..=0xD7 => {
                Err(X3Error::UnsupportedOpcode(op))
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
    execute_x3bc_with_slots(payload, gas_limit, &[])
}

/// Execute the first (entry) function in an X3BC module with the chain's contract slots visible.
///
/// `seeds` is the chain's `X3ContractStorage`: the 32-byte slot keys and the payloads a previous
/// execution persisted. Seeding them is what lets a *second* comit read what the first one wrote —
/// without it `evm_sload` saw the interpreter's own empty map and answered EVM's zero for a slot
/// the chain had held a value in since an earlier block, so contract state existed on the chain and
/// was invisible to the contract that owned it.
///
/// The seeds carry no authority of their own: a payload whose tag, length or width this ISA did not
/// write is refused with [`X3Error::CorruptStorageSlot`] when a program reads it, and a seeded slot
/// a program does not touch costs it nothing.
pub fn execute_x3bc_with_slots(
    payload: &[u8],
    gas_limit: u64,
    seeds: &[([u8; 32], [u8; 32])],
) -> Result<X3ExecResult, X3Error> {
    execute_x3bc_with_slots_and_policy(payload, gas_limit, seeds, false)
}

/// Execute an X3BC module, honouring the compiled policy in its header.
///
/// `private_channel_available` is the *execution context's* answer to "can this program be
/// submitted privately here?" — for a block that is the chain's own posture
/// (`pallet-x3-kernel::Config::PrivateSubmissionChannel`, bound in the runtime to
/// `pallet_private_execution::Enabled`). A module whose header demands private submission is
/// refused with [`X3Error::PrivateSubmissionRequired`] when the answer is `false`, and runs when it
/// is `true`.
///
/// The [`execute_x3bc_with_slots`] wrapper above passes `false`, which is the fail-closed default: a
/// caller that has not said it can offer a private channel has not offered one.
pub fn execute_x3bc_with_slots_and_policy(
    payload: &[u8],
    gas_limit: u64,
    seeds: &[([u8; 32], [u8; 32])],
    private_channel_available: bool,
) -> Result<X3ExecResult, X3Error> {
    let module = parse_module(payload)?;
    if module.requires_private_submission && !private_channel_available {
        return Err(X3Error::PrivateSubmissionRequired);
    }
    if module.functions.is_empty() {
        return Err(X3Error::FunctionNotFound);
    }
    // Execution admits exactly what validation admits: the runtime validates before it executes,
    // and so does this, so no caller can run a module the validator would refuse.
    verify_code(&module)?;
    let mut vm = Vm::new(&module, gas_limit);
    vm.seed_slots(seeds);
    let func_entry = module.functions[0].entry as usize;
    vm.call_stack.push(CallFrame {
        ret_dst: 0,
        ip: func_entry,
        base: 0,
        ret_addr: usize::MAX,
        func_idx: 0,
    });
    let ret = vm.run()?;
    Ok(X3ExecResult {
        return_val: ret.unwrap_or(MiniValue::Unit),
        gas_used: vm.gas_used,
        instructions_executed: vm.instructions_executed,
        storage_writes: vm.writes,
        events: vm.events,
    })
}

/// Read the X3BC envelope and tables — the format only, not the code — without executing.
///
/// This is the counterpart of `x3-backend`'s `BytecodeModule::from_bytes`, and the two are held to
/// agreeing on every input (`tests/bytecode_robustness.rs`). Whether the module may *run* on chain
/// is `validate_x3bc`'s question.
pub fn read_x3bc(payload: &[u8]) -> Result<(), X3Error> {
    parse_module(payload).map(|_| ())
}

/// Validate an X3BC module for on-chain execution, without executing it.
///
/// This used to parse the envelope and stop, so the runtime — whose X3 adapter validates with this
/// function in the wasm build — admitted any instruction stream behind a well-formed header, while
/// the std build refused the same bytes with `x3-vm`'s verifier. Beyond the envelope this now checks
/// the code the way the interpreter will read it:
///
/// - the code decodes from its first byte to its last into instructions this engine implements;
/// - every function entry and every jump target is the start of an instruction;
/// - every call names an existing function with exactly its parameter count;
/// - every constant and global index is in range;
/// - the entry function (index 0, which the runtime calls with no arguments) has no parameters,
///   and every function's frame fits the register file;
/// - float arithmetic, which the on-chain verifier of `x3-vm` forbids, is refused.
pub fn validate_x3bc(payload: &[u8]) -> Result<(), X3Error> {
    let module = parse_module(payload)?;
    verify_code(&module)
}

/// The length of the instruction at `ip`, or an error if it is not one this engine implements.
fn instruction_len(code: &[u8], ip: usize) -> Result<usize, X3Error> {
    let op = *code.get(ip).ok_or(X3Error::UnexpectedEof)?;
    // One row per arm of `Vm::exec`: the implemented opcodes with their widths, the opcodes it
    // refuses by name, and everything else is not an opcode.
    let len = match op {
        0x00 | 0x06 | 0x07 | 0xF3 => 1,
        0x05 | 0x19..=0x1B | 0x84 | 0x93 | 0xF0 | 0xF1 => 2,
        0x11 | 0x18 | 0x25 | 0x35 | 0x53 | 0x5A | 0x60..=0x68 | 0x90..=0x92 | 0xB3 | 0xB4 => 3,
        0x20..=0x24 | 0x30..=0x34 | 0x40..=0x4B | 0x50..=0x52 | 0x54..=0x56 | 0x58 | 0x59 => 4,
        0x01 => 5,
        0x02 | 0x03 | 0x10 | 0x12 | 0x13 | 0xF2 => 6,
        0x04 => {
            let argc = code
                .get(ip + 6..ip + 8)
                .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
                .ok_or(X3Error::UnexpectedEof)?;
            8 + argc
        }
        0xA2 => {
            let argc = code
                .get(ip + 2..ip + 4)
                .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
                .ok_or(X3Error::UnexpectedEof)?;
            4 + argc
        }
        0x14..=0x17
        | 0x70..=0x75
        | 0x80..=0x83
        | 0x85
        | 0xA0
        | 0xA1
        | 0xB0..=0xB2
        | 0xB5..=0xB9
        | 0xC0..=0xC7
        | 0xD0..=0xD7 => return Err(X3Error::UnsupportedOpcode(op)),
        _ => return Err(X3Error::InvalidOpcode(op)),
    };
    if ip + len > code.len() {
        return Err(X3Error::UnexpectedEof);
    }
    Ok(len)
}

fn read_u32_at(code: &[u8], at: usize) -> u32 {
    // Only called on an instruction `instruction_len` has already bounded.
    u32::from_le_bytes([code[at], code[at + 1], code[at + 2], code[at + 3]])
}

fn verify_code(module: &MiniModule) -> Result<(), X3Error> {
    let code = &module.code;
    let entry = module.functions.first().ok_or(X3Error::FunctionNotFound)?;
    if entry.param_count != 0 {
        // The runtime calls the entry with no arguments; `x3-vm` refuses the same module.
        return Err(X3Error::ArgumentCountMismatch);
    }
    for function in &module.functions {
        if function.param_count as usize + function.local_count as usize > MAX_REGS {
            return Err(X3Error::RegisterOutOfBounds);
        }
    }

    // Pass 1: decode, and record where instructions start.
    let mut starts = vec![false; code.len()];
    let mut ip = 0;
    while ip < code.len() {
        starts[ip] = true;
        ip += instruction_len(code, ip)?;
    }
    let is_start = |target: u32| (target as usize) < code.len() && starts[target as usize];

    for function in &module.functions {
        if !is_start(function.entry) {
            return Err(X3Error::InvalidJumpTarget(function.entry));
        }
    }

    // Pass 2: operands.
    let mut ip = 0;
    while ip < code.len() {
        let op = code[ip];
        let len = instruction_len(code, ip)?;
        match op {
            0x01 => {
                let target = read_u32_at(code, ip + 1);
                if !is_start(target) {
                    return Err(X3Error::InvalidJumpTarget(target));
                }
            }
            0x02 | 0x03 => {
                let target = read_u32_at(code, ip + 2);
                if !is_start(target) {
                    return Err(X3Error::InvalidJumpTarget(target));
                }
            }
            0x04 => {
                let callee = read_u32_at(code, ip + 2) as usize;
                let argc = len - 8;
                let function = module
                    .functions
                    .get(callee)
                    .ok_or(X3Error::FunctionNotFound)?;
                if function.param_count as usize != argc {
                    return Err(X3Error::ArgumentCountMismatch);
                }
            }
            0x10 => {
                if read_u32_at(code, ip + 2) as usize >= module.const_pool.len() {
                    return Err(X3Error::ConstPoolOutOfBounds);
                }
            }
            0x12 => {
                if read_u32_at(code, ip + 2) as usize >= module.globals.len() {
                    return Err(X3Error::GlobalOutOfBounds);
                }
            }
            0x13 => {
                if read_u32_at(code, ip + 1) as usize >= module.globals.len() {
                    return Err(X3Error::GlobalOutOfBounds);
                }
            }
            // AddF, SubF, MulF, DivF, ModF: forbidden on chain, as `x3-vm`'s
            // `VerifyOptions::on_chain` forbids them. ModF was refused as unimplemented instead,
            // so the engines gave different reasons for one policy (the `compile_and_run` fuzz
            // target, on a program taking `%` of floats).
            0x30..=0x34 => return Err(X3Error::ForbiddenOnChain(op)),
            // DebugPrint, Breakpoint: `x3-vm`'s on-chain verifier forbids them too.
            0xF0 | 0xF1 => return Err(X3Error::ForbiddenOnChain(op)),
            // The numeric conversions and `ctx_gas` are refused at intake (TICKET-149). This engine
            // executed them and `crates/x3-vm` did not, so the same program answered on chain and
            // failed everywhere else, and neither half was right:
            // * The value model has only `i64` and `f64`, so the 32-bit conversions had no honest
            //   meaning here: `I64ToI32` returned its input untruncated, `F64ToF32` did not round,
            //   `F32ToI32` did not saturate — identities under names that promise otherwise.
            // * `ctx_gas` returned this engine's remaining gas, and nothing holds the two engines to
            //   the same per-opcode charges, so a program branching on it could take different
            //   paths on the two. A value the engines might disagree about is not one to put in
            //   consensus state.
            // The chain compiler emits none of these (only `x3-backend`'s HIR-direct `lower.rs`,
            // which the chain path does not use), so this refuses no compiled program.
            0x60..=0x68 | 0x84 => return Err(X3Error::UnsupportedOpcode(op)),
            _ => {}
        }
        ip += len;
    }
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
        b.extend_from_slice(x3_common::bytecode::MAGIC);
        b.extend_from_slice(&x3_common::bytecode::VERSION.to_le_bytes()); // 1.0.0
        b.extend_from_slice(&0u32.to_le_bytes()); // flags
        b.extend_from_slice(&0u32.to_le_bytes()); // checksum (filled at the end, as the writer does)
        b.extend_from_slice(&x3_common::bytecode::MIN_SUPPORTED_VERSION.to_le_bytes());
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

        // The checksum the writer computes, over everything after the header.
        let checksum = x3_common::bytecode::checksum(&b[x3_common::bytecode::HEADER_LEN..]);
        let at = x3_common::bytecode::CHECKSUM_OFFSET;
        b[at..at + 4].copy_from_slice(&checksum.to_le_bytes());
        b
    }

    #[test]
    fn test_a_corrupted_body_is_rejected() {
        // The header is intact and the structure would parse; only the body changed.
        let mut payload = make_simple_module();
        let last = payload.len() - 1;
        payload[last] ^= 0xFF;
        assert!(matches!(
            parse_module(&payload),
            Err(X3Error::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_a_future_format_version_is_rejected() {
        // The checksum covers the body, not the header, so this is a well-formed envelope
        // announcing a version this loader does not read.
        let mut payload = make_simple_module();
        payload[4..8].copy_from_slice(&x3_common::bytecode::MAX_SUPPORTED_VERSION.to_le_bytes());
        assert!(matches!(
            parse_module(&payload),
            Err(X3Error::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn test_a_module_requiring_a_newer_loader_is_rejected() {
        let mut payload = make_simple_module();
        // A minor newer than this loader reads.
        let newer_minor = x3_common::bytecode::READER_VERSION + (1u32 << 8);
        payload[16..20].copy_from_slice(&newer_minor.to_le_bytes());
        assert!(matches!(
            parse_module(&payload),
            Err(X3Error::UnsupportedVersion(_))
        ));
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

    /// The same envelope as `make_simple_module`, with a different code section.
    ///
    /// It used to declare version `1` and `min_version` `1` — neither of which is a packed
    /// `(major, minor, patch)`, so under the format's own rules the module was unreadable
    /// (`1 >> 16 == 0`, a different major) and the payload was only accepted because the reader
    /// skipped the header. The constants and the checksum come from the shared definition now.
    fn rebuild_with_code(code: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(x3_common::bytecode::MAGIC);
        b.extend_from_slice(&x3_common::bytecode::VERSION.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&x3_common::bytecode::MIN_SUPPORTED_VERSION.to_le_bytes());
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

        let checksum = x3_common::bytecode::checksum(&b[x3_common::bytecode::HEADER_LEN..]);
        let at = x3_common::bytecode::CHECKSUM_OFFSET;
        b[at..at + 4].copy_from_slice(&checksum.to_le_bytes());
        b
    }

    /// A module with one `main` holding exactly `code`, written by the real envelope writer so a
    /// test only has to name the instructions it is about.
    fn module_bytes(code: &[u8]) -> Vec<u8> {
        let mut module = x3_backend::BytecodeModule::new();
        module.functions.push(x3_backend::FunctionEntry {
            name: "main".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 16,
            max_stack: 16,
            return_type_tag: 1,
        });
        module.code = code.to_vec();
        module.to_bytes()
    }

    /// Drive the interpreter directly and hand the finished `Vm` to `check`, so a test can inspect
    /// state a caller never receives (the slot map and the journal after an aborted window).
    fn inspect<F: FnOnce(&Vm<'_>)>(code: &[u8], check: F) -> X3Result<Option<MiniValue>> {
        let bytes = module_bytes(code);
        let module = parse_module(&bytes).expect("the module parses");
        let mut vm = Vm::new(&module, 1_000_000);
        vm.call_stack.push(CallFrame {
            ret_dst: 0,
            ip: module.functions[0].entry as usize,
            base: 0,
            ret_addr: usize::MAX,
            func_idx: 0,
        });
        let outcome = vm.run();
        check(&vm);
        outcome
    }

    #[test]
    fn a_payload_round_trips_exactly_and_an_oversized_one_is_refused() {
        // The two cases an untagged or zero-padded layout cannot tell apart. If either collapsed,
        // a program would read back a value it never stored.
        for value in [
            MiniValue::I64(-5),
            MiniValue::I64(197_121),
            MiniValue::Bool(true),
            MiniValue::F64(1.5),
            MiniValue::Bytes(vec![1, 2]),
            MiniValue::Bytes(vec![1, 2, 0]),
        ] {
            let payload = encode_slot_payload(&value).expect("a representable value encodes");
            assert_eq!(
                decode_slot_payload(&payload).unwrap(),
                value,
                "a store/load round trip must be exact"
            );
        }

        assert_eq!(
            encode_slot_payload(&MiniValue::Bytes(vec![7u8; SLOT_PAYLOAD_MAX + 1])).unwrap_err(),
            X3Error::UnencodableStorageValue("byte-string is longer than a slot holds")
        );
        assert_eq!(
            encode_slot_payload(&MiniValue::Unit).unwrap_err(),
            X3Error::UnencodableStorageValue("unit is not a storable value")
        );

        // A payload this ISA did not write carries no value, so it is refused rather than guessed.
        let mut unknown_tag = [0u8; 32];
        unknown_tag[0] = 0x7F;
        assert_eq!(
            decode_slot_payload(&unknown_tag).unwrap_err(),
            X3Error::CorruptStorageSlot
        );
    }

    #[test]
    fn slot_keys_live_outside_the_global_keyspace_and_slots_are_integers() {
        // `LoadGlobal`/`StoreGlobal` read their index as a `u32`, so a domain tag above `u32::MAX`
        // is what makes the two keyspaces disjoint rather than merely unlikely to collide.
        assert!(EVM_SLOT_DOMAIN > u32::MAX as u64);
        assert_ne!(evm_slot_key(0), evm_slot_key(1));

        assert_eq!(
            slot_number(&MiniValue::I64(-1)).unwrap_err(),
            X3Error::InvalidStorageSlot("negative slot index")
        );
        assert_eq!(
            slot_number(&MiniValue::Bool(true)).unwrap_err(),
            X3Error::InvalidStorageSlot("slot index is not an integer")
        );
        assert_eq!(slot_number(&MiniValue::I64(4)).unwrap(), 4);
    }

    #[test]
    fn a_rolled_back_window_restores_the_slot_it_wrote() {
        // The rollback aborts, so a caller never sees this state — but the interpreter's own state
        // has to agree with the journal it would have reported, or the `storage_writes` field's
        // promise ("a write abandoned by a reverted window is not here") rests on nothing.
        let outcome = inspect(
            &[
                0x90, 0x00, 0x00, // AtomicBegin 0
                0x18, 0x01, 0x00, // LoadImm r1, 0  (slot)
                0x18, 0x02, 0x07, // LoadImm r2, 7  (value)
                0xB4, 0x01, 0x02, // EvmSstore slot=r1 value=r2
                0x92, 0x00, 0x00, // AtomicRollback 0
            ],
            |vm| {
                assert!(
                    vm.slots.is_empty(),
                    "the slot the window wrote must be gone"
                );
                assert!(
                    vm.writes.is_empty(),
                    "an undone write must not be journaled"
                );
                assert!(vm.undo_log.is_empty(), "the undo records are consumed");
            },
        );
        assert_eq!(outcome.unwrap_err(), X3Error::AtomicAborted);
    }

    #[test]
    fn a_committed_window_keeps_the_slot_it_wrote() {
        let outcome = inspect(
            &[
                0x90, 0x00, 0x00, // AtomicBegin 0
                0x18, 0x01, 0x00, // LoadImm r1, 0
                0x18, 0x02, 0x07, // LoadImm r2, 7
                0xB4, 0x01, 0x02, // EvmSstore
                0x91, 0x00, 0x00, // AtomicCommit 0
                0x06, // RetVoid
            ],
            |vm| {
                let payload = encode_slot_payload(&MiniValue::I64(7)).expect("7 is storable");
                assert_eq!(
                    vm.slots.get(&evm_slot_key(0)),
                    Some(&payload),
                    "a committed window leaves its write in the slot map"
                );
                assert_eq!(
                    vm.writes.len(),
                    1,
                    "and in the journal a receipt is built from"
                );
                assert_eq!(vm.writes[0].new_value, Some(payload));
                assert_eq!(
                    vm.writes[0].old_value, None,
                    "the slot was empty in this view"
                );
            },
        );
        assert!(outcome.is_ok(), "a committed window runs to the end");
    }

    /// The chain's own slots, as `X3ContractStorage` holds them: key → tagged payload.
    fn chain_slots(entries: &[(u64, MiniValue)]) -> Vec<([u8; 32], [u8; 32])> {
        entries
            .iter()
            .map(|(slot, value)| {
                (
                    evm_slot_key(*slot),
                    encode_slot_payload(value).expect("the test value is storable"),
                )
            })
            .collect()
    }

    /// `evm_sload r0 <- slot r1` then `ret r0`, with the slot index in the middle operand.
    fn load_slot(slot: u8) -> Vec<u8> {
        vec![
            0x18, 0x01, slot, // LoadImm r1, slot
            0xB3, 0x00, 0x01, // EvmSload dst=r0 slot=r1
            0x05, 0x00, // Ret r0
        ]
    }

    #[test]
    fn a_slot_a_previous_comit_wrote_reads_back_as_that_value() {
        // The gap this closes: the chain persisted a slot and the contract could not read it, so a
        // second comit saw EVM's zero for state its own first comit had written.
        let bytes = module_bytes(&load_slot(0));
        let seeds = chain_slots(&[(0, MiniValue::I64(42))]);

        let seeded = execute_x3bc_with_slots(&bytes, 100_000, &seeds).expect("the load runs");
        assert_eq!(
            seeded.return_val,
            MiniValue::I64(42),
            "the chain's value, not the interpreter's empty default"
        );
        assert!(
            seeded.storage_writes.is_empty(),
            "reading the chain's state is not a write"
        );
    }

    #[test]
    fn a_chain_holding_no_slot_still_reads_zero() {
        // The control for the test above: the same program on the same view shape, with no seed,
        // must answer EVM's zero — so the assertion above is about the seed and not about the
        // opcode returning a constant.
        let bytes = module_bytes(&load_slot(0));
        let empty = execute_x3bc_with_slots(&bytes, 100_000, &[]).expect("the load runs");
        assert_eq!(empty.return_val, MiniValue::I64(0));
    }

    #[test]
    fn a_store_over_a_seeded_slot_reports_the_chain_value_as_its_old_value() {
        // `old_value` is what the kernel's receipt channel carries, and what a cross-VM delta sync
        // reads. Reporting `None` over a slot the chain held a value in would describe a change
        // from "nothing" — a state transition that did not happen.
        let code = [
            0x18, 0x01, 0x00, // LoadImm r1, 0  (slot)
            0x18, 0x02, 0x05, // LoadImm r2, 5  (new value)
            0xB4, 0x01, 0x02, // EvmSstore slot=r1 value=r2
            0x06, // RetVoid
        ];
        let bytes = module_bytes(&code);
        let seeds = chain_slots(&[(0, MiniValue::I64(42))]);

        let result = execute_x3bc_with_slots(&bytes, 100_000, &seeds).expect("the store runs");
        assert_eq!(result.storage_writes.len(), 1);
        assert_eq!(
            result.storage_writes[0].old_value,
            Some(encode_slot_payload(&MiniValue::I64(42)).unwrap()),
            "the value the chain held, not None"
        );
        assert_eq!(
            result.storage_writes[0].new_value,
            Some(encode_slot_payload(&MiniValue::I64(5)).unwrap())
        );
    }

    #[test]
    fn a_seeded_payload_this_isa_did_not_write_is_refused_when_read() {
        // A payload the chain holds that this ISA cannot decode must not become a value: the read
        // fails closed rather than handing the program a guess.
        let mut unknown_tag = [0u8; 32];
        unknown_tag[0] = 0x7F;
        let bytes = module_bytes(&load_slot(0));

        let outcome = execute_x3bc_with_slots(&bytes, 100_000, &[(evm_slot_key(0), unknown_tag)]);
        assert_eq!(outcome.unwrap_err(), X3Error::CorruptStorageSlot);
    }

    #[test]
    fn an_execution_that_never_reads_a_corrupt_slot_still_succeeds() {
        // The mirror of the test above, and the reason the refusal lives at the read rather than in
        // `seed_slots`: one contract's undecodable slot must not stop a program that never touches
        // it from running.
        let mut unknown_tag = [0u8; 32];
        unknown_tag[0] = 0x7F;
        let code = [0x18, 0x00, 0x07, 0x05, 0x00]; // LoadImm r0, 7 + Ret r0
        let bytes = module_bytes(&code);

        let result = execute_x3bc_with_slots(&bytes, 100_000, &[(evm_slot_key(0), unknown_tag)])
            .expect("the program never reads the corrupt slot");
        assert_eq!(result.return_val, MiniValue::I64(7));
    }
}
