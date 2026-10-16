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
/// The X3BC envelope's fixed header — the part a decoder must check before it trusts anything else.
///
/// Two decoders speak this format: `x3-backend::bc_format` (std) writes and reads it, and
/// `x3-integration::mini_x3` re-implements the reader for `no_std` builds, which is the one the
/// runtime uses. They disagreed about how much of the header mattered: the no-std reader checked
/// the magic and skipped the other twenty bytes, and neither reader verified the checksum the
/// writer already computed (TICKET-108). The definition lives here, in a crate both can depend on
/// without `std`, so there is one checksum algorithm and one set of version bounds.
pub mod bytecode {
    /// Magic bytes identifying X3 bytecode.
    pub const MAGIC: &[u8; 4] = b"X3BC";

    /// Fixed header length: magic, version, flags, checksum, min-version, features.
    pub const HEADER_LEN: usize = 24;

    /// Byte offset of the checksum inside the header.
    pub const CHECKSUM_OFFSET: usize = 12;

    /// Byte offset of the feature-flags word inside the header.
    pub const FEATURE_FLAGS_OFFSET: usize = 20;

    /// Feature flag: the program's compiled policy requires private submission.
    ///
    /// This is a *capability the artifact carries*, not a format change. AGENTS.md §11 says the
    /// security policy has to come from the compiled artifact, and §12 that a check the runtime
    /// does not interpret is worse than no check at all — so a bit set here means the loader must
    /// refuse the module unless the execution context it is loading into actually offers a private
    /// channel. A reader that reads the header and ignores this bit turns "this program demands
    /// privacy" into "this program runs in the clear", which is the one direction a private
    /// submission policy must never take.
    ///
    /// It lives here, beside [`MAGIC`] and [`VERSION`], because `x3-integration::mini_x3` — the
    /// interpreter a block runs — cannot depend on `x3-backend` (TICKET-108) and both readers have
    /// to name the same bit.
    pub const FEATURE_PRIVATE_SUBMISSION_REQUIRED: u32 = 1 << 8;

    /// Format version this loader writes: major 1, minor 0, patch 0, which is what
    /// `(major << 16) | (minor << 8) | patch` packs to.
    pub const VERSION: u32 = 1 << 16;

    /// Oldest format version this loader can read.
    pub const MIN_SUPPORTED_VERSION: u32 = VERSION;

    /// First format version this loader cannot read: the next major.
    pub const MAX_SUPPORTED_VERSION: u32 = 2 << 16;

    /// The envelope's body checksum, exactly as the writer computes it.
    ///
    /// A wrapping multiply-and-add over the bytes after the header. It is a corruption check, not a
    /// security boundary — anyone can recompute it — and it is here so that the writer, the std
    /// reader and the no-std reader cannot drift apart about what it means.
    pub fn checksum(body: &[u8]) -> u32 {
        let mut sum: u32 = 0;
        for byte in body {
            sum = sum.wrapping_add(*byte as u32);
            sum = sum.wrapping_mul(31);
        }
        sum
    }

    /// Is a module that declares `version` readable by this loader?
    ///
    /// Same major, no newer minor — equivalent to `VersionInfo::can_read` in `x3-backend`, which
    /// compares the same two fields. Patch differences are compatible: the format's own semantic
    /// versioning says so ("patch version changes are bug fixes (fully compatible)"), and the
    /// `version <= VERSION` this used to compare refused a *patch* bump — so `1.0.1` was accepted by
    /// `x3-backend` and refused by the no-std reader that executes on chain, and a module the
    /// compiler's own rules call compatible could not run (TICKET-137). `x3-backend`'s
    /// `version_rule_parity` test compares this against `can_read` version by version.
    // The lint is right that the bound is unreachable *today* — `VERSION`'s minor field is 0, so
    // "no newer minor" and "the same minor" are the same test — but writing `==` would silently start
    // refusing older minors the day `VERSION`'s minor moves, which is the bug this rule exists to
    // avoid. The general comparison stays, with the reason the lint is answered here.
    #[allow(clippy::absurd_extreme_comparisons)]
    pub const fn version_is_readable(version: u32) -> bool {
        (version >> 16) == (VERSION >> 16) && ((version >> 8) & 0xFF) <= ((VERSION >> 8) & 0xFF)
    }

    /// Does this loader satisfy a module that requires at least `min_version`?
    ///
    /// Equivalent to `VersionInfo::satisfies` for the same reason.
    pub const fn loader_satisfies(min_version: u32) -> bool {
        min_version <= VERSION
    }

    /// The feature-flags word of an X3BC header, or `None` when these bytes are not a header.
    ///
    /// `None` means "not an X3BC module", not "no demands": the magic and the header length are the
    /// two facts a caller needs before it can read the field at all. Callers that gate on a flag
    /// keep the refusal for a non-module payload with the component whose job that is, so this
    /// function answers only the question it can answer.
    pub fn feature_flags(bytes: &[u8]) -> Option<u32> {
        if bytes.len() < HEADER_LEN || &bytes[0..4] != MAGIC {
            return None;
        }
        let mut word = [0u8; 4];
        word.copy_from_slice(&bytes[FEATURE_FLAGS_OFFSET..FEATURE_FLAGS_OFFSET + 4]);
        Some(u32::from_le_bytes(word))
    }

    /// Does this payload declare that its compiled policy requires private submission?
    ///
    /// `false` covers both "the module does not demand it" and "this is not a module"; a caller that
    /// needs to tell those apart uses [`feature_flags`] directly.
    pub fn requires_private_submission(bytes: &[u8]) -> bool {
        matches!(
            feature_flags(bytes),
            Some(flags) if flags & FEATURE_PRIVATE_SUBMISSION_REQUIRED != 0
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use alloc::vec::Vec;

        /// A header whose feature word the caller chooses, with a body whose checksum is genuine so
        /// the reader under test is exercised rather than a malformed blob.
        fn header_with_features(features: u32) -> Vec<u8> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(MAGIC);
            bytes.extend_from_slice(&VERSION.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes()); // flags
            bytes.extend_from_slice(&checksum(&[]).to_le_bytes());
            bytes.extend_from_slice(&VERSION.to_le_bytes()); // min_version
            bytes.extend_from_slice(&features.to_le_bytes());
            assert_eq!(bytes.len(), HEADER_LEN);
            bytes
        }

        #[test]
        fn the_private_submission_bit_is_read_from_the_feature_word() {
            // Both directions, because a reader that always answers `false` is exactly the bug this
            // flag exists to fix: the demand would be carried and ignored.
            assert!(requires_private_submission(&header_with_features(
                FEATURE_PRIVATE_SUBMISSION_REQUIRED
            )));
            assert!(!requires_private_submission(&header_with_features(0)));
            // A different flag in the same word must not be mistaken for this one.
            assert!(!requires_private_submission(&header_with_features(1 << 7)));
            // A neighbouring bit inside the same word must not mask the demand either.
            assert!(requires_private_submission(&header_with_features(
                FEATURE_PRIVATE_SUBMISSION_REQUIRED | (1 << 7)
            )));
        }

        #[test]
        fn a_payload_that_is_not_a_module_is_not_a_demand() {
            // `None`, not `Some(0)`: the callers keep "this is not a program" owned by the adapter
            // that refuses it, and this function must not answer a question it cannot read.
            assert_eq!(feature_flags(&[]), None);
            assert_eq!(feature_flags(b"X3BC"), None);
            assert_eq!(feature_flags(&[0u8; HEADER_LEN]), None, "wrong magic");
            let mut short = header_with_features(FEATURE_PRIVATE_SUBMISSION_REQUIRED);
            short.truncate(HEADER_LEN - 1);
            assert_eq!(feature_flags(&short), None);
            assert!(!requires_private_submission(b"not a program at all"));
        }

        #[test]
        fn the_checksum_is_order_sensitive_and_deterministic() {
            // Two bodies with the same multiset of bytes in a different order must differ, or a
            // reordering corruption would pass. The algorithm is sensitive to order by construction
            // (multiply after each add); this pins it.
            assert_ne!(checksum(b"ab"), checksum(b"ba"));
            assert_eq!(checksum(b"ab"), checksum(b"ab"));
            assert_eq!(checksum(&[]), 0);
        }

        #[test]
        fn a_patch_bump_is_readable_and_a_minor_bump_is_not() {
            // The field layout: major << 16 | minor << 8 | patch.
            assert!(
                version_is_readable(VERSION + 1),
                "a patch bump is compatible"
            );
            assert!(
                !version_is_readable(VERSION + 0x100),
                "a newer minor is not readable by this loader"
            );
            assert!(
                !version_is_readable(VERSION + 0x1_0000),
                "a newer major is not readable by this loader"
            );
        }

        #[test]
        fn version_bounds_match_the_packing() {
            assert_eq!(VERSION, 0x0001_0000);
            assert_eq!(MAX_SUPPORTED_VERSION, 0x0002_0000);
            assert!(version_is_readable(VERSION));
            assert!(version_is_readable(MIN_SUPPORTED_VERSION));
            assert!(
                !version_is_readable(MAX_SUPPORTED_VERSION),
                "the next major is read by a loader that knows it"
            );
            assert!(
                !version_is_readable((1 << 16) | (1 << 8)),
                "a newer minor this loader does not know"
            );
            assert!(loader_satisfies(VERSION));
            assert!(!loader_satisfies((1 << 16) | (1 << 8)));
            assert!(!loader_satisfies(MAX_SUPPORTED_VERSION));
        }
    }
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
