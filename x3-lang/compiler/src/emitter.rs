//! X3IR -> Bytecode/Runtime Dispatch emitter.
//!
//! This module converts X3IR operations into executable bytecode suitable
//! for the X3 runtime or specific chain emitters (EVM, SVM, etc.).

use crate::ir::{
    ChainMetricKind, CrdtKind, EmergencyKind, LifecycleKind, Operation, ProofKind, SerialFormat,
    StorageKind, VectorOp, X3IR,
};
use std::io::Write;
use x3_lang_common::{encode_capability_payload, CapabilityPayload, X3Error};
/// Pad `bytecode` so its length is a multiple of 4. The X3 VM verifier
/// requires every instruction to start on a 4-byte boundary.
fn pad_to_4(bytecode: &mut Vec<u8>) {
    let rem = bytecode.len() % 4;
    if rem != 0 {
        for _ in 0..(4 - rem) {
            bytecode.push(0);
        }
    }
}



/// Emit X3IR to bytecode suitable for the X3 runtime
pub fn emit_x3ir(ir: &X3IR) -> Result<Vec<u8>, X3Error> {
    let mut bytecode = Vec::new();

    // Header: version + metadata
    bytecode.write_all(&[0x01])?; // Version 1

    // Encode metadata
    if let Some(nonce) = &ir.metadata.nonce {
        bytecode.write_all(&[0x10])?; // Metadata marker for nonce
        bytecode.write_all(&(nonce.len() as u16).to_le_bytes())?;
        bytecode.write_all(nonce.as_bytes())?;
    }

    if let Some(chain_id) = ir.metadata.chain_id {
        bytecode.write_all(&[0x11])?; // Metadata marker for chain_id
        bytecode.write_all(&chain_id.to_le_bytes())?;
    }

    // Encode operations
    for op in &ir.operations {
        emit_operation(op, &mut bytecode)?;
    }

    // Pad to 4-byte alignment
    while bytecode.len() % 4 != 0 {
        bytecode.push(0);
    }

    Ok(bytecode)
}

/// Emit a single operation to bytecode
fn emit_operation(op: &Operation, bytecode: &mut Vec<u8>) -> Result<(), X3Error> {
    match op {
        Operation::Lock { .. } => {
            // Opcode: 0x20 (LOCK) — no operand bytes; pad_to_4 fills.
            // The IR carries the structured data (chain, asset, amount,
            // from) for the runtime; the verifier does not parse
            // operand bytes for these instructions.
            bytecode.write_all(&[0x20])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::Mint { .. } => {
            // Opcode: 0x21 (MINT)
            bytecode.write_all(&[0x21])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::Burn { .. } => {
            // Opcode: 0x22 (BURN)
            bytecode.write_all(&[0x22])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::Release { .. } => {
            // Opcode: 0x23 (RELEASE)
            bytecode.write_all(&[0x23])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::Swap {
            from_chain,
            from_asset,
            to_asset,
            input_amount,
            min_output,
            dex,
        } => {
            // Opcode: 0x24 (SWAP)
            bytecode.write_all(&[0x24])?;
            let dex_str = dex.as_ref().map(|s| s.as_str()).unwrap_or("none");
            let payload = format!(
                "{};{};{};{};{};{}",
                from_chain, from_asset, to_asset, input_amount, min_output, dex_str
            );
            bytecode.write_all(&(payload.len() as u16).to_le_bytes())?;
            bytecode.write_all(payload.as_bytes())?;
        }
        Operation::AtomicBegin => {
            // Opcode: 0x50 (ATOMIC_BEGIN)
            bytecode.write_all(&[0x50])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::AtomicEnd => {
            // Opcode: 0x51 (ATOMIC_END)
            bytecode.write_all(&[0x51])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::If {
            condition,
            then_ops,
            else_ops,
        } => {
            // Opcode: 0x30 (IF)
            bytecode.write_all(&[0x30])?;
            let cond_str = format!("{:?}", condition);
            bytecode.write_all(&(cond_str.len() as u16).to_le_bytes())?;
            bytecode.write_all(cond_str.as_bytes())?;

            // Then branch length
            let mut then_bytecode = Vec::new();
            for op in then_ops {
                emit_operation(op, &mut then_bytecode)?;
            }
            bytecode.write_all(&(then_bytecode.len() as u32).to_le_bytes())?;
            bytecode.write_all(&then_bytecode)?;

            // Else branch (if exists)
            if let Some(else_blk) = else_ops {
                let mut else_bytecode = Vec::new();
                for op in else_blk {
                    emit_operation(op, &mut else_bytecode)?;
                }
                bytecode.write_all(&(else_bytecode.len() as u32).to_le_bytes())?;
                bytecode.write_all(&else_bytecode)?;
            } else {
                bytecode.write_all(&0u32.to_le_bytes())?;
            }
        }
        Operation::Loop {
            max_iterations,
            body,
        } => {
            // Opcode: 0x31 (LOOP)
            bytecode.write_all(&[0x31])?;
            bytecode.write_all(&max_iterations.to_le_bytes())?;

            // Loop body
            let mut body_bytecode = Vec::new();
            for op in body {
                emit_operation(op, &mut body_bytecode)?;
            }
            bytecode.write_all(&(body_bytecode.len() as u32).to_le_bytes())?;
            bytecode.write_all(&body_bytecode)?;
        }
        Operation::Require { .. } => {
            // Opcode: 0x40 (REQUIRE) — no operand bytes.
            bytecode.write_all(&[0x40])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::OnFail { .. } => {
            // Opcode: 0x41 (ON_FAIL) — no operand bytes.
            bytecode.write_all(&[0x41])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::OnTimeout { .. } => {
            // Opcode: 0x42 (ON_TIMEOUT) — no operand bytes.
            bytecode.write_all(&[0x42])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
        Operation::Emit { name, data } => {
            // Opcode: 0x60 (EMIT)
            bytecode.write_all(&[0x60])?;
            let payload = format!("{}:{:?}", name, data);
            bytecode.write_all(&(payload.len() as u16).to_le_bytes())?;
            bytecode.write_all(payload.as_bytes())?;
        }
        Operation::Call { function, args } => {
            // Opcode: 0x61 (CALL)
            bytecode.write_all(&[0x61])?;
            let payload = format!("{}:{:?}", function, args);
            bytecode.write_all(&(payload.len() as u16).to_le_bytes())?;
            bytecode.write_all(payload.as_bytes())?;
        }
        Operation::GpuDispatch { .. } => emit_payload_op(0x80, op, bytecode)?,
        Operation::Simulate { .. } => emit_payload_op(0x81, op, bytecode)?,
        Operation::ScheduledDispatch { .. } => emit_payload_op(0x82, op, bytecode)?,
        Operation::IntentResolve { .. } => emit_payload_op(0x83, op, bytecode)?,
        Operation::CrdtOp { .. } => emit_payload_op(0x84, op, bytecode)?,
        Operation::ProofVerify { .. } => emit_payload_op(0x85, op, bytecode)?,
        Operation::StorageOp { .. } => emit_payload_op(0x86, op, bytecode)?,
        Operation::Pathfind { .. } => emit_payload_op(0x87, op, bytecode)?,
        Operation::MempoolScan { .. } => emit_payload_op(0x88, op, bytecode)?,
        Operation::OracleRequest { .. } => emit_payload_op(0x89, op, bytecode)?,
        Operation::EmergencyControl { .. } => emit_payload_op(0x8A, op, bytecode)?,
        Operation::Lifecycle { .. } => emit_payload_op(0x8B, op, bytecode)?,
        Operation::Serialize { .. } => emit_payload_op(0x8C, op, bytecode)?,
        Operation::Deserialize { .. } => emit_payload_op(0x8D, op, bytecode)?,
        Operation::GasEstimate { .. } => emit_payload_op(0x8E, op, bytecode)?,
        Operation::ChainMetric { .. } => emit_payload_op(0x8F, op, bytecode)?,
        Operation::EventProvenance { .. } => emit_payload_op(0x90, op, bytecode)?,
        Operation::MultiHopSwap { .. } => emit_payload_op(0x91, op, bytecode)?,
        Operation::VectorMath { .. } => emit_payload_op(0x92, op, bytecode)?,
        Operation::RoleCheck { .. } => emit_payload_op(0x93, op, bytecode)?,
        Operation::MultisigCheck { .. } => emit_payload_op(0x94, op, bytecode)?,
        Operation::VersionMeta { .. } => emit_payload_op(0x95, op, bytecode)?,
        Operation::StorageNamespace { .. } => emit_payload_op(0x96, op, bytecode)?,
        Operation::AbiExport { .. } => emit_payload_op(0x97, op, bytecode)?,
        Operation::DocEmbed { .. } => emit_payload_op(0x98, op, bytecode)?,
        Operation::GasAdaptive { .. } => emit_payload_op(0x99, op, bytecode)?,
        Operation::Bounty { .. } => emit_payload_op(0x9A, op, bytecode)?,
        Operation::Nop => {
            // Opcode: 0x00 (NOP)
            bytecode.write_all(&[0x00])?;
            bytecode.write_all(&0u16.to_le_bytes())?;
        }
    }
    pad_to_4(bytecode);
    Ok(())
}

fn emit_payload_op(opcode: u8, op: &Operation, bytecode: &mut Vec<u8>) -> Result<(), X3Error> {
    bytecode.write_all(&[opcode])?;
    let payload = encode_capability_payload(&operation_to_payload(op)?).map_err(|err| {
        X3Error::CodegenError {
            message: format!("failed to encode capability payload: {err}"),
            span: None,
        }
    })?;
    if payload.len() > u16::MAX as usize {
        return Err(X3Error::CodegenError {
            message: format!("operation payload too large for opcode 0x{opcode:02x}"),
            span: None,
        });
    }
    bytecode.write_all(&(payload.len() as u16).to_le_bytes())?;
    bytecode.write_all(&payload)?;
    Ok(())
}

fn operation_to_payload(op: &Operation) -> Result<CapabilityPayload, X3Error> {
    let payload = match op {
        Operation::GpuDispatch {
            kernel,
            args,
            is_simd,
        } => CapabilityPayload::GpuDispatch {
            kernel: kernel.clone(),
            args: args.clone(),
            is_simd: *is_simd,
        },
        Operation::Simulate { body, receipt_slot } => CapabilityPayload::Simulate {
            body_ops: body.len() as u32,
            receipt_slot: receipt_slot.clone(),
        },
        Operation::ScheduledDispatch {
            period_blocks,
            entry,
        } => CapabilityPayload::ScheduledDispatch {
            period_blocks: *period_blocks,
            entry_ops: entry.len() as u32,
        },
        Operation::IntentResolve {
            constraints,
            resolver,
        } => CapabilityPayload::IntentResolve {
            constraints: constraints.clone(),
            resolver: resolver.clone(),
        },
        Operation::CrdtOp { kind, key, value } => CapabilityPayload::CrdtOp {
            kind: crdt_kind_id(kind),
            key: key.clone(),
            value: value.clone(),
        },
        Operation::ProofVerify {
            kind,
            proof,
            input,
            key_or_threshold,
        } => CapabilityPayload::ProofVerify {
            kind: proof_kind_id(kind),
            proof: proof.clone(),
            input: input.clone(),
            key_or_threshold: key_or_threshold.clone(),
        },
        Operation::StorageOp { kind, data } => CapabilityPayload::StorageOp {
            kind: storage_kind_id(kind),
            data: data.clone(),
        },
        Operation::Pathfind {
            from,
            to,
            max_depth,
        } => CapabilityPayload::Pathfind {
            from: from.clone(),
            to: to.clone(),
            max_depth: *max_depth,
        },
        Operation::MempoolScan { max_results } => CapabilityPayload::MempoolScan {
            max_results: *max_results,
        },
        Operation::OracleRequest { token, reward } => CapabilityPayload::OracleRequest {
            token: token.clone(),
            reward: *reward,
        },
        Operation::EmergencyControl { kind } => CapabilityPayload::EmergencyControl {
            kind: emergency_kind_id(kind),
        },
        Operation::Lifecycle { kind, target } => CapabilityPayload::Lifecycle {
            kind: lifecycle_kind_id(kind),
            target: target.clone(),
        },
        Operation::Serialize { format, data } => CapabilityPayload::Serialize {
            format: serial_format_id(format),
            data: data.clone(),
        },
        Operation::Deserialize { format, data } => CapabilityPayload::Deserialize {
            format: serial_format_id(format),
            data: data.clone(),
        },
        Operation::GasEstimate { chain, route } => CapabilityPayload::GasEstimate {
            chain: chain.clone(),
            route: route.clone(),
        },
        Operation::ChainMetric { metric } => CapabilityPayload::ChainMetric {
            metric: chain_metric_id(metric),
        },
        Operation::EventProvenance { event_type, data } => CapabilityPayload::EventProvenance {
            event_type: event_type.clone(),
            data: data.clone(),
        },
        Operation::MultiHopSwap { path, amount } => CapabilityPayload::MultiHopSwap {
            path: path.clone(),
            amount: *amount,
        },
        Operation::VectorMath { op, a, b, size } => CapabilityPayload::VectorMath {
            op: vector_op_id(op),
            a: a.clone(),
            b: b.clone(),
            size: *size,
        },
        Operation::RoleCheck { role } => CapabilityPayload::RoleCheck { role: role.clone() },
        Operation::MultisigCheck { required, total } => CapabilityPayload::MultisigCheck {
            required: *required,
            total: *total,
        },
        Operation::VersionMeta {
            version,
            upgrade_from,
        } => CapabilityPayload::VersionMeta {
            version: version.clone(),
            upgrade_from: upgrade_from.clone(),
        },
        Operation::StorageNamespace { package, key } => CapabilityPayload::StorageNamespace {
            package: package.clone(),
            key: key.clone(),
        },
        Operation::AbiExport {
            function,
            params,
            ret,
        } => CapabilityPayload::AbiExport {
            function: function.clone(),
            params: params.clone(),
            ret: ret.clone(),
        },
        Operation::DocEmbed { content } => CapabilityPayload::DocEmbed {
            content: content.clone(),
        },
        Operation::GasAdaptive {
            high_gas_ops,
            low_gas_ops,
        } => CapabilityPayload::GasAdaptive {
            high_gas_ops: high_gas_ops.len() as u32,
            low_gas_ops: low_gas_ops.len() as u32,
        },
        Operation::Bounty { amount, condition } => CapabilityPayload::Bounty {
            amount: *amount,
            condition: condition.clone(),
        },
        _ => {
            return Err(X3Error::CodegenError {
                message: "operation is not a capability payload".to_string(),
                span: None,
            })
        }
    };
    Ok(payload)
}

fn crdt_kind_id(kind: &CrdtKind) -> u8 {
    match kind {
        CrdtKind::Get => 0,
        CrdtKind::Set => 1,
        CrdtKind::Append => 2,
        CrdtKind::Merge => 3,
    }
}

fn proof_kind_id(kind: &ProofKind) -> u8 {
    match kind {
        ProofKind::Zk => 0,
        ProofKind::Mpc => 1,
    }
}

fn storage_kind_id(kind: &StorageKind) -> u8 {
    match kind {
        StorageKind::Store => 0,
        StorageKind::Load => 1,
    }
}

fn emergency_kind_id(kind: &EmergencyKind) -> u8 {
    match kind {
        EmergencyKind::Pause => 0,
        EmergencyKind::Resume => 1,
    }
}

fn lifecycle_kind_id(kind: &LifecycleKind) -> u8 {
    match kind {
        LifecycleKind::Destroy => 0,
        LifecycleKind::Migrate => 1,
    }
}

fn serial_format_id(format: &SerialFormat) -> u8 {
    match format {
        SerialFormat::Rlp => 0,
        SerialFormat::Cbor => 1,
        SerialFormat::Json => 2,
        SerialFormat::Ssz => 3,
    }
}

fn chain_metric_id(metric: &ChainMetricKind) -> u8 {
    match metric {
        ChainMetricKind::Snapshot => 0,
        ChainMetricKind::Congestion => 1,
        ChainMetricKind::BaseFee => 2,
        ChainMetricKind::FinalityLag => 3,
        ChainMetricKind::BlockTime => 4,
    }
}

fn vector_op_id(op: &VectorOp) -> u8 {
    match op {
        VectorOp::Add => 0,
        VectorOp::DotProduct => 1,
        VectorOp::Mul => 2,
        VectorOp::Sub => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ChainMetricKind, CrdtKind, EmergencyKind, LifecycleKind, ProofKind, SerialFormat,
        StorageKind, VectorOp,
    };
    use x3_lang_common::{decode_capability_payload, CapabilityPayload};

    #[test]
    fn emits_all_capability_opcodes_0x80_through_0x9a() {
        let mut ir = X3IR::new();
        ir.operations = vec![
            Operation::GpuDispatch {
                kernel: "k".into(),
                args: vec!["a".into()],
                is_simd: true,
            },
            Operation::Simulate {
                body: vec![Operation::Nop],
                receipt_slot: "receipt".into(),
            },
            Operation::ScheduledDispatch {
                period_blocks: 1,
                entry: vec![Operation::Nop],
            },
            Operation::IntentResolve {
                constraints: vec!["c".into()],
                resolver: "r".into(),
            },
            Operation::CrdtOp {
                kind: CrdtKind::Set,
                key: "k".into(),
                value: Some("v".into()),
            },
            Operation::ProofVerify {
                kind: ProofKind::Zk,
                proof: "p".into(),
                input: "i".into(),
                key_or_threshold: "vk".into(),
            },
            Operation::StorageOp {
                kind: StorageKind::Store,
                data: "d".into(),
            },
            Operation::Pathfind {
                from: "a".into(),
                to: "b".into(),
                max_depth: 2,
            },
            Operation::MempoolScan { max_results: 3 },
            Operation::OracleRequest {
                token: "X3".into(),
                reward: 4,
            },
            Operation::EmergencyControl {
                kind: EmergencyKind::Pause,
            },
            Operation::Lifecycle {
                kind: LifecycleKind::Migrate,
                target: Some("next".into()),
            },
            Operation::Serialize {
                format: SerialFormat::Rlp,
                data: "d".into(),
            },
            Operation::Deserialize {
                format: SerialFormat::Cbor,
                data: "d".into(),
            },
            Operation::GasEstimate {
                chain: "evm".into(),
                route: "r".into(),
            },
            Operation::ChainMetric {
                metric: ChainMetricKind::BaseFee,
            },
            Operation::EventProvenance {
                event_type: "e".into(),
                data: "d".into(),
            },
            Operation::MultiHopSwap {
                path: vec!["a".into(), "b".into()],
                amount: 5,
            },
            Operation::VectorMath {
                op: VectorOp::DotProduct,
                a: "a".into(),
                b: "b".into(),
                size: 2,
            },
            Operation::RoleCheck {
                role: "admin".into(),
            },
            Operation::MultisigCheck {
                required: 2,
                total: 3,
            },
            Operation::VersionMeta {
                version: "1.0.0".into(),
                upgrade_from: Some("0.9.0".into()),
            },
            Operation::StorageNamespace {
                package: "pkg".into(),
                key: "k".into(),
            },
            Operation::AbiExport {
                function: "f".into(),
                params: vec!["u64".into()],
                ret: "()".into(),
            },
            Operation::DocEmbed {
                content: "docs".into(),
            },
            Operation::GasAdaptive {
                high_gas_ops: vec![Operation::Nop],
                low_gas_ops: vec![Operation::Nop],
            },
            Operation::Bounty {
                amount: 6,
                condition: "done".into(),
            },
        ];

        let bytecode = emit_x3ir(&ir).expect("capability operations should emit");
        let mut cursor = 1usize;
        for expected in 0x80u8..=0x9A {
            assert_eq!(bytecode[cursor], expected);
            let len = u16::from_le_bytes([bytecode[cursor + 1], bytecode[cursor + 2]]) as usize;
            let payload =
                decode_capability_payload(expected, &bytecode[cursor + 3..cursor + 3 + len])
                    .expect("emitted capability payload should decode");
            if expected == 0x94 {
                assert_eq!(
                    payload,
                    CapabilityPayload::MultisigCheck {
                        required: 2,
                        total: 3,
                    }
                );
            }
            cursor += 3 + len;
        }
    }
}
