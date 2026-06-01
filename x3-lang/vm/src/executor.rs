//! X3 VM executor - fetch-decode-execute loop and handlers for opcodes.

use crate::x3_lang_vm::VM;
use sha2::{Digest, Sha256};
use x3_lang_common::{decode_capability_payload, CapabilityPayload};

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
        let pc_next = vm.state.pc + 4;

        if vm.state.paused && opcode != 0x8A {
            return Err(ExecError::Panic("X3_PAUSED".to_string()));
        }

        // Gas accounting (simplified: 1 unit per instruction)
        let cost = gas_cost_for_opcode(opcode);
        if vm.state.gas < cost {
            return Err(ExecError::OutOfGas);
        }
        vm.state.gas -= cost;

        match opcode {
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
            0x10 => {
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
            0x11 => {
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
            0x20 => {
                // PUSH_IMM - operand is immediate 16-bit sign-extended
                let imm16 = operand as i16 as i128 as u128;
                vm.state.registers[0] = imm16; // use R0 as push target then increment SP
                vm.state.sp = vm.state.sp.wrapping_add(1);
            }
            0x21 => {
                // POP dest
                // operand holds dest register
                let dest = (operand & 0xFF) as usize;
                if vm.state.sp == 0 {
                    return Err(ExecError::Panic("stack underflow".to_string()));
                }
                vm.state.sp -= 1;
                vm.state.registers[dest] = vm.state.registers[0];
            }
            0x30 => {
                // JMP offset
                let rel = operand as i16;
                let dst = (pc_next as i32).wrapping_add(rel as i32) as usize;
                vm.state.pc = dst;
                continue;
            }
            0x31 => {
                // JZ - jump if top-of-stack zero
                let dest = operand as i16;
                let top = vm.state.registers[0];
                if top == 0 {
                    let dst = (pc_next as i32).wrapping_add(dest as i32) as usize;
                    vm.state.pc = dst;
                    continue;
                }
            }
            0x32 => {
                // CALL - push return and jump
                let addr = operand as usize;
                vm.state.call_stack.push(pc_next);
                vm.state.pc = addr;
                continue;
            }
            0x33 => {
                // RET
                let retpc = vm
                    .state
                    .call_stack
                    .pop()
                    .ok_or_else(|| ExecError::Panic("call stack underflow".to_string()))?;
                vm.state.pc = retpc;
                continue;
            }
            0x40 => {
                // CRYPTO_SHA256 - hash low 16 bytes from R[b], write first 16 bytes to R[a].
                let (ra, rb, _) = decode_reg_reg_imm(operand);
                let mut input = [0u8; 16];
                let value = vm.state.registers[rb as usize];
                for i in 0..16 {
                    input[i] = ((value >> (i * 8)) & 0xFF) as u8;
                }
                let digest = Sha256::digest(input);
                vm.state.registers[ra as usize] = bytes_to_register(&digest[..16]);
            }
            0x50 => {
                // ATOMIC_BEGIN - push current pc onto atomic stack
                vm.state.atomic_stack.push(vm.state.pc + 4);
            }
            0x51 => {
                // ATOMIC_COMMIT - pop atomic stack
                vm.state.atomic_stack.pop();
            }
            0x52 => {
                // ATOMIC_ROLLBACK - pop and jump to begin
                if let Some(begin_pc) = vm.state.atomic_stack.pop() {
                    vm.state.pc = begin_pc;
                    continue;
                }
            }
            0x60 => {
                // EVM_CALL - use bridge adapter
                // For simplicity, use the next 2 bytes as dummy data
                let data = &vm.code.as_slice()[vm.state.pc + 2..vm.state.pc + 4];
                let res = bridge_result(vm.bridge.evm_call(data))?;
                vm.state.registers[0] = bytes_to_register(&res);
            }
            0x61 => {
                // SVM_CALL - use bridge adapter
                let data = &vm.code.as_slice()[vm.state.pc + 2..vm.state.pc + 4];
                let res = bridge_result(vm.bridge.svm_call(data))?;
                vm.state.registers[0] = bytes_to_register(&res);
            }
            0x70 => {
                // SIMD_ADD_VV - Vector Add placeholder
                let (va, vb, vc) = decode_regtriplet(operand);
                let mut out = [0u8; 16];
                for i in 0..16 {
                    out[i] = vm.state.vector_registers[vb as usize][i]
                        .wrapping_add(vm.state.vector_registers[vc as usize][i]);
                }
                vm.state.vector_registers[va as usize] = out;
            }
            0x80..=0x9B => {
                let payload = read_len_payload(vm.code.as_slice(), vm.state.pc)?.to_vec();
                let result = dispatch_host_opcode(vm, opcode, &payload)?;
                vm.state.registers[0] = bytes_to_register(&result);
                vm.state.pc += 3 + payload.len();
                continue;
            }
            0x00 => { // NOP
            }
            0xFF => {
                // HALT
                return Ok(());
            }
            other => return Err(ExecError::InvalidOpcode(other)),
        }

        vm.state.pc = pc_next;
    }
}

fn gas_cost_for_opcode(opcode: u8) -> u128 {
    match opcode {
        0x01 | 0x02 => 1,
        0x10 | 0x11 => 5,
        0x20 | 0x21 => 1,
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
