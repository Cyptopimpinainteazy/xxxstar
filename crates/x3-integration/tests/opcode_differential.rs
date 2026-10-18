//! Per-opcode differential coverage for every opcode the chain admits without the trading
//! extension (`x3bc_core::spec`, `OnChain::Admitted`).
//!
//! Each opcode runs in a small program on the production engine (`x3-vm`) and on the retired
//! reference interpreter (`mini_x3`), with the same operands, and the two must agree: the same
//! return value, or both refuse. An opcode the reference never implemented (aggregates, context
//! reads, events, contract slots) is named with the suite that holds the production engine to a
//! reference for it instead, and that suite must exist. The trading opcodes' differential is
//! `x3lang_trading_differential.rs` (against the nested `TradingVm`).
//!
//! `x3-vm/tests/opcode_spec_exhaustive.rs` fails if an admitted opcode names no coverage at all.

#![cfg(feature = "std")]

use x3_backend::bc_format::GlobalEntry;
use x3_backend::opcode::{ConstIdx, Opcode};
use x3_backend::{BytecodeModule, FunctionEntry};
use x3_vm::{Value, VM};
use x3_x3_integration::mini_x3::{execute_x3bc, MiniValue};
use x3bc_core::spec::{self, OnChain, OperandKind};

const GAS: u64 = 1_000_000;

/// Opcodes the reference interpreter never implemented, and the suite that holds the production
/// engine to a reference for each.
const COVERED_ELSEWHERE: &[(Opcode, &str)] = &[
    (Opcode::LoadIndex, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::StoreIndex, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::LoadField, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::StoreField, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::NewArray, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::ArrayLen, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::ArrayPush, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::ArrayPop, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::NewTuple, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::TupleGet, "differential.rs::aggregates_compute_what_the_source_states_at_every_level_and_order"),
    (Opcode::CtxSender, "chain_engine.rs::context_opcodes_read_what_the_host_supplied_and_refuse_what_it_did_not"),
    (Opcode::CtxBlockHeight, "chain_engine.rs::context_opcodes_read_what_the_host_supplied_and_refuse_what_it_did_not"),
    (Opcode::CtxTimestamp, "chain_engine.rs::context_opcodes_read_what_the_host_supplied_and_refuse_what_it_did_not"),
    (Opcode::CtxValue, "chain_engine.rs::context_opcodes_read_what_the_host_supplied_and_refuse_what_it_did_not"),
    (Opcode::CtxChainId, "chain_engine.rs::context_opcodes_read_what_the_host_supplied_and_refuse_what_it_did_not"),
    (Opcode::Emit, "events.rs::an_emitted_event_reaches_the_receipt_with_its_name_and_arguments"),
    (Opcode::EvmSload, "interpreter_agreement.rs::slot_storage_is_implemented_by_both_engines_and_names_the_same_slot"),
    (Opcode::EvmSstore, "interpreter_agreement.rs::slot_storage_is_implemented_by_both_engines_and_names_the_same_slot"),
];

/// Opcodes the reference interpreter refuses but the production engine implements, held instead
/// to the value the operation is defined to produce on the sweep's operands (`r1 = 7`).
const ARITHMETIC_ORACLE: &[(Opcode, &str)] = &[(Opcode::Inc, "i64 8"), (Opcode::Dec, "i64 6")];

fn is_float_op(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::NegF
            | Opcode::EqF
            | Opcode::NeF
            | Opcode::LtF
            | Opcode::LeF
            | Opcode::GtF
            | Opcode::GeF
            | Opcode::AddF
            | Opcode::SubF
            | Opcode::MulF
            | Opcode::DivF
            | Opcode::ModF
    )
}

fn module(code: Vec<u8>, functions: Vec<FunctionEntry>, with_pool: bool) -> Vec<u8> {
    let mut module = BytecodeModule::new();
    module.functions = functions;
    if with_pool {
        module.const_pool.add_integer(41).expect("a one-entry pool");
        // Float operands for the float-family opcodes (constants 1 and 2).
        module.const_pool.add_float(2.5).expect("pool");
        module.const_pool.add_float(-1.5).expect("pool");
        module.globals.push(GlobalEntry {
            name: "g".into(),
            type_tag: 1,
            mutable: true,
            init_const: ConstIdx(0),
        });
    }
    module.code = code;
    module.to_bytes()
}

fn main_fn() -> FunctionEntry {
    FunctionEntry {
        name: "main".into(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 1,
    }
}

/// `r1 = 7; r2 = 3; r3 = -2; <op>; ret r0` with register operands `r0, r1, r2, ...` in order,
/// `Imm8` = 5, and the index/target operands chosen so the instruction is well formed.
fn program_for(opcode: Opcode) -> Vec<u8> {
    let entry = spec::of(opcode);
    let mut code = vec![
        Opcode::LoadImm as u8,
        1,
        7,
        Opcode::LoadImm as u8,
        2,
        3,
        Opcode::LoadImm as u8,
        3,
        0xFE, // -2
        Opcode::LoadImm as u8,
        0,
        0,
    ];
    let mut functions = vec![main_fn()];
    let mut with_pool = false;
    if is_float_op(opcode) {
        // r1 = 2.5, r2 = -1.5: float operations need float operands, and on chain the only way
        // to get one is a constant.
        with_pool = true;
        code.extend_from_slice(&[Opcode::LoadConst as u8, 1, 1, 0, 0, 0]);
        code.extend_from_slice(&[Opcode::LoadConst as u8, 2, 2, 0, 0, 0]);
    }
    let start = code.len();
    let length = entry.fixed_len();
    code.push(opcode as u8);
    let mut next_reg = 0u8;
    for kind in entry.operands {
        match kind {
            OperandKind::Reg => {
                code.push(next_reg);
                next_reg += 1;
            }
            OperandKind::Imm8 => code.push(5),
            OperandKind::Index8 => code.push(0),
            OperandKind::Index16 | OperandKind::AtomicId | OperandKind::TradeOp => {
                code.extend_from_slice(&0u16.to_le_bytes())
            }
            OperandKind::Index32 => {
                with_pool = true;
                if opcode == Opcode::Call {
                    code.extend_from_slice(&1u32.to_le_bytes());
                } else {
                    code.extend_from_slice(&0u32.to_le_bytes());
                }
            }
            OperandKind::Target32 => {
                // Jump to the instruction right after this one (taken or not, it lands there).
                code.extend_from_slice(&((start + length) as u32).to_le_bytes());
            }
            OperandKind::RegList16 | OperandKind::PairList16 => {
                code.extend_from_slice(&0u16.to_le_bytes())
            }
            OperandKind::TopicList8 => code.push(0),
        }
    }
    // Windows must balance statically: close the one this instruction opened.
    if opcode == Opcode::AtomicBegin {
        code.extend_from_slice(&[Opcode::AtomicCommit as u8, 0, 0]);
    }
    if matches!(opcode, Opcode::AtomicCommit | Opcode::AtomicRollback) {
        code.splice(start..start, [Opcode::AtomicBegin as u8, 0, 0]);
    }
    let ret_at = code.len();
    code.extend_from_slice(&[Opcode::Ret as u8, 0]);
    if opcode == Opcode::Call {
        // Function 1: `r0 = 9; ret r0`.
        functions.push(FunctionEntry {
            name: "callee".into(),
            entry_point: (ret_at + 2) as u32,
            param_count: 0,
            local_count: 4,
            max_stack: 4,
            return_type_tag: 1,
        });
        code.extend_from_slice(&[Opcode::LoadImm as u8, 0, 9, Opcode::Ret as u8, 0]);
    }
    module(code, functions, with_pool)
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Value(String),
    Refused,
}

fn production(bytes: &[u8]) -> Verdict {
    let Ok(mut vm) = VM::from_bytes(bytes) else {
        return Verdict::Refused;
    };
    vm.config.gas_limit = GAS;
    match vm.call_function(0, &[]) {
        Ok(result) => Verdict::Value(match result.value {
            Some(Value::I64(v)) => format!("i64 {v}"),
            Some(Value::Bool(b)) => format!("bool {b}"),
            Some(Value::F64(f)) => format!("f64 {}", f.to_bits()),
            Some(Value::Unit) | None => "unit".into(),
            Some(other) => format!("{other:?}"),
        }),
        Err(_) => Verdict::Refused,
    }
}

fn reference(bytes: &[u8]) -> Verdict {
    match execute_x3bc(bytes, GAS) {
        Ok(result) => Verdict::Value(match result.return_val {
            MiniValue::I64(v) => format!("i64 {v}"),
            MiniValue::Bool(b) => format!("bool {b}"),
            MiniValue::F64(f) => format!("f64 {}", f.to_bits()),
            MiniValue::Unit => "unit".into(),
            other => format!("{other:?}"),
        }),
        Err(_) => Verdict::Refused,
    }
}

#[test]
fn every_on_chain_opcode_agrees_with_the_reference_engine() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut disagreements = Vec::new();
    let mut compared = 0;
    for entry in spec::ALL.iter() {
        if entry.on_chain != OnChain::Admitted {
            continue;
        }
        if let Some((_, suite)) = COVERED_ELSEWHERE.iter().find(|(op, _)| *op == entry.opcode) {
            let (file, test) = suite.split_once("::").expect("file::test");
            let text = std::fs::read_to_string(root.join(file)).expect("suite file");
            assert!(
                text.contains(&format!("fn {test}(")),
                "{:?}: {suite} is missing",
                entry.opcode
            );
            continue;
        }
        let bytes = program_for(entry.opcode);
        let ours = production(&bytes);
        let theirs = match ARITHMETIC_ORACLE.iter().find(|(op, _)| *op == entry.opcode) {
            Some((_, expected)) => {
                // The reference refuses these; record that, then compare with the definition.
                assert_eq!(reference(&bytes), Verdict::Refused, "{:?}", entry.opcode);
                Verdict::Value((*expected).to_string())
            }
            None => reference(&bytes),
        };
        // These stop the program by design (a rollback aborts, a panic panics, an assertion on
        // `r0 = 0` fails): both engines must refuse.
        let refuses_by_design = matches!(
            entry.opcode,
            Opcode::AtomicRollback | Opcode::Panic | Opcode::Assert
        );
        if refuses_by_design {
            if ours != Verdict::Refused || theirs != Verdict::Refused {
                disagreements.push(format!(
                    "{:?}: must stop the program on both engines; production {ours:?}, reference {theirs:?}",
                    entry.opcode
                ));
            }
        // A program both engines refuse proves nothing about the opcode; require the production
        // engine to run every other opcode it admits on chain.
        } else if !matches!(ours, Verdict::Value(_)) {
            disagreements.push(format!(
                "{:?}: the production engine refused its sweep program",
                entry.opcode
            ));
        } else if ours != theirs {
            disagreements.push(format!(
                "{:?}: production {ours:?}, reference {theirs:?}",
                entry.opcode
            ));
        }
        compared += 1;
    }
    assert!(compared > 50, "only {compared} opcodes compared");
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}
