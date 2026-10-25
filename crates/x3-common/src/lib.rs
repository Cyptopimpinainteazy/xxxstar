#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use core::{fmt, ops::Add, str::FromStr};

// Shared building blocks for the X3 compiler pipeline.

/// A byte index span that locates tokens and AST nodes inside source text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a dummy span for testing purposes.
    pub const fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl Add for Span {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.merge(rhs)
    }
}

/// Literals that can appear inside the language.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    /// Unit value - represents absence of meaningful value (like void/()).
    Unit,
}

/// Keywords recognized by the lexer and parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    Loop,
    For,
    Return,
    Break,
    Continue,
    Struct,
    Enum,
    Match,
    True,
    False,
    Atomic,
    Emit,
    Agent,
    Context,
    Const,
    In,
    /// `automation Name { ... }`
    Automation,
    /// `trigger { ... }` section of an automation
    Trigger,
    /// `permissions { ... }` section of an automation
    Permissions,
    /// `execute { ... }` section of an automation
    Execute,
    /// `on_failure { ... }` section of an automation
    OnFailure,
}

impl Keyword {
    pub fn parse(src: &str) -> Option<Self> {
        match src {
            "fn" => Some(Self::Fn),
            "let" => Some(Self::Let),
            "mut" => Some(Self::Mut),
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "while" => Some(Self::While),
            "loop" => Some(Self::Loop),
            "for" => Some(Self::For),
            "return" => Some(Self::Return),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "struct" => Some(Self::Struct),
            "enum" => Some(Self::Enum),
            "match" => Some(Self::Match),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "atomic" => Some(Self::Atomic),
            "emit" => Some(Self::Emit),
            "agent" => Some(Self::Agent),
            "context" => Some(Self::Context),
            "const" => Some(Self::Const),
            "in" => Some(Self::In),
            "automation" => Some(Self::Automation),
            "trigger" => Some(Self::Trigger),
            "permissions" => Some(Self::Permissions),
            "execute" => Some(Self::Execute),
            "on_failure" => Some(Self::OnFailure),
            _ => None,
        }
    }
}

/// Error returned when a keyword does not match a known value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeywordParseError;

impl fmt::Display for KeywordParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid keyword")
    }
}

impl core::error::Error for KeywordParseError {}

impl FromStr for Keyword {
    type Err = KeywordParseError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Keyword::parse(src).ok_or(KeywordParseError)
    }
}

/// Symbols used for delimiters and operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Equals,
    DoubleEquals,
    Bang,
    BangEquals,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Amp,
    Pipe,
    Or,
    Arrow,
    FatArrow,
    Colon,
    Comma,
    Dot,
    Semicolon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
}

/// Token kinds produced by the lexer.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Keyword(Keyword),
    Symbol(Symbol),
    Literal(Literal),
    Eof,
}

impl TokenKind {
    pub fn symbol(&self) -> Option<Symbol> {
        match self {
            TokenKind::Symbol(sym) => Some(*sym),
            _ => None,
        }
    }
}

/// A token plus its span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

// Re-export signing module for external use.
/// The X3BC envelope's fixed header.
///
/// The definition moved to `x3bc-core` (the canonical X3BC semantic core, free of Substrate so the
/// `x3-lang` compiler can share it); this module re-exports it so every existing reader keeps one
/// checksum algorithm, one set of version bounds and one feature-bit table (TICKET-108).
pub mod bytecode {
    pub use x3bc_core::bytecode::*;
}

// Signing requires std (uses SS58 codec, format!, secp256k1 RNG, mnemonic phrases).
// Off-chain consumers (node RPC, bridge host code) build with std.
//
// This attribute sat one item higher until 2026-09-24: inserting the `bytecode` module above this
// line put `#[cfg(feature = "std")]` on that module instead of on `signing`, so a no-std build
// compiled `signing` — and failed, taking seven crates' no-default-features builds and the runtime's
// WASM build with it. A scripted insertion anchored on a bare `pub mod X;` moves whatever attribute
// precedes it; anchor on the attribute too.
/// How a value is tagged when it leaves a program: in a storage slot, or in an event's payload.
///
/// Both interpreters write these, and a reader that disagrees about a tag hands a program back a
/// value it never produced. They were `const SLOT_TAG_*` in `crates/x3-vm` and again in
/// `x3-integration::mini_x3`, two copies that happened to agree; events made a third consumer, so
/// the definition moved here, beside the envelope, for the reason the envelope is here
/// (TICKET-108): one definition, both readers.
///
/// A tagged value is `[tag][len][data]`. A slot pads that to 32 bytes; an event payload
/// concatenates them, so a reader walks the run without needing the values' types in advance.
pub mod value_tags {
    pub const INT: u8 = 1;
    pub const BOOL: u8 = 2;
    pub const F64: u8 = 3;
    pub const ADDR: u8 = 4;
    pub const BYTES: u8 = 5;
    pub const STRING: u8 = 6;
    /// An array in a returned value (`x3_vm::Value::return_bytes`). Never stored in a slot or an
    /// event: an aggregate is refused there.
    pub const ARRAY: u8 = 7;
    /// A tuple (or a struct, which the engine holds as a tuple of its fields) in a returned value.
    pub const TUPLE: u8 = 8;
    /// Unit inside a returned aggregate.
    pub const UNIT: u8 = 9;
}

/// Host calls a chain program can make by name: the VM's storage opcodes, reachable from source.
///
/// `x3-backend` has emitted `evm_sload`/`evm_sstore` (0xB3/0xB4) and both interpreters have
/// executed them (`crates/x3-vm`, and `mini_x3`, which a block runs) with one slot key and one
/// value encoding, but no stage of the compiler would let a program *call* them: the resolver
/// reported `undefined variable 'evm_sstore'`. This table is the one place the names, arities and
/// value kinds live; the resolver, the type checker and HIR consult it only for a name the program
/// has not declared itself, so a user's own `evm_sload` still wins.
///
/// A host call reaches MIR as an ordinary `Call` to a reserved symbol id at the top of the id
/// space. Every optimizer pass already treats a call as having effects (dead-code elimination
/// keeps it, hoisting refuses to move it, constant propagation does not look through it), which is
/// the treatment a storage access needs: a load must not be merged across a store, and a store
/// whose result is unused must not be deleted. The backend emits the opcode for a reserved id.
pub mod intrinsics {
    /// One host call.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Intrinsic {
        pub name: &'static str,
        /// Every parameter is a 64-bit integer.
        pub arity: usize,
        /// Whether the call produces an `i64` (otherwise it produces unit).
        pub returns_value: bool,
        /// The reserved symbol id the call is lowered to (`usize::MAX - n`).
        pub symbol: usize,
    }

    /// `evm_sload(slot: i64) -> i64`: the slot's value, or 0 if it was never written.
    pub const EVM_SLOAD: Intrinsic = Intrinsic {
        name: "evm_sload",
        arity: 1,
        returns_value: true,
        symbol: usize::MAX,
    };

    /// `evm_sstore(slot: i64, value: i64)`: write the slot. A negative slot fails at run time
    /// (`InvalidStorageSlot`) on both engines.
    pub const EVM_SSTORE: Intrinsic = Intrinsic {
        name: "evm_sstore",
        arity: 2,
        returns_value: false,
        symbol: usize::MAX - 1,
    };

    /// `ctx_block_height() -> i64`: the block the call executes in.
    ///
    /// The four context reads are host calls rather than expressions so they reach MIR as calls,
    /// which no optimizer pass merges or removes: two reads in one execution see the same block,
    /// but the value is the host's, not something constant propagation may assume. Each fails at
    /// run time (`ContextUnavailable`) when the host supplied no such field.
    pub const CTX_BLOCK_HEIGHT: Intrinsic = Intrinsic {
        name: "ctx_block_height",
        arity: 0,
        returns_value: true,
        symbol: usize::MAX - 3,
    };

    /// `ctx_timestamp() -> i64`: the block timestamp, milliseconds since the Unix epoch.
    pub const CTX_TIMESTAMP: Intrinsic = Intrinsic {
        name: "ctx_timestamp",
        arity: 0,
        returns_value: true,
        symbol: usize::MAX - 4,
    };

    /// `ctx_value() -> i64`: native value attached to the call (0 when none).
    pub const CTX_VALUE: Intrinsic = Intrinsic {
        name: "ctx_value",
        arity: 0,
        returns_value: true,
        symbol: usize::MAX - 5,
    };

    /// `ctx_chain_id() -> i64`: the chain the call executes on.
    pub const CTX_CHAIN_ID: Intrinsic = Intrinsic {
        name: "ctx_chain_id",
        arity: 0,
        returns_value: true,
        symbol: usize::MAX - 6,
    };

    // `ctx_sender` is not here yet: its value is a 32-byte account, and every host call in this
    // table is typed as taking and returning `i64`. It needs an address type in the signature.

    pub const ALL: [Intrinsic; 6] = [
        EVM_SLOAD,
        EVM_SSTORE,
        CTX_BLOCK_HEIGHT,
        CTX_TIMESTAMP,
        CTX_VALUE,
        CTX_CHAIN_ID,
    ];

    /// The reserved symbol `emit` lowers to.
    ///
    /// Not in [`ALL`]: those are host calls a program writes as `name(args)` with a fixed arity and
    /// `i64` operands. `emit` is a statement of the language, its first operand is the event's name
    /// and the rest are the payload, and it takes as many as the program writes — so it is named
    /// here, where both the lowering and the backend can see it, rather than pretending to be a
    /// function.
    pub const EMIT_SYMBOL: usize = usize::MAX - 2;

    // Aggregate operations, lowered as calls to these reserved symbols.
    //
    // Not in [`ALL`] either: their operands are arrays and tuples, not the `i64`s a named host call
    // takes, and source spells them as literals, `a[i]`, `a[i] = v` and `a.len()`, not as calls.
    // A call is what every optimizer pass already treats safely (its arguments are uses, it is
    // never merged with another call or dropped), so the aggregates need no new MIR form for the
    // passes to learn.
    //
    // Aggregates have value semantics: `a[i] = v` produces a new array equal to `a` with element
    // `i` replaced, and the variable is rebound to it. That is what keeps MIR in SSA form, and it
    // matches the engine, where every register owns its value and `mov` copies it.

    /// `[e0, e1, ...]`: a new array of the arguments, in order.
    pub const ARRAY_NEW_SYMBOL: usize = usize::MAX - 7;
    /// `(e0, e1, ...)`: a new tuple of the arguments, in order.
    pub const TUPLE_NEW_SYMBOL: usize = usize::MAX - 8;
    /// `base[index]`: the element, for an array or a tuple.
    pub const INDEX_GET_SYMBOL: usize = usize::MAX - 9;
    /// `base[index] = value`, as a value: `base` with that element replaced.
    pub const INDEX_SET_SYMBOL: usize = usize::MAX - 10;
    /// `base.len()`: the number of elements.
    pub const LEN_SYMBOL: usize = usize::MAX - 11;
    /// `s.field = value` (and a tuple's element), as a value: `base` with the element at
    /// `index` replaced. `index` must be a constant: a tuple's and a struct's elements are named
    /// at compile time, and the engine writes one with `store_field`, which takes its position as
    /// an immediate.
    pub const FIELD_SET_SYMBOL: usize = usize::MAX - 12;

    pub fn by_name(name: &str) -> Option<Intrinsic> {
        ALL.iter().copied().find(|intrinsic| intrinsic.name == name)
    }

    pub fn by_symbol(symbol: usize) -> Option<Intrinsic> {
        ALL.iter()
            .copied()
            .find(|intrinsic| intrinsic.symbol == symbol)
    }
}

/// Signing and verifying a standalone X3BC artifact (see the module docs). `std` only: the chain
/// path authenticates an artifact through the extrinsic that carries it, so the runtime's no_std
/// build has no use for this.
#[cfg(feature = "std")]
pub mod artifact;

#[cfg(feature = "std")]
pub mod signing;
#[cfg(feature = "std")]
pub use signing::{
    verify_signature, verify_signature_hash, Ed25519Signer, PublicKey, Secp256k1Signer, Signature,
    Signer, Sr25519Signer,
};

/// Key type identifier for cryptographic schemes.
///
/// Defined at crate root (not in `signing`) so it remains available in `no_std`
/// builds for weight metering and other on-chain consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyType {
    /// ed25519 for SVM/Cosmos
    Ed25519,
    /// secp256k1 for EVM
    Secp256k1,
    /// sr25519 for Substrate/X3
    Sr25519,
}

// Re-export weight metering module for external use
pub mod weight_metering;
pub use weight_metering::{
    ComputeMeter, GasMeter, HashAlgorithm, Operation, OperationCosts, WeightConfig, WeightError,
    WeightMeter, WeightResult,
};
