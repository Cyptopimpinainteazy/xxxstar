//! X3 Common Utilities
//!
//! Shared types, spans, symbols, and utilities used across the X3 compiler.

pub mod diagnostic;
pub mod error;
pub mod source;
pub mod span;
pub mod symbol;
pub mod token;

pub use diagnostic::{Diagnostic, DiagnosticBuilder, DiagnosticLevel};
pub use error::{X3Error, X3Result};
pub use source::{SourceFile, SourceMap};
pub use span::{BytePos, Span, Spanned};
pub use symbol::{Symbol, SymbolInterner};
pub use token::{BinOp, DurationUnit, FloatSuffix, IntBase, IntSuffix, SizeUnit, UnOp};
