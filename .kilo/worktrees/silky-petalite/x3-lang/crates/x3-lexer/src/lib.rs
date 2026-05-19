//! X3 Lexer - Lexical analysis for the X3 programming language
//!
//! This module provides tokenization of X3 source code, converting raw text into
//! a stream of tokens that can be consumed by the parser. The lexer is implemented
//! using the `logos` crate for high-performance lexing.
//!
//! # Token Categories
//!
//! - **Keywords**: `fn`, `let`, `agent`, `atomic`, `bundle`, etc.
//! - **Identifiers**: Variable and function names
//! - **Literals**: Numbers, strings, addresses, hashes
//! - **Operators**: Arithmetic, logical, comparison
//! - **Delimiters**: Brackets, braces, punctuation
//! - **Comments**: Line and block comments
//!
//! # Example
//!
//! ```
//! use x3_lang_lexer::Lexer;
//!
//! let source = "fn main() { let x = 42; }";
//! let mut lexer = Lexer::new(source, 0);
//!
//! for token in lexer {
//!     println!("{:?}", token);
//! }
//! ```

pub mod cursor;
pub mod lexer;
pub mod token;

pub use cursor::Cursor;
pub use lexer::Lexer;
pub use token::{BinOp, Delimiter, Keyword, Literal, Token, TokenKind, UnOp};
