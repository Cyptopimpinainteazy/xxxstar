//! X3 VM core data structures and helper functions.

use crate::executor::{ExecError, ExecResult, GasCost};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use x3_lang_common::Span;

pub type Register = u128; // 128-bit to match u256-like operations; adjust as needed

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionStream(Arc<Vec<u8>>);

impl InstructionStream {
    pub fn new(bytes: Vec<u8>) -> Self {
        InstructionStream(Arc::new(bytes))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct VMConfig {
    pub max_registers: usize,
    pub max_stack: usize,
    pub max_memory_pages: usize,
}

impl Default for VMConfig {
    fn default() -> Self {
        VMConfig {
            max_registers: 32,
            max_stack: 65536,
            max_memory_pages: 256,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VMState {
    pub registers: Vec<Register>,
    pub vector_registers: Vec<[u8; 16]>,
    pub pc: usize,
    pub sp: usize,
    pub fp: usize,
    pub gas: u128,
    pub memory: Vec<u8>,
}

impl VMState {
    pub fn new(config: &VMConfig, initial_gas: u128) -> Self {
        VMState {
            registers: vec![0u128; config.max_registers],
            vector_registers: vec![[0u8; 16]; 8],
            pc: 0,
            sp: 0,
            fp: 0,
            gas: initial_gas,
            memory: vec![0u8; config.max_memory_pages * 64 * 1024],
        }
    }
}

pub struct VM {
    pub config: VMConfig,
    pub state: VMState,
    pub code: InstructionStream,
}

impl VM {
    pub fn new(code: Vec<u8>, cfg: VMConfig, initial_gas: u128) -> Self {
        VM {
            config: cfg.clone(),
            state: VMState::new(&cfg, initial_gas),
            code: InstructionStream::new(code),
        }
    }

    pub fn execute(&mut self) -> ExecResult<()> {
        use crate::executor::execute;
        execute(self)
    }
}
