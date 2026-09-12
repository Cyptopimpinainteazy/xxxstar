//! Adapter from source text to the workspace compiler's canonical bytecode.
//!
//! Parsing, lowering, verification, and code generation remain owned by
//! `x3-compiler`. This module only selects conservative compilation options,
//! serializes the resulting `BytecodeModule`, and translates errors into the
//! integration crate's public error type.

use crate::{X3IntegrationError, X3Result};
use x3_compiler::{CompilationOptions, Compiler};

#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

/// Compile valid `.x3` source into canonical, runtime-loadable X3 bytecode.
///
/// The returned bytes use the versioned `X3BC` format implemented by
/// `x3_backend::BytecodeModule`. Invalid source and every compiler pipeline
/// failure are returned as `CompilationFailed`; this function never substitutes
/// empty or synthetic bytecode.
pub fn compile_source(source: &str) -> X3Result<Vec<u8>> {
    let output = Compiler::compile(source, CompilationOptions::contract_mode())
        .map_err(|error| X3IntegrationError::CompilationFailed(format!("{error:?}")))?;

    let bytes = output.bytecode.to_bytes();
    if bytes.is_empty() {
        return Err(X3IntegrationError::CompilationFailed(
            "compiler returned an empty bytecode module".into(),
        ));
    }

    Ok(bytes)
}
