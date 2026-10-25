//! Bytecode Verifier
//!
//! Performs structural, CFG, and semantic validation of X3 bytecode modules
//! before execution. This is a mandatory gate for both local execution and
//! on-chain deployment.
//!
//! # Verification Passes
//!
//! 1. **Structural**: Magic bytes, version, section bounds
//! 2. **Decode**: All instructions decode correctly with valid operands
//! 3. **CFG**: Jump targets are valid instruction boundaries
//! 4. **Const Pool**: All const references are in bounds
//! 5. **Atomic**: Atomic begin/end markers are balanced
//! 6. **Gas**: Conservative gas estimate doesn't exceed limits
//! 7. **On-Chain**: Debug/JIT opcodes forbidden in production

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};

use x3_backend::bc_format::BytecodeModule;
use x3_backend::opcode::Opcode;
use x3bc_core::spec::{self, OnChain, OperandKind};
use x3bc_core::trading::TradeOp;

use crate::error::{VerifierError, VerifierErrorKind, VerifierResult};

/// Maximum gas budget for verification (prevents DoS).
pub const MAX_GAS_BUDGET: u128 = 1 << 60;

/// Maximum register index allowed.
pub const MAX_REGISTER: u16 = 65535;

/// Maximum call stack depth for verification.
pub const MAX_CALL_DEPTH: usize = 1024;

/// Verification options.
#[derive(Debug, Clone)]
pub struct VerifyOptions {
    /// Whether this is on-chain context (forbids debug opcodes).
    pub on_chain: bool,
    /// Maximum allowed gas budget per function.
    pub max_gas_per_function: u128,
    /// Whether to perform CFG reachability analysis.
    pub check_reachability: bool,
    /// Maximum module size in bytes.
    pub max_module_size: usize,
    /// Reject floating-point arithmetic in consensus-sensitive contexts.
    pub deny_float_arithmetic: bool,
    /// Admit the trading extension (format 1.1, opcodes `0xE0–0xE9`). Off chain by default; on
    /// chain only through [`VerifyOptions::on_chain_trading`], which the runtime selects for a
    /// trade submission.
    pub allow_trading: bool,
    /// Exact parameter count required on function 0.
    ///
    /// The chain calls function 0 with no arguments. Deterministic strategy simulation has an
    /// explicit argument ABI, so it uses this field to admit a non-zero exact count while applying
    /// every other on-chain admission rule.
    pub expected_entry_params: Option<u16>,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            on_chain: false,
            max_gas_per_function: MAX_GAS_BUDGET,
            check_reachability: true,
            max_module_size: 16 * 1024 * 1024, // 16 MB
            deny_float_arithmetic: false,
            allow_trading: true,
            expected_entry_params: None,
        }
    }
}

impl VerifyOptions {
    /// Options for on-chain deployment.
    pub fn on_chain() -> Self {
        Self {
            on_chain: true,
            max_gas_per_function: 10_000_000, // 10M gas per function
            check_reachability: true,
            max_module_size: 1024 * 1024, // 1 MB
            deny_float_arithmetic: true,
            allow_trading: false,
            expected_entry_params: Some(0),
        }
    }

    /// The chain's options for a trade submission: every on-chain rule, plus the trading extension.
    pub fn on_chain_trading() -> Self {
        Self {
            allow_trading: true,
            ..Self::on_chain()
        }
    }

    /// The deterministic ABI used for off-chain strategy simulation.
    ///
    /// This applies the on-chain admission restrictions (no debug, GPU, cross-VM, or other
    /// host-dependent opcodes), forbids floating-point arithmetic, and requires function 0 to have
    /// exactly `entry_params` parameters. The trading extension stays disabled: a simulator
    /// decision is not a trade submission and must not mutate chain trading state.
    pub fn deterministic_simulation(entry_params: u16) -> Self {
        Self {
            allow_trading: false,
            expected_entry_params: Some(entry_params),
            ..Self::on_chain()
        }
    }
}

/// Does the chain's verifier, configured with `options`, admit `opcode`?
///
/// The admission rule as one predicate over the canonical spec, so the verifier's passes and the
/// exhaustiveness test cannot disagree about it.
pub fn admitted_on_chain(opcode: Opcode, options: &VerifyOptions) -> bool {
    match spec::of(opcode).on_chain {
        OnChain::Admitted => true,
        OnChain::TradingOnly => options.allow_trading,
        OnChain::FloatDenied => !options.deny_float_arithmetic,
        OnChain::Forbidden | OnChain::NotExecutable => false,
    }
}

/// Decoded instruction with metadata for verification.
#[derive(Debug, Clone)]
pub struct DecodedInstr {
    /// The opcode byte.
    pub opcode: u8,
    /// Byte offset in code section.
    pub offset: usize,
    /// Total instruction size in bytes.
    pub size: usize,
    /// Decoded operands (registers, indices, etc.).
    pub operands: Vec<u64>,
    /// Estimated gas cost.
    pub gas_cost: u64,
}

/// Bytecode verifier.
pub struct Verifier;

impl Verifier {
    /// Verify a bytecode module from raw bytes.
    pub fn verify_module_bytes(
        bytes: &[u8],
        options: &VerifyOptions,
    ) -> VerifierResult<BytecodeModule> {
        // Size check
        if bytes.is_empty() {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::EmptyModule,
            ));
        }
        if bytes.len() > options.max_module_size {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::ModuleTooLarge(bytes.len(), options.max_module_size),
            ));
        }

        // Parse module
        let module = BytecodeModule::from_bytes(bytes).map_err(|e| {
            VerifierError::without_offset(VerifierErrorKind::ParseError(format!("{}", e)))
        })?;

        // The checksum is verified by the parse above — `BytecodeModule::from_bytes` compares the
        // header's value against the same function the writer used, and refuses on mismatch.
        //
        // This used to be a second, *different* check here: a CRC32 computed over the body and
        // compared only when the header's value was non-zero. The writer has always written a
        // wrapping multiply-and-add sum, so the two disagreed, and every module a compiler produced
        // was rejected by this gate — while a module with a zeroed checksum field was accepted
        // without any check at all (TICKET-108). One format, one algorithm, checked always.

        // Run verification passes
        Self::verify_module(&module, options)?;

        Ok(module)
    }

    /// Verify a pre-parsed bytecode module.
    pub fn verify_module(module: &BytecodeModule, options: &VerifyOptions) -> VerifierResult<()> {
        // 1. Structural validation
        Self::verify_structure(module)?;

        // 2. Function entry points
        Self::verify_function_entries(module)?;

        // 3. Decode all instructions
        let instrs = Self::decode_all_instructions(&module.code)?;

        // 4. Build instruction offset set
        let offsets: BTreeSet<usize> = instrs.iter().map(|i| i.offset).collect();

        // 5. CFG validation (jump targets)
        Self::verify_cfg(&instrs, &offsets, module)?;

        // 6. Const pool validation
        Self::verify_const_pool_refs(&instrs, module)?;

        // 6b. Linkage: entries, calls, globals and frames agree with the module's tables.
        Self::verify_linkage(&instrs, &offsets, module)?;

        // 7. Atomic balance validation
        Self::verify_atomic_balance(&instrs)?;

        // 7b. The trading extension: section soundness, opcode/section agreement, atomic windows.
        Self::verify_trading(module, &instrs, options)?;

        // 8. Gas estimation
        Self::verify_gas(&instrs, options)?;

        // 9. On-chain restrictions
        if options.on_chain {
            Self::verify_on_chain_restrictions(&instrs, options)?;
            Self::verify_engine_executes(&instrs)?;
        }

        if options.deny_float_arithmetic {
            Self::verify_float_determinism(&instrs)?;
        }

        // Last, so a module refused for what its code does is told that rather than that its
        // function table is incomplete.
        if options.on_chain {
            Self::verify_on_chain_entry(module, options)?;
        }

        Ok(())
    }

    /// The trading extension's static rules.
    ///
    /// A module with trading opcodes must carry a sound trading section
    /// (`x3bc_core::trading::verify_section`, the stateful verifier: bindings and debts resolve,
    /// capabilities are in the catalog at this engine's version, costs are charged once and before
    /// every profit floor, every debt is repaid). Its code must agree with the section: each
    /// indexed opcode names an operation of its own kind, at most once and in section order, after
    /// `trade_begin` and before `trade_commit`, always inside an atomic window (whose snapshot is
    /// what makes a trap roll the trade back). After `trade_charge_costs` the code runs straight to
    /// the end, so the gas charged there for the rest of the function is exact.
    fn verify_trading(
        module: &BytecodeModule,
        instrs: &[DecodedInstr],
        options: &VerifyOptions,
    ) -> VerifierResult<()> {
        let trading = |reason: &str| {
            VerifierError::without_offset(VerifierErrorKind::Trading(reason.to_string()))
        };
        let at = |offset: usize, reason: &str| {
            VerifierError::new(VerifierErrorKind::Trading(reason.to_string()), offset)
        };
        let has_trade_ops = instrs.iter().any(|instr| {
            Opcode::from_byte(instr.opcode).is_some_and(|opcode| opcode.is_trading_op())
        });
        let Some(section) = module.trading.as_ref() else {
            if has_trade_ops {
                return Err(trading("trading opcodes without a trading section"));
            }
            return Ok(());
        };
        if !options.allow_trading {
            return Err(trading(
                "the trading extension is not enabled for this context",
            ));
        }
        x3bc_core::trading::verify_section(section).map_err(|error| {
            trading(&format!(
                "unsound trading section{}: {}",
                error
                    .op_index
                    .map(|index| format!(" at operation {index}"))
                    .unwrap_or_default(),
                error.reason
            ))
        })?;
        if module.functions.len() != 1 {
            return Err(trading("a trading module has exactly one function"));
        }
        let private = module
            .features
            .has(x3_backend::bc_format::FeatureFlags::PRIVATE_SUBMISSION_REQUIRED);
        if private != section.policy.private_submission_required {
            return Err(trading(
                "the header's private-submission bit must match the compiled policy",
            ));
        }

        let mut depth: usize = 0;
        let mut begun = false;
        let mut committed = false;
        let mut next_index: u64 = 0;
        let mut charged_at: Option<usize> = None;
        for (position, instr) in instrs.iter().enumerate() {
            let Some(opcode) = Opcode::from_byte(instr.opcode) else {
                continue;
            };
            match opcode {
                Opcode::AtomicBegin => depth += 1,
                Opcode::AtomicCommit | Opcode::AtomicRollback => depth = depth.saturating_sub(1),
                _ if opcode.is_trading_op() => {
                    if depth == 0 {
                        return Err(at(instr.offset, "trading opcode outside an atomic window"));
                    }
                    match opcode {
                        Opcode::TradeBegin => {
                            if begun {
                                return Err(at(instr.offset, "trade begun twice"));
                            }
                            begun = true;
                        }
                        Opcode::TradeCommit => {
                            if !begun || committed {
                                return Err(at(instr.offset, "trade committed out of order"));
                            }
                            committed = true;
                        }
                        Opcode::TradeLoadProfit => {
                            if !begun {
                                return Err(at(instr.offset, "profit read before the trade began"));
                            }
                        }
                        _ => {
                            if !begun || committed {
                                return Err(at(
                                    instr.offset,
                                    "trading operation outside begin/commit",
                                ));
                            }
                            let index = instr.operands[0];
                            let op: &TradeOp = section
                                .ops
                                .get(index as usize)
                                .ok_or_else(|| at(instr.offset, "operation index out of range"))?;
                            if op.opcode() != opcode {
                                return Err(at(
                                    instr.offset,
                                    "opcode does not match the operation it indexes",
                                ));
                            }
                            if index < next_index {
                                return Err(at(
                                    instr.offset,
                                    "operations must run once each, in section order",
                                ));
                            }
                            next_index = index + 1;
                            if opcode == Opcode::TradeChargeCosts {
                                charged_at = Some(position);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(position) = charged_at {
            for instr in &instrs[position + 1..] {
                if matches!(
                    Opcode::from_byte(instr.opcode),
                    Some(
                        Opcode::Jump
                            | Opcode::JumpIf
                            | Opcode::JumpUnless
                            | Opcode::Call
                            | Opcode::Halt
                    )
                ) {
                    return Err(at(
                        instr.offset,
                        "control flow after costs are charged: the gas charged must be exact",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Verify structural integrity.
    fn verify_structure(module: &BytecodeModule) -> VerifierResult<()> {
        // Version check - compare packed u32 representations
        if !x3bc_core::bytecode::version_is_readable(module.version.to_packed()) {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::UnsupportedVersion(module.version.to_packed()),
            ));
        }

        Ok(())
    }

    /// Verify function entry points are within code section.
    fn verify_function_entries(module: &BytecodeModule) -> VerifierResult<()> {
        let code_len = module.code.len();

        for (idx, func) in module.functions.iter().enumerate() {
            if func.entry_point as usize >= code_len {
                return Err(VerifierError::without_offset(
                    VerifierErrorKind::FunctionEntryOutOfBounds(idx, func.entry_point),
                ));
            }
        }

        Ok(())
    }

    /// Decode all instructions from the code section.
    pub fn decode_all_instructions(code: &[u8]) -> VerifierResult<Vec<DecodedInstr>> {
        let mut instrs = Vec::new();
        let mut pc = 0;
        let len = code.len();

        while pc < len {
            let opcode = code[pc];
            let (size, operands) = Self::decode_operands(code, pc, opcode)?;

            if pc + size > len {
                return Err(VerifierError::new(
                    VerifierErrorKind::OperandOutOfBounds,
                    pc,
                ));
            }

            instrs.push(DecodedInstr {
                opcode,
                offset: pc,
                size,
                operands,
                gas_cost: opcode_gas_cost(opcode),
            });

            pc += size;
        }

        Ok(instrs)
    }

    /// Decode operands for a single instruction, from the canonical operand layout
    /// (`x3bc_core::spec`). Returns `(total_size, operands)`.
    ///
    /// The operand vector is in encoding order; a list operand contributes its count followed by
    /// its elements (a pair list, each pair's two bytes). Registers are one byte, `Imm8` is
    /// sign-extended. There is no per-opcode table here any more: the spec is the table, and
    /// `tests/opcode_spec_exhaustive.rs` holds this decoder to it.
    fn decode_operands(
        code: &[u8],
        pc: usize,
        opcode_byte: u8,
    ) -> VerifierResult<(usize, Vec<u64>)> {
        let opcode = Opcode::from_byte(opcode_byte)
            .ok_or_else(|| VerifierError::new(VerifierErrorKind::InvalidOpcode(opcode_byte), pc))?;
        let out_of_bounds = || VerifierError::new(VerifierErrorKind::OperandOutOfBounds, pc);
        let read = |offset: usize, width: usize| -> VerifierResult<u64> {
            let start = pc.checked_add(offset).ok_or_else(out_of_bounds)?;
            let end = start.checked_add(width).ok_or_else(out_of_bounds)?;
            let bytes = code.get(start..end).ok_or_else(out_of_bounds)?;
            let mut value: u64 = 0;
            for (i, byte) in bytes.iter().enumerate() {
                value |= u64::from(*byte) << (8 * i);
            }
            Ok(value)
        };

        let mut offset = 1usize;
        let mut operands = Vec::new();
        for kind in spec::of(opcode).operands {
            match kind {
                OperandKind::Imm8 => {
                    operands.push(read(offset, 1)? as u8 as i8 as i64 as u64);
                    offset += 1;
                }
                OperandKind::RegList16 | OperandKind::PairList16 | OperandKind::TopicList8 => {
                    let count = read(offset, kind.width())?;
                    offset += kind.width();
                    operands.push(count);
                    let elements = (count as usize)
                        .checked_mul(kind.element_width())
                        .ok_or_else(out_of_bounds)?;
                    for i in 0..elements {
                        operands.push(read(offset + i, 1)?);
                    }
                    offset += elements;
                }
                _ => {
                    operands.push(read(offset, kind.width())?);
                    offset += kind.width();
                }
            }
        }
        Ok((offset, operands))
    }

    /// Verify CFG: all jump targets are valid instruction boundaries.
    fn verify_cfg(
        instrs: &[DecodedInstr],
        offsets: &BTreeSet<usize>,
        module: &BytecodeModule,
    ) -> VerifierResult<()> {
        let code_len = module.code.len();

        for instr in instrs {
            let opcode = Opcode::from_byte(instr.opcode);

            match opcode {
                Some(Opcode::Jump) => {
                    let target = instr.operands[0] as u32;
                    Self::check_jump_target(target, code_len, offsets, instr.offset)?;
                }
                Some(Opcode::JumpIf) | Some(Opcode::JumpUnless) => {
                    let target = instr.operands[1] as u32;
                    Self::check_jump_target(target, code_len, offsets, instr.offset)?;
                }
                Some(Opcode::Call) => {
                    let func_idx = instr.operands[1] as u32;
                    if func_idx as usize >= module.functions.len() {
                        return Err(VerifierError::new(
                            VerifierErrorKind::FunctionIndexOutOfBounds(
                                func_idx,
                                module.functions.len(),
                            ),
                            instr.offset,
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check that every reference into the module's tables matches the table.
    ///
    /// These were checked only when the interpreter reached them, so a module that called a
    /// function with the wrong number of arguments, named a global that does not exist, or entered
    /// a function in the middle of an instruction was admitted and failed at run time. The runtime
    /// interpreter this engine replaces refused each of them at intake; so does this.
    fn verify_linkage(
        instrs: &[DecodedInstr],
        offsets: &BTreeSet<usize>,
        module: &BytecodeModule,
    ) -> VerifierResult<()> {
        for (idx, func) in module.functions.iter().enumerate() {
            if !offsets.contains(&(func.entry_point as usize)) {
                return Err(VerifierError::without_offset(
                    VerifierErrorKind::FunctionEntryUnaligned(idx, func.entry_point),
                ));
            }
            let frame = func.param_count as usize + func.local_count as usize;
            if frame > crate::vm::MAX_REGISTERS {
                return Err(VerifierError::without_offset(
                    VerifierErrorKind::FunctionFrameTooLarge(idx, frame, crate::vm::MAX_REGISTERS),
                ));
            }
        }

        let globals = module.globals.len();
        for instr in instrs {
            match Opcode::from_byte(instr.opcode) {
                Some(Opcode::Call) => {
                    // operands: dst, func, argc, args...
                    let func_idx = instr.operands[1] as u32;
                    let argc = instr.operands[2] as usize;
                    // `verify_cfg` has already refused an index past the table.
                    if let Some(func) = module.functions.get(func_idx as usize) {
                        if func.param_count as usize != argc {
                            return Err(VerifierError::new(
                                VerifierErrorKind::CallArgumentCountMismatch(
                                    func_idx,
                                    func.param_count,
                                    argc,
                                ),
                                instr.offset,
                            ));
                        }
                    }
                }
                Some(Opcode::LoadGlobal) | Some(Opcode::StoreGlobal) => {
                    // LoadGlobal: dst, idx. StoreGlobal: idx, src.
                    let index = if instr.opcode == Opcode::LoadGlobal as u8 {
                        instr.operands[1]
                    } else {
                        instr.operands[0]
                    } as u32;
                    if index as usize >= globals {
                        return Err(VerifierError::new(
                            VerifierErrorKind::GlobalIndexOutOfBounds(index, globals),
                            instr.offset,
                        ));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// On chain the runtime calls function 0 with no arguments. Deterministic simulation may use
    /// an explicit argument ABI and declares its exact parameter count in `options`.
    fn verify_on_chain_entry(
        module: &BytecodeModule,
        options: &VerifyOptions,
    ) -> VerifierResult<()> {
        let entry = module.functions.first().ok_or_else(|| {
            VerifierError::without_offset(VerifierErrorKind::MissingEntryFunction)
        })?;
        let expected = options.expected_entry_params.unwrap_or(0);
        if expected > u8::MAX as u16 || entry.param_count != expected as u8 {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::InvalidParamCount(expected as usize, entry.param_count),
            ));
        }
        Ok(())
    }

    /// Refuse, at intake, any opcode the interpreter has no implementation for.
    ///
    /// Without this a module carrying one was admitted, stored, and then failed with
    /// `UnimplementedOpcode` on the block that reached it. An admitted program is a promise that
    /// the chain can run it; this keeps the promise checkable before anything is charged.
    fn verify_engine_executes(instrs: &[DecodedInstr]) -> VerifierResult<()> {
        for instr in instrs {
            if let Some(opcode) = Opcode::from_byte(instr.opcode) {
                if !crate::vm::engine_executes(opcode) {
                    return Err(VerifierError::new(
                        VerifierErrorKind::NotExecutable(instr.opcode),
                        instr.offset,
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_jump_target(
        target: u32,
        code_len: usize,
        offsets: &BTreeSet<usize>,
        from_offset: usize,
    ) -> VerifierResult<()> {
        if target as usize >= code_len {
            return Err(VerifierError::new(
                VerifierErrorKind::JumpTargetOutOfBounds(target, code_len),
                from_offset,
            ));
        }
        if !offsets.contains(&(target as usize)) {
            return Err(VerifierError::new(
                VerifierErrorKind::JumpTargetUnaligned(target),
                from_offset,
            ));
        }
        Ok(())
    }

    /// Verify all const pool references are in bounds.
    fn verify_const_pool_refs(
        instrs: &[DecodedInstr],
        module: &BytecodeModule,
    ) -> VerifierResult<()> {
        let pool_len = module.const_pool.len();

        for instr in instrs {
            if instr.opcode == Opcode::LoadConst.to_byte() {
                let idx = instr.operands[1] as u32;
                if idx as usize >= pool_len {
                    return Err(VerifierError::new(
                        VerifierErrorKind::ConstPoolIndexOutOfBounds(idx, pool_len),
                        instr.offset,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Verify atomic begin/end markers are balanced.
    fn verify_atomic_balance(instrs: &[DecodedInstr]) -> VerifierResult<()> {
        // Track active atomic blocks by ID
        let mut active: BTreeMap<u16, usize> = BTreeMap::new();

        for instr in instrs {
            match Opcode::from_byte(instr.opcode) {
                Some(Opcode::AtomicBegin) => {
                    let id = instr.operands[0] as u16;
                    active.insert(id, instr.offset);
                }
                Some(Opcode::AtomicCommit) | Some(Opcode::AtomicRollback) => {
                    let id = instr.operands[0] as u16;
                    if active.remove(&id).is_none() {
                        return Err(VerifierError::new(
                            VerifierErrorKind::AtomicIdMismatch(id),
                            instr.offset,
                        ));
                    }
                }
                _ => {}
            }
        }

        if !active.is_empty() {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::AtomicUnbalanced(active.len() as i32),
            ));
        }

        Ok(())
    }

    /// Verify gas budget doesn't exceed limits.
    fn verify_gas(instrs: &[DecodedInstr], options: &VerifyOptions) -> VerifierResult<()> {
        let total_gas: u128 = instrs.iter().map(|i| i.gas_cost as u128).sum();

        if total_gas > options.max_gas_per_function {
            return Err(VerifierError::without_offset(
                VerifierErrorKind::GasBudgetExceeded(total_gas, options.max_gas_per_function),
            ));
        }

        Ok(())
    }

    /// Verify on-chain restrictions (no debug opcodes).
    fn verify_on_chain_restrictions(
        instrs: &[DecodedInstr],
        options: &VerifyOptions,
    ) -> VerifierResult<()> {
        for instr in instrs {
            if let Some(opcode) = Opcode::from_byte(instr.opcode) {
                // GPU intrinsics are host calls into whatever device the node has; debug opcodes
                // print; the 32-bit conversions have no honest meaning in the value model and
                // `ctx_gas` answers with a count the engines are not held to agree on (TICKET-149).
                // The list is the spec's `OnChain::Forbidden`; trading opcodes are forbidden unless
                // the options admit the extension.
                let forbidden = match spec::of(opcode).on_chain {
                    OnChain::Forbidden => true,
                    OnChain::TradingOnly => !options.allow_trading,
                    _ => false,
                };
                if forbidden {
                    return Err(VerifierError::new(
                        VerifierErrorKind::ForbiddenOnChain(instr.opcode),
                        instr.offset,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Verify that platform-dependent floating-point arithmetic is not used.
    fn verify_float_determinism(instrs: &[DecodedInstr]) -> VerifierResult<()> {
        for instr in instrs {
            if let Some(opcode) = Opcode::from_byte(instr.opcode) {
                if spec::of(opcode).on_chain == OnChain::FloatDenied {
                    return Err(VerifierError::new(
                        VerifierErrorKind::ForbiddenOnChain(instr.opcode),
                        instr.offset,
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Get gas cost for an opcode.
/// The gas one execution of `opcode` costs: the single table both the verifier's static bound and
/// the interpreter's charge read.
///
/// The interpreter used to keep its own table, which disagreed on 78 of 114 opcodes: an `emit`
/// (whose payload is consensus state) cost 1, an atomic window (which copies every register and
/// global) cost 5, and the verifier's bound described charges no block made. One table means a
/// simulation's figure, the verifier's estimate and a block's charge are the same arithmetic.
///
/// Every instruction costs at least 1, so the gas a run consumed bounds the instructions it ran.
pub fn opcode_gas_cost(opcode: u8) -> u64 {
    match Opcode::from_byte(opcode) {
        Some(op) => spec::of(op).gas,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_backend::bc_format::{BytecodeModule, ConstPool, FeatureFlags, VersionInfo};
    use x3_backend::opcode::Opcode;

    fn make_simple_module(code: Vec<u8>) -> BytecodeModule {
        BytecodeModule {
            version: VersionInfo::new(1, 0, 0),
            min_version: VersionInfo::new(1, 0, 0),
            flags: Default::default(),
            features: FeatureFlags(0),
            const_pool: ConstPool::new(),
            functions: vec![],
            globals: vec![],
            code,
            debug_info: None,
            metadata: None,
            trading: None,
        }
    }

    /// The checksum the *writer* puts in the header is the one the verifier must expect.
    ///
    /// `BytecodeModule::to_bytes` writes a wrapping multiply-and-add over the body; the verifier
    /// recomputed a CRC32 and compared only when the header value was non-zero, so every module a
    /// compiler produced carried a checksum the verifier read as wrong (TICKET-108).
    #[test]
    fn a_written_module_passes_its_own_checksum() {
        let module = make_simple_module(vec![Opcode::Nop.to_byte(), Opcode::Halt.to_byte()]);
        let bytes = module.to_bytes();
        let result = Verifier::verify_module_bytes(&bytes, &VerifyOptions::default());
        assert!(
            result.is_ok(),
            "a module written by the compiler must verify, got {:?}",
            result.err().map(|e| e.kind)
        );
    }

    #[test]
    fn a_corrupted_body_fails_the_checksum() {
        let module = make_simple_module(vec![Opcode::Nop.to_byte(), Opcode::Halt.to_byte()]);
        let mut bytes = module.to_bytes();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        let error = Verifier::verify_module_bytes(&bytes, &VerifyOptions::default())
            .expect_err("a body corrupted after compilation must not verify");
        match error.kind {
            // The parse refuses it: the checksum the writer computed no longer matches.
            VerifierErrorKind::ParseError(ref message) => assert!(
                message.contains("checksum"),
                "expected the checksum to be named, got {message}"
            ),
            other => panic!("expected a parse refusal naming the checksum, got {other:?}"),
        }
    }

    #[test]
    fn verify_empty_module() {
        let result = Verifier::verify_module_bytes(&[], &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::EmptyModule
        ));
    }

    #[test]
    fn verify_nop_halt() {
        let module = make_simple_module(vec![Opcode::Nop.to_byte(), Opcode::Halt.to_byte()]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_invalid_opcode() {
        let module = make_simple_module(vec![0xFF]); // Invalid opcode
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::InvalidOpcode(0xFF)
        ));
    }

    #[test]
    fn verify_jump_target_oob() {
        let module = make_simple_module(vec![
            Opcode::Jump.to_byte(),
            0xFF,
            0x00,
            0x00,
            0x00, // Target 255 (out of bounds)
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::JumpTargetOutOfBounds(255, _)
        ));
    }

    #[test]
    fn verify_atomic_unbalanced() {
        let module = make_simple_module(vec![
            Opcode::AtomicBegin.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::Halt.to_byte(),
            // Missing AtomicCommit/Rollback
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::AtomicUnbalanced(_)
        ));
    }

    #[test]
    fn verify_atomic_balanced() {
        let module = make_simple_module(vec![
            Opcode::AtomicBegin.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::AtomicCommit.to_byte(),
            0x00,
            0x00, // ID 0
            Opcode::Halt.to_byte(),
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_debug_forbidden_on_chain() {
        let module = make_simple_module(vec![
            Opcode::DebugPrint.to_byte(),
            0x00, // Register 0
        ]);
        let result = Verifier::verify_module(&module, &VerifyOptions::on_chain());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::ForbiddenOnChain(_)
        ));
    }

    #[test]
    fn deterministic_simulation_allows_only_the_declared_entry_arity() {
        let mut module = make_simple_module(vec![Opcode::Nop.to_byte(), Opcode::Halt.to_byte()]);
        module.functions.push(entry(8, 8, 0));

        Verifier::verify_module(&module, &VerifyOptions::deterministic_simulation(8))
            .expect("declared ABI arity");
        let refusal = Verifier::verify_module(&module, &VerifyOptions::deterministic_simulation(7))
            .expect_err("wrong ABI arity");
        assert!(matches!(
            refusal.kind,
            VerifierErrorKind::InvalidParamCount(7, 8)
        ));
    }

    #[test]
    fn deterministic_simulation_forbids_host_dependent_opcodes() {
        let mut module = make_simple_module(vec![
            Opcode::DebugPrint.to_byte(),
            0x00,
            Opcode::Halt.to_byte(),
        ]);
        module.functions.push(entry(8, 8, 0));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::deterministic_simulation(8))
            .expect_err("debug output is not deterministic simulation");
        assert!(matches!(
            refusal.kind,
            VerifierErrorKind::ForbiddenOnChain(_)
        ));
    }

    #[test]
    fn verify_conversions_and_ctx_gas_forbidden_on_chain() {
        // TICKET-149: the runtime interpreter refuses these at intake, so the on-chain verifier
        // must too, or the two readers of one artifact disagree about whether it may run.
        for (op, len) in [
            (Opcode::I32ToI64, 3usize),
            (Opcode::I64ToI32, 3),
            (Opcode::I64ToF64, 3),
            (Opcode::F64ToI64, 3),
            (Opcode::F64ToF32, 3),
            (Opcode::ToBool, 3),
            (Opcode::CtxGas, 2),
        ] {
            let mut code = vec![op.to_byte(), 0x00, 0x01][..len].to_vec();
            code.push(Opcode::Halt.to_byte());
            let module = make_simple_module(code);
            let refusal = Verifier::verify_module(&module, &VerifyOptions::on_chain())
                .expect_err("refused on chain");
            assert!(
                matches!(refusal.kind, VerifierErrorKind::ForbiddenOnChain(byte) if byte == op.to_byte()),
                "{op:?}: {refusal:?}"
            );
        }
    }

    fn entry(params: u8, locals: u16, at: u32) -> x3_backend::bc_format::FunctionEntry {
        x3_backend::bc_format::FunctionEntry {
            name: "f".to_string(),
            entry_point: at,
            param_count: params,
            local_count: locals,
            max_stack: 8,
            return_type_tag: 1,
        }
    }

    #[test]
    fn an_opcode_the_engine_cannot_run_is_refused_on_chain_and_admitted_off_it() {
        // `agent_self dst` — decodes, and the interpreter has no arm for it (no agent registry).
        let mut module = make_simple_module(vec![
            Opcode::AgentSelf.to_byte(),
            0x00,
            Opcode::Halt.to_byte(),
        ]);
        module.functions.push(entry(0, 4, 0));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::on_chain())
            .expect_err("a block could not run it");
        assert!(
            matches!(refusal.kind, VerifierErrorKind::NotExecutable(b) if b == Opcode::AgentSelf.to_byte()),
            "{refusal:?}"
        );
        // Off chain the verifier stays a structural check: tools may inspect such a module.
        Verifier::verify_module(&module, &VerifyOptions::default()).expect("admitted off chain");
    }

    #[test]
    fn a_call_with_the_wrong_number_of_arguments_is_refused() {
        // main: call f1 with one argument; f1 declares two parameters.
        let mut code = vec![Opcode::Call.to_byte(), 0x00];
        code.extend_from_slice(&1u32.to_le_bytes());
        code.extend_from_slice(&1u16.to_le_bytes());
        code.push(0x02); // the one argument register
        code.push(Opcode::Halt.to_byte());
        let f1_at = code.len() as u32;
        code.push(Opcode::RetVoid.to_byte());
        let mut module = make_simple_module(code);
        module.functions.push(entry(0, 4, 0));
        module.functions.push(entry(2, 4, f1_at));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::default())
            .expect_err("argument count must match");
        assert!(
            matches!(
                refusal.kind,
                VerifierErrorKind::CallArgumentCountMismatch(1, 2, 1)
            ),
            "{refusal:?}"
        );
        // The control: declaring one parameter makes it admissible.
        module.functions[1].param_count = 1;
        Verifier::verify_module(&module, &VerifyOptions::default()).expect("now it matches");
    }

    #[test]
    fn a_global_index_past_the_table_is_refused() {
        let mut code = vec![Opcode::LoadGlobal.to_byte(), 0x00];
        code.extend_from_slice(&0u32.to_le_bytes());
        code.push(Opcode::Halt.to_byte());
        let mut module = make_simple_module(code);
        module.functions.push(entry(0, 4, 0));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::default())
            .expect_err("no globals declared");
        assert!(
            matches!(
                refusal.kind,
                VerifierErrorKind::GlobalIndexOutOfBounds(0, 0)
            ),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_function_entering_mid_instruction_or_with_an_oversized_frame_is_refused() {
        // LoadImm is three bytes; offset 1 is inside it.
        let code = vec![
            Opcode::LoadImm.to_byte(),
            0x00,
            0x05,
            Opcode::Halt.to_byte(),
        ];
        let mut module = make_simple_module(code.clone());
        module.functions.push(entry(0, 4, 1));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::default())
            .expect_err("entry inside an instruction");
        assert!(
            matches!(
                refusal.kind,
                VerifierErrorKind::FunctionEntryUnaligned(0, 1)
            ),
            "{refusal:?}"
        );

        let mut module = make_simple_module(code);
        module.functions.push(entry(10, 250, 0));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::default())
            .expect_err("frame larger than the register file");
        assert!(
            matches!(
                refusal.kind,
                VerifierErrorKind::FunctionFrameTooLarge(0, 260, 256)
            ),
            "{refusal:?}"
        );
    }

    #[test]
    fn on_chain_the_entry_function_must_exist_and_take_no_arguments() {
        let code = vec![Opcode::Halt.to_byte()];
        let module = make_simple_module(code.clone());
        let refusal = Verifier::verify_module(&module, &VerifyOptions::on_chain())
            .expect_err("no entry function");
        assert!(matches!(
            refusal.kind,
            VerifierErrorKind::MissingEntryFunction
        ));

        let mut module = make_simple_module(code);
        module.functions.push(entry(1, 4, 0));
        let refusal = Verifier::verify_module(&module, &VerifyOptions::on_chain())
            .expect_err("the runtime calls the entry with no arguments");
        assert!(
            matches!(refusal.kind, VerifierErrorKind::InvalidParamCount(0, 1)),
            "{refusal:?}"
        );
        Verifier::verify_module(&module, &VerifyOptions::default())
            .expect("off chain a tool may call it with arguments");
    }

    #[test]
    fn the_verifier_decodes_aggregate_instructions_exactly_as_the_emitter_writes_them() {
        use x3_backend::opcode::Register as R;
        let mut emitter = x3_backend::BytecodeEmitter::new();
        emitter.emit_new_array(R(1), 300);
        emitter.emit_new_tuple(R(2), &[R(1), R(1)]);
        emitter.emit_load_field(R(3), R(2), 1);
        emitter.emit_store_field(R(2), 0, R(3));
        emitter.emit_tuple_get(R(4), R(2), 1);
        let code = emitter.code().to_vec();
        let instrs = Verifier::decode_all_instructions(&code).expect("decodes");
        let lens: Vec<usize> = instrs.iter().map(|i| i.size).collect();
        assert_eq!(
            lens,
            vec![4, 6, 5, 5, 5],
            "each instruction's length is the emitter's"
        );
        assert_eq!(
            lens.iter().sum::<usize>(),
            code.len(),
            "no byte left over or skipped"
        );
        // And the u16 operands are read whole.
        assert_eq!(instrs[0].operands[1], 300);
    }

    #[test]
    fn verify_float_arithmetic_forbidden_on_chain() {
        let module = make_simple_module(vec![
            Opcode::AddF.to_byte(),
            0x00,
            0x01,
            0x02,
            Opcode::Halt.to_byte(),
        ]);

        let result = Verifier::verify_module(&module, &VerifyOptions::on_chain());
        assert!(matches!(
            result.unwrap_err().kind,
            VerifierErrorKind::ForbiddenOnChain(op) if op == Opcode::AddF.to_byte()
        ));
    }

    #[test]
    fn gas_costs_reasonable() {
        // Ensure all costs are set, and that none is free.
        assert_eq!(opcode_gas_cost(Opcode::Nop.to_byte()), 1);
        for byte in 0u8..=255 {
            if Opcode::from_byte(byte).is_some() {
                assert!(opcode_gas_cost(byte) >= 1, "0x{byte:02X} is free");
            }
        }
        assert_eq!(opcode_gas_cost(Opcode::AddI.to_byte()), 2);
        assert_eq!(opcode_gas_cost(Opcode::MulI.to_byte()), 3);
        assert_eq!(opcode_gas_cost(Opcode::EvmSstore.to_byte()), 5000);
    }

    // ========================================================================
    // Comprehensive decode_operands tests for all opcode categories
    // ========================================================================

    #[test]
    fn decode_no_operand_instructions() {
        // Nop, Halt, RetVoid, Breakpoint - all 1 byte
        let code = vec![Opcode::Nop.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 1);
        assert!(instrs[0].operands.is_empty());

        let code = vec![Opcode::Halt.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 1);

        let code = vec![Opcode::Breakpoint.to_byte()];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 1);
    }

    #[test]
    fn decode_single_register_instructions() {
        // LoadZero, CtxSender, etc. - 2 bytes (opcode + reg:u8)
        let code = vec![
            Opcode::LoadZero.to_byte(),
            0x05, // r5
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 2);
        assert_eq!(instrs[0].operands, vec![5]);

        // Ret - 2 bytes
        let code = vec![
            Opcode::Ret.to_byte(),
            0x07, // r7
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].operands, vec![7]);
    }

    #[test]
    fn decode_two_register_instructions() {
        // Mov, NegI, Not, ArrayLen, etc. - 3 bytes
        let code = vec![
            Opcode::Mov.to_byte(),
            0x01, // dst: r1
            0x02, // src: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 3);
        assert_eq!(instrs[0].operands, vec![1, 2]);
    }

    #[test]
    fn decode_three_register_instructions() {
        // AddI, SubI, MulI, EqI, LoadIndex, etc. - 4 bytes
        let code = vec![
            Opcode::AddI.to_byte(),
            0x03, // dst: r3
            0x01, // a: r1
            0x02, // b: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 4);
        assert_eq!(instrs[0].operands, vec![3, 1, 2]);
    }

    #[test]
    fn decode_jump_instructions() {
        // Jump - 5 bytes (opcode + target:u32)
        let code = vec![
            Opcode::Jump.to_byte(),
            0x10,
            0x00,
            0x00,
            0x00, // target: 16
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![16]);

        // JumpIf - 6 bytes (opcode + cond:u8 + target:u32)
        let code = vec![
            Opcode::JumpIf.to_byte(),
            0x05, // cond: r5
            0x20,
            0x00,
            0x00,
            0x00, // target: 32
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 6);
        assert_eq!(instrs[0].operands, vec![5, 32]);
    }

    #[test]
    fn decode_load_const() {
        // LoadConst - 6 bytes (opcode + dst:u8 + idx:u32)
        let code = vec![
            Opcode::LoadConst.to_byte(),
            0x01, // dst: r1
            0x00,
            0x01,
            0x00,
            0x00, // idx: 256
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 6);
        assert_eq!(instrs[0].operands, vec![1, 256]);
    }

    #[test]
    fn decode_load_imm() {
        // LoadImm - 3 bytes (opcode + dst:u8 + val:i8)
        let code = vec![
            Opcode::LoadImm.to_byte(),
            0x05, // dst: r5
            0xFE, // val: -2 (signed)
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 3);
        // -2 as i8 sign-extended to u64
        assert_eq!(instrs[0].operands[0], 5);
        assert_eq!(instrs[0].operands[1] as i64, -2);
    }

    #[test]
    fn decode_call_variable_length() {
        // Call - variable: opcode + dst:u8 + func:u32 + argc:u16 + [args:u8...]
        // 0 args: 8 bytes
        let code = vec![
            Opcode::Call.to_byte(),
            0x00, // dst: r0
            0x05,
            0x00,
            0x00,
            0x00, // func: 5
            0x00,
            0x00, // argc: 0
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 8);
        assert_eq!(instrs[0].operands, vec![0, 5, 0]);

        // 2 args: 10 bytes
        let code = vec![
            Opcode::Call.to_byte(),
            0x00, // dst: r0
            0x03,
            0x00,
            0x00,
            0x00, // func: 3
            0x02,
            0x00, // argc: 2
            0x01, // arg0: r1
            0x02, // arg1: r2
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 10);
        assert_eq!(instrs[0].operands, vec![0, 3, 2, 1, 2]);
    }

    #[test]
    fn decode_new_tuple_variable_length() {
        // NewTuple - variable: opcode + dst:u8 + count:u16 + [elements:u8...]
        let code = vec![
            Opcode::NewTuple.to_byte(),
            0x05, // dst: r5
            0x03,
            0x00, // count: 3
            0x01, // elem0: r1
            0x02, // elem1: r2
            0x03, // elem2: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 7); // 4 + 3
        assert_eq!(instrs[0].operands, vec![5, 3, 1, 2, 3]);
    }

    #[test]
    fn decode_emit_variable_length() {
        // Emit - variable: opcode + name:u8 + argc:u16 + [args:u8...]
        //
        // The name was `event_id:u32` here, matching an encoder no caller could use: the backend
        // lowers values to registers and cannot hand back the constant index a `LoadConst` filled
        // one from. It is a register now, like every other operand.
        let code = vec![
            Opcode::Emit.to_byte(),
            0x03, // name: r3
            0x01,
            0x00, // argc: 1
            0x07, // arg0: r7
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].size, 5); // 4 + 1
        assert_eq!(instrs[0].operands, vec![3, 1, 7]);
    }

    #[test]
    fn decode_evm_intrinsics() {
        // EvmCall - 6 bytes
        let code = vec![
            Opcode::EvmCall.to_byte(),
            0x00, // dst: r0
            0x01, // gas: r1
            0x02, // addr: r2
            0x03, // value: r3
            0x04, // data: r4
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 6);
        assert_eq!(instrs[0].operands, vec![0, 1, 2, 3, 4]);

        // EvmSload - 3 bytes
        let code = vec![
            Opcode::EvmSload.to_byte(),
            0x05, // dst: r5
            0x06, // slot: r6
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 3);
        assert_eq!(instrs[0].operands, vec![5, 6]);
    }

    #[test]
    fn decode_svm_intrinsics() {
        // SvmInvoke - 5 bytes
        let code = vec![
            Opcode::SvmInvoke.to_byte(),
            0x00, // dst: r0
            0x01, // program: r1
            0x02, // accounts: r2
            0x03, // data: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![0, 1, 2, 3]);

        // SvmTransfer - 4 bytes
        let code = vec![
            Opcode::SvmTransfer.to_byte(),
            0x01, // from: r1
            0x02, // to: r2
            0x03, // lamports: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 4);
        assert_eq!(instrs[0].operands, vec![1, 2, 3]);

        // SvmGetClock - 2 bytes
        let code = vec![
            Opcode::SvmGetClock.to_byte(),
            0x09, // dst: r9
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 2);
        assert_eq!(instrs[0].operands, vec![9]);
    }

    #[test]
    fn decode_debug_instructions() {
        // Assert - 6 bytes (opcode + cond:u8 + msg_idx:u32)
        let code = vec![
            Opcode::Assert.to_byte(),
            0x01, // cond: r1
            0x00,
            0x00,
            0x00,
            0x00, // msg_idx: 0
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 6);
        assert_eq!(instrs[0].operands, vec![1, 0]);

        // Panic - 5 bytes (opcode + msg_idx:u32)
        let code = vec![
            Opcode::Panic.to_byte(),
            0x01,
            0x00,
            0x00,
            0x00, // msg_idx: 1
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 5);
        assert_eq!(instrs[0].operands, vec![1]);
    }

    #[test]
    fn decode_evm_log_variable_length() {
        // EvmLog - variable: opcode + topic_count:u8 + [topics:u8...] + data:u8
        // 0 topics: 3 bytes
        let code = vec![
            Opcode::EvmLog.to_byte(),
            0x00, // topic_count: 0
            0x05, // data: r5
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 3);
        assert_eq!(instrs[0].operands, vec![0, 5]);

        // 2 topics: 5 bytes
        let code = vec![
            Opcode::EvmLog.to_byte(),
            0x02, // topic_count: 2
            0x01, // topic0: r1
            0x02, // topic1: r2
            0x03, // data: r3
        ];
        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs[0].size, 5); // 3 + 2
        assert_eq!(instrs[0].operands, vec![2, 1, 2, 3]);
    }

    #[test]
    fn decode_multi_instruction_sequence() {
        // Test a realistic instruction sequence
        let mut code = Vec::new();

        // load_const r1, c0 (6 bytes)
        code.push(Opcode::LoadConst.to_byte());
        code.push(1u8); // dst: r1
        code.extend_from_slice(&0u32.to_le_bytes());

        // load_const r2, c1 (6 bytes)
        code.push(Opcode::LoadConst.to_byte());
        code.push(2u8); // dst: r2
        code.extend_from_slice(&1u32.to_le_bytes());

        // add_i r3, r1, r2 (4 bytes)
        code.push(Opcode::AddI.to_byte());
        code.push(3u8); // dst: r3
        code.push(1u8); // a: r1
        code.push(2u8); // b: r2

        // ret r3 (2 bytes)
        code.push(Opcode::Ret.to_byte());
        code.push(3u8); // src: r3

        let instrs = Verifier::decode_all_instructions(&code).unwrap();
        assert_eq!(instrs.len(), 4);

        // Verify offsets
        assert_eq!(instrs[0].offset, 0);
        assert_eq!(instrs[1].offset, 6);
        assert_eq!(instrs[2].offset, 12);
        assert_eq!(instrs[3].offset, 16);

        // Total should be 18 bytes
        assert_eq!(code.len(), 18);
    }

    #[test]
    fn decode_all_context_ops() {
        // All context ops are 2 bytes
        for opcode in [
            Opcode::CtxSender,
            Opcode::CtxBlockHeight,
            Opcode::CtxTimestamp,
            Opcode::CtxValue,
            Opcode::CtxGas,
            Opcode::CtxChainId,
        ] {
            let code = vec![
                opcode.to_byte(),
                0x01, // dst: r1
            ];
            let instrs = Verifier::decode_all_instructions(&code).unwrap();
            assert_eq!(instrs[0].size, 2, "Failed for {:?}", opcode);
            assert_eq!(instrs[0].operands, vec![1]);
        }
    }

    #[test]
    fn decode_all_type_conversions() {
        // All type conversions are 3 bytes (dst:u8 + src:u8)
        for opcode in [
            Opcode::I32ToI64,
            Opcode::I64ToI32,
            Opcode::I32ToF32,
            Opcode::I64ToF64,
            Opcode::F32ToI32,
            Opcode::F64ToI64,
            Opcode::F32ToF64,
            Opcode::F64ToF32,
            Opcode::ToBool,
        ] {
            let code = vec![
                opcode.to_byte(),
                0x02, // dst: r2
                0x01, // src: r1
            ];
            let instrs = Verifier::decode_all_instructions(&code).unwrap();
            assert_eq!(instrs[0].size, 3, "Failed for {:?}", opcode);
            assert_eq!(instrs[0].operands, vec![2, 1]);
        }
    }

    #[test]
    fn gas_costs_complete() {
        // Verify all newly added opcodes have gas costs
        assert_eq!(opcode_gas_cost(Opcode::EvmLog.to_byte()), 375);
        assert_eq!(opcode_gas_cost(Opcode::EvmBalance.to_byte()), 400);
        assert_eq!(opcode_gas_cost(Opcode::EvmCodeSize.to_byte()), 100);

        assert_eq!(opcode_gas_cost(Opcode::SvmInvoke.to_byte()), 5000);
        assert_eq!(opcode_gas_cost(Opcode::SvmInvokeSigned.to_byte()), 6000);
        assert_eq!(opcode_gas_cost(Opcode::SvmCreateAccount.to_byte()), 10000);
        assert_eq!(opcode_gas_cost(Opcode::SvmTransfer.to_byte()), 3000);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetData.to_byte()), 200);
        assert_eq!(opcode_gas_cost(Opcode::SvmSetData.to_byte()), 1000);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetRent.to_byte()), 100);
        assert_eq!(opcode_gas_cost(Opcode::SvmGetClock.to_byte()), 100);

        // The floor: no instruction is free (`gas_costs_reasonable` checks every opcode).
        assert_eq!(opcode_gas_cost(Opcode::Breakpoint.to_byte()), 1);
        assert_eq!(opcode_gas_cost(Opcode::Assert.to_byte()), 5);
        assert_eq!(opcode_gas_cost(Opcode::Panic.to_byte()), 5);
    }
}
