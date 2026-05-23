//! JIT skeleton for X3
//!
//! This module houses structures and glue for recording hot paths and compiling
//! them to native code. For v0.1 we provide hints and a simple interface used by
//! runtime to mark hot basic blocks.

pub struct JitCompiler {
    pub threshold: u32,
}

impl JitCompiler {
    pub fn new(threshold: u32) -> Self {
        JitCompiler { threshold }
    }
    pub fn maybe_compile(&self, _code: &[u8], _pc: usize) -> bool {
        false
    }
}

/// Representation of a compiled function (placeholder)
pub struct CompiledFn;
