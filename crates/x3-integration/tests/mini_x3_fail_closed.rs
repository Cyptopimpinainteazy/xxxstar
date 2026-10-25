//! The interpreter the chain actually runs must refuse what it cannot execute.
//!
//! **Since the runtime switched engines:** blocks execute `crates/x3-vm` through
//! `X3Executor::execute_on_chain` in every build (`tests/chain_engine.rs`). `mini_x3` is no longer
//! on any chain path; it stays as the reference this file holds the new engine to, until the old
//! engines are retired. Where the text below says a block runs `mini_x3`, read "used to run".
//!
//! There are two readers of X3BC and two interpreters of its opcodes: `x3-backend`/`x3-vm` on the
//! `std` side, and `x3-integration::mini_x3`, which exists because `pallets/x3-kernel` has to
//! execute `.x3` inside the runtime, and the runtime has no `std`. The node builds its executor
//! with `sc_service::new_wasm_executor` (`node/src/service.rs`), so **`mini_x3` is the only
//! interpreter that runs on chain** — the `std` VM is reached by tools, tests and off-chain
//! simulation, never by a block.
//!
//! That makes any fabricated answer in `mini_x3` a fabricated answer in consensus. It used to hold
//! a block of arms that decoded their operands, wrote a placeholder into the destination register
//! and moved on:
//!
//! ```text
//! 0x14 / 0x15  LoadIndex / StoreIndex      dst = I64(0)
//! 0x16 / 0x17  LoadField / StoreField      dst = I64(0)
//! 0x70..0x75   arrays and tuples           dst = I64(0) / Unit, or a silent no-op
//! 0x80..0x85   context reads               a zero sender, height, timestamp, value, or 3375
//! 0x90..0x93   atomic window               tracked and skipped, nothing ever reverted
//! 0xA0..0xA1   agent                       Unit
//! 0xB0..0xB9, 0xC0..0xC7, 0xD0..0xD7       EVM / SVM / GPU  dst = I64(0), skip six bytes
//! ```
//!
//! A program that read `array[0]` therefore returned `0` on chain rather than failing, a program
//! that asked for the block height got `0`, an EVM call reported success with a zero result, and a
//! GPU signature check that never ran returned `I64(0)`. `AGENTS.md` §5 says uncertainty must not
//! become success and §18 says an accelerator must never be accepted merely because it answered;
//! these arms did both. The six-byte skip was also wrong for most of the intrinsics (`EvmSstore` is
//! three bytes, `SvmCreateAccount` seven), so the arms desynchronised the instruction stream.
//!
//! These tests hold the on-chain interpreter to the opposite rule: an opcode it cannot really
//! execute is refused, by name, before it is answered.

#![cfg(feature = "std")]

use x3_backend::{BytecodeModule, FunctionEntry};
use x3_x3_integration::mini_x3::{execute_x3bc, MiniValue, X3Error};

/// A module whose `main` is exactly `code`, wrapped by the real envelope writer so the header,
/// checksum and section layout are the ones the compiler produces.
fn module_with_code(code: &[u8]) -> Vec<u8> {
    let mut module = BytecodeModule::new();
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 1,
    });
    module.code = code.to_vec();
    module.to_bytes()
}

/// `instruction` followed by `Nop`s and a `RetVoid`, so a placeholder that skips its operand bytes
/// runs to a clean end and reports a value rather than an unrelated `UnexpectedEof`.
fn code_with(instruction: &[u8]) -> Vec<u8> {
    let mut code = instruction.to_vec();
    code.extend_from_slice(&[0x00; 8]); // Nop
    code.push(0x06); // RetVoid
    code
}

/// Every instruction the parser accepts and the on-chain interpreter has no implementation for,
/// with the operand bytes the compiler's emitter gives it (`crates/x3-backend/src/emit.rs`).
fn unexecutable_instructions() -> Vec<(&'static str, Vec<u8>)> {
    let mut cases: Vec<(&'static str, Vec<u8>)> = vec![
        // Arrays and aggregate fields — the interpreter has no aggregate representation.
        ("load_index", vec![0x14, 0x00, 0x01, 0x02]),
        ("store_index", vec![0x15, 0x00, 0x01, 0x02]),
        ("load_field", vec![0x16, 0x00, 0x01, 0x00, 0x00]),
        ("store_field", vec![0x17, 0x01, 0x00, 0x00, 0x00]),
        ("new_array", vec![0x70, 0x00, 0x04]),
        ("array_len", vec![0x71, 0x00, 0x01]),
        ("array_push", vec![0x72, 0x00, 0x01]),
        ("array_pop", vec![0x73, 0x00]),
        ("new_tuple", vec![0x74, 0x00, 0x02, 0x00, 0x01]),
        ("tuple_get", vec![0x75, 0x00, 0x01, 0x00, 0x00]),
        // Execution context — nothing supplies it to the runtime interpreter, so a value here is
        // invented. `ctx_gas` (0x84) used to be the exception, answered from this engine's own
        // budget; it is refused now because the `std` engine is not held to the same per-opcode
        // charges, so the two could answer differently (TICKET-149).
        ("ctx_gas", vec![0x84, 0x00]),
        ("ctx_sender", vec![0x80, 0x00]),
        ("ctx_block_height", vec![0x81, 0x00]),
        ("ctx_timestamp", vec![0x82, 0x00]),
        ("ctx_value", vec![0x83, 0x00]),
        ("ctx_chain_id", vec![0x85, 0x00]),
        // Numeric conversions (TICKET-149): the 32-bit ones were identities under names that
        // promise truncation, rounding or saturation, and the value model has no 32-bit types.
        ("i32_to_i64", vec![0x60, 0x00, 0x01]),
        ("i64_to_i32", vec![0x61, 0x00, 0x01]),
        ("i32_to_f32", vec![0x62, 0x00, 0x01]),
        ("i64_to_f64", vec![0x63, 0x00, 0x01]),
        ("f32_to_i32", vec![0x64, 0x00, 0x01]),
        ("f64_to_i64", vec![0x65, 0x00, 0x01]),
        ("f32_to_f64", vec![0x66, 0x00, 0x01]),
        ("f64_to_f32", vec![0x67, 0x00, 0x01]),
        ("to_bool", vec![0x68, 0x00, 0x01]),
        // Agents — no agent registry reaches this interpreter. `emit` (0xA2) is not here: it is
        // implemented, and `tests/events.rs` holds it to the `std` engine's journal.
        ("agent_self", vec![0xA0, 0x00]),
        ("agent_init", vec![0xA1, 0x00, 0x00, 0x00]),
    ];

    // Cross-VM and GPU intrinsics. Each is a real opcode with a defined encoding; none has a host
    // to call in the runtime build, and a GPU result that no device produced is exactly what §18
    // forbids accepting. `0xB3`/`0xB4` (`EvmSload`/`EvmSstore`) are deliberately absent: they are
    // the slot-storage pair and are implemented for real, which the tests below hold them to.
    for byte in (0xB0u8..=0xB9)
        .chain(0xC0..=0xC7)
        .chain(0xD0..=0xD7)
        .filter(|byte| *byte != 0xB3 && *byte != 0xB4)
    {
        cases.push(("intrinsic", vec![byte, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
    }
    cases
}

#[test]
fn a_supported_program_still_runs_so_the_refusals_are_not_vacuous() {
    // `LoadImm r0, 7` + `Ret r0`: the control that shows a module built by this helper executes.
    let bytes = module_with_code(&[0x18, 0x00, 7, 0x05, 0x00]);
    let result = execute_x3bc(&bytes, 100_000).expect("a scalar program must run");
    assert_eq!(result.return_val, MiniValue::I64(7));
}

#[test]
fn the_on_chain_interpreter_refuses_every_instruction_it_cannot_execute() {
    let mut fabricated: Vec<String> = Vec::new();
    let mut refused = 0usize;

    for (name, instruction) in unexecutable_instructions() {
        let byte = instruction[0];
        let bytes = module_with_code(&code_with(&instruction));
        match execute_x3bc(&bytes, 100_000) {
            Err(X3Error::UnsupportedOpcode(refused_byte)) if refused_byte == byte => {
                refused += 1;
            }
            Err(other) => fabricated.push(format!(
                "{name} (0x{byte:02X}): refused with {other:?}, which does not name the opcode"
            )),
            Ok(result) => fabricated.push(format!(
                "{name} (0x{byte:02X}): answered {:?} instead of refusing",
                result.return_val
            )),
        }
    }

    let expected = unexecutable_instructions().len();
    assert!(
        fabricated.is_empty(),
        "{} of {expected} instructions were answered rather than refused:\n  {}",
        fabricated.len(),
        fabricated.join("\n  ")
    );
    assert_eq!(refused, expected, "every case must be refused by name");
}

/// The atomic window is the one opcode family in that block that can be implemented for real here.
///
/// `mini_x3` has no storage and no events, so the only state an atomic window can protect is the
/// module's globals — which means the guarantee is small but complete, and the interpreter can
/// honestly provide it instead of skipping the window. The `std` VM (`crates/x3-vm/src/vm.rs`)
/// already does this for registers, globals and storage; these tests pin the runtime side to the
/// same lifecycle, including the refusal of a commit or rollback with no window open.
#[test]
fn an_atomic_window_with_nothing_open_is_refused() {
    let commit = module_with_code(&code_with(&[0x91, 0x00, 0x00]));
    assert_eq!(
        execute_x3bc(&commit, 100_000).unwrap_err(),
        X3Error::AtomicEndWithoutBegin,
        "a commit outside a window must not be a no-op"
    );

    let rollback = module_with_code(&code_with(&[0x92, 0x00, 0x00]));
    assert_eq!(
        execute_x3bc(&rollback, 100_000).unwrap_err(),
        X3Error::AtomicRollbackWithoutBegin,
        "a rollback outside a window must not be a no-op"
    );
}

/// `atomic_check` reports whether a window is open, and it has to be a real query: the compiler
/// emits it, and a constant answer sends every program down the same branch.
#[test]
fn atomic_check_reports_the_window_it_is_in() {
    // Outside any window: `atomic_check r0`, then `Ret r0` (which must be `false`).
    let outside = module_with_code(&[0x93, 0x00, 0x05, 0x00]);
    let result = execute_x3bc(&outside, 100_000).expect("the query itself runs");
    assert_eq!(
        result.return_val,
        MiniValue::Bool(false),
        "no window is open, so the query is false"
    );

    // Inside a window: `AtomicBegin`, `atomic_check r0`, `AtomicCommit`, `Ret r0`.
    let inside = module_with_code(&[
        0x90, 0x00, 0x00, // AtomicBegin 0
        0x93, 0x00, // atomic_check r0
        0x91, 0x00, 0x00, // AtomicCommit 0
        0x05, 0x00, // Ret r0
    ]);
    let result = execute_x3bc(&inside, 100_000).expect("a window that commits runs");
    assert_eq!(
        result.return_val,
        MiniValue::Bool(true),
        "the query inside the window must say so"
    );
}

/// A rollback restores what the window changed and ends the execution, exactly as the `std` VM
/// does.
///
/// The abort makes the restored global unobservable *from this program*, so a test that only
/// asserted the error would pass with no snapshot at all. Two things make it non-vacuous: the same
/// artifact with the rollback replaced by a commit reads back the value the window wrote, so the
/// write the window had to undo is real; and the engine that tools run (`x3-vm`) has to abort on
/// the same bytes, so the two interpreters cannot drift apart here.
#[test]
fn a_rollback_aborts_and_both_engines_abort_with_it() {
    // This module needs a global to write, so it cannot use `module_with_code` (which builds a
    // module with no globals); the envelope is otherwise the same.
    let build = |code: &[u8]| -> Vec<u8> {
        let mut module = BytecodeModule::new();
        let init = module
            .const_pool
            .add_integer(11)
            .expect("the writer must accept an integer constant");
        module.globals.push(x3_backend::bc_format::GlobalEntry {
            name: "g".to_string(),
            type_tag: 1,
            mutable: true,
            init_const: init,
        });
        module.functions.push(FunctionEntry {
            name: "main".to_string(),
            entry_point: 0,
            param_count: 0,
            local_count: 16,
            max_stack: 16,
            return_type_tag: 1,
        });
        module.code = code.to_vec();
        module.to_bytes()
    };
    // The body is built twice: once ending the window with a rollback, once with a commit.
    let body = |window_end: u8| -> Vec<u8> {
        vec![
            0x90, 0x00, 0x00, // AtomicBegin 0
            0x18, 0x01, 99, // LoadImm r1, 99
            0x13, 0x00, 0x00, 0x00, 0x00, 0x01, // StoreGlobal 0 <- r1
            window_end, 0x00, 0x00, // AtomicRollback / AtomicCommit 0
            0x12, 0x00, 0x00, 0x00, 0x00, 0x00, // LoadGlobal r0 <- 0
            0x05, 0x00, // Ret r0
        ]
    };

    let rollback = build(&body(0x92));
    assert_eq!(
        execute_x3bc(&rollback, 100_000).unwrap_err(),
        X3Error::AtomicAborted,
        "an atomic rollback aborts the execution; it does not continue past the window"
    );

    // The engine a block runs and the engine tools run must agree on that.
    let mut vm = x3_vm::VM::from_bytes(&rollback).expect("the std VM loads the same artifact");
    let error = vm
        .call_function(0, &[])
        .expect_err("the std VM must abort on a rollback too");
    assert!(
        matches!(error.kind, x3_vm::VMErrorKind::AtomicAborted),
        "the std VM's rollback must be AtomicAborted, got {:?}",
        error.kind
    );

    // Control: with the window committed instead of rolled back, the write survives and is
    // observable. Without this, the test would pass against an interpreter that reverted nothing.
    let committed = build(&body(0x91));
    let result = execute_x3bc(&committed, 100_000).expect("a committed window runs to the end");
    assert_eq!(
        result.return_val,
        MiniValue::I64(99),
        "the store inside the window must be real"
    );
}

/// The window depth is bounded, so a program cannot make the interpreter hold an unbounded number
/// of globals copies. `MAX_ATOMIC_DEPTH` is the advertised limit, and both sides of it are
/// asserted: the deepest legal nesting runs, one window more is refused by name. A limit that is
/// only enforced in the code and never on either side of its boundary is a number, not a bound.
#[test]
fn nesting_more_windows_than_the_limit_is_refused() {
    let nested = |depth: usize| -> Vec<u8> {
        let mut code = Vec::new();
        for _ in 0..depth {
            code.extend_from_slice(&[0x90, 0x00, 0x00]); // AtomicBegin 0
        }
        code.push(0x06); // RetVoid
        module_with_code(&code)
    };

    assert!(
        execute_x3bc(&nested(32), 1_000_000).is_ok(),
        "the deepest legal nesting must still run"
    );
    assert_eq!(
        execute_x3bc(&nested(33), 1_000_000).unwrap_err(),
        X3Error::AtomicDepthExceeded,
        "one window past the limit must be refused, not tracked"
    );
}

/// A value written to a slot must reach the caller as a write the chain can apply.
///
/// `0xB3`/`0xB4` are the only way an `.x3` program carries a value between executions, so this is
/// also the only place an on-chain program can change chain state at all. The *key* matters as much
/// as the value: it is the 32 bytes the kernel applies, so it is asserted literally rather than
/// compared against the interpreter's own helper (which would agree with itself whatever it did).
#[test]
fn a_slot_written_on_chain_is_reported_as_a_write_the_receipt_can_carry() {
    // LoadImm r1, 0; LoadImm r2, 5; EvmSstore r1 <- r2; EvmSload r3 <- r1; Ret r3
    let bytes = module_with_code(&[
        0x18, 0x01, 0x00, //
        0x18, 0x02, 0x05, //
        0xB4, 0x01, 0x02, //
        0xB3, 0x03, 0x01, //
        0x05, 0x03, //
    ]);
    let result = execute_x3bc(&bytes, 100_000).expect("a storing program must run");

    assert_eq!(
        result.return_val,
        MiniValue::I64(5),
        "the load must read back what the store wrote"
    );
    assert_eq!(result.storage_writes.len(), 1, "one store, one write");

    let write = &result.storage_writes[0];
    // The EVM domain tag in the first eight bytes, the slot index little-endian in the next eight,
    // the rest zero. These are the bytes `crates/x3-vm` produces for the same slot, and
    // `interpreter_agreement.rs` holds the two engines to it; the literal is here so a change to
    // the derivation cannot pass by changing the helper that computes it.
    let mut expected_key = [0u8; 32];
    expected_key[..8].copy_from_slice(&0x5833_4556_4D5F_534Cu64.to_le_bytes());
    assert_eq!(write.key.as_bytes(), &expected_key);

    assert_eq!(
        write.old_value, None,
        "the slot was empty in this execution's view"
    );
    let mut expected_payload = [0u8; 32];
    expected_payload[0] = 1; // integer tag
    expected_payload[1] = 8; // eight bytes of payload
    expected_payload[2..10].copy_from_slice(&5i64.to_le_bytes());
    assert_eq!(write.new_value, Some(expected_payload));
}

/// An unwritten slot reads as zero — EVM's rule, and the only rule an interpreter that starts with
/// no state can honestly apply. Anything else would be a fabricated value.
#[test]
fn an_unwritten_slot_reads_as_zero_and_a_read_is_not_a_write() {
    // LoadImm r1, 9 (never written); EvmSload r3 <- r1; Ret r3
    let bytes = module_with_code(&[0x18, 0x01, 0x09, 0xB3, 0x03, 0x01, 0x05, 0x03]);
    let result = execute_x3bc(&bytes, 100_000).expect("a reading program must run");

    assert_eq!(result.return_val, MiniValue::I64(0));
    assert!(
        result.storage_writes.is_empty(),
        "a load must not be journaled as a write"
    );
}

/// A store the slot cannot carry is refused by name instead of being dropped.
///
/// `StoreGlobal` ignores a `Unit` write, so the program is told a write it did not get succeeded.
/// The slot store must not repeat that: the register here is never initialised, so it holds `Unit`.
#[test]
fn a_store_of_a_value_a_slot_cannot_carry_is_refused() {
    // LoadImm r1, 0; EvmSstore r1 <- r2 (r2 is still Unit); RetVoid
    let bytes = module_with_code(&[0x18, 0x01, 0x00, 0xB4, 0x01, 0x02, 0x06]);
    assert_eq!(
        execute_x3bc(&bytes, 100_000).unwrap_err(),
        X3Error::UnencodableStorageValue("unit is not a storable value")
    );

    // A slot index that is not a non-negative integer is refused too: a negative index is not a
    // key, and reinterpreting it as one would move the write to a slot nobody asked for.
    // LoadImm r1, -1 is not expressible here, so the register is loaded with a Bool instead.
    let bool_slot = module_with_code(&[0x18, 0x01, 0x00, 0xB3, 0x03, 0x02, 0x06]);
    assert_eq!(
        execute_x3bc(&bool_slot, 100_000).unwrap_err(),
        X3Error::InvalidStorageSlot("slot index is not an integer")
    );
}
