//! X3 Virtual Machine - Deterministic Interpreter
//!
//! A register-based bytecode interpreter for X3BC modules.
//!
//! # Features
//!
//! - **Deterministic execution**: Same inputs always produce same outputs
//! - **Gas metering**: Configurable gas limits for bounded execution
//! - **Hostcall interface**: Extensible external function hooks
//! - **Atomic windows**: Track atomic begin/end for transaction safety
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                      VM                         │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
//! │  │ Module   │  │ Registers│  │ Call Stack   │  │
//! │  │ (code,   │  │ (256 max)│  │ (64 depth)   │  │
//! │  │  consts) │  │          │  │              │  │
//! │  └──────────┘  └──────────┘  └──────────────┘  │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
//! │  │ Operand  │  │ Gas      │  │ Atomic       │  │
//! │  │ Stack    │  │ Counter  │  │ Depth        │  │
//! │  └──────────┘  └──────────┘  └──────────────┘  │
//! └─────────────────────────────────────────────────┘
//! ```

use crate::prelude::*;
use x3_backend::bc_format::{BytecodeModule, ConstValue};
use x3_backend::opcode::Opcode;

use crate::context::BlockContext;
use crate::error::{VMError, VMErrorKind, VMResult};
use crate::events::{EventBuffer, VmEvent};
use crate::hostcall::HostcallRegistry;
use crate::isolation::IsolationContext;
#[cfg(feature = "std")]
use crate::jit_compiler::{JitCompiler, JitConfig, JitStats};
use crate::state::{StateMachine, VmState};
use crate::storage::{StorageValue, VmStorage, WriteRecord};

/// Maximum register count.
pub const MAX_REGISTERS: usize = 256;

/// Whether this interpreter has an implementation for `opcode`.
///
/// The match has no wildcard arm on purpose: an opcode added to the instruction set does not
/// compile until someone decides here whether the engine runs it. The on-chain verifier refuses
/// every opcode this returns `false` for, and `engine_executes_matches_the_interpreter` (below)
/// checks each answer against what the interpreter actually does.
///
/// `true` means the interpreter has an arm, not that every context may run it: GPU intrinsics,
/// debug opcodes and floating-point arithmetic are executed off chain and refused on chain by
/// separate verifier rules.
pub const fn engine_executes(opcode: Opcode) -> bool {
    match opcode {
        Opcode::Nop
        | Opcode::Jump
        | Opcode::JumpIf
        | Opcode::JumpUnless
        | Opcode::Call
        | Opcode::Ret
        | Opcode::RetVoid
        | Opcode::Halt
        | Opcode::LoadConst
        | Opcode::Mov
        | Opcode::LoadGlobal
        | Opcode::StoreGlobal
        | Opcode::LoadImm
        | Opcode::LoadZero
        | Opcode::LoadTrue
        | Opcode::LoadFalse
        | Opcode::AddI
        | Opcode::SubI
        | Opcode::MulI
        | Opcode::DivI
        | Opcode::ModI
        | Opcode::NegI
        | Opcode::AddF
        | Opcode::SubF
        | Opcode::MulF
        | Opcode::DivF
        | Opcode::ModF
        | Opcode::NegF
        | Opcode::EqI
        | Opcode::NeI
        | Opcode::LtI
        | Opcode::LeI
        | Opcode::GtI
        | Opcode::GeI
        | Opcode::EqF
        | Opcode::NeF
        | Opcode::LtF
        | Opcode::LeF
        | Opcode::GtF
        | Opcode::GeF
        | Opcode::And
        | Opcode::Or
        | Opcode::Xor
        | Opcode::Not
        | Opcode::Shl
        | Opcode::Shr
        | Opcode::UShr
        | Opcode::LAnd
        | Opcode::LOr
        | Opcode::LNot
        | Opcode::AtomicBegin
        | Opcode::AtomicCommit
        | Opcode::AtomicRollback
        | Opcode::AtomicCheck
        | Opcode::LoadIndex
        | Opcode::StoreIndex
        | Opcode::LoadField
        | Opcode::StoreField
        | Opcode::NewArray
        | Opcode::ArrayLen
        | Opcode::ArrayPush
        | Opcode::ArrayPop
        | Opcode::NewTuple
        | Opcode::TupleGet
        | Opcode::Inc
        | Opcode::Dec
        | Opcode::CtxSender
        | Opcode::CtxBlockHeight
        | Opcode::CtxTimestamp
        | Opcode::CtxValue
        | Opcode::CtxChainId
        | Opcode::Emit
        | Opcode::EvmSload
        | Opcode::EvmSstore
        | Opcode::GpuSha256Batch
        | Opcode::GpuEd25519Verify
        | Opcode::GpuPohChain
        | Opcode::GpuSha256Streamed
        | Opcode::GpuDeviceCount
        | Opcode::GpuBenchmark
        | Opcode::GpuKeccak256Batch
        | Opcode::GpuSecp256k1Verify
        | Opcode::DebugPrint
        | Opcode::Breakpoint
        | Opcode::Assert
        | Opcode::Panic => true,

        // Conversions: the value model has only i64 and f64 (TICKET-149).
        Opcode::I32ToI64
        | Opcode::I64ToI32
        | Opcode::I32ToF32
        | Opcode::I64ToF64
        | Opcode::F32ToI32
        | Opcode::F64ToI64
        | Opcode::F32ToF64
        | Opcode::F64ToF32
        | Opcode::ToBool
        // `ctx_gas` answers with the engine's remaining gas, so a program branching on it would
        // change behaviour whenever a runtime upgrade reprices an opcode (TICKET-149).
        | Opcode::CtxGas
        // Agents: no agent registry.
        | Opcode::AgentSelf
        | Opcode::AgentInit
        // Cross-VM calls: need the kernel's EVM and SVM adapters wired as hosts.
        | Opcode::EvmCall
        | Opcode::EvmStaticCall
        | Opcode::EvmDelegateCall
        | Opcode::EvmCreate
        | Opcode::EvmCreate2
        | Opcode::EvmLog
        | Opcode::EvmBalance
        | Opcode::EvmCodeSize
        | Opcode::SvmInvoke
        | Opcode::SvmInvokeSigned
        | Opcode::SvmCreateAccount
        | Opcode::SvmTransfer
        | Opcode::SvmGetData
        | Opcode::SvmSetData
        | Opcode::SvmGetRent
        | Opcode::SvmGetClock => false,
    }
}

/// Deepest nesting of atomic windows one execution may open.
///
/// Each open window holds a copy of the registers and globals, so this bounds the memory a program
/// can make the engine hold. The figure is the one the runtime interpreter enforced before this
/// engine replaced it.
pub const MAX_ATOMIC_DEPTH: usize = 32;

/// Maximum call stack depth.
pub const MAX_CALL_DEPTH: usize = 64;

/// Maximum operand stack size.
pub const MAX_STACK_SIZE: usize = 1024;

/// Default gas limit.
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// VM configuration.
#[derive(Clone, Debug)]
pub struct VMConfig {
    /// Maximum gas allowed.
    pub gas_limit: u64,
    /// Maximum call stack depth.
    pub max_call_depth: usize,
    /// Maximum operand stack size.
    pub max_stack_size: usize,
    /// Enable debug tracing.
    pub trace: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            gas_limit: DEFAULT_GAS_LIMIT,
            max_call_depth: MAX_CALL_DEPTH,
            max_stack_size: MAX_STACK_SIZE,
            trace: false,
        }
    }
}

/// Runtime value in the VM.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Value {
    /// 64-bit signed integer.
    I64(i64),
    /// 64-bit floating point.
    F64(f64),
    /// Boolean.
    Bool(bool),
    /// String (heap allocated).
    String(String),
    /// Byte array.
    Bytes(Vec<u8>),
    /// Address/pointer.
    Addr(u64),
    /// Array: an ordered, growable list (`new_array`, `array_push`, `load_index`, ...).
    Array(Vec<Value>),
    /// Tuple: a fixed list (`new_tuple`, `tuple_get`). A struct is held as the tuple of its
    /// fields, addressed by field index (`load_field`, `store_field`).
    Tuple(Vec<Value>),
    /// Unit (void/null).
    #[default]
    Unit,
}

/// Most values one aggregate may hold, counting nested elements and the aggregate itself.
///
/// Registers, atomic snapshots and `mov` all copy whole values, so an unbounded aggregate is a
/// memory amplification: `a = [a, a]` doubles in two pushes. The cap bounds what any one register
/// can hold, and so what a snapshot of 256 registers can cost.
pub const MAX_AGGREGATE_SIZE: usize = 256;

impl Value {
    /// How many values this one holds, itself included: 1 for a scalar, 1 plus each element's size
    /// for an aggregate.
    pub fn size(&self) -> usize {
        match self {
            Value::Array(items) | Value::Tuple(items) => {
                1 + items.iter().map(Value::size).sum::<usize>()
            }
            _ => 1,
        }
    }

    /// The bytes a receipt carries for this value when an execution returns it.
    ///
    /// A scalar is its own little-endian bytes (an integer, a float's bits, a bool's byte, a byte
    /// string or a string's UTF-8), unchanged from before aggregates existed. An aggregate is
    /// self-describing so a reader can walk it without knowing its type: `[ARRAY|TUPLE tag]`,
    /// a `u16` element count, then each element as `[tag][u32 length][its return bytes]`.
    pub fn return_bytes(&self) -> Vec<u8> {
        match self {
            Value::I64(v) => v.to_le_bytes().to_vec(),
            Value::F64(v) => v.to_bits().to_le_bytes().to_vec(),
            Value::Bool(v) => vec![u8::from(*v)],
            Value::Bytes(v) => v.clone(),
            Value::String(s) => s.as_bytes().to_vec(),
            Value::Addr(a) => a.to_le_bytes().to_vec(),
            Value::Unit => Vec::new(),
            Value::Array(items) | Value::Tuple(items) => {
                let kind = if matches!(self, Value::Array(_)) {
                    x3_common::value_tags::ARRAY
                } else {
                    x3_common::value_tags::TUPLE
                };
                // `MAX_AGGREGATE_SIZE` keeps the count far below `u16::MAX`.
                let mut out = vec![kind];
                out.extend_from_slice(&(items.len() as u16).to_le_bytes());
                for item in items {
                    let body = item.return_bytes();
                    out.push(item.kind_tag());
                    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
                    out.extend_from_slice(&body);
                }
                out
            }
        }
    }

    fn kind_tag(&self) -> u8 {
        use x3_common::value_tags as tag;
        match self {
            Value::I64(_) => tag::INT,
            Value::Bool(_) => tag::BOOL,
            Value::F64(_) => tag::F64,
            Value::Addr(_) => tag::ADDR,
            Value::Bytes(_) => tag::BYTES,
            Value::String(_) => tag::STRING,
            Value::Array(_) => tag::ARRAY,
            Value::Tuple(_) => tag::TUPLE,
            Value::Unit => tag::UNIT,
        }
    }

    /// Convert constant value to runtime value.
    pub fn from_const(c: &ConstValue) -> Self {
        match c {
            ConstValue::Integer(i) => Value::I64(*i),
            ConstValue::Float(f) => Value::F64(*f),
            ConstValue::String(s) => Value::String(s.clone()),
            ConstValue::Bool(b) => Value::Bool(*b),
            ConstValue::Bytes(b) => Value::Bytes(b.clone()),
        }
    }

    /// Get as i64.
    pub fn as_i64(&self) -> VMResult<i64> {
        match self {
            Value::I64(v) => Ok(*v),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "i64".to_string(),
                format!("{:?}", self),
            ))),
        }
    }

    /// Get as f64.
    pub fn as_f64(&self) -> VMResult<f64> {
        match self {
            Value::F64(v) => Ok(*v),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "f64".to_string(),
                format!("{:?}", self),
            ))),
        }
    }

    /// Get as bool.
    pub fn as_bool(&self) -> VMResult<bool> {
        match self {
            Value::Bool(v) => Ok(*v),
            // Truthy conversion
            Value::I64(v) => Ok(*v != 0),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "bool".to_string(),
                format!("{:?}", self),
            ))),
        }
    }
}

/// Call frame on the call stack.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Instruction pointer (offset in code).
    pub ip: usize,
    /// Base register index for this frame.
    pub base: usize,
    /// Return address (IP to return to).
    pub ret_addr: usize,
    /// Function index.
    pub func_idx: usize,
    /// Register **in the caller's frame** the callee's result is written to: the `dst` operand of
    /// the `Call` that made this frame. The format documents it (`[op][dst:u8][func:u32]…`) and the
    /// compiler emits it, but both interpreters used to ignore it and write the result to the
    /// caller's `r0` instead, so a call whose result was allocated anywhere else read `Unit`:
    /// measured, the compiler's own `fib.x3` failed with `TypeMismatch("i64", "Unit")` because
    /// `fib(n - 1) + fib(n - 2)` reads the two results from their own registers (TICKET-131).
    pub ret_dst: usize,
}

/// Execution result.
#[derive(Clone, Debug)]
pub struct ExecutionResult {
    /// Return value (if any).
    pub value: Option<Value>,
    /// Gas consumed.
    pub gas_used: u64,
    /// Number of instructions executed.
    pub instruction_count: u64,
}

/// The X3 Virtual Machine.
pub struct VM {
    /// Loaded module.
    module: BytecodeModule,
    /// Register file.
    regs: Vec<Value>,
    /// Operand stack.
    #[allow(dead_code)]
    stack: Vec<Value>,
    /// Call stack.
    call_stack: Vec<Frame>,
    /// Configuration.
    pub config: VMConfig,
    /// Gas consumed.
    gas_used: u64,
    /// Atomic nesting depth.
    atomic_depth: usize,
    /// What the host told the engine about this execution (see `context.rs`).
    context: BlockContext,
    /// Snapshot stack for atomic windows (regs, globals)
    atomic_snapshots: Vec<(Vec<Value>, Vec<Value>)>,
    /// Global storage (module.globals length)
    globals: Vec<Value>,
    /// Hostcall registry.
    hostcalls: HostcallRegistry,
    /// Instruction count.
    instruction_count: u64,
    /// Formal execution lifecycle.
    state_machine: StateMachine,
    /// Per-execution isolation context.
    isolation: IsolationContext,
    /// Atomic event buffer.
    event_buffer: EventBuffer,
    /// Journaled VM storage.
    storage: VmStorage,
    /// Hot-path tracker and compiled-function cache.
    // The JIT has no backend outside tests (`backend_available()` is false), and needs std.
    #[cfg(feature = "std")]
    jit: JitCompiler,
}

impl VM {
    /// Create a new VM with the given module.
    pub fn new(module: BytecodeModule) -> Self {
        Self::with_config(module, VMConfig::default())
    }

    /// Create a new VM with custom configuration.
    pub fn with_config(module: BytecodeModule, config: VMConfig) -> Self {
        Self::with_config_and_seeds(module, config, &[])
            .expect("no seeds cannot exceed the storage key limit")
    }

    /// Create a VM with custom configuration and the chain's contract slots already loaded.
    ///
    /// `seeds` is the chain's contract storage: the 32-byte slot keys and the payloads a previous
    /// execution persisted. They are loaded as state the execution inherited — not journaled, not
    /// reported as writes — which is what lets a later `evm_sload` read what an earlier execution
    /// stored, and what makes a subsequent `evm_sstore` report the chain's value as its
    /// `old_value`. Fails rather than silently dropping a seed the storage limit refuses.
    pub fn with_config_and_seeds(
        module: BytecodeModule,
        config: VMConfig,
        seeds: &[([u8; 32], [u8; 32])],
    ) -> VMResult<Self> {
        // initialize globals from module (use const pool initializers where present)
        let mut globals: Vec<Value> = Vec::new();
        for g in &module.globals {
            let idx = g.init_const.0 as usize;
            let val = module
                .const_pool
                .entries
                .get(idx)
                .map(Value::from_const)
                .unwrap_or(Value::Unit);
            globals.push(val);
        }

        // The isolation context enforces the same depth limit the interpreter does; it used to
        // enforce a private constant of 10 while the configuration said 32, so a program the
        // executor admitted was refused nine calls in (TICKET-131).
        let isolation_depth = config.max_call_depth as u32;
        let mut storage = VmStorage::new();
        for (key, value) in seeds {
            storage.seed_slot(*key, *value).map_err(|err| {
                VMError::without_ip(VMErrorKind::HostcallError(format!("seed refused: {err:?}")))
            })?;
        }

        Ok(Self {
            module,
            regs: vec![Value::Unit; MAX_REGISTERS],
            stack: Vec::with_capacity(config.max_stack_size),
            call_stack: Vec::with_capacity(config.max_call_depth),
            config,
            gas_used: 0,
            atomic_depth: 0,
            context: BlockContext::unknown(),
            atomic_snapshots: Vec::new(),
            globals,
            hostcalls: HostcallRegistry::with_standard(),
            instruction_count: 0,
            state_machine: StateMachine::new(),
            isolation: IsolationContext::new([0u8; 32]).with_max_call_depth(isolation_depth),
            event_buffer: EventBuffer::new(),
            storage,
            #[cfg(feature = "std")]
            jit: JitCompiler::new(JitConfig::default()),
        })
    }

    /// Create a VM from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> VMResult<Self> {
        let module = BytecodeModule::from_bytes(bytes)
            .map_err(|e| VMError::without_ip(VMErrorKind::ModuleLoadError(format!("{:?}", e))))?;
        Ok(Self::new(module))
    }

    /// Register a hostcall.
    pub fn register_hostcall<F>(
        &mut self,
        id: u8,
        name: impl Into<String>,
        arg_count: usize,
        handler: F,
    ) where
        F: Fn(&[Value]) -> VMResult<Option<Value>> + Send + Sync + 'static,
    {
        self.hostcalls.register(id, name, arg_count, handler);
    }

    /// Invoke a hostcall directly from the host.
    pub fn invoke_hostcall(&self, id: u8, args: &[Value]) -> VMResult<Option<Value>> {
        self.hostcalls.invoke(id, args)
    }

    /// Drain events committed by a successful atomic execution.
    pub fn drain_events(&mut self) -> Vec<VmEvent> {
        let events = self.event_buffer.drain_committed();
        if matches!(self.state_machine.state(), VmState::Committing) {
            let _ = self.state_machine.finish_commit();
            let _ = self.state_machine.reset();
        }
        events
    }

    /// Tell the engine what the host knows about this execution.
    ///
    /// Without a call to this every context opcode refuses with `ContextUnavailable`, and so does
    /// one whose field is `None` here.
    pub fn set_context(&mut self, context: BlockContext) {
        self.context = context;
    }

    /// The context the engine is executing with.
    pub fn context(&self) -> &BlockContext {
        &self.context
    }

    /// Drain storage writes for cross-VM delta sync.
    pub fn drain_storage_journal(&mut self) -> Vec<WriteRecord> {
        self.storage.drain_journal()
    }

    /// Snapshot JIT counters and compilation cache statistics.
    #[cfg(feature = "std")]
    pub fn jit_stats(&self) -> JitStats {
        self.jit.stats()
    }

    /// Get the loaded module.
    pub fn module(&self) -> &BytecodeModule {
        &self.module
    }

    /// Get gas used.
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    /// Instructions executed so far.
    ///
    /// `ExecutionResult` carries this for a completed call; the accessor is for the paths that
    /// return an error, where the receipt still has to report the work that did happen rather than
    /// zero (TICKET-130).
    pub fn instruction_count(&self) -> u64 {
        self.instruction_count
    }

    /// Set a register value directly.
    ///
    /// Useful for testing or pre-initializing registers before execution.
    /// Panics if the register index is out of bounds.
    pub fn set_register(&mut self, idx: usize, value: Value) {
        self.regs[idx] = value;
    }

    /// Get a register value.
    pub fn get_register(&self, idx: usize) -> &Value {
        &self.regs[idx]
    }

    /// Resolve a virtual register index to the underlying physical register
    /// using the current frame base. Returns an error if out of bounds.
    fn resolve_reg_checked(&self, reg: usize, ip: usize) -> VMResult<usize> {
        let base = self.call_stack.last().map(|f| f.base).unwrap_or(0);
        let idx = base + reg;
        if idx >= self.regs.len() {
            return Err(self.error_at(ip, VMErrorKind::RegisterOutOfBounds(reg as u16)));
        }
        Ok(idx)
    }

    /// Call a function by index.
    pub fn call_function(&mut self, func_idx: usize, args: &[Value]) -> VMResult<ExecutionResult> {
        if matches!(
            self.state_machine.state(),
            VmState::Committed | VmState::Reverted
        ) {
            self.state_machine.reset().map_err(|err| {
                VMError::without_ip(VMErrorKind::InvalidFunction(format!("{err:?}")))
            })?;
        }
        self.state_machine
            .begin_execution()
            .map_err(|err| VMError::without_ip(VMErrorKind::InvalidFunction(format!("{err:?}"))))?;
        self.hostcalls.reset_execution_count();
        self.isolation
            .enter_call()
            .map_err(|err| VMError::without_ip(VMErrorKind::HostcallError(format!("{err:?}"))))?;

        let result = self.call_function_inner(func_idx, args);
        self.isolation.exit_call();
        match result {
            Ok(result) => {
                self.state_machine.signal_success().map_err(|err| {
                    VMError::without_ip(VMErrorKind::InvalidFunction(format!("{err:?}")))
                })?;
                // An execution that returns is the commit for anything still pending. `emit`
                // outside an atomic window left its events pending for ever, and `drain_events`
                // takes only committed ones — the mirror of the `rollback` below, which was the
                // only half that existed while nothing emitted.
                self.event_buffer.commit();
                Ok(result)
            }
            Err(err) => {
                let _ = self.state_machine.revert();
                self.event_buffer.rollback();
                Err(err)
            }
        }
    }

    fn call_function_inner(
        &mut self,
        func_idx: usize,
        args: &[Value],
    ) -> VMResult<ExecutionResult> {
        // Validate function index
        if func_idx >= self.module.functions.len() {
            return Err(VMError::without_ip(VMErrorKind::FunctionNotFound(func_idx)));
        }

        let func = &self.module.functions[func_idx];

        // Validate argument count
        if args.len() != func.param_count as usize {
            return Err(VMError::without_ip(VMErrorKind::ArgumentCountMismatch(
                func.param_count as usize,
                args.len(),
            )));
        }

        // Set up registers with arguments
        for (i, arg) in args.iter().enumerate() {
            self.regs[i] = arg.clone();
        }

        // Push initial frame
        self.call_stack.push(Frame {
            ip: func.entry_point as usize,
            base: 0,
            ret_addr: usize::MAX, // Sentinel for top-level return
            func_idx,
            // Nothing reads this on the top-level return: the value is returned to the caller of
            // `call_function`, not to a register.
            ret_dst: 0,
        });

        // Execute
        let result = self.execute()?;

        Ok(ExecutionResult {
            value: result,
            gas_used: self.gas_used,
            instruction_count: self.instruction_count,
        })
    }

    /// Call a function by name.
    pub fn call_function_by_name(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> VMResult<ExecutionResult> {
        let func_idx = self
            .module
            .functions
            .iter()
            .position(|f| f.name == name)
            .ok_or_else(|| {
                VMError::without_ip(VMErrorKind::FunctionNotFoundByName(name.to_string()))
            })?;
        self.call_function(func_idx, args)
    }

    /// Main execution loop.
    fn execute(&mut self) -> VMResult<Option<Value>> {
        loop {
            // Check gas limit
            if self.gas_used >= self.config.gas_limit {
                return Err(self.error(VMErrorKind::GasLimitExceeded));
            }

            // Get current frame
            let frame = match self.call_stack.last_mut() {
                Some(f) => f,
                None => return Ok(None), // No frames left
            };

            let ip = frame.ip;

            // Bounds check
            if ip >= self.module.code.len() {
                return Err(self.error_at(ip, VMErrorKind::InstructionPointerOutOfBounds));
            }

            // Fetch opcode
            let opcode_byte = self.module.code[ip];
            let opcode = Opcode::from_byte(opcode_byte)
                .ok_or_else(|| self.error_at(ip, VMErrorKind::InvalidOpcode(opcode_byte)))?;

            // Consume gas
            self.gas_used += self.opcode_gas_cost(opcode);
            self.instruction_count += 1;

            // Trace if enabled
            if self.config.trace {
                log::trace!("[VM] IP={:04x} {:?}", ip, opcode);
            }

            // Execute instruction
            match self.execute_instruction(opcode, ip)? {
                StepResult::Continue(next_ip) => {
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = next_ip;
                    }
                }
                StepResult::Return(value) => {
                    // Pop frame (should never underflow if VM logic is correct)
                    let frame = self
                        .call_stack
                        .pop()
                        .ok_or_else(|| self.error_at(ip, VMErrorKind::ReturnFromEmptyStack))?;
                    if frame.ret_addr == usize::MAX {
                        // Top-level return
                        return Ok(value);
                    }
                    // The value goes to the register the caller named in the `Call`'s `dst`
                    // operand, resolved in the caller's frame — not unconditionally to its `r0`.
                    if let Some(v) = value {
                        if let Some(caller) = self.call_stack.last() {
                            // `caller.base + ret_dst` is already absolute: resolving it again would
                            // add the base a second time, which is invisible at the top level (base
                            // 0) and wrong one frame in — measured, `fib.x3` read `Unit` from the
                            // register it expected a result in (TICKET-131).
                            let idx = caller.base + frame.ret_dst;
                            if idx >= self.regs.len() {
                                return Err(
                                    self.error_at(ip, VMErrorKind::RegisterOutOfBounds(idx as u16))
                                );
                            }
                            self.regs[idx] = v;
                        } else {
                            self.regs[0] = v;
                        }
                    }
                    // Resume at return address
                    if let Some(f) = self.call_stack.last_mut() {
                        f.ip = frame.ret_addr;
                    }
                }
                StepResult::Halt => {
                    return Ok(None);
                }
            }
        }
    }

    /// Execute a single instruction.
    fn execute_instruction(&mut self, opcode: Opcode, ip: usize) -> VMResult<StepResult> {
        let _code = &self.module.code;

        match opcode {
            // ================================================================
            // Control Flow
            // ================================================================
            Opcode::Nop => Ok(StepResult::Continue(ip + 1)),

            Opcode::Jump => {
                let target = self.read_u32(ip + 1)? as usize;
                Ok(StepResult::Continue(target))
            }

            Opcode::JumpIf => {
                // Frame-relative, like every other register operand. These two arms indexed
                // `self.regs[cond_reg]` directly, so inside a callee frame the condition was read
                // from the *caller's* register of that number: measured, the compiler's own
                // `match_cond.x3` failed with `TypeMismatch("bool", "Unit")` because the register it
                // read belongs to another frame (TICKET-131).
                let cond_reg = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let target = self.read_u32(ip + 2)? as usize;
                if self.regs[cond_reg].as_bool()? {
                    Ok(StepResult::Continue(target))
                } else {
                    Ok(StepResult::Continue(ip + 6))
                }
            }

            Opcode::JumpUnless => {
                // Frame-relative, for the same reason as `JumpIf` (TICKET-131).
                let cond_reg = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let target = self.read_u32(ip + 2)? as usize;
                if !self.regs[cond_reg].as_bool()? {
                    Ok(StepResult::Continue(target))
                } else {
                    Ok(StepResult::Continue(ip + 6))
                }
            }

            Opcode::Call => {
                // call dst:reg func:u32 argc:u16 [args:reg...]
                let dst = self.read_u8(ip + 1)? as usize;
                let func_idx = self.read_u32(ip + 2)? as usize;
                let argc = self.read_u16(ip + 6)? as usize;

                if func_idx >= self.module.functions.len() {
                    return Err(self.error_at(ip, VMErrorKind::FunctionNotFound(func_idx)));
                }

                if self.call_stack.len() >= self.config.max_call_depth {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::StackOverflow(
                            self.call_stack.len(),
                            self.config.max_call_depth,
                        ),
                    ));
                }

                #[cfg(feature = "std")]
                {
                    let func_id = func_idx as u32;
                    self.jit.record_execution(func_id);
                    if self.jit.should_compile(func_id)
                        && self.jit.get_compiled(func_id).is_none()
                        && self.jit.backend_available()
                    {
                        self.jit
                            .compile(func_id, &self.module.code)
                            .map_err(|err| self.error_at(ip, VMErrorKind::HostcallError(err)))?;
                    }
                }

                self.isolation.enter_call().map_err(|err| {
                    self.error_at(ip, VMErrorKind::HostcallError(format!("{err:?}")))
                })?;

                // Read argument registers from caller (respect caller base)
                let mut args = Vec::with_capacity(argc);
                for i in 0..argc {
                    let arg_reg = self.read_u8(ip + 8 + i)? as usize;
                    let resolved = self.resolve_reg_checked(arg_reg, ip)?;
                    args.push(self.regs[resolved].clone());
                }

                let func = &self.module.functions[func_idx];
                let ret_addr = ip + 8 + argc;

                // The callee's window starts after the caller's **whole** frame: its parameters and
                // its locals/temporaries. Using `local_count` alone started every callee inside its
                // caller, so a recursive call overwrote the registers the caller was still using
                // (TICKET-131).
                let (caller_base, caller_footprint) = self
                    .call_stack
                    .last()
                    .map(|f| {
                        let entry = &self.module.functions[f.func_idx];
                        (
                            f.base,
                            entry.param_count as usize + entry.local_count as usize,
                        )
                    })
                    .unwrap_or((0, 0));
                let callee_base = caller_base + caller_footprint;

                // A call passes exactly the callee's parameters: the arguments are written into the
                // callee's window below and `argc` is a `u16` from the code stream, so a larger
                // count wrote past the register file and a smaller one left parameters holding a
                // previous frame's values. `mini_x3` applies the same two rules.
                if argc != func.param_count as usize {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::ArgumentCountMismatch(func.param_count as usize, argc),
                    ));
                }

                // Bounds check for the callee window: parameters and locals both have to fit. A
                // window that does not fit means the call stack's registers are spent — a stack
                // overflow, as `mini_x3` reports it too, not a bad register operand.
                if callee_base + func.param_count as usize + func.local_count as usize
                    > MAX_REGISTERS
                {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::StackOverflow(
                            self.call_stack.len(),
                            self.config.max_call_depth,
                        ),
                    ));
                }

                // Place args into callee register window
                for (i, arg) in args.into_iter().enumerate() {
                    self.regs[callee_base + i] = arg;
                }

                // Push new frame with computed base
                self.call_stack.push(Frame {
                    ip: func.entry_point as usize,
                    base: callee_base,
                    ret_addr,
                    func_idx,
                    ret_dst: dst,
                });

                Ok(StepResult::Continue(func.entry_point as usize))
            }

            Opcode::Ret => {
                let src = self.read_u8(ip + 1)? as usize;
                let src_resolved = self.resolve_reg_checked(src, ip)?;
                let value = self.regs[src_resolved].clone();
                self.isolation.exit_call();
                Ok(StepResult::Return(Some(value)))
            }

            Opcode::RetVoid => {
                self.isolation.exit_call();
                Ok(StepResult::Return(None))
            }

            Opcode::Halt => Ok(StepResult::Halt),

            // ================================================================
            // Load/Store
            // ================================================================
            Opcode::LoadConst => {
                let dst = self.read_u8(ip + 1)? as usize;
                let idx = self.read_u32(ip + 2)? as usize;

                if idx >= self.module.const_pool.entries.len() {
                    return Err(self.error_at(ip, VMErrorKind::ConstPoolOutOfBounds(idx)));
                }

                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = Value::from_const(&self.module.const_pool.entries[idx]);
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::Mov => {
                let dst = self.read_u8(ip + 1)? as usize;
                let src = self.read_u8(ip + 2)? as usize;
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                let src_r = self.resolve_reg_checked(src, ip)?;
                self.regs[dst_r] = self.regs[src_r].clone();
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::LoadGlobal => {
                let dst = self.read_u8(ip + 1)? as usize;
                let idx = self.read_u32(ip + 2)? as usize;
                if idx >= self.globals.len() {
                    return Err(self.error_at(ip, VMErrorKind::GlobalOutOfBounds(idx as u32)));
                }
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = self.globals[idx].clone();
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::StoreGlobal => {
                let idx = self.read_u32(ip + 1)? as usize;
                let src = self.read_u8(ip + 5)? as usize;
                if idx >= self.globals.len() {
                    return Err(self.error_at(ip, VMErrorKind::GlobalOutOfBounds(idx as u32)));
                }
                let src_r = self.resolve_reg_checked(src, ip)?;
                // enforce mutability if module metadata says so
                if !self
                    .module
                    .globals
                    .get(idx)
                    .map(|g| g.mutable)
                    .unwrap_or(false)
                {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::UserPanic(format!("global {} is immutable", idx)),
                    ));
                }
                self.globals[idx] = self.regs[src_r].clone();
                if let Some(value) = value_to_storage_value(&self.regs[src_r]) {
                    self.storage
                        .set(storage_key_for_global(idx), Some(value))
                        .map_err(|err| {
                            self.error_at(ip, VMErrorKind::HostcallError(format!("{err:?}")))
                        })?;
                }
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::LoadImm => {
                let dst = self.read_u8(ip + 1)? as usize;
                let val = self.read_i8(ip + 2)?;
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = Value::I64(val as i64);
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::LoadZero => {
                let dst = self.read_u8(ip + 1)? as usize;
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = Value::I64(0);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::LoadTrue => {
                let dst = self.read_u8(ip + 1)? as usize;
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = Value::Bool(true);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::LoadFalse => {
                let dst = self.read_u8(ip + 1)? as usize;
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = Value::Bool(false);
                Ok(StepResult::Continue(ip + 2))
            }

            // ================================================================
            // Integer Arithmetic
            // ================================================================
            Opcode::AddI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_add(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::SubI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_sub(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::MulI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va.wrapping_mul(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::DivI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                if vb == 0 {
                    return Err(self.error_at(ip, VMErrorKind::DivisionByZero));
                }
                // `i64::MIN / -1` overflows, and Rust panics on it in release builds too: a program
                // dividing those two values would take the executor down with it. Integer
                // arithmetic here wraps (add, sub, mul and neg already do), so division does too.
                self.regs[dst] = Value::I64(va.wrapping_div(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::ModI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                if vb == 0 {
                    return Err(self.error_at(ip, VMErrorKind::DivisionByZero));
                }
                // `i64::MIN % -1` panics for the same reason as the division above; it wraps to 0.
                self.regs[dst] = Value::I64(va.wrapping_rem(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NegI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let src = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let v = self.regs[src].as_i64()?;
                self.regs[dst] = Value::I64(v.wrapping_neg());
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Float Arithmetic
            // ================================================================
            Opcode::AddF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va + vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::SubF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va - vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::MulF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::F64(va * vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::DivF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                // Float division by zero produces infinity, not error
                self.regs[dst] = Value::F64(va / vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::ModF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                // IEEE remainder with the sign of the dividend, like `DivF` producing infinity
                // rather than an error: `x % 0.0` is NaN. Floats are refused on chain by the
                // verifier (`deny_float_arithmetic`), so this arm only serves off-chain tools.
                self.regs[dst] = Value::F64(va % vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NegF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let src = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let v = self.regs[src].as_f64()?;
                self.regs[dst] = Value::F64(-v);
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Comparisons
            // ================================================================
            Opcode::EqI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va == vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NeI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va != vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LtI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va < vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LeI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va <= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GtI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va > vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GeI => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::Bool(va >= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            // Float comparisons
            Opcode::EqF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va == vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NeF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va != vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LtF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va < vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LeF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va <= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GtF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va > vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GeF => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_f64()?;
                let vb = self.regs[b].as_f64()?;
                self.regs[dst] = Value::Bool(va >= vb);
                Ok(StepResult::Continue(ip + 4))
            }

            // ================================================================
            // Bitwise Operations
            // ================================================================
            Opcode::And => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va & vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Or => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va | vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Xor => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()?;
                self.regs[dst] = Value::I64(va ^ vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Not => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let src = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let v = self.regs[src].as_i64()?;
                self.regs[dst] = Value::I64(!v);
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::Shl => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shl(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::Shr => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()?;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shr(vb));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::UShr => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_i64()? as u64;
                let vb = self.regs[b].as_i64()? as u32;
                self.regs[dst] = Value::I64(va.wrapping_shr(vb) as i64);
                Ok(StepResult::Continue(ip + 4))
            }

            // ================================================================
            // Logical Operations
            // ================================================================
            Opcode::LAnd => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_bool()?;
                let vb = self.regs[b].as_bool()?;
                self.regs[dst] = Value::Bool(va && vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LOr => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let a = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let b = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let va = self.regs[a].as_bool()?;
                let vb = self.regs[b].as_bool()?;
                self.regs[dst] = Value::Bool(va || vb);
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::LNot => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let src = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let v = self.regs[src].as_bool()?;
                self.regs[dst] = Value::Bool(!v);
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Atomic Operations
            // ================================================================
            Opcode::AtomicBegin => {
                // Every open window holds a full copy of the registers and globals, so the nesting
                // depth bounds the memory a program can make the engine hold on the chain's behalf.
                // An unbounded `AtomicBegin` in a loop is a memory amplification; refuse it.
                if self.atomic_depth >= MAX_ATOMIC_DEPTH {
                    return Err(
                        self.error_at(ip, VMErrorKind::AtomicDepthExceeded(MAX_ATOMIC_DEPTH))
                    );
                }
                // snapshot regs + globals
                self.atomic_snapshots
                    .push((self.regs.clone(), self.globals.clone()));
                self.storage.snapshot();
                self.atomic_depth += 1;
                Ok(StepResult::Continue(ip + 3)) // opcode + id:u16
            }

            Opcode::AtomicCommit => {
                if self.atomic_depth == 0 {
                    return Err(self.error_at(ip, VMErrorKind::AtomicEndWithoutBegin));
                }
                // commit: discard last snapshot
                self.atomic_snapshots.pop();
                self.storage.commit().map_err(|err| {
                    self.error_at(ip, VMErrorKind::HostcallError(format!("{err:?}")))
                })?;
                self.event_buffer.commit();
                self.atomic_depth -= 1;
                Ok(StepResult::Continue(ip + 3)) // opcode + id:u16
            }

            // ================================================================
            // Aggregates: arrays, tuples, and structs held as tuples of fields.
            //
            // Operand layouts are the emitter's (`x3-backend/src/emit.rs`): field, capacity and
            // tuple indices are u16. Every write that can grow a value is checked against
            // `MAX_AGGREGATE_SIZE` before it lands.
            // ================================================================
            Opcode::Inc | Opcode::Dec => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let src = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let v = self.regs[src].as_i64()?;
                // Wrapping, like `AddI`/`SubI`.
                self.regs[dst] = Value::I64(if opcode == Opcode::Inc {
                    v.wrapping_add(1)
                } else {
                    v.wrapping_sub(1)
                });
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::NewArray => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let capacity = self.read_u16(ip + 2)? as usize;
                // The capacity is a hint; the size cap, not it, bounds what the array can hold.
                self.regs[dst] = Value::Array(Vec::with_capacity(capacity.min(MAX_AGGREGATE_SIZE)));
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::ArrayLen => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let arr = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let len = match &self.regs[arr] {
                    Value::Array(items) | Value::Tuple(items) => items.len(),
                    Value::Bytes(bytes) => bytes.len(),
                    Value::String(text) => text.len(),
                    other => return Err(self.type_mismatch(ip, "array", other)),
                };
                self.regs[dst] = Value::I64(len as i64);
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::ArrayPush => {
                let arr = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let val = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let item = self.regs[val].clone();
                let grown = self.regs[arr].size() + item.size();
                self.check_aggregate_size(ip, grown)?;
                match &mut self.regs[arr] {
                    Value::Array(items) => items.push(item),
                    other => {
                        let other = other.clone();
                        return Err(self.type_mismatch(ip, "array", &other));
                    }
                }
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::ArrayPop => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let arr = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let popped = match &mut self.regs[arr] {
                    Value::Array(items) => items.pop(),
                    other => {
                        let other = other.clone();
                        return Err(self.type_mismatch(ip, "array", &other));
                    }
                };
                let item = popped.ok_or_else(|| self.error_at(ip, VMErrorKind::EmptyArray))?;
                self.regs[dst] = item;
                Ok(StepResult::Continue(ip + 3))
            }

            Opcode::LoadIndex => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let arr = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let idx = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let index = self.regs[idx].as_i64()?;
                let item = match &self.regs[arr] {
                    Value::Array(items) | Value::Tuple(items) => {
                        let at = self.checked_index(ip, index, items.len())?;
                        items[at].clone()
                    }
                    Value::Bytes(bytes) => {
                        let at = self.checked_index(ip, index, bytes.len())?;
                        Value::I64(i64::from(bytes[at]))
                    }
                    other => return Err(self.type_mismatch(ip, "array", other)),
                };
                self.regs[dst] = item;
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::StoreIndex => {
                let arr = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let idx = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let val = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let index = self.regs[idx].as_i64()?;
                let item = self.regs[val].clone();
                let (at, replaced_size) = match &self.regs[arr] {
                    Value::Array(items) => {
                        let at = self.checked_index(ip, index, items.len())?;
                        (at, items[at].size())
                    }
                    // A tuple's shape is fixed by its constructor; its elements change through
                    // `store_field`, which names them by position at compile time.
                    other => return Err(self.type_mismatch(ip, "array", other)),
                };
                let grown = self.regs[arr].size() - replaced_size + item.size();
                self.check_aggregate_size(ip, grown)?;
                if let Value::Array(items) = &mut self.regs[arr] {
                    items[at] = item;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::NewTuple => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let count = self.read_u16(ip + 2)? as usize;
                let mut items = Vec::with_capacity(count.min(MAX_AGGREGATE_SIZE));
                let mut size = 1usize;
                for i in 0..count {
                    let reg = self.resolve_reg_checked(self.read_u8(ip + 4 + i)? as usize, ip)?;
                    size += self.regs[reg].size();
                    self.check_aggregate_size(ip, size)?;
                    items.push(self.regs[reg].clone());
                }
                self.regs[dst] = Value::Tuple(items);
                Ok(StepResult::Continue(ip + 4 + count))
            }

            Opcode::TupleGet | Opcode::LoadField => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let obj = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let field = self.read_u16(ip + 3)?;
                let item = match &self.regs[obj] {
                    Value::Tuple(items) => items.get(field as usize).cloned().ok_or_else(|| {
                        self.error_at(ip, VMErrorKind::TupleIndexOutOfBounds(field, items.len()))
                    })?,
                    other => return Err(self.type_mismatch(ip, "tuple", other)),
                };
                self.regs[dst] = item;
                Ok(StepResult::Continue(ip + 5))
            }

            Opcode::StoreField => {
                let obj = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let field = self.read_u16(ip + 2)?;
                let val = self.resolve_reg_checked(self.read_u8(ip + 4)? as usize, ip)?;
                let item = self.regs[val].clone();
                let replaced_size = match &self.regs[obj] {
                    Value::Tuple(items) => {
                        items.get(field as usize).map(Value::size).ok_or_else(|| {
                            self.error_at(
                                ip,
                                VMErrorKind::TupleIndexOutOfBounds(field, items.len()),
                            )
                        })?
                    }
                    other => return Err(self.type_mismatch(ip, "tuple", other)),
                };
                let grown = self.regs[obj].size() - replaced_size + item.size();
                self.check_aggregate_size(ip, grown)?;
                if let Value::Tuple(items) = &mut self.regs[obj] {
                    items[field as usize] = item;
                }
                Ok(StepResult::Continue(ip + 5))
            }

            // ================================================================
            // Block context (see `context.rs`): a field the host did not supply is refused,
            // never answered with a default.
            // ================================================================
            Opcode::CtxSender => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let sender = self
                    .context
                    .sender
                    .ok_or_else(|| self.error_at(ip, VMErrorKind::ContextUnavailable("sender")))?;
                self.regs[dst] = Value::Bytes(sender.to_vec());
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::CtxBlockHeight => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let height = self.context.block_height.ok_or_else(|| {
                    self.error_at(ip, VMErrorKind::ContextUnavailable("block height"))
                })?;
                self.regs[dst] = Value::I64(self.context_i64(ip, height as u128)?);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::CtxTimestamp => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let millis = self.context.timestamp_ms.ok_or_else(|| {
                    self.error_at(ip, VMErrorKind::ContextUnavailable("timestamp"))
                })?;
                self.regs[dst] = Value::I64(self.context_i64(ip, millis as u128)?);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::CtxValue => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let value = self
                    .context
                    .value
                    .ok_or_else(|| self.error_at(ip, VMErrorKind::ContextUnavailable("value")))?;
                self.regs[dst] = Value::I64(self.context_i64(ip, value)?);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::CtxChainId => {
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let chain = self.context.chain_id.ok_or_else(|| {
                    self.error_at(ip, VMErrorKind::ContextUnavailable("chain id"))
                })?;
                self.regs[dst] = Value::I64(self.context_i64(ip, chain as u128)?);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::AtomicCheck => {
                // `atomic_check dst`: whether an atomic window is open at this instruction.
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                self.regs[dst] = Value::Bool(self.atomic_depth > 0);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::AtomicRollback => {
                if self.atomic_depth == 0 {
                    return Err(self.error_at(ip, VMErrorKind::AtomicRollbackWithoutBegin));
                }
                // restore last snapshot
                if let Some((regs_snap, globals_snap)) = self.atomic_snapshots.pop() {
                    self.regs = regs_snap;
                    self.globals = globals_snap;
                }
                self.storage.rollback().map_err(|err| {
                    self.error_at(ip, VMErrorKind::HostcallError(format!("{err:?}")))
                })?;
                self.event_buffer.rollback();
                self.atomic_depth -= 1;
                Err(self.error_at(ip, VMErrorKind::AtomicAborted))
            }

            // ================================================================
            // Contract storage (the EVM slot keyspace)
            //
            // These two opcodes are the only way a program can persist a value
            // across calls: `StoreGlobal` writes a module-scoped global into the
            // storage map under the *global* keyspace, and until these arms
            // existed nothing in the interpreter wrote a contract slot at all —
            // `evm_sstore` reached the `_` arm and returned `UnimplementedOpcode`,
            // so a deployed X3VM program could not carry state between blocks.
            //
            // Operand encoding is the one `x3-backend`'s emitters use:
            //   evm_sload  dst:reg slot:reg
            //   evm_sstore slot:reg val:reg
            // Both operands are registers; the slot register must hold a
            // non-negative integer.
            // ================================================================
            Opcode::EvmSload => {
                let dst = self.read_u8(ip + 1)? as usize;
                let slot_reg = self.read_u8(ip + 2)? as usize;
                let slot_r = self.resolve_reg_checked(slot_reg, ip)?;
                let slot = slot_number(&self.regs[slot_r])
                    .map_err(|reason| self.error_at(ip, VMErrorKind::InvalidStorageSlot(reason)))?;
                // A slot that was never written reads as zero. That is the EVM's
                // own rule and it is a defined value, not an uncertainty: the key
                // space is disjoint from the global key space (see `evm_slot_key`),
                // so "absent" cannot be confused with a global's value.
                let value = match self.storage.get(&evm_slot_key(slot)) {
                    Some(payload) => decode_slot_payload(payload).map_err(|reason| {
                        self.error_at(ip, VMErrorKind::CorruptStorageSlot(reason))
                    })?,
                    None => Value::I64(0),
                };
                let dst_r = self.resolve_reg_checked(dst, ip)?;
                self.regs[dst_r] = value;
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Events
            // ================================================================
            //
            // `emit name:reg argc:u16 args:reg*`. The name is a byte string in a register; the
            // event's topic is its SHA-256, so a reader who knows the name can find its events
            // without the artifact. The payload is each argument as `[tag][len][data]`
            // (`x3_common::value_tags`), the same tagging a storage slot uses, so one run of bytes
            // decodes without knowing the argument types in advance.
            //
            // Nothing implemented this opcode in either engine: `EventBuffer` existed and no
            // instruction ever filled it, and the compiler refused `emit` at lowering. A program
            // whose receipt is supposed to carry an event produced a receipt without one.
            Opcode::Emit => {
                let name_reg = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let argc = self.read_u16(ip + 2)? as usize;
                let name = match &self.regs[name_reg] {
                    Value::Bytes(bytes) => bytes.clone(),
                    Value::String(text) => text.as_bytes().to_vec(),
                    other => {
                        return Err(self.error_at(
                            ip,
                            VMErrorKind::UnjournalableEvent(format!(
                                "an event's name must be a byte string, got {other:?}"
                            )),
                        ))
                    }
                };
                if name.is_empty() {
                    return Err(self.error_at(
                        ip,
                        VMErrorKind::UnjournalableEvent("an event must be named".to_string()),
                    ));
                }
                let mut data = Vec::new();
                for index in 0..argc {
                    let arg =
                        self.resolve_reg_checked(self.read_u8(ip + 4 + index)? as usize, ip)?;
                    let (tag, bytes) = tagged_value(&self.regs[arg]).map_err(|reason| {
                        self.error_at(ip, VMErrorKind::UnjournalableEvent(reason))
                    })?;
                    data.push(tag);
                    data.push(bytes.len() as u8);
                    data.extend_from_slice(&bytes);
                }
                let topic = sp_core::hashing::sha2_256(&name);
                // The emitter is the contract identity, which this engine does not have: an
                // execution here is a module, not a deployed account. It stays zero rather than
                // inventing one, and the receipt carries the topic and the payload, not this field.
                self.event_buffer
                    .emit([0u8; 32], vec![topic], data)
                    .map_err(|error| {
                        self.error_at(ip, VMErrorKind::UnjournalableEvent(format!("{error:?}")))
                    })?;
                Ok(StepResult::Continue(ip + 4 + argc))
            }

            Opcode::EvmSstore => {
                let slot_reg = self.read_u8(ip + 1)? as usize;
                let val_reg = self.read_u8(ip + 2)? as usize;
                let slot_r = self.resolve_reg_checked(slot_reg, ip)?;
                let val_r = self.resolve_reg_checked(val_reg, ip)?;
                let slot = slot_number(&self.regs[slot_r])
                    .map_err(|reason| self.error_at(ip, VMErrorKind::InvalidStorageSlot(reason)))?;
                // Refuse rather than truncate: `value_to_storage_value` silently
                // clips anything longer than 32 bytes, which for a contract write
                // would persist a different value than the program handed us.
                let payload = encode_slot_payload(&self.regs[val_r]).map_err(|reason| {
                    self.error_at(ip, VMErrorKind::UnencodableStorageValue(reason))
                })?;
                self.storage
                    .set(evm_slot_key(slot), Some(payload))
                    .map_err(|err| {
                        self.error_at(ip, VMErrorKind::HostcallError(format!("{err:?}")))
                    })?;
                Ok(StepResult::Continue(ip + 3))
            }

            // ================================================================
            // Debug Operations (no-op in production)
            // ================================================================
            Opcode::DebugPrint => {
                let src = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                log::debug!("[DEBUG] r{} = {:?}", src, self.regs[src]);
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::Breakpoint => {
                log::debug!("[DEBUG] BREAK at IP={}", ip);
                Ok(StepResult::Continue(ip + 1))
            }

            Opcode::Assert => {
                let cond = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let _msg_idx = self.read_u32(ip + 2)?;
                if !self.regs[cond].as_bool()? {
                    return Err(self.error_at(ip, VMErrorKind::AssertionFailed));
                }
                Ok(StepResult::Continue(ip + 6))
            }

            Opcode::Panic => {
                let msg_idx = self.read_u32(ip + 1)? as usize;
                let msg = if msg_idx < self.module.const_pool.entries.len() {
                    if let ConstValue::String(s) = &self.module.const_pool.entries[msg_idx] {
                        s.clone()
                    } else {
                        "panic".to_string()
                    }
                } else {
                    "panic".to_string()
                };
                Err(self.error_at(ip, VMErrorKind::UserPanic(msg)))
            }

            // ================================================================
            // GPU Intrinsics (0xD0 – 0xD5)
            // Dispatch to registered hostcalls which call real CUDA kernels
            // via libloading FFI (see gpu_hostcalls.rs).
            //
            // Encoding:
            //   GpuSha256Batch:    [0xD0] dst:u8 inputs:u8 count:u8   → 4 bytes
            //   GpuEd25519Verify:  [0xD1] dst:u8 sigs:u8 count:u8     → 4 bytes
            //   GpuPohChain:       [0xD2] dst:u8 seeds:u8 count:u8 chain_len:u8 → 5 bytes
            //   GpuSha256Streamed: [0xD3] dst:u8 inputs:u8 count:u8 streams:u8  → 5 bytes
            //   GpuDeviceCount:    [0xD4] dst:u8                       → 2 bytes
            //   GpuBenchmark:      [0xD5] dst:u8 count:u8 streams:u8  → 4 bytes
            // ================================================================
            // GPU intrinsics read their register operands frame-relative and checked, like every
            // other instruction: they used to index `regs` with the raw operand, so inside a callee
            // they read and wrote the caller's registers.
            Opcode::GpuSha256Batch => {
                // gpu_sha256_batch(inputs: Bytes, count: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let inputs_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let args = vec![self.regs[inputs_reg].clone(), self.regs[count_reg].clone()];
                let result = self
                    .hostcalls
                    .invoke(0xD0, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GpuEd25519Verify => {
                // gpu_ed25519_verify(sigs: Bytes, count: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let sigs_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let args = vec![self.regs[sigs_reg].clone(), self.regs[count_reg].clone()];
                let result = self
                    .hostcalls
                    .invoke(0xD1, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GpuPohChain => {
                // gpu_poh_chain(seeds: Bytes, num_chains: I64, chain_length: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let seeds_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let chain_len_reg = self.resolve_reg_checked(self.read_u8(ip + 4)? as usize, ip)?;
                let args = vec![
                    self.regs[seeds_reg].clone(),
                    self.regs[count_reg].clone(),
                    self.regs[chain_len_reg].clone(),
                ];
                let result = self
                    .hostcalls
                    .invoke(0xD2, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 5))
            }

            Opcode::GpuSha256Streamed => {
                // gpu_sha256_streamed(inputs: Bytes, count: I64, streams: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let inputs_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let streams_reg = self.resolve_reg_checked(self.read_u8(ip + 4)? as usize, ip)?;
                let args = vec![
                    self.regs[inputs_reg].clone(),
                    self.regs[count_reg].clone(),
                    self.regs[streams_reg].clone(),
                ];
                let result = self
                    .hostcalls
                    .invoke(0xD3, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 5))
            }

            Opcode::GpuDeviceCount => {
                // gpu_device_count() → I64
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let result = self
                    .hostcalls
                    .invoke(0xD4, &[])
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 2))
            }

            Opcode::GpuBenchmark => {
                // gpu_benchmark(count: I64, streams: I64) → Bytes (JSON)
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let streams_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let args = vec![self.regs[count_reg].clone(), self.regs[streams_reg].clone()];
                let result = self
                    .hostcalls
                    .invoke(0xD5, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GpuKeccak256Batch => {
                // gpu_keccak256_batch(inputs: Bytes, count: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let inputs_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let args = vec![self.regs[inputs_reg].clone(), self.regs[count_reg].clone()];
                let result = self
                    .hostcalls
                    .invoke(0xD6, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            Opcode::GpuSecp256k1Verify => {
                // gpu_secp256k1_verify(sigs: Bytes, count: I64) → Bytes
                let dst = self.resolve_reg_checked(self.read_u8(ip + 1)? as usize, ip)?;
                let sigs_reg = self.resolve_reg_checked(self.read_u8(ip + 2)? as usize, ip)?;
                let count_reg = self.resolve_reg_checked(self.read_u8(ip + 3)? as usize, ip)?;
                let args = vec![self.regs[sigs_reg].clone(), self.regs[count_reg].clone()];
                let result = self
                    .hostcalls
                    .invoke(0xD7, &args)
                    .map_err(|e| self.error_at(ip, e.kind))?;
                if let Some(v) = result {
                    self.regs[dst] = v;
                }
                Ok(StepResult::Continue(ip + 4))
            }

            // ================================================================
            // Unimplemented opcodes return error
            // ================================================================
            _ => {
                let opc = self.module.code[ip];
                Err(self.error_at(ip, VMErrorKind::UnimplementedOpcode(opc)))
            }
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    fn read_u8(&self, offset: usize) -> VMResult<u8> {
        self.module
            .code
            .get(offset)
            .copied()
            .ok_or_else(|| self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds))
    }

    fn read_i8(&self, offset: usize) -> VMResult<i8> {
        Ok(self.read_u8(offset)? as i8)
    }

    fn read_u16(&self, offset: usize) -> VMResult<u16> {
        if offset + 2 > self.module.code.len() {
            return Err(self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds));
        }
        Ok(u16::from_le_bytes([
            self.module.code[offset],
            self.module.code[offset + 1],
        ]))
    }

    fn read_u32(&self, offset: usize) -> VMResult<u32> {
        if offset + 4 > self.module.code.len() {
            return Err(self.error_at(offset, VMErrorKind::InstructionPointerOutOfBounds));
        }
        Ok(u32::from_le_bytes([
            self.module.code[offset],
            self.module.code[offset + 1],
            self.module.code[offset + 2],
            self.module.code[offset + 3],
        ]))
    }

    fn error(&self, kind: VMErrorKind) -> VMError {
        VMError::without_ip(kind)
    }

    fn error_at(&self, ip: usize, kind: VMErrorKind) -> VMError {
        VMError::at_ip(ip, kind)
    }

    fn type_mismatch(&self, ip: usize, expected: &str, found: &Value) -> VMError {
        self.error_at(
            ip,
            VMErrorKind::TypeMismatch(expected.to_string(), format!("{found:?}")),
        )
    }

    /// `index` as a position in a sequence of `len`, refused when negative or past the end.
    fn checked_index(&self, ip: usize, index: i64, len: usize) -> VMResult<usize> {
        usize::try_from(index)
            .ok()
            .filter(|at| *at < len)
            .ok_or_else(|| self.error_at(ip, VMErrorKind::ArrayIndexOutOfBounds(index, len)))
    }

    fn check_aggregate_size(&self, ip: usize, size: usize) -> VMResult<()> {
        if size > MAX_AGGREGATE_SIZE {
            return Err(self.error_at(ip, VMErrorKind::AggregateTooLarge(size, MAX_AGGREGATE_SIZE)));
        }
        Ok(())
    }

    /// A context figure as the engine's integer, refused rather than wrapped when it does not fit.
    fn context_i64(&self, ip: usize, figure: u128) -> VMResult<i64> {
        i64::try_from(figure).map_err(|_| self.error_at(ip, VMErrorKind::IntegerOverflow))
    }

    /// What one execution of `opcode` costs: the verifier's table, the only one (see
    /// `verifier::opcode_gas_cost`).
    fn opcode_gas_cost(&self, opcode: Opcode) -> u64 {
        crate::verifier::opcode_gas_cost(opcode as u8)
    }
}

/// Result of executing one instruction.
enum StepResult {
    /// Continue to next IP.
    Continue(usize),
    /// Return from current function.
    Return(Option<Value>),
    /// Halt execution.
    Halt,
}

fn storage_key_for_global(idx: usize) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&(idx as u64).to_le_bytes());
    key
}

/// Domain tag that separates the EVM slot keyspace from the global-variable keyspace
/// inside the one storage map the interpreter owns.
///
/// `storage_key_for_global` writes a global index into the first eight bytes and leaves
/// bytes 8..32 zero; the interpreter reads global indices as `u32` (`LoadGlobal` reads a
/// `u32`), so no global can address a key whose first eight bytes are this tag. The
/// disjointness is asserted by a test, because "the slot was never written" (reads zero)
/// must never be reachable for a key a global owns.
const EVM_SLOT_DOMAIN: u64 = 0x5833_4556_4D5F_534C; // "X3EVM_SL"

/// Storage key for one EVM slot: domain tag, then the slot number, then zeros.
fn evm_slot_key(slot: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&EVM_SLOT_DOMAIN.to_le_bytes());
    key[8..16].copy_from_slice(&slot.to_le_bytes());
    key
}

// Slot payload tags. A slot holds a tag byte, a length byte and at most 30 bytes of data,
// which is what makes a store/load round trip exact: the alternative (the untagged layout
// `value_to_storage_value` uses) cannot tell `Bytes([1, 2, 3])` from `I64(197_121)`, and a
// zero-padded variable-length payload cannot tell `Bytes([1, 2])` from `Bytes([1, 2, 0])`,
// so either layout would hand a program back a value it never stored.
use x3_common::value_tags::{
    ADDR as SLOT_TAG_ADDR, BOOL as SLOT_TAG_BOOL, BYTES as SLOT_TAG_BYTES, F64 as SLOT_TAG_F64,
    INT as SLOT_TAG_INT, STRING as SLOT_TAG_STRING,
};

/// Bytes of data a slot can carry: the 32-byte word minus the tag and length bytes.
const SLOT_PAYLOAD_MAX: usize = 30;

/// Read the slot number out of a slot operand.
///
/// Slots are addressed by a non-negative integer. A negative or non-integer operand is
/// refused by name rather than coerced: `-1 as u64` would silently become slot
/// 18446744073709551615, which is a different slot than the program asked for.
fn slot_number(value: &Value) -> Result<u64, String> {
    match value {
        Value::I64(n) if *n >= 0 => Ok(*n as u64),
        Value::I64(n) => Err(format!("negative slot {n}")),
        other => Err(format!("non-integer slot {:?}", other)),
    }
}

/// Encode a value into a 32-byte slot payload, refusing anything that does not fit.
/// A value as `(tag, data)`, the form both a storage slot and an event payload are built from.
///
/// One function, so a slot and an event can never disagree about what a value is. The length is
/// one byte in both forms, so a value longer than 255 bytes has no tagged form at all; a slot is
/// tighter still and applies its own limit on top.
fn tagged_value(value: &Value) -> Result<(u8, Vec<u8>), String> {
    let (tag, data): (u8, Vec<u8>) = match value {
        Value::I64(n) => (SLOT_TAG_INT, n.to_le_bytes().to_vec()),
        Value::Bool(b) => (SLOT_TAG_BOOL, vec![u8::from(*b)]),
        Value::F64(f) => (SLOT_TAG_F64, f.to_bits().to_le_bytes().to_vec()),
        Value::Addr(a) => (SLOT_TAG_ADDR, a.to_le_bytes().to_vec()),
        Value::Bytes(bytes) => (SLOT_TAG_BYTES, bytes.clone()),
        Value::String(text) => (SLOT_TAG_STRING, text.as_bytes().to_vec()),
        // Unlike `StoreGlobal`, which ignores a Unit write, a contract store of Unit is refused:
        // ignoring it would report success for a write the program asked for and did not get.
        // Deleting a slot is not spelled this way, and an event argument that is Unit is a value
        // the program never computed.
        Value::Unit => return Err("unit is not a storable value".to_string()),
        // A slot is 32 bytes and an event argument is one tagged scalar; an aggregate is neither,
        // and flattening it would store something the program did not write.
        Value::Array(_) | Value::Tuple(_) => {
            return Err("an array or tuple is not a storable value".to_string())
        }
    };
    if data.len() > u8::MAX as usize {
        return Err(format!(
            "{} bytes of value; a tagged value carries at most {}",
            data.len(),
            u8::MAX
        ));
    }
    Ok((tag, data))
}

/// Encode a value into a 32-byte slot payload, refusing anything that does not fit.
///
/// A slot holds a tag byte, a length byte and at most `SLOT_PAYLOAD_MAX` bytes of data, which is
/// what makes a store/load round trip exact: the untagged layout `value_to_storage_value` uses
/// cannot tell `Bytes([1, 2, 3])` from `I64(197_121)`, and a zero-padded variable-length payload
/// cannot tell `Bytes([1, 2])` from `Bytes([1, 2, 0])`, so either would hand a program back a
/// value it never stored. A byte-string longer than the slot is a refusal, not a truncation.
fn encode_slot_payload(value: &Value) -> Result<[u8; 32], String> {
    let (tag, data) = tagged_value(value)?;
    if data.len() > SLOT_PAYLOAD_MAX {
        return Err(format!(
            "{} bytes of value, slot holds {SLOT_PAYLOAD_MAX}",
            data.len()
        ));
    }
    let mut out = [0u8; 32];
    out[0] = tag;
    out[1] = data.len() as u8;
    out[2..2 + data.len()].copy_from_slice(&data);
    Ok(out)
}

/// The payload bytes, which must be exactly `width` long for a fixed-width kind.
fn fixed_width_payload(body: &[u8], width: usize) -> Result<&[u8], String> {
    if body.len() == width {
        Ok(body)
    } else {
        Err(format!(
            "payload carries {} bytes where {width} are required",
            body.len()
        ))
    }
}

/// Decode a slot payload back into the exact value that was stored.
///
/// An unknown tag is refused rather than guessed at: a payload that this ISA did not
/// write is not evidence of anything. The dispatcher therefore fails closed on state
/// written by a different (or corrupted) producer.
fn decode_slot_payload(payload: &StorageValue) -> Result<Value, String> {
    let len = payload[1] as usize;
    if len > SLOT_PAYLOAD_MAX {
        return Err(format!("payload length {len} exceeds {SLOT_PAYLOAD_MAX}"));
    }
    let body = &payload[2..2 + len];
    match payload[0] {
        SLOT_TAG_INT => Ok(Value::I64(i64::from_le_bytes(
            fixed_width_payload(body, 8)?
                .try_into()
                .map_err(|_| "bad integer payload".to_string())?,
        ))),
        SLOT_TAG_BOOL => match fixed_width_payload(body, 1)?[0] {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            other => Err(format!("boolean payload byte {other} is not 0 or 1")),
        },
        SLOT_TAG_F64 => Ok(Value::F64(f64::from_bits(u64::from_le_bytes(
            fixed_width_payload(body, 8)?
                .try_into()
                .map_err(|_| "bad float payload".to_string())?,
        )))),
        SLOT_TAG_ADDR => Ok(Value::Addr(u64::from_le_bytes(
            fixed_width_payload(body, 8)?
                .try_into()
                .map_err(|_| "bad address payload".to_string())?,
        ))),
        SLOT_TAG_BYTES => Ok(Value::Bytes(body.to_vec())),
        SLOT_TAG_STRING => {
            let text = String::from_utf8(body.to_vec())
                .map_err(|e| format!("string payload is not UTF-8: {e}"))?;
            Ok(Value::String(text))
        }
        tag => Err(format!("unknown slot payload tag {tag}")),
    }
}

fn value_to_storage_value(value: &Value) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    match value {
        Value::I64(value) => out[..8].copy_from_slice(&value.to_le_bytes()),
        Value::F64(value) => out[..8].copy_from_slice(&value.to_bits().to_le_bytes()),
        Value::Bool(value) => out[0] = u8::from(*value),
        Value::Bytes(bytes) => {
            let len = bytes.len().min(32);
            out[..len].copy_from_slice(&bytes[..len]);
        }
        Value::String(value) => {
            let bytes = value.as_bytes();
            let len = bytes.len().min(32);
            out[..len].copy_from_slice(&bytes[..len]);
        }
        Value::Addr(value) => out[..8].copy_from_slice(&value.to_le_bytes()),
        // An aggregate has no 32-byte form, so a global holding one is not mirrored to storage,
        // as with Unit; the global itself still holds the whole value.
        Value::Unit | Value::Array(_) | Value::Tuple(_) => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::vec_init_then_push)]
    use super::*;
    use x3_backend::bc_format_helpers;

    /// Stamp the envelope's checksum, as the writer does, before loading hand-assembled bytes.
    ///
    /// These envelopes used to leave the checksum field zero and load anyway, because nothing read
    /// it; `BytecodeModule::from_bytes` verifies it now (TICKET-108), and a zero field is a mismatch
    /// like any other. Hand-assembled test bytecode has to be as valid as compiled bytecode, or the
    /// test is not testing what it says.
    fn sealed(mut bytes: Vec<u8>) -> Vec<u8> {
        let checksum = x3_common::bytecode::checksum(&bytes[x3_common::bytecode::HEADER_LEN..]);
        let at = x3_common::bytecode::CHECKSUM_OFFSET;
        bytes[at..at + 4].copy_from_slice(&checksum.to_le_bytes());
        bytes
    }

    #[test]
    fn vm_smoke_add() {
        // Use the helper to assemble a simple module
        let bytes = bc_format_helpers::assemble_simple_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Call function 0 with no arguments
        let result = vm.call_function(0, &[]).expect("execution should succeed");

        // Should return 42 + 7 = 49
        assert_eq!(result.value, Some(Value::I64(49)));
        assert!(result.gas_used > 0);
        assert!(result.instruction_count > 0);
    }

    #[test]
    fn vm_with_parameters() {
        let bytes = bc_format_helpers::assemble_param_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let result = vm
            .call_function(0, &[Value::I64(10), Value::I64(20)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(30)));
    }

    #[test]
    fn vm_branch_positive() {
        let bytes = bc_format_helpers::assemble_branch_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Positive value: should return the value
        let result = vm
            .call_function(0, &[Value::I64(5)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(5)));
    }

    #[test]
    fn vm_branch_negative() {
        let bytes = bc_format_helpers::assemble_branch_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Negative value: should return 0
        let result = vm
            .call_function(0, &[Value::I64(-5)])
            .expect("execution should succeed");

        assert_eq!(result.value, Some(Value::I64(0)));
    }

    #[test]
    fn vm_halt() {
        let bytes = bc_format_helpers::assemble_halt_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let result = vm.call_function(0, &[]).expect("execution should succeed");

        assert_eq!(result.value, None);
    }

    #[test]
    fn vm_gas_limit() {
        let bytes = bc_format_helpers::assemble_simple_module();
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        // Set very low gas limit
        vm.config.gas_limit = 1;

        let result = vm.call_function(0, &[]);
        assert!(result.is_err());
        match result {
            Err(e) => assert!(matches!(e.kind, VMErrorKind::GasLimitExceeded)),
            _ => panic!("expected gas limit error"),
        }
    }

    #[test]
    fn vm_call_frame_base_and_register_isolation() {
        use x3_backend::bc_format::FunctionEntry;
        use x3_backend::opcode::Opcode;

        // Build a module with two functions: caller (0) and callee (1).
        // Caller: LoadImm r1,100; Call func 1; Ret r1
        // Callee: LoadImm r1,200; RetVoid
        let mut code: Vec<u8> = Vec::new();
        // -- func 0 (entry 0)
        code.push(Opcode::LoadImm as u8); // dst r1
        code.push(1u8);
        code.push(100u8);

        code.push(Opcode::Call as u8);
        code.push(0u8); // dst (ignored)
        code.extend_from_slice(&1u32.to_le_bytes()); // func idx 1
        code.extend_from_slice(&0u16.to_le_bytes()); // argc 0

        code.push(Opcode::Ret as u8);
        code.push(1u8); // return r1

        // -- func 1 (will start at offset = len so far)
        let func1_entry = code.len() as u32;
        code.push(Opcode::LoadImm as u8);
        code.push(1u8);
        code.push(200u8);
        code.push(Opcode::RetVoid as u8);

        // Build module bytes
        let mut out: Vec<u8> = Vec::new();
        use x3_backend::bc_format::{MAGIC, VERSION};
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // flags
        out.extend_from_slice(&0u32.to_le_bytes()); // checksum
        out.extend_from_slice(&VERSION.to_le_bytes()); // min_version
        out.extend_from_slice(&0u32.to_le_bytes()); // features

        // empty const pool
        out.extend_from_slice(&0u32.to_le_bytes());

        // functions table (2)
        out.extend_from_slice(&2u32.to_le_bytes());
        // func 0
        let f0 = FunctionEntry {
            name: "caller".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 2, // r0, r1
            max_stack: 4,
            return_type_tag: 1,
        };
        out.extend_from_slice(&(f0.name.len() as u16).to_le_bytes());
        out.extend_from_slice(f0.name.as_bytes());
        out.extend_from_slice(&f0.entry_point.to_le_bytes());
        out.push(f0.param_count);
        out.extend_from_slice(&f0.local_count.to_le_bytes());
        out.extend_from_slice(&f0.max_stack.to_le_bytes());
        out.push(f0.return_type_tag);
        // func 1
        let f1 = FunctionEntry {
            name: "callee".to_string(),
            entry_point: func1_entry,
            param_count: 0,
            local_count: 2,
            max_stack: 2,
            return_type_tag: 0,
        };
        out.extend_from_slice(&(f1.name.len() as u16).to_le_bytes());
        out.extend_from_slice(f1.name.as_bytes());
        out.extend_from_slice(&f1.entry_point.to_le_bytes());
        out.push(f1.param_count);
        out.extend_from_slice(&f1.local_count.to_le_bytes());
        out.extend_from_slice(&f1.max_stack.to_le_bytes());
        out.push(f1.return_type_tag);

        // no globals
        out.extend_from_slice(&0u32.to_le_bytes());

        // code section
        out.extend_from_slice(&(code.len() as u32).to_le_bytes());
        out.extend_from_slice(&code);
        out.push(0u8); // debug
        out.push(0u8); // metadata

        let mut vm = VM::from_bytes(&sealed(out)).expect("module should load");
        let result = vm.call_function(0, &[]).expect("execution should succeed");

        // Caller r1 should remain 100 (callee's r1 must not clobber caller)
        assert_eq!(result.value, Some(Value::I64(100)));
        // Verify caller's r1 still present in regs at base 0 + 1
        assert_eq!(vm.get_register(1), &Value::I64(100));
    }

    #[test]
    fn vm_globals_load_store_and_atomic_rollback() {
        use x3_backend::bc_format::FunctionEntry;
        use x3_backend::opcode::Opcode;

        // Module that: initializes global0 = 7; then in main does:
        // StoreGlobal and LoadGlobal and demonstrates rollback
        let mut code: Vec<u8> = Vec::new();
        // Test 1: store then load -> return updated value
        // LoadImm r1, 13
        code.push(Opcode::LoadImm as u8);
        code.push(1u8);
        code.push(13i8 as u8);
        // StoreGlobal idx=0, src=r1
        code.push(Opcode::StoreGlobal as u8);
        code.extend_from_slice(&0u32.to_le_bytes());
        code.push(1u8);
        // LoadGlobal r2, idx=0
        code.push(Opcode::LoadGlobal as u8);
        code.push(2u8);
        code.extend_from_slice(&0u32.to_le_bytes());
        // Ret r2
        code.push(Opcode::Ret as u8);
        code.push(2u8);

        // Build module bytes
        let mut out: Vec<u8> = Vec::new();
        use x3_backend::bc_format::{MAGIC, VERSION};
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        // const pool: one integer 7
        out.extend_from_slice(&1u32.to_le_bytes());
        out.push(0u8); // integer tag
        out.extend_from_slice(&7i64.to_le_bytes());

        // functions (1)
        out.extend_from_slice(&1u32.to_le_bytes());
        let f = FunctionEntry {
            name: "main".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 3,
            max_stack: 4,
            return_type_tag: 1,
        };
        out.extend_from_slice(&(f.name.len() as u16).to_le_bytes());
        out.extend_from_slice(f.name.as_bytes());
        out.extend_from_slice(&f.entry_point.to_le_bytes());
        out.push(f.param_count);
        out.extend_from_slice(&f.local_count.to_le_bytes());
        out.extend_from_slice(&f.max_stack.to_le_bytes());
        out.push(f.return_type_tag);

        // globals: one mutable global with init const idx 0
        out.extend_from_slice(&1u32.to_le_bytes());
        // GlobalEntry: name_len(u16)+name + type_tag(u8)+mutable(bool as u8)+init_const(u32)
        let gname = "g0";
        out.extend_from_slice(&(gname.len() as u16).to_le_bytes());
        out.extend_from_slice(gname.as_bytes());
        out.push(1u8); // type tag (int)
        out.push(1u8); // mutable
        out.extend_from_slice(&0u32.to_le_bytes()); // init_const = 0

        // code
        out.extend_from_slice(&(code.len() as u32).to_le_bytes());
        out.extend_from_slice(&code);
        out.push(0u8);
        out.push(0u8);

        // Execute and verify store/load
        let mut vm = VM::from_bytes(&sealed(out)).expect("module should load");
        let res = vm.call_function(0, &[]).expect("exec");
        assert_eq!(res.value, Some(Value::I64(13)));

        // Now test atomic rollback: build small module that begins atomic, writes, rollbacks
        let mut code2: Vec<u8> = Vec::new();
        // AtomicBegin id=0
        code2.push(Opcode::AtomicBegin as u8);
        code2.extend_from_slice(&0u16.to_le_bytes());
        // LoadImm r0, 1
        code2.push(Opcode::LoadImm as u8);
        code2.push(0u8);
        code2.push(1i8 as u8);
        // StoreGlobal idx=0, src=r0
        code2.push(Opcode::StoreGlobal as u8);
        code2.extend_from_slice(&0u32.to_le_bytes());
        code2.push(0u8);
        // AtomicRollback id=0
        code2.push(Opcode::AtomicRollback as u8);
        code2.extend_from_slice(&0u16.to_le_bytes());
        // LoadGlobal r1, idx=0
        code2.push(Opcode::LoadGlobal as u8);
        code2.push(1u8);
        code2.extend_from_slice(&0u32.to_le_bytes());
        // Ret r1
        code2.push(Opcode::Ret as u8);
        code2.push(1u8);

        // build module using same const/global layout
        // replace code section (overwrite code len + bytes at the end of out)
        // quick-and-dirty: reserialize header..const..func..globals then new code
        // For simplicity reconstruct minimal module like above
        let mut outb: Vec<u8> = Vec::new();
        outb.extend_from_slice(MAGIC);
        outb.extend_from_slice(&VERSION.to_le_bytes());
        outb.extend_from_slice(&0u32.to_le_bytes());
        outb.extend_from_slice(&0u32.to_le_bytes());
        outb.extend_from_slice(&VERSION.to_le_bytes());
        outb.extend_from_slice(&0u32.to_le_bytes());
        // const pool (1)
        outb.extend_from_slice(&1u32.to_le_bytes());
        outb.push(0u8);
        outb.extend_from_slice(&7i64.to_le_bytes());
        // functions
        outb.extend_from_slice(&1u32.to_le_bytes());
        outb.extend_from_slice(&(f.name.len() as u16).to_le_bytes());
        outb.extend_from_slice(f.name.as_bytes());
        outb.extend_from_slice(&0u32.to_le_bytes());
        outb.push(f.param_count);
        outb.extend_from_slice(&f.local_count.to_le_bytes());
        outb.extend_from_slice(&f.max_stack.to_le_bytes());
        outb.push(f.return_type_tag);
        // globals
        outb.extend_from_slice(&1u32.to_le_bytes());
        outb.extend_from_slice(&(gname.len() as u16).to_le_bytes());
        outb.extend_from_slice(gname.as_bytes());
        outb.push(1u8);
        outb.push(1u8);
        outb.extend_from_slice(&0u32.to_le_bytes());
        // code2
        outb.extend_from_slice(&(code2.len() as u32).to_le_bytes());
        outb.extend_from_slice(&code2);
        outb.push(0u8);
        outb.push(0u8);

        let mut vm2 = VM::from_bytes(&sealed(outb)).expect("module should load");
        let r = vm2
            .call_function(0, &[])
            .expect_err("atomic rollback should abort");
        assert!(matches!(r.kind, VMErrorKind::AtomicAborted));

        // After rollback, global value must remain initial (7)
        // Execute a small module to read global
        let mut check_code: Vec<u8> = Vec::new();
        check_code.push(Opcode::LoadGlobal as u8);
        check_code.push(0u8);
        check_code.extend_from_slice(&0u32.to_le_bytes());
        check_code.push(Opcode::Ret as u8);
        check_code.push(0u8);

        let mut outc: Vec<u8> = Vec::new();
        outc.extend_from_slice(MAGIC);
        outc.extend_from_slice(&VERSION.to_le_bytes());
        outc.extend_from_slice(&0u32.to_le_bytes());
        outc.extend_from_slice(&0u32.to_le_bytes());
        outc.extend_from_slice(&VERSION.to_le_bytes());
        outc.extend_from_slice(&0u32.to_le_bytes());
        // const pool
        outc.extend_from_slice(&1u32.to_le_bytes());
        outc.push(0u8);
        outc.extend_from_slice(&7i64.to_le_bytes());
        // functions
        outc.extend_from_slice(&1u32.to_le_bytes());
        outc.extend_from_slice(&(f.name.len() as u16).to_le_bytes());
        outc.extend_from_slice(f.name.as_bytes());
        outc.extend_from_slice(&0u32.to_le_bytes());
        outc.push(f.param_count);
        outc.extend_from_slice(&f.local_count.to_le_bytes());
        outc.extend_from_slice(&f.max_stack.to_le_bytes());
        outc.push(f.return_type_tag);
        // globals
        outc.extend_from_slice(&1u32.to_le_bytes());
        outc.extend_from_slice(&(gname.len() as u16).to_le_bytes());
        outc.extend_from_slice(gname.as_bytes());
        outc.push(1u8);
        outc.push(1u8);
        outc.extend_from_slice(&0u32.to_le_bytes());
        // code
        outc.extend_from_slice(&(check_code.len() as u32).to_le_bytes());
        outc.extend_from_slice(&check_code);
        outc.push(0u8);
        outc.push(0u8);

        let mut vmc = VM::from_bytes(&sealed(outc)).expect("module should load");
        let r = vmc.call_function(0, &[]).expect("read global");
        assert_eq!(r.value, Some(Value::I64(7)));
    }

    /// VM-001: Integration test for X3 VM nested call handling with shared global state.
    ///
    /// This verifies:
    /// 1. Caller writes a value to a global, then calls a nested function.
    /// 2. Callee can read the global written by the caller (shared global state).
    /// 3. Callee overwrites the global, returns void.
    /// 4. After callee returns, caller reads the global and sees the callee's write.
    /// 5. Register isolation holds: callee's local registers don't affect caller's.
    #[test]
    fn vm_nested_call_with_global_state() {
        use x3_backend::bc_format::{FunctionEntry, MAGIC, VERSION};
        use x3_backend::opcode::Opcode;

        // Global index 0: starts at 0 (const pool index 0 = integer 0)
        // func 0 (caller):
        //   LoadImm r1, 42          -- write 42 into r1
        //   StoreGlobal g0, r1      -- global0 = 42
        //   Call r0, func=1, argc=0 -- call callee (no args)
        //   LoadGlobal r2, g0       -- r2 = global0 (should be 99 after callee)
        //   Ret r2                  -- return r2
        //
        // func 1 (callee):
        //   LoadGlobal r1, g0       -- r1 = global0 (should be 42 written by caller)
        //   LoadImm r2, 99          -- r2 = 99
        //   Add r1, r1, r2          -- r1 = 42+99 = 141  (verifies it saw 42)
        //   LoadImm r3, 99          -- r3 = 99
        //   StoreGlobal g0, r3      -- global0 = 99
        //   RetVoid

        let mut code_f0: Vec<u8> = Vec::new();
        // LoadImm r1, 42
        code_f0.push(Opcode::LoadImm as u8);
        code_f0.push(1u8);
        code_f0.push(42i8 as u8);
        // StoreGlobal idx=0, src=r1
        code_f0.push(Opcode::StoreGlobal as u8);
        code_f0.extend_from_slice(&0u32.to_le_bytes());
        code_f0.push(1u8);
        // Call dst=r0, func=1, argc=0
        code_f0.push(Opcode::Call as u8);
        code_f0.push(0u8); // dst
        code_f0.extend_from_slice(&1u32.to_le_bytes()); // func idx 1
        code_f0.extend_from_slice(&0u16.to_le_bytes()); // argc = 0
                                                        // LoadGlobal r2, idx=0
        code_f0.push(Opcode::LoadGlobal as u8);
        code_f0.push(2u8);
        code_f0.extend_from_slice(&0u32.to_le_bytes());
        // Ret r2
        code_f0.push(Opcode::Ret as u8);
        code_f0.push(2u8);

        let func1_entry = code_f0.len() as u32;

        let mut code_f1: Vec<u8> = Vec::new();
        // LoadGlobal r1, idx=0  (read what caller wrote)
        code_f1.push(Opcode::LoadGlobal as u8);
        code_f1.push(1u8);
        code_f1.extend_from_slice(&0u32.to_le_bytes());
        // LoadImm r2, 99
        code_f1.push(Opcode::LoadImm as u8);
        code_f1.push(2u8);
        code_f1.push(99i8 as u8);
        // AddI r1, r1, r2  (asserts callee sees 42 from global; r1 = 141)
        code_f1.push(Opcode::AddI as u8);
        code_f1.push(1u8);
        code_f1.push(1u8);
        code_f1.push(2u8);
        // LoadImm r3, 99  (write this to global)
        code_f1.push(Opcode::LoadImm as u8);
        code_f1.push(3u8);
        code_f1.push(99i8 as u8);
        // StoreGlobal idx=0, src=r3
        code_f1.push(Opcode::StoreGlobal as u8);
        code_f1.extend_from_slice(&0u32.to_le_bytes());
        code_f1.push(3u8);
        // RetVoid
        code_f1.push(Opcode::RetVoid as u8);

        let mut code: Vec<u8> = code_f0;
        code.extend_from_slice(&code_f1);

        // Build module bytes
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // flags
        out.extend_from_slice(&0u32.to_le_bytes()); // checksum
        out.extend_from_slice(&VERSION.to_le_bytes()); // min_version
        out.extend_from_slice(&0u32.to_le_bytes()); // features

        // const pool: one integer 0 (initial value of global0)
        out.extend_from_slice(&1u32.to_le_bytes());
        out.push(0u8); // integer tag
        out.extend_from_slice(&0i64.to_le_bytes());

        // functions table (2)
        out.extend_from_slice(&2u32.to_le_bytes());
        // func 0: caller — needs r0..r2
        let f0 = FunctionEntry {
            name: "caller".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 3, // r0, r1, r2
            max_stack: 4,
            return_type_tag: 1,
        };
        out.extend_from_slice(&(f0.name.len() as u16).to_le_bytes());
        out.extend_from_slice(f0.name.as_bytes());
        out.extend_from_slice(&f0.entry_point.to_le_bytes());
        out.push(f0.param_count);
        out.extend_from_slice(&f0.local_count.to_le_bytes());
        out.extend_from_slice(&f0.max_stack.to_le_bytes());
        out.push(f0.return_type_tag);
        // func 1: callee — needs r1..r3 in its own window
        let f1 = FunctionEntry {
            name: "callee".to_string(),
            entry_point: func1_entry,
            param_count: 0,
            local_count: 4, // r0..r3
            max_stack: 4,
            return_type_tag: 0,
        };
        out.extend_from_slice(&(f1.name.len() as u16).to_le_bytes());
        out.extend_from_slice(f1.name.as_bytes());
        out.extend_from_slice(&f1.entry_point.to_le_bytes());
        out.push(f1.param_count);
        out.extend_from_slice(&f1.local_count.to_le_bytes());
        out.extend_from_slice(&f1.max_stack.to_le_bytes());
        out.push(f1.return_type_tag);

        // globals: one mutable global g0 initialized from const 0 (value=0)
        out.extend_from_slice(&1u32.to_le_bytes());
        let gname = "g0";
        out.extend_from_slice(&(gname.len() as u16).to_le_bytes());
        out.extend_from_slice(gname.as_bytes());
        out.push(1u8); // type tag integer
        out.push(1u8); // mutable
        out.extend_from_slice(&0u32.to_le_bytes()); // init from const index 0

        // code section
        out.extend_from_slice(&(code.len() as u32).to_le_bytes());
        out.extend_from_slice(&code);
        out.push(0u8); // debug
        out.push(0u8); // metadata

        let mut vm = VM::from_bytes(&sealed(out)).expect("module should load");
        let result = vm
            .call_function(0, &[])
            .expect("nested call should succeed");

        // Caller returns global0 value after callee wrote 99 into it
        assert_eq!(
            result.value,
            Some(Value::I64(99)),
            "caller should see callee's global write (99)"
        );

        // Verify global0 is 99 in VM state
        assert_eq!(
            vm.globals[0],
            Value::I64(99),
            "global state must reflect callee's write"
        );

        // Verify gas was charged (both functions executed)
        assert!(result.gas_used > 0, "gas must be charged");
        assert!(
            result.instruction_count >= 5,
            "at least 5 instructions executed"
        );
    }

    // ========================================================================
    // Contract storage: `evm_sstore` / `evm_sload`
    //
    // Before these arms existed, both opcodes were declared in the ISA
    // (`x3-backend/src/opcode.rs`), emitted by the backend
    // (`emit_evm_sstore`/`emit_evm_sload`), decoded and gas-priced by the verifier
    // (`verifier.rs`), and priced by this interpreter's table — and both reached the
    // `_` arm and returned `UnimplementedOpcode`. A deployed X3VM program therefore
    // could not carry a single value from one call to the next, and the atomic
    // window's storage rollback (measured on 2026-09-26 to have dropped pre-window
    // writes) could only be proven at the storage unit level, because nothing in the
    // interpreter wrote a key.
    // ========================================================================

    /// Build a one-function X3BC module from a hand-assembled instruction stream.
    ///
    /// `BytecodeModule::to_bytes` writes the envelope and its checksum, so these
    /// fixtures are as valid as compiled bytecode — the reader rejects a wrong
    /// checksum (TICKET-108).
    fn storage_module(code: Vec<u8>, param_count: u8, local_count: u16) -> Vec<u8> {
        use x3_backend::bc_format::{BytecodeModule, FunctionEntry};

        let mut module = BytecodeModule::new();
        module.functions.push(FunctionEntry {
            name: "main".to_string(),
            entry_point: 0,
            param_count,
            local_count,
            max_stack: 8,
            return_type_tag: 1,
        });
        module.code = code;
        module.to_bytes()
    }

    fn load_imm(dst: u8, value: i8) -> Vec<u8> {
        vec![Opcode::LoadImm as u8, dst, value as u8]
    }

    fn sstore(slot_reg: u8, val_reg: u8) -> Vec<u8> {
        vec![Opcode::EvmSstore as u8, slot_reg, val_reg]
    }

    fn sload(dst: u8, slot_reg: u8) -> Vec<u8> {
        vec![Opcode::EvmSload as u8, dst, slot_reg]
    }

    fn ret(reg: u8) -> Vec<u8> {
        vec![Opcode::Ret as u8, reg]
    }

    fn atomic(opcode: Opcode, id: u16) -> Vec<u8> {
        let mut out = vec![opcode as u8];
        out.extend_from_slice(&id.to_le_bytes());
        out
    }

    /// `main(value: r0) { sstore(0, r0); r2 = sload(0); return r2; }`
    fn store_then_load(value_operand: u8) -> Vec<u8> {
        let mut code = Vec::new();
        code.extend(load_imm(1, 0)); // slot 0
        code.extend(sstore(1, value_operand));
        code.extend(sload(2, 1));
        code.extend(ret(2));
        code
    }

    #[test]
    fn evm_store_and_load_round_trip_every_value_kind() {
        let cases = [
            Value::I64(0),
            Value::I64(7),
            Value::I64(-1),
            Value::I64(i64::MIN),
            Value::I64(i64::MAX),
            Value::Bool(false),
            Value::Bool(true),
            Value::F64(1.5),
            Value::F64(-0.0),
            Value::Bytes(vec![]),
            Value::Bytes(vec![1, 2, 3]),
            // Trailing zeros are the reason the slot layout carries a length: a
            // zero-padded payload cannot tell these two apart.
            Value::Bytes(vec![1, 2, 0]),
            Value::Bytes(vec![0; 30]),
            Value::String("x3".to_string()),
            Value::Addr(9),
        ];

        for input in cases {
            let bytes = storage_module(store_then_load(0), 1, 4);
            let mut vm = VM::from_bytes(&bytes).expect("module should load");
            let result = vm
                .call_function(0, core::slice::from_ref(&input))
                .unwrap_or_else(|e| panic!("store/load of {input:?} failed: {e:?}"));
            assert_eq!(
                result.value,
                Some(input.clone()),
                "slot did not round trip {input:?}"
            );
        }
    }

    #[test]
    fn evm_load_of_an_unwritten_slot_reads_zero() {
        let mut code = Vec::new();
        code.extend(load_imm(1, 42)); // never written
        code.extend(sload(2, 1));
        code.extend(ret(2));
        let bytes = storage_module(code, 0, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let result = vm.call_function(0, &[]).expect("read should succeed");
        assert_eq!(result.value, Some(Value::I64(0)));
        assert!(
            vm.storage.get(&evm_slot_key(42)).is_none(),
            "a load must not create the slot it read"
        );
    }

    #[test]
    fn evm_store_refuses_a_negative_slot() {
        let mut code = Vec::new();
        code.extend(load_imm(1, -1));
        code.extend(sstore(1, 0));
        code.extend(ret(0));
        let bytes = storage_module(code, 1, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let err = vm
            .call_function(0, &[Value::I64(7)])
            .expect_err("a negative slot must be refused");
        assert!(
            matches!(err.kind, VMErrorKind::InvalidStorageSlot(_)),
            "expected InvalidStorageSlot, got {:?}",
            err.kind
        );
        assert_eq!(vm.storage.len(), 0, "a refused store writes nothing");
    }

    #[test]
    fn evm_store_refuses_a_non_integer_slot() {
        let mut code = Vec::new();
        code.extend(sstore(0, 0)); // r0 is both slot and value
        code.extend(ret(0));
        let bytes = storage_module(code, 1, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let err = vm
            .call_function(0, &[Value::Bytes(vec![1])])
            .expect_err("a byte-string slot must be refused");
        assert!(
            matches!(err.kind, VMErrorKind::InvalidStorageSlot(_)),
            "expected InvalidStorageSlot, got {:?}",
            err.kind
        );
    }

    #[test]
    fn evm_store_refuses_a_payload_that_does_not_fit_a_slot() {
        let bytes = storage_module(store_then_load(0), 1, 4);

        // 30 payload bytes fit; 31 do not, and must be refused rather than
        // truncated to something the program never asked to store.
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        vm.call_function(0, &[Value::Bytes(vec![7; 30])])
            .expect("30 payload bytes fit a slot");

        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        let err = vm
            .call_function(0, &[Value::Bytes(vec![7; 31])])
            .expect_err("31 payload bytes must be refused");
        assert!(
            matches!(err.kind, VMErrorKind::UnencodableStorageValue(_)),
            "expected UnencodableStorageValue, got {:?}",
            err.kind
        );
        assert_eq!(vm.storage.len(), 0, "a refused store writes nothing");
    }

    #[test]
    fn evm_store_refuses_unit() {
        let bytes = storage_module(store_then_load(0), 1, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let err = vm
            .call_function(0, &[Value::Unit])
            .expect_err("unit is not a storable value");
        assert!(
            matches!(err.kind, VMErrorKind::UnencodableStorageValue(_)),
            "expected UnencodableStorageValue, got {:?}",
            err.kind
        );
    }

    #[test]
    fn evm_load_fails_closed_on_a_payload_this_isa_did_not_write() {
        let mut code = Vec::new();
        code.extend(load_imm(1, 0));
        code.extend(sload(2, 1));
        code.extend(ret(2));
        let bytes = storage_module(code, 0, 4);

        // Unknown tag.
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        vm.storage
            .set(evm_slot_key(0), Some([0xFF; 32]))
            .expect("raw write");
        let err = vm
            .call_function(0, &[])
            .expect_err("an unknown payload tag must fail closed");
        assert!(
            matches!(err.kind, VMErrorKind::CorruptStorageSlot(_)),
            "expected CorruptStorageSlot, got {:?}",
            err.kind
        );

        // Known tag, wrong width for that tag (integer carrying 3 bytes).
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        let mut payload = [0u8; 32];
        payload[0] = 1; // integer tag
        payload[1] = 3; // ...but only three payload bytes
        vm.storage
            .set(evm_slot_key(0), Some(payload))
            .expect("raw write");
        let err = vm
            .call_function(0, &[])
            .expect_err("a payload shorter than its kind must fail closed");
        assert!(
            matches!(err.kind, VMErrorKind::CorruptStorageSlot(_)),
            "expected CorruptStorageSlot, got {:?}",
            err.kind
        );
    }

    #[test]
    fn evm_store_inside_a_rolled_back_window_is_abandoned() {
        // sstore(0, r0=7); atomic begin; sstore(0, r1=9); rollback
        let mut code = Vec::new();
        code.extend(load_imm(1, 0)); // slot 0
        code.extend(sstore(1, 0)); // 7 outside the window
        code.extend(atomic(Opcode::AtomicBegin, 0));
        code.extend(load_imm(2, 9));
        code.extend(sstore(1, 2)); // 9 inside the window
        code.extend(atomic(Opcode::AtomicRollback, 0));
        code.extend(ret(0));
        let bytes = storage_module(code, 1, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");

        let err = vm
            .call_function(0, &[Value::I64(7)])
            .expect_err("the rollback aborts the call");
        assert!(matches!(err.kind, VMErrorKind::AtomicAborted));

        let stored = vm
            .storage
            .get(&evm_slot_key(0))
            .expect("the pre-window write must survive the rollback");
        assert_eq!(
            decode_slot_payload(stored).expect("decodable"),
            Value::I64(7),
            "the window's write must be abandoned and the earlier one kept"
        );

        // The journal is the write delta for cross-VM sync: the abandoned write
        // must not appear in it, and the surviving one must.
        let journal = vm.drain_storage_journal();
        assert_eq!(
            journal.len(),
            1,
            "the rolled-back write must not be part of the delta"
        );
        assert_eq!(journal[0].key, evm_slot_key(0));
        assert_eq!(
            journal[0].new_value,
            Some(encode_slot_payload(&Value::I64(7)).unwrap())
        );
    }

    #[test]
    fn evm_store_journals_the_write_for_cross_vm_sync() {
        let bytes = storage_module(store_then_load(0), 1, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        vm.call_function(0, &[Value::I64(7)])
            .expect("store and load");

        let journal = vm.drain_storage_journal();
        assert_eq!(journal.len(), 1, "one store, one journal record");
        assert_eq!(journal[0].key, evm_slot_key(0));
        assert_eq!(journal[0].old_value, None);
        assert_eq!(
            journal[0].new_value,
            Some(encode_slot_payload(&Value::I64(7)).unwrap())
        );
        assert!(
            vm.drain_storage_journal().is_empty(),
            "draining the journal empties it"
        );
    }

    #[test]
    fn evm_storage_is_charged_the_verifier_table_cost() {
        // LoadImm(1) + LoadImm(1) + EvmSstore(5000) + Ret(2)
        let mut code = Vec::new();
        code.extend(load_imm(1, 0));
        code.extend(load_imm(2, 7));
        code.extend(sstore(1, 2));
        code.extend(ret(2));
        let bytes = storage_module(code, 0, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        let result = vm.call_function(0, &[]).expect("store");
        assert_eq!(result.gas_used, 5004, "a store costs the verifier's 5000");

        // LoadImm(1) + EvmSload(200) + Ret(2)
        let mut code = Vec::new();
        code.extend(load_imm(1, 0));
        code.extend(sload(2, 1));
        code.extend(ret(2));
        let bytes = storage_module(code, 0, 4);
        let mut vm = VM::from_bytes(&bytes).expect("module should load");
        let result = vm.call_function(0, &[]).expect("load");
        assert_eq!(result.gas_used, 203, "a load costs the verifier's 200");
    }

    #[test]
    fn evm_slot_keys_are_disjoint_from_global_keys() {
        // A global index is read as a `u32` (`LoadGlobal`/`StoreGlobal`), so the
        // largest key the global keyspace can name has this in its first eight bytes;
        // the slot tag is larger, which is what makes the two keyspaces disjoint.
        assert_ne!(
            u32::MAX as u64,
            EVM_SLOT_DOMAIN,
            "no addressable global may produce a slot-domain key"
        );

        for slot in [0u64, 1, 255, 65_535, u32::MAX as u64, u64::MAX] {
            for global in [0usize, 1, 255, u32::MAX as usize] {
                assert_ne!(
                    evm_slot_key(slot),
                    storage_key_for_global(global),
                    "slot {slot} collides with global {global}"
                );
            }
        }

        // Distinct slots are distinct keys, and slot 0 is not the all-zero key a
        // default-initialised map would hand back.
        assert_ne!(evm_slot_key(0), evm_slot_key(1));
        assert_ne!(evm_slot_key(0), [0u8; 32]);
    }

    // ------------------------------------------------------------------------
    // The engine's own table of what it executes, held to the interpreter
    // ------------------------------------------------------------------------

    /// Run `instruction` as the first instruction of `main`, padded so an engine that steps past it
    /// reaches a clean `RetVoid`.
    fn run_single(instruction: &[u8]) -> VMResult<ExecutionResult> {
        let mut code = instruction.to_vec();
        code.extend_from_slice(&[Opcode::Nop as u8; 16]);
        code.push(Opcode::RetVoid as u8);
        let bytes = storage_module(code, 0, 16);
        let mut vm = VM::from_bytes(&bytes).expect("the module loads");
        vm.config.gas_limit = 10_000;
        vm.call_function(0, &[])
    }

    #[test]
    fn engine_executes_matches_the_interpreter() {
        let mut wrong = Vec::new();
        for byte in 0u8..=255 {
            let Some(opcode) = Opcode::from_byte(byte) else {
                continue;
            };
            // Operand bytes are zero: register r0, index 0, count 0, jump target 0.
            let mut instruction = vec![byte];
            instruction.extend_from_slice(&[0u8; 9]);
            let refused_as_unimplemented = matches!(
                run_single(&instruction),
                Err(VMError {
                    kind: VMErrorKind::UnimplementedOpcode(b),
                    ..
                }) if b == byte
            );
            if engine_executes(opcode) == refused_as_unimplemented {
                wrong.push(format!(
                    "{opcode:?} (0x{byte:02X}): table says {}, interpreter {}",
                    engine_executes(opcode),
                    if refused_as_unimplemented {
                        "refuses it"
                    } else {
                        "runs it"
                    }
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "table and interpreter disagree:\n  {}",
            wrong.join("\n  ")
        );
    }

    #[test]
    fn atomic_check_reports_whether_a_window_is_open() {
        // r1 = atomic_check (outside); begin; r2 = atomic_check (inside); commit; r3 = r1 + 2*r2 ...
        // Booleans are returned through a branch so the result is one integer.
        let code = vec![
            Opcode::AtomicCheck as u8,
            1, // r1 = false
            Opcode::AtomicBegin as u8,
            0,
            0,
            Opcode::AtomicCheck as u8,
            2, // r2 = true
            Opcode::AtomicCommit as u8,
            0,
            0,
            Opcode::AtomicCheck as u8,
            3, // r3 = false again
            Opcode::Ret as u8,
            2,
        ];
        let bytes = storage_module(code.clone(), 0, 16);
        let mut vm = VM::from_bytes(&bytes).expect("loads");
        assert_eq!(
            vm.call_function(0, &[]).unwrap().value,
            Some(Value::Bool(true))
        );

        let mut outside = code;
        let last = outside.len() - 1;
        outside[last] = 1; // return r1, read before the window opened
        let mut vm = VM::from_bytes(&storage_module(outside.clone(), 0, 16)).expect("loads");
        assert_eq!(
            vm.call_function(0, &[]).unwrap().value,
            Some(Value::Bool(false))
        );
        outside[last] = 3; // and r3, read after it closed
        let mut vm = VM::from_bytes(&storage_module(outside, 0, 16)).expect("loads");
        assert_eq!(
            vm.call_function(0, &[]).unwrap().value,
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn the_atomic_window_past_the_depth_cap_is_refused() {
        let mut code = Vec::new();
        for _ in 0..=MAX_ATOMIC_DEPTH {
            code.extend_from_slice(&[Opcode::AtomicBegin as u8, 0, 0]);
        }
        code.push(Opcode::RetVoid as u8);
        let mut vm = VM::from_bytes(&storage_module(code, 0, 16)).expect("loads");
        let err = vm.call_function(0, &[]).expect_err("one window too many");
        assert!(
            matches!(err.kind, VMErrorKind::AtomicDepthExceeded(MAX_ATOMIC_DEPTH)),
            "{err:?}"
        );

        // The control: exactly the cap is allowed.
        let mut code = Vec::new();
        for _ in 0..MAX_ATOMIC_DEPTH {
            code.extend_from_slice(&[Opcode::AtomicBegin as u8, 0, 0]);
        }
        for _ in 0..MAX_ATOMIC_DEPTH {
            code.extend_from_slice(&[Opcode::AtomicCommit as u8, 0, 0]);
        }
        code.push(Opcode::RetVoid as u8);
        let mut vm = VM::from_bytes(&storage_module(code, 0, 16)).expect("loads");
        vm.call_function(0, &[]).expect("32 nested windows run");
    }

    #[test]
    fn mod_f_is_the_ieee_remainder() {
        let mut module = x3_backend::bc_format::BytecodeModule::new();
        let seven_half = module.const_pool.add_float(7.5).unwrap();
        let two = module.const_pool.add_float(2.0).unwrap();
        let mut code = vec![Opcode::LoadConst as u8, 1];
        code.extend_from_slice(&seven_half.0.to_le_bytes());
        code.extend_from_slice(&[Opcode::LoadConst as u8, 2]);
        code.extend_from_slice(&two.0.to_le_bytes());
        code.extend_from_slice(&[Opcode::ModF as u8, 3, 1, 2, Opcode::Ret as u8, 3]);
        module.functions.push(x3_backend::bc_format::FunctionEntry {
            name: "main".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 16,
            max_stack: 8,
            return_type_tag: 1,
        });
        module.code = code;
        let mut vm = VM::from_bytes(&module.to_bytes()).expect("loads");
        assert_eq!(
            vm.call_function(0, &[]).unwrap().value,
            Some(Value::F64(1.5))
        );
    }

    // ------------------------------------------------------------------------
    // Block context
    // ------------------------------------------------------------------------

    fn run_with_context(code: Vec<u8>, context: Option<BlockContext>) -> VMResult<ExecutionResult> {
        let mut vm = VM::from_bytes(&storage_module(code, 0, 16)).expect("loads");
        if let Some(context) = context {
            vm.set_context(context);
        }
        vm.call_function(0, &[])
    }

    fn full_context() -> BlockContext {
        BlockContext {
            sender: Some([7u8; 32]),
            block_height: Some(1_234),
            timestamp_ms: Some(1_760_000_000_000),
            value: Some(0),
            chain_id: Some(650_000),
        }
    }

    #[test]
    fn each_context_opcode_reads_its_own_field() {
        let cases: [(Opcode, Value); 5] = [
            (Opcode::CtxSender, Value::Bytes(vec![7u8; 32])),
            (Opcode::CtxBlockHeight, Value::I64(1_234)),
            (Opcode::CtxTimestamp, Value::I64(1_760_000_000_000)),
            (Opcode::CtxValue, Value::I64(0)),
            (Opcode::CtxChainId, Value::I64(650_000)),
        ];
        for (opcode, expected) in cases {
            let code = vec![opcode as u8, 1, Opcode::Ret as u8, 1];
            let result = run_with_context(code, Some(full_context()))
                .unwrap_or_else(|e| panic!("{opcode:?}: {e:?}"));
            assert_eq!(result.value, Some(expected), "{opcode:?}");
        }
    }

    #[test]
    fn a_context_field_the_host_did_not_supply_is_refused_not_answered_as_zero() {
        for opcode in [
            Opcode::CtxSender,
            Opcode::CtxBlockHeight,
            Opcode::CtxTimestamp,
            Opcode::CtxValue,
            Opcode::CtxChainId,
        ] {
            let code = vec![opcode as u8, 1, Opcode::Ret as u8, 1];
            // No context at all.
            let err = run_with_context(code.clone(), None).expect_err("no context was given");
            assert!(
                matches!(err.kind, VMErrorKind::ContextUnavailable(_)),
                "{opcode:?}: {err:?}"
            );
            // A context missing only this field is refused the same way: the others do not leak.
            let mut partial = full_context();
            match opcode {
                Opcode::CtxSender => partial.sender = None,
                Opcode::CtxBlockHeight => partial.block_height = None,
                Opcode::CtxTimestamp => partial.timestamp_ms = None,
                Opcode::CtxValue => partial.value = None,
                _ => partial.chain_id = None,
            }
            let err = run_with_context(code, Some(partial)).expect_err("this field is missing");
            assert!(
                matches!(err.kind, VMErrorKind::ContextUnavailable(_)),
                "{opcode:?}: {err:?}"
            );
        }
    }

    #[test]
    fn a_value_too_large_for_the_engines_integer_is_refused_not_wrapped() {
        let mut context = full_context();
        context.value = Some(u128::from(u64::MAX) + 1);
        let code = vec![Opcode::CtxValue as u8, 1, Opcode::Ret as u8, 1];
        let err = run_with_context(code, Some(context)).expect_err("does not fit an i64");
        assert!(matches!(err.kind, VMErrorKind::IntegerOverflow), "{err:?}");
    }

    // ------------------------------------------------------------------------
    // Aggregates
    // ------------------------------------------------------------------------

    fn run_code(code: Vec<u8>) -> VMResult<ExecutionResult> {
        let mut vm = VM::from_bytes(&storage_module(code, 0, 16)).expect("loads");
        vm.call_function(0, &[])
    }

    fn op(o: Opcode) -> u8 {
        o as u8
    }

    #[test]
    fn an_array_is_built_indexed_written_measured_and_popped() {
        let code = vec![
            op(Opcode::NewArray),
            1,
            4,
            0, // r1 = []
            op(Opcode::LoadImm),
            2,
            10,
            op(Opcode::ArrayPush),
            1,
            2, // push 10
            op(Opcode::LoadImm),
            2,
            20,
            op(Opcode::ArrayPush),
            1,
            2, // push 20
            op(Opcode::LoadImm),
            2,
            30,
            op(Opcode::ArrayPush),
            1,
            2, // push 30
            op(Opcode::LoadImm),
            3,
            1,
            op(Opcode::LoadImm),
            4,
            99, //
            op(Opcode::StoreIndex),
            1,
            3,
            4, // r1[1] = 99
            op(Opcode::ArrayPop),
            5,
            1, // r5 = pop() = 30
            op(Opcode::ArrayLen),
            6,
            1, // r6 = 2
            op(Opcode::LoadIndex),
            7,
            1,
            3, // r7 = r1[1] = 99
            op(Opcode::NewTuple),
            8,
            3,
            0,
            5,
            6,
            7, // r8 = (30, 2, 99)
            op(Opcode::Ret),
            8,
        ];
        let result = run_code(code).expect("runs");
        assert_eq!(
            result.value,
            Some(Value::Tuple(vec![
                Value::I64(30),
                Value::I64(2),
                Value::I64(99)
            ]))
        );
    }

    #[test]
    fn a_struct_is_a_tuple_whose_fields_are_read_and_written_by_index() {
        let code = vec![
            op(Opcode::LoadImm),
            1,
            7,
            op(Opcode::LoadTrue),
            2, //
            op(Opcode::NewTuple),
            3,
            2,
            0,
            1,
            2, // r3 = (7, true)
            op(Opcode::LoadImm),
            4,
            8, //
            op(Opcode::StoreField),
            3,
            0,
            0,
            4, // r3.0 = 8
            op(Opcode::LoadField),
            5,
            3,
            0,
            0, // r5 = r3.0
            op(Opcode::TupleGet),
            6,
            3,
            1,
            0, // r6 = r3.1
            op(Opcode::Inc),
            5,
            5, // r5 = 9
            op(Opcode::NewTuple),
            7,
            2,
            0,
            5,
            6, //
            op(Opcode::Ret),
            7,
        ];
        let result = run_code(code).expect("runs");
        assert_eq!(
            result.value,
            Some(Value::Tuple(vec![Value::I64(9), Value::Bool(true)]))
        );
    }

    #[test]
    fn indexing_past_the_end_or_below_zero_is_refused() {
        for index in [3i8, -1] {
            let code = vec![
                op(Opcode::NewArray),
                1,
                0,
                0, //
                op(Opcode::LoadImm),
                2,
                5,
                op(Opcode::ArrayPush),
                1,
                2, //
                op(Opcode::LoadImm),
                3,
                index as u8, //
                op(Opcode::LoadIndex),
                4,
                1,
                3, //
                op(Opcode::Ret),
                4,
            ];
            let err = run_code(code).expect_err("out of bounds");
            assert!(
                matches!(err.kind, VMErrorKind::ArrayIndexOutOfBounds(i, 1) if i == index as i64),
                "{err:?}"
            );
        }
    }

    #[test]
    fn popping_an_empty_array_is_refused() {
        let code = vec![
            op(Opcode::NewArray),
            1,
            0,
            0,
            op(Opcode::ArrayPop),
            2,
            1,
            op(Opcode::Ret),
            2,
        ];
        let err = run_code(code).expect_err("nothing to pop");
        assert!(matches!(err.kind, VMErrorKind::EmptyArray), "{err:?}");
    }

    #[test]
    fn a_self_doubling_array_hits_the_size_cap_instead_of_exhausting_memory() {
        // r1 = [0]; loop { push r1 into a copy of itself } — size doubles each round.
        let mut code = vec![
            op(Opcode::NewArray),
            1,
            0,
            0, //
            op(Opcode::LoadImm),
            2,
            0,
            op(Opcode::ArrayPush),
            1,
            2, //
        ];
        for _ in 0..12 {
            code.extend_from_slice(&[op(Opcode::Mov), 3, 1, op(Opcode::ArrayPush), 1, 3]);
        }
        code.extend_from_slice(&[op(Opcode::RetVoid)]);
        let err = run_code(code).expect_err("2^12 values would exceed the cap");
        assert!(
            matches!(
                err.kind,
                VMErrorKind::AggregateTooLarge(_, MAX_AGGREGATE_SIZE)
            ),
            "{err:?}"
        );
    }

    #[test]
    fn an_aggregate_cannot_be_stored_in_a_slot() {
        let code = vec![
            op(Opcode::NewArray),
            1,
            0,
            0, //
            op(Opcode::LoadImm),
            2,
            0, //
            op(Opcode::EvmSstore),
            2,
            1, //
            op(Opcode::RetVoid),
        ];
        let err = run_code(code).expect_err("a slot holds a scalar");
        assert!(
            matches!(err.kind, VMErrorKind::UnencodableStorageValue(_)),
            "{err:?}"
        );
    }

    #[test]
    fn a_returned_aggregate_is_self_describing() {
        let value = Value::Tuple(vec![Value::I64(1), Value::Array(vec![Value::Bool(true)])]);
        let bytes = value.return_bytes();
        // [TUPLE][count=2]
        assert_eq!(&bytes[..3], &[x3_common::value_tags::TUPLE, 2, 0]);
        // element 0: [INT][len=8][1i64]
        assert_eq!(bytes[3], x3_common::value_tags::INT);
        assert_eq!(&bytes[4..8], &8u32.to_le_bytes());
        assert_eq!(&bytes[8..16], &1i64.to_le_bytes());
        // element 1: [ARRAY][len][ARRAY][count=1][BOOL][len=1][1]
        assert_eq!(bytes[16], x3_common::value_tags::ARRAY);
        assert_eq!(&bytes[21..24], &[x3_common::value_tags::ARRAY, 1, 0]);
        assert_eq!(&bytes[24..], &[x3_common::value_tags::BOOL, 1, 0, 0, 0, 1]);
        // A scalar's bytes are unchanged from before aggregates existed.
        assert_eq!(Value::I64(5).return_bytes(), 5i64.to_le_bytes().to_vec());
    }

    #[test]
    fn the_interpreter_charges_exactly_the_verifiers_table() {
        // A straight-line program: the charge must be the sum of the table over what ran.
        let code = vec![
            op(Opcode::LoadImm),
            1,
            3, // 1
            op(Opcode::LoadImm),
            2,
            4, // 1
            op(Opcode::MulI),
            3,
            1,
            2, // 3
            op(Opcode::AtomicBegin),
            0,
            0, // 100
            op(Opcode::AtomicCommit),
            0,
            0,               // 100
            op(Opcode::Nop), // 1
            op(Opcode::Ret),
            3, // 2
        ];
        let mut vm = VM::from_bytes(&storage_module(code.clone(), 0, 16)).expect("loads");
        vm.call_function(0, &[]).expect("runs");
        let instrs = crate::verifier::Verifier::decode_all_instructions(&code).expect("decodes");
        let expected: u64 = instrs
            .iter()
            .map(|i| crate::verifier::opcode_gas_cost(i.opcode))
            .sum();
        assert_eq!(expected, 208);
        assert_eq!(
            vm.gas_used(),
            expected,
            "one table: the verifier's figure is the charge"
        );
    }
}
