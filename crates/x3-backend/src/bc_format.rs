//! The X3BC container format.
//!
//! The canonical reader and writer live in `x3bc-core` (`x3bc_core::format`), shared with the
//! engine and the `x3-lang` compiler; this module re-exports them. Format errors convert into
//! [`crate::error::BackendError`] with the same kinds and messages they always had.

pub use x3bc_core::format::*;

use crate::error::{BackendError, BackendErrorKind};
use x3bc_core::error::{FormatError, FormatErrorKind};

impl From<FormatError> for BackendError {
    fn from(error: FormatError) -> Self {
        BackendError::without_span(match error.kind {
            FormatErrorKind::UnexpectedEof => BackendErrorKind::UnexpectedEof,
            FormatErrorKind::InvalidMagic => BackendErrorKind::InvalidMagic,
            FormatErrorKind::UnsupportedVersion(version) => {
                BackendErrorKind::UnsupportedVersion(version)
            }
            FormatErrorKind::ChecksumMismatch { expected, found } => {
                BackendErrorKind::ChecksumMismatch { expected, found }
            }
            FormatErrorKind::CorruptedBytecode { offset } => {
                BackendErrorKind::CorruptedBytecode { offset }
            }
            FormatErrorKind::ConstPoolOverflow { max } => {
                BackendErrorKind::ConstPoolOverflow { max }
            }
            FormatErrorKind::StringTooLong { len, max } => {
                BackendErrorKind::StringTooLong { len, max }
            }
            FormatErrorKind::TradingSection { offset, .. } => {
                BackendErrorKind::CorruptedBytecode { offset }
            }
        })
    }
}
