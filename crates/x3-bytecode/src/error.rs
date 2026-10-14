//! Errors raised while reading or building an X3BC module.
//!
//! These are the only failures the format itself can produce. The compiler's
//! wider error type (`x3_backend::BackendError`) converts from this one, so a
//! decode failure reads the same whichever crate reports it.

use core::fmt;

/// A failure while decoding X3BC bytes or filling a constant pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeError {
    /// The bytes do not start with the X3BC magic.
    InvalidMagic,
    /// The module's version, or the minimum loader version it requires, is not one this reader accepts.
    UnsupportedVersion(u32),
    /// The header's checksum disagrees with the checksum of the body.
    ChecksumMismatch { expected: u32, found: u32 },
    /// A section holds a tag or value the format does not define.
    CorruptedBytecode { offset: usize },
    /// The bytes end before a section the header promises.
    UnexpectedEof,
    /// A constant pool would exceed the format's entry limit.
    ConstPoolOverflow { max: u32 },
    /// A string constant is longer than the format allows.
    StringTooLong { len: usize, max: usize },
}

impl fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "invalid bytecode magic"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported bytecode version: {v}"),
            Self::ChecksumMismatch { expected, found } => write!(
                f,
                "bytecode checksum mismatch: header says {found:#010x}, the body computes to {expected:#010x}"
            ),
            Self::CorruptedBytecode { offset } => {
                write!(f, "corrupted bytecode at offset {offset}")
            }
            Self::UnexpectedEof => write!(f, "unexpected end of bytecode"),
            Self::ConstPoolOverflow { max } => {
                write!(f, "constant pool overflow: exceeded {max} entries")
            }
            Self::StringTooLong { len, max } => {
                write!(f, "string too long: {len} bytes (max {max})")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytecodeError {}

/// Result of a bytecode-format operation.
pub type BytecodeResult<T> = Result<T, BytecodeError>;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    // The compiler's error type printed these exact words before the format moved here; tooling
    // and tests that read a decode failure as text keep working only if the words stay the same.
    #[test]
    fn messages_match_the_words_the_compiler_printed_before_the_move() {
        assert_eq!(
            BytecodeError::InvalidMagic.to_string(),
            "invalid bytecode magic"
        );
        assert_eq!(
            BytecodeError::UnsupportedVersion(7).to_string(),
            "unsupported bytecode version: 7"
        );
        assert_eq!(
            BytecodeError::ChecksumMismatch {
                expected: 1,
                found: 2
            }
            .to_string(),
            "bytecode checksum mismatch: header says 0x00000002, the body computes to 0x00000001"
        );
        assert_eq!(
            BytecodeError::CorruptedBytecode { offset: 9 }.to_string(),
            "corrupted bytecode at offset 9"
        );
        assert_eq!(
            BytecodeError::UnexpectedEof.to_string(),
            "unexpected end of bytecode"
        );
        assert_eq!(
            BytecodeError::ConstPoolOverflow { max: 3 }.to_string(),
            "constant pool overflow: exceeded 3 entries"
        );
        assert_eq!(
            BytecodeError::StringTooLong { len: 5, max: 4 }.to_string(),
            "string too long: 5 bytes (max 4)"
        );
    }
}
