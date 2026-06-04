//! X3 VM core data structures and helper functions.

use crate::executor::{ExecError, ExecResult};
use std::sync::Arc;

pub type Register = u128; // 128-bit to match u256-like operations; adjust as needed

#[derive(Clone, Debug)]
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
    pub atomic_stack: Vec<usize>, // stack of pc for atomic begin
    pub call_stack: Vec<usize>,
    pub paused: bool,
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
            atomic_stack: Vec::new(),
            call_stack: Vec::new(),
            paused: false,
        }
    }
}

pub struct VM {
    pub config: VMConfig,
    pub state: VMState,
    pub code: InstructionStream,
    pub bridge: Box<dyn crate::bridge::BridgeAdapter>,
}

impl VM {
    pub fn new(code: Vec<u8>, cfg: VMConfig, initial_gas: u128) -> Self {
        VM {
            config: cfg.clone(),
            state: VMState::new(&cfg, initial_gas),
            code: InstructionStream::new(code),
            bridge: Box::new(crate::bridge::DryRunBridge),
        }
    }

    pub fn execute(&mut self) -> ExecResult<()> {
        self.verify_and_execute()
    }

    pub fn verify_and_execute(&mut self) -> ExecResult<()> {
        crate::verifier::verify(&self.code)
            .map_err(|err| ExecError::Panic(format!("X3_VERIFY_FAILED: {err:?}")))?;
        self.execute_unverified()
    }

    pub(crate) fn execute_unverified(&mut self) -> ExecResult<()> {
        crate::executor::execute(self)
    }
}
