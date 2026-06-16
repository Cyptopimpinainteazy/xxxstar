//! X3 VM executor - fetch-decode-execute loop and handlers for opcodes.

use crate::x3_lang_vm::VM;
// Import shared opcode constants
use crate::spec::opcodes::*;
use x3_lang_common::{
    decode_asset_op_payload, decode_bridge_payload, decode_capability_payload, AssetOpPayload,
    CapabilityPayload,
};

pub type ExecResult<T> = Result<T, ExecError>;

#[derive(Debug)]
pub enum ExecError {
    OutOfGas,
    InvalidOpcode(u8),
    InvalidOperand,
    MemoryOutOfBounds,
    Panic(String),
}

pub type GasCost = u128;

/// Execute the VM until halt or out of gas.
pub fn execute(vm: &mut VM) -> ExecResult<()> {
    let has_compiler_header = has_compiler_header(vm.code.as_slice());
    if vm.state.pc == 0 {
        vm.state.pc = first_instruction_pc(vm.code.as_slice())?;
    }
    loop {
        if vm.state.pc >= vm.code.len() {
            return Ok(());
        }
        // Fetch instruction
        let opcode = vm.code.as_slice()[vm.state.pc];
        let _flags = vm
            .code
            .as_slice()
            .get(vm.state.pc + 1)
            .copied()
            .unwrap_or(0);
        let operand = read_u16_le(vm.code.as_slice(), vm.state.pc + 2).unwrap_or(0);
        let pc_next = if has_compiler_header {
            align4(vm.state.pc + 3)
        } else {
            vm.state.pc + 4
        };

        if vm.state.paused && opcode != EMERGENCY_CONTROL {
            return Err(ExecError::Panic("X3_PAUSED".to_string()));
        }

        // Gas accounting (simplified: 1 unit per instruction)
        let cost = gas_cost_for_opcode(opcode);
        if vm.state.gas < cost {
            return Err(ExecError::OutOfGas);
        }
        vm.state.gas -= cost;

        match opcode {
            0x0A => {
                // POW_RRR - power: ra = rb ^ rc (saturating)
                let (ra, rb, rc) = decode_regtriplet(operand);
                let base = vm.state.registers[rb as usize];
                let exp = vm.state.registers[rc as usize];
                // Saturating pow: produce u128::MAX on overflow
                let result = base.saturating_pow(exp as u32);
                vm.state.registers[ra as usize] = result;
            }
            0x01 => {
                // ADD_RRR - REG-REG-REG: operand encodes registers
                // flags: REG3
                let (ra, rb, rc) = decode_regtriplet(operand);
                vm.state.registers[ra as usize] =
                    vm.state.registers[rb as usize].wrapping_add(vm.state.registers[rc as usize]);
            }
            0x02 => {
                // SUB_RRR
                let (ra, rb, rc) = decode_regtriplet(operand);
                vm.state.registers[ra as usize] =
                    vm.state.registers[rb as usize].wrapping_sub(vm.state.registers[rc as usize]);
            }
            META_NONCE => {
                // LOAD_RAI: R[a] = mem[R[b] + imm16]
                let (ra, rb, imm) = decode_reg_reg_imm(operand);
                let addr = (vm.state.registers[rb as usize] as usize).wrapping_add(imm as usize);
                if addr + 16 > vm.state.memory.len() {
                    return Err(ExecError::MemoryOutOfBounds);
                }
                // Read 16 bytes and produce u128 (little endian)
                let mut val = 0u128;
                for i in 0..16 {
                    val |= (vm.state.memory[addr + i] as u128) << (i * 8);
                }
                vm.state.registers[ra as usize] = val;
            }
            META_CHAIN_ID => {
                // STORE_RAI
                let (ra, rb, imm) = decode_reg_reg_imm(operand);
                let addr = (vm.state.registers[rb as usize] as usize).wrapping_add(imm as usize);
                if addr + 16 > vm.state.memory.len() {
                    return Err(ExecError::MemoryOutOfBounds);
                }
                let val = vm.state.registers[ra as usize];
                for i in 0..16 {
                    vm.state.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                }
            }
            LOCK => {
                execute_asset_opcode(vm, LOCK, has_compiler_header)?;
                if has_compiler_header {
                    continue;
                }
            }
            MINT => {
                execute_asset_opcode(vm, MINT, has_compiler_header)?;
                if has_compiler_header {
                    continue;
                }
            }
            BURN => {
                execute_asset_opcode(vm, BURN, has_compiler_header)?;
                if has_compiler_header {
                    continue;
                }
            }
            RELEASE => {
                execute_asset_opcode(vm, RELEASE, has_compiler_header)?;
                if has_compiler_header {
                    continue;
                }
            }
            SWAP => {
                execute_asset_opcode(vm, SWAP, has_compiler_header)?;
                if has_compiler_header {
                    continue;
                }
            }
            BRIDGE => {
                if !has_compiler_header {
                    return Err(ExecError::InvalidOpcode(BRIDGE));
                }
                execute_bridge_opcode(vm)?;
                continue;
            }
            IF => {
                // The current bytecode encoding stores the condition as a
                // non-executable debug string. Until the compiler emits an
                // evaluable condition, executing an `if` would silently skip
                // user guards. Fail closed instead of partial execution.
                return Err(ExecError::Panic(
                    "X3_IF_NOT_EXECUTABLE: condition evaluation is not yet implemented".to_string(),
                ));
            }
            LOOP => {
                // As with `if`, loop iteration conditions are not encoded as
                // executable bytecode. Fail closed to prevent infinite or
                // silently skipped loops.
                return Err(ExecError::Panic(
                    "X3_LOOP_NOT_EXECUTABLE: loop iteration is not yet implemented".to_string(),
                ));
            }
            CALL => {
                // CALL - push return and jump
                let addr = operand as usize;
                vm.state.call_stack.push(pc_next);
                vm.state.pc = addr;
                continue;
            }
            RET => {
                // RET
                let retpc = vm
                    .state
                    .call_stack
                    .pop()
                    .ok_or_else(|| ExecError::Panic("call stack underflow".to_string()))?;
                vm.state.pc = retpc;
                continue;
            }
            REQUIRE => {
                // `require` guards must evaluate their condition and abort when
                // it is false. The current encoding only records the opcode;
                // fail closed until real guard evaluation is wired.
                return Err(ExecError::Panic(
                    "X3_REQUIRE_NOT_ENFORCED: require guards are not yet evaluated".to_string(),
                ));
            }
            ON_FAIL => {
                // Failure-action registration needs rollback handler wiring.
                return Err(ExecError::Panic(
                    "X3_ON_FAIL_NOT_ENFORCED: failure actions are not yet implemented".to_string(),
                ));
            }
            ON_TIMEOUT => {
                // Timeout scheduling needs a real deadline mechanism.
                return Err(ExecError::Panic(
                    "X3_ON_TIMEOUT_NOT_ENFORCED: timeout guards are not yet implemented"
                        .to_string(),
                ));
            }
            ATOMIC_BEGIN => {
                // Asset reservation / locking is required before an atomic
                // scope can be safely executed. Without it, rollback cannot
                // revert state. Fail closed.
                return Err(ExecError::Panic(
                    "X3_ATOMIC_BEGIN_NOT_IMPLEMENTED: asset reservation/locking is not wired"
                        .to_string(),
                ));
            }
            ATOMIC_END => {
                // Commit without prior reservation is unsafe.
                return Err(ExecError::Panic(
                    "X3_ATOMIC_END_NOT_IMPLEMENTED: atomic commit without reservation is unsafe"
                        .to_string(),
                ));
            }
            ATOMIC_ROLLBACK => {
                // State reversion is not implemented; jumping back to the
                // start of the atomic block would re-execute corrupted state.
                return Err(ExecError::Panic(
                    "X3_ATOMIC_ROLLBACK_NOT_IMPLEMENTED: state reversion is not wired".to_string(),
                ));
            }
            EMIT => {
                let data = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
                let res = bridge_result(vm.bridge.evm_call(&data))?;
                vm.state.registers[0] = bytes_to_register(&res);
                vm.state.pc = align4(vm.state.pc + 3 + data.len());
                continue;
            }
            CALL_HOST => {
                let data = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
                let res = bridge_result(vm.bridge.svm_call(&data))?;
                vm.state.registers[0] = bytes_to_register(&res);
                vm.state.pc = align4(vm.state.pc + 3 + data.len());
                continue;
            }
            GPU_DISPATCH..=SUB_EXEC => {
                let payload = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
                let result = dispatch_host_opcode(vm, opcode, &payload)?;
                vm.state.registers[0] = bytes_to_register(&result);
                vm.state.pc = align4(vm.state.pc + 3 + payload.len());
                continue;
            }
            NOP => { // NOP
            }
            HALT => {
                // HALT
                return Ok(());
            }
            other => return Err(ExecError::InvalidOpcode(other)),
        }

        vm.state.pc = pc_next;
    }
}

fn has_compiler_header(bytes: &[u8]) -> bool {
    bytes.first() == Some(&BYTECODE_VERSION_1) && bytes.get(1).copied().unwrap_or(NOP) != NOP
}

fn first_instruction_pc(bytes: &[u8]) -> ExecResult<usize> {
    if !has_compiler_header(bytes) {
        return Ok(0);
    }

    let mut pc = 1usize;
    while pc < bytes.len() {
        match bytes[pc] {
            META_NONCE => {
                let len = read_u16_le(bytes, pc + 1).ok_or(ExecError::InvalidOperand)? as usize;
                pc = pc.checked_add(3 + len).ok_or(ExecError::InvalidOperand)?;
            }
            META_CHAIN_ID => {
                pc = pc.checked_add(9).ok_or(ExecError::InvalidOperand)?;
            }
            _ => return Ok(pc),
        }
    }

    Ok(pc)
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}

fn execute_asset_opcode(vm: &mut VM, opcode: u8, compiler_stream: bool) -> ExecResult<()> {
    if !compiler_stream {
        // Raw bytecode streams lack the structured asset payload needed to
        // identify chain, asset, amount, and receiver. Executing the opcode
        // would be a silent no-op. Fail closed.
        return Err(ExecError::Panic(format!(
            "X3_ASSET_OP_NOT_EXECUTABLE: opcode 0x{opcode:02x} requires a compiler-stream payload"
        )));
    }

    let payload = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
    let decoded =
        decode_asset_op_payload(opcode, &payload).map_err(|_| ExecError::InvalidOperand)?;
    apply_asset_payload(vm, decoded);
    vm.state.pc = align4(vm.state.pc + 3 + payload.len());
    Ok(())
}

fn execute_bridge_opcode(vm: &mut VM) -> ExecResult<()> {
    let payload = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
    let decoded = decode_bridge_payload(&payload).map_err(|_| ExecError::InvalidOperand)?;
    let receipt = bridge_result(vm.bridge.bridge_transfer(
        &decoded.via,
        &decoded.from_chain,
        &decoded.from_asset,
        &decoded.to_chain,
        &decoded.to_asset,
        decoded.amount,
        decoded.receiver.as_bytes(),
        &decoded.source_finality_proof,
        &decoded.transfer_proof,
    ))?;
    vm.state.registers[0] = bytes_to_register(&receipt);
    vm.state.registers[1] = decoded.amount;
    vm.state.bridge_ops.push(decoded);
    vm.state.bridge_receipts.push(receipt);
    vm.state.pc = align4(vm.state.pc + 3 + payload.len());
    Ok(())
}

fn apply_asset_payload(vm: &mut VM, payload: AssetOpPayload) {
    match &payload {
        AssetOpPayload::Lock { amount, .. }
        | AssetOpPayload::Mint { amount, .. }
        | AssetOpPayload::Burn { amount, .. } => {
            vm.state.registers[0] = *amount;
        }
        AssetOpPayload::Release { .. } => {
            vm.state.registers[0] = 0;
        }
        AssetOpPayload::Swap {
            input_amount,
            min_output,
            ..
        } => {
            vm.state.registers[0] = *input_amount;
            vm.state.registers[1] = *min_output;
        }
    }
    vm.state.asset_ops.push(payload);
}

fn gas_cost_for_opcode(opcode: u8) -> u128 {
    match opcode {
        0x0A => 50,
        0x01 | 0x02 => 1,
        0x10 | 0x11 => 5,
        0x20 | 0x21 => 1,
        BRIDGE => 100,
        0x30 | 0x31 => 2,
        0x32 | 0x33 => 5,
        0x40 => 10,
        0x50 | 0x51 => 250,
        0x60 | 0x61 => 100,
        0x70 => 2,
        0x80 => 500,
        0x81 => 200,
        0x82 => 100,
        0x83 => 150,
        0x84 => 10,
        0x85 => 500,
        0x86 => 50,
        0x87 => 100,
        0x88 | 0x89 => 50,
        0x8A => 10,
        0x8B => 500,
        0x8C | 0x8D => 20,
        0x8E => 30,
        0x8F => 10,
        0x90 => 20,
        0x91 => 200,
        0x92 => 5,
        0x93 | 0x94 | 0x95 | 0x96 | 0x97 | 0x98 => 10,
        0x99 => 50,
        0x9A => 50,
        0x9B => 50,
        0xFF => 0,
        _ => 1,
    }
}

fn read_len_payload(bytes: &[u8], pc: usize) -> ExecResult<&[u8]> {
    let len = read_u16_le(bytes, pc + 1).ok_or(ExecError::InvalidOperand)? as usize;
    let start = pc + 3;
    let end = start.checked_add(len).ok_or(ExecError::InvalidOperand)?;
    if end > bytes.len() {
        return Err(ExecError::InvalidOperand);
    }
    Ok(&bytes[start..end])
}

fn dispatch_host_opcode(vm: &mut VM, opcode: u8, payload: &[u8]) -> ExecResult<Vec<u8>> {
    let decoded =
        decode_capability_payload(opcode, payload).map_err(|_| ExecError::InvalidOperand)?;
    let result = match decoded {
        CapabilityPayload::GpuDispatch { kernel, args, .. } => {
            bridge_result(vm.bridge.gpu_dispatch(&kernel, args.join("\0").as_bytes()))
        }
        CapabilityPayload::Simulate { body_ops, .. } => {
            bridge_result(vm.bridge.simulate(&body_ops.to_le_bytes()))
        }
        CapabilityPayload::ScheduledDispatch {
            period_blocks,
            entry_ops,
        } => bridge_result(
            vm.bridge
                .scheduled_dispatch(period_blocks, &entry_ops.to_le_bytes()),
        ),
        CapabilityPayload::IntentResolve { constraints, .. } => {
            bridge_result(vm.bridge.intent_resolve(constraints.join("\0").as_bytes()))
        }
        CapabilityPayload::CrdtOp { kind, key, value } => bridge_result(vm.bridge.crdt_op(
            kind,
            key.as_bytes(),
            value.as_deref().unwrap_or_default().as_bytes(),
        )),
        CapabilityPayload::ProofVerify {
            kind,
            proof,
            input,
            key_or_threshold,
        } => bridge_result(vm.bridge.proof_verify(
            kind,
            proof.as_bytes(),
            input.as_bytes(),
            key_or_threshold.as_bytes(),
        )),
        CapabilityPayload::StorageOp { kind, data } => {
            bridge_result(vm.bridge.storage_op(kind, data.as_bytes()))
        }
        CapabilityPayload::Pathfind {
            from,
            to,
            max_depth,
        } => bridge_result(
            vm.bridge
                .pathfind(from.as_bytes(), to.as_bytes(), max_depth),
        ),
        CapabilityPayload::MempoolScan { max_results } => {
            bridge_result(vm.bridge.mempool_scan(max_results))
        }
        CapabilityPayload::OracleRequest { token, reward } => {
            bridge_result(vm.bridge.oracle_request(token.as_bytes(), reward))
        }
        CapabilityPayload::EmergencyControl { kind } => {
            let res = bridge_result(vm.bridge.emergency_control(kind));
            if res.is_ok() {
                vm.state.paused = kind == 0;
            }
            res
        }
        CapabilityPayload::Lifecycle { kind, target } => bridge_result(
            vm.bridge
                .lifecycle(kind, target.as_deref().unwrap_or_default().as_bytes()),
        ),
        CapabilityPayload::Serialize { format, data } => {
            bridge_result(vm.bridge.serialize(format, data.as_bytes()))
        }
        CapabilityPayload::Deserialize { format, data } => {
            bridge_result(vm.bridge.deserialize(format, data.as_bytes()))
        }
        CapabilityPayload::GasEstimate { chain, route } => {
            bridge_result(vm.bridge.gas_estimate(chain.as_bytes(), route.as_bytes()))
        }
        CapabilityPayload::ChainMetric { metric } => bridge_result(vm.bridge.chain_metric(metric)),
        CapabilityPayload::EventProvenance { event_type, data } => bridge_result(
            vm.bridge
                .event_provenance(event_type.as_bytes(), data.as_bytes()),
        ),
        CapabilityPayload::MultiHopSwap { path, amount } => {
            bridge_result(vm.bridge.multi_hop_swap(path.join("\0").as_bytes(), amount))
        }
        CapabilityPayload::VectorMath { op, a, b, size } => {
            bridge_result(vm.bridge.vector_math(op, a.as_bytes(), b.as_bytes(), size))
        }
        CapabilityPayload::RoleCheck { role } => vm
            .bridge
            .role_check(role.as_bytes())
            .map_err(|_| ExecError::Panic("X3_ROLE_DENIED".to_string())),
        CapabilityPayload::MultisigCheck { required, total } => vm
            .bridge
            .multisig_check(required, total)
            .map_err(|_| ExecError::Panic("X3_MULTISIG_THRESHOLD_NOT_MET".to_string())),
        CapabilityPayload::VersionMeta { version, .. } => Ok(version.into_bytes()),
        CapabilityPayload::StorageNamespace { package, key } => {
            Ok([package.as_bytes(), b":", key.as_bytes()].concat())
        }
        CapabilityPayload::AbiExport { function, .. } => Ok(function.into_bytes()),
        CapabilityPayload::DocEmbed { content } => Ok(content.into_bytes()),
        CapabilityPayload::GasAdaptive { .. } => bridge_result(vm.bridge.gas_adaptive_select()),
        CapabilityPayload::Bounty { amount, condition } => {
            bridge_result(vm.bridge.bounty_escrow(amount, condition.as_bytes()))
        }
        CapabilityPayload::SubExec { .. } => {
            Err(ExecError::Panic("X3_SUB_EXEC_UNSUPPORTED".to_string()))
        }
    }?;
    Ok(result)
}

fn bridge_result(result: Result<Vec<u8>, Box<dyn std::error::Error>>) -> ExecResult<Vec<u8>> {
    result.map_err(|err| ExecError::Panic(err.to_string()))
}

fn bytes_to_register(bytes: &[u8]) -> u128 {
    let mut value = 0u128;
    for (idx, byte) in bytes.iter().take(16).enumerate() {
        value |= (*byte as u128) << (idx * 8);
    }
    value
}

fn read_u16_le(bytes: &[u8], idx: usize) -> Option<u16> {
    if idx + 1 >= bytes.len() {
        return None;
    }
    Some((bytes[idx] as u16) | ((bytes[idx + 1] as u16) << 8))
}

fn decode_regtriplet(operand: u16) -> (u8, u8, u8) {
    // operand packs three 5-bit registers: r0[0..4], r1[5..9], r2[10..14]
    let ra = (operand & 0x1F) as u8;
    let rb = ((operand >> 5) & 0x1F) as u8;
    let rc = ((operand >> 10) & 0x1F) as u8;
    (ra, rb, rc)
}

fn decode_reg_reg_imm(operand: u16) -> (u8, u8, u16) {
    // operand: low 5 bits ra, next 5 bits rb, top 6 bits imm6 - extend
    let ra = (operand & 0x1F) as u8;
    let rb = ((operand >> 5) & 0x1F) as u8;
    let imm = (operand >> 10) & 0x3F;
    (ra, rb, imm)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x3_lang_vm::{VM, VMConfig};

    /// Encode a register-triplet operand: ra in bits 0-4, rb in 5-9, rc in 10-14.
    fn enc_tt(ra: u8, rb: u8, rc: u8) -> u16 {
        (ra as u16) | ((rb as u16) << 5) | ((rc as u16) << 10)
    }

    /// Run bytecode and return r0.
    fn run(bytecode: &[u8], r0_init: u128, r1_init: u128, r2_init: u128, gas: u128) -> u128 {
        let mut vm = VM::new(bytecode.to_vec(), VMConfig::default(), gas);
        vm.state.registers[0] = r0_init;
        vm.state.registers[1] = r1_init;
        vm.state.registers[2] = r2_init;
        execute(&mut vm).unwrap();
        vm.state.registers[0]
    }

    #[test]
    fn pow_2_pow_3_eq_8() {
        // POW r0, r1, r2  (0x0A) then HALT (0xFF)
        let code = &[0x0A, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 2, 3, 1_000_000);
        assert_eq!(result, 8, "2 ^ 3");
    }

    #[test]
    fn pow_5_pow_0_eq_1() {
        let code = &[0x0A, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 5, 0, 1_000_000);
        assert_eq!(result, 1, "5 ^ 0");
    }

    #[test]
    fn pow_0_pow_5_eq_0() {
        let code = &[0x0A, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 0, 5, 1_000_000);
        assert_eq!(result, 0, "0 ^ 5");
    }

    #[test]
    fn pow_overflow_saturates() {
        // 2 ^ 128 overflows u128 — saturating_pow returns u128::MAX
        let code = &[0x0A, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 2, 128, 1_000_000);
        assert_eq!(result, u128::MAX, "2 ^ 128 saturates to MAX");
    }

    #[test]
    fn add_rrr_works() {
        // ADD r0, r1, r2  (0x01) then HALT (0xFF)
        let code = &[0x01, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 10, 20, 1_000_000);
        assert_eq!(result, 30, "10 + 20");
    }

    #[test]
    fn sub_rrr_works() {
        let code = &[0x02, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let result = run(code, 0, 100, 7, 1_000_000);
        assert_eq!(result, 93, "100 - 7");
    }

    #[test]
    fn chained_add_pow() {
        // ADD r0, r1, r2  -> r0 = 2 + 1 = 3
        // POW r0, r0, r2 -> r0 = 3 ^ 1 = 3,  wait, need to adjust regs.
        // Let's do: r1=2, r2=1. ADD r0,r1,r2 -> r0=3. Then POW r0,r0,r2 -> 3^1=3.
        // Better: r1=2, r2=3. ADD r0,r1,r2 -> r0=5. POW r3,r0,r2 -> r3=5^3. But our run() only checks r0.
        // Let's do: r1=2, r2=3. ADD r0,r1,r2=5. Then POW r0,r0,r1 -> 5^2=25.
        let op_add = enc_tt(0, 1, 2);  // ADD r0, r1, r2
        let op_pow = enc_tt(0, 0, 1);  // POW r0, r0, r1
        let code: &[u8] = &[
            0x01, 0, (op_add & 0xFF) as u8, (op_add >> 8) as u8,
            0x0A, 0, (op_pow & 0xFF) as u8, (op_pow >> 8) as u8,
            0xFF, 0, 0, 0,
        ];
        let result = run(code, 0, 2, 3, 1_000_000);
        assert_eq!(result, 25, "(2+3)^2 = 25");
    }

    #[test]
    fn pow_gas_is_consumed() {
        let code = &[0x0A, 0, (enc_tt(0, 1, 2) & 0xFF) as u8, (enc_tt(0, 1, 2) >> 8) as u8,
                     0xFF, 0, 0, 0];
        let mut vm = VM::new(code.to_vec(), VMConfig::default(), 100);
        vm.state.registers[1] = 2;
        vm.state.registers[2] = 3;
        execute(&mut vm).unwrap();
        assert_eq!(vm.state.gas, 100 - 50, "Pow costs 50 gas");
        assert_eq!(vm.state.registers[0], 8);
    }

    #[test]
    fn e2e_halt_stops_vm() {
        let code = &[0xFF, 0, 0, 0];
        let mut vm = VM::new(code.to_vec(), VMConfig::default(), 100);
        execute(&mut vm).unwrap();
        // HALT returns Ok so we just verify it completed
    }
}
