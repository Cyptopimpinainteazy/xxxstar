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
// Signing requires std (uses SS58 codec, format!, secp256k1 RNG, mnemonic phrases).
// Off-chain consumers (node RPC, bridge host code) build with std.
#[cfg(feature = "std")]
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
    /// Equivalent to `VersionInfo::can_read` in `x3-backend` (same major, no newer minor), expressed
    /// on the packed integer because the packing is order-preserving. `x3-backend`'s tests compare
    /// the two so they cannot drift.
    pub const fn version_is_readable(version: u32) -> bool {
        (version >> 16) == (VERSION >> 16) && version <= VERSION
    }

    /// Does this loader satisfy a module that requires at least `min_version`?
    ///
    /// Equivalent to `VersionInfo::satisfies` for the same reason.
    pub const fn loader_satisfies(min_version: u32) -> bool {
        min_version <= VERSION
    }

    #[cfg(test)]
    mod tests {
        use super::*;

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
