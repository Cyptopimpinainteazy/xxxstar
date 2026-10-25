//! Strict validator intake for the canonical trading profile.
//! Unsupported canonical instructions are rejected, never skipped.
use crate::{ir::TradingOperation, spec::opcodes::*, verify::verify_trading_program};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Bound decoding before allocating JSON or collecting operations.
pub const MAX_ARTIFACT_BYTES: usize = 256 * 1024;
pub const MAX_TRADING_OPERATIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingArtifact {
    pub operations: Vec<TradingOperation>,
    pub requires_private_submission: bool,
    pub gas_used: u64,
}

pub fn trading_opcode(op: &TradingOperation) -> u8 {
    match op {
        TradingOperation::BeginAtomicTrade { .. } => TRADING_BEGIN,
        TradingOperation::OpenDebt { .. } => TRADING_OPEN_DEBT,
        TradingOperation::ExecuteSwap { .. } => TRADING_EXECUTE_SWAP,
        TradingOperation::CloseDebt { .. } => TRADING_CLOSE_DEBT,
        TradingOperation::AssertMinNetProfit { .. } => TRADING_ASSERT_MIN_PROFIT,
        TradingOperation::AssertAllDebtsClosed => TRADING_ASSERT_ALL_DEBTS,
        TradingOperation::AssertInvariant { .. } => TRADING_ASSERT_INVARIANT,
        TradingOperation::Bridge { .. } => TRADING_BRIDGE,
        TradingOperation::EmitTradeReceipt => TRADING_EMIT_RECEIPT,
        TradingOperation::CommitAtomicTrade => TRADING_COMMIT,
        TradingOperation::AbortAtomicTrade => TRADING_ABORT,
    }
}

pub fn decode_trading_operation(opcode: u8, payload: &[u8]) -> Result<TradingOperation, String> {
    let op = serde_json::from_slice(payload).map_err(|e| format!("trading payload: {e}"))?;
    if trading_opcode(&op) != opcode {
        return Err("trading opcode/payload mismatch".into());
    }
    Ok(op)
}

/// Parse the compiler's versioned stream without redefining its opcode or gas table.
pub fn decode_artifact(bytes: &[u8]) -> Result<TradingArtifact, String> {
    if bytes.is_empty() || bytes.len() > MAX_ARTIFACT_BYTES || bytes.len() % 4 != 0 {
        return Err("invalid canonical artifact length".into());
    }
    let version = bytes[0];
    if !is_supported_version(version) {
        return Err(format!("unsupported canonical version {version}"));
    }
    // This profile does not offer metadata nonce or numeric-chain capabilities.
    // The signed runtime call supplies chain identity and replay protection. An
    // artifact that asks for additional metadata semantics is refused, not ignored.
    if bytes.get(1) != Some(&META_VERSIONS) {
        return Err("unsupported canonical metadata profile".into());
    }
    let fields = bytes.get(2..1 + VERSIONS_RECORD_LEN).ok_or("truncated versions")?;
    for (field, expected) in fields.chunks_exact(2).zip([
        LANGUAGE_VERSION,
        COMPILER_FORMAT_VERSION,
        IR_VERSION,
        VM_VERSION,
        POLICY_VERSION,
    ]) {
        if u16::from_le_bytes([field[0], field[1]]) != expected {
            return Err("canonical version binding mismatch".into());
        }
    }
    let mut pc = 1 + VERSIONS_RECORD_LEN;
    let mut operations = Vec::new();
    let mut private = false;
    let mut gas = u64::try_from(bytes.len()).map_err(|_| "code deposit overflow")?;
    while pc < bytes.len() {
        if bytes[pc..].iter().all(|b| *b == 0) {
            break;
        }
        let opcode = bytes[pc];
        if opcode_version(opcode).is_none_or(|v| v > version) {
            return Err(format!("opcode 0x{opcode:02x} not in canonical version"));
        }
        if !(TRADING_BEGIN..=TRADING_BRIDGE).contains(&opcode) && opcode != MODE_CHECK {
            return Err(format!("unsupported canonical runtime opcode 0x{opcode:02x}"));
        }
        let len = read_len(bytes, pc + 1)?;
        let end = pc + 3 + len;
        let payload = bytes.get(pc + 3..end).ok_or("truncated trading instruction")?;
        let next = (end + 3) & !3;
        if bytes.get(end..next).ok_or("truncated padding")?.iter().any(|b| *b != 0) {
            return Err("nonzero canonical padding".into());
        }
        if opcode == MODE_CHECK {
            let mut expected = Vec::new();
            for s in ["submission", "private_required"] {
                expected.extend_from_slice(&(s.len() as u16).to_le_bytes());
                expected.extend_from_slice(s.as_bytes());
            }
            if private || !operations.is_empty() || payload != expected {
                return Err("unsupported canonical mode check".into());
            }
            private = true;
        } else {
            if operations.len() >= MAX_TRADING_OPERATIONS {
                return Err("too many trading operations".into());
            }
            operations.push(decode_trading_operation(opcode, payload)?);
        }
        gas = gas
            .checked_add(u64::try_from(base_gas_cost(opcode)).map_err(|_| "gas overflow")?)
            .ok_or("gas overflow")?;
        pc = next;
    }
    verify_trading_program(&operations).map_err(|e| e.join("; "))?;
    match operations.first() {
        Some(TradingOperation::BeginAtomicTrade { policy, .. }) if policy.require_private_submission == private => {}
        _ => return Err("private submission binding mismatch".into()),
    }
    Ok(TradingArtifact {
        operations,
        requires_private_submission: private,
        gas_used: gas,
    })
}
fn read_len(bytes: &[u8], pc: usize) -> Result<usize, String> {
    let data = bytes.get(pc..pc + 2).ok_or("truncated instruction length")?;
    Ok(usize::from(u16::from_le_bytes([data[0], data[1]])))
}
