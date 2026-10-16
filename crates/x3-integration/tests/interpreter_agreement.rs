//! The two interpreters must refuse the same opcodes.
//!
//! **Since the runtime switched engines:** blocks execute `crates/x3-vm` through
//! `X3Executor::execute_on_chain` in every build (`tests/chain_engine.rs`). `mini_x3` is no longer
//! on any chain path; it stays as the reference this file holds the new engine to, until the old
//! engines are retired. Where the text below says a block runs `mini_x3`, read "used to run".
//!
//! `crates/x3-vm` (reached by tests, tools and simulation) and
//! `crates/x3-integration::mini_x3` (reached by a block — the node builds a `WasmExecutor`, so the
//! runtime executes the `no_std` one) are two implementations of one ISA. `compiler_bridge.rs`
//! already holds them to the same *answer* on the compiler's fixture corpus; this file holds them
//! to the same *refusals*, which is the half a corpus of working programs cannot show.
//!
//! The rule is the fail-closed one: for every opcode in the table below, neither engine may hand the
//! program a value. Either engine completing one of these is a fail-open path — the program receives
//! a result no implementation produced. The engines need not refuse *identically* to satisfy this
//! (the runtime names the opcode, the `std` VM may report a missing hostcall instead); they must
//! refuse. The divergences that remain after that are listed at the bottom of this file, so an
//! opcode only one engine implements is recorded rather than silently averaged away.

#![cfg(feature = "std")]

use x3_backend::{BytecodeModule, FunctionEntry};
use x3_vm::VM;
use x3_x3_integration::mini_x3::{execute_x3bc, execute_x3bc_with_slots, X3Error};

/// A module whose `main` is `instruction`, then `Nop`s and a `RetVoid`, written by the real
/// envelope writer.
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

/// `instruction`, then `Nop`s and a `RetVoid`, so an engine that skips the instruction reaches a
/// clean end instead of an unrelated `UnexpectedEof`.
fn module_with(instruction: &[u8]) -> Vec<u8> {
    let mut code = instruction.to_vec();
    code.extend_from_slice(&[0x00; 8]);
    code.push(0x06); // RetVoid
    module_with_code(&code)
}

/// Opcodes the compiler can emit and neither interpreter implements, with one reasoning note each.
/// Kept as a table so a divergence shows up as a changed refusal rather than as a silent success.
fn agreed_refusals() -> Vec<(u8, &'static str)> {
    let mut table: Vec<(u8, &'static str)> = vec![
        (0x14, "load_index"),
        (0x15, "store_index"),
        (0x16, "load_field"),
        (0x17, "store_field"),
        (0x70, "new_array"),
        (0x71, "array_len"),
        (0x72, "array_push"),
        (0x73, "array_pop"),
        (0x74, "new_tuple"),
        (0x75, "tuple_get"),
        // The context reads below are implemented by `x3-vm` now, and still refused here because
        // `VM::from_bytes` is given no context: with nothing supplied, neither engine answers.
        (0x80, "ctx_sender"),
        (0x81, "ctx_block_height"),
        (0x82, "ctx_timestamp"),
        (0x83, "ctx_value"),
        (0x85, "ctx_chain_id"),
        (0xA0, "agent_self"),
        (0xA1, "agent_init"),
        // `0xA2` (`emit`) is deliberately absent: both engines implement it now, and
        // `tests/events.rs` compares their journals rather than asserting they both refuse it.
        // TICKET-149: the runtime interpreter used to execute these and `crates/x3-vm` did not.
        // Both refuse them now; see `mini_x3::verify_code` for why neither half was right.
        (0x60, "i32_to_i64"),
        (0x61, "i64_to_i32"),
        (0x62, "i32_to_f32"),
        (0x63, "i64_to_f64"),
        (0x64, "f32_to_i32"),
        (0x65, "f64_to_i64"),
        (0x66, "f32_to_f64"),
        (0x67, "f64_to_f32"),
        (0x68, "to_bool"),
        (0x84, "ctx_gas"),
    ];
    // `0xB3`/`0xB4` (`EvmSload`/`EvmSstore`) are excluded because both engines now genuinely
    // implement them; the test below is what holds that pair rather than this refusal table.
    for byte in (0xB0u8..=0xB9)
        .chain(0xC0..=0xC7)
        .chain(0xD0..=0xD7)
        .filter(|byte| *byte != 0xB3 && *byte != 0xB4)
    {
        table.push((byte, "cross-vm / gpu intrinsic"));
    }
    table
}

#[test]
fn neither_interpreter_executes_an_opcode_the_other_one_refuses() {
    let mut disagreements: Vec<String> = Vec::new();

    for (byte, name) in agreed_refusals() {
        let bytes = module_with(&[byte, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // The runtime interpreter, which is the one a block runs. It has no implementation for any
        // of these, so its refusal must name the opcode.
        match execute_x3bc(&bytes, 100_000) {
            Err(X3Error::UnsupportedOpcode(refused)) if refused == byte => {}
            Err(other) => disagreements.push(format!(
                "{name} (0x{byte:02X}): the runtime interpreter refused it as {other:?} rather than naming the opcode"
            )),
            Ok(result) => disagreements.push(format!(
                "{name} (0x{byte:02X}): the runtime interpreter answered {:?}",
                result.return_val
            )),
        }

        // The `std` interpreter, which tests and tools reach. Whether it refuses because the opcode
        // has no arm or because the host is missing is an implementation detail; answering is not.
        let mut vm = VM::from_bytes(&bytes)
            .unwrap_or_else(|e| panic!("{name} (0x{byte:02X}): the std VM must load it: {e:?}"));
        if let Ok(result) = vm.call_function(0, &[]) {
            disagreements.push(format!(
                "{name} (0x{byte:02X}): the std VM answered {:?}",
                result.value
            ));
        }
    }

    assert!(
        disagreements.is_empty(),
        "{} of {} agreed-refusal opcodes were executed by one of the engines:\n  {}",
        disagreements.len(),
        agreed_refusals().len(),
        disagreements.join("\n  ")
    );
}

/// The control: the table is not "every opcode refuses". A scalar program that both engines do
/// implement runs in both, so the assertions above are about these opcodes and not about a
/// broken harness.
#[test]
fn both_engines_execute_the_same_supported_program() {
    // `LoadImm r0, 21` + `Ret r0`, with no padding: the program returns what it loaded.
    let bytes = module_with_code(&[0x18, 0x00, 21, 0x05, 0x00]);

    let runtime = execute_x3bc(&bytes, 100_000).expect("the runtime interpreter runs it");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(21)
    );

    let mut vm = VM::from_bytes(&bytes).expect("the std VM loads it");
    let result = vm.call_function(0, &[]).expect("the std VM runs it");
    assert_eq!(result.value, Some(x3_vm::Value::I64(21)));
}

/// Both engines implement slot storage, and they must name the same slot.
///
/// `crates/x3-vm` journaled `evm_sstore`/`evm_sload` while the runtime interpreter refused both, so
/// a contract could carry a slot between calls off chain and could not carry one on chain at all —
/// which also meant the kernel's storage-write channel had no producer on the canonical path. Both
/// engines implement the pair now, over the same key derivation and the same tagged payload, and
/// this test is the thing that makes "the same slot" a checked claim: the two journals are compared
/// key byte for key byte and value byte for value byte, because a contract whose state depends on
/// which engine ran it is not a contract.
#[test]
fn slot_storage_is_implemented_by_both_engines_and_names_the_same_slot() {
    // slot = r1 (0), value = r2 (5); store; load back into r3; return r3.
    let bytes = module_with_code(&[
        0x18, 0x01, 0x00, // LoadImm r1, 0
        0x18, 0x02, 0x05, // LoadImm r2, 5
        0xB4, 0x01, 0x02, // EvmSstore slot=r1 val=r2
        0xB3, 0x03, 0x01, // EvmSload dst=r3 slot=r1
        0x05, 0x03, // Ret r3
    ]);

    let mut vm = VM::from_bytes(&bytes).expect("the std VM loads it");
    let result = vm
        .call_function(0, &[])
        .expect("the std VM implements slot storage");
    assert_eq!(
        result.value,
        Some(x3_vm::Value::I64(5)),
        "the std VM must read back what it stored"
    );
    let journal = vm.drain_storage_journal();
    assert_eq!(journal.len(), 1, "the std VM journals the write");

    let runtime = execute_x3bc(&bytes, 100_000).expect("the runtime interpreter implements it too");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(5),
        "and must read back what it stored"
    );
    assert_eq!(
        runtime.storage_writes.len(),
        1,
        "the on-chain interpreter journals the write the kernel is handed"
    );

    assert_eq!(
        runtime.storage_writes[0].key.as_bytes(),
        &journal[0].key,
        "the two engines must name the same 32-byte slot"
    );
    assert_eq!(
        runtime.storage_writes[0].new_value, journal[0].new_value,
        "and must persist the same bytes in it"
    );
    assert_eq!(
        runtime.storage_writes[0].old_value, journal[0].old_value,
        "including the value the write replaced"
    );
}

/// Both engines must read the chain's seeded slot, and agree on what it makes the old value.
///
/// Seeding is the half of slot storage that a chain needs and a single execution cannot show: the
/// slot a *previous* comit wrote has to be visible to the next one, on both engines, with the same
/// `old_value` in the journal — because `old_value` is what describes the state transition the
/// receipt claims. The control below runs the same program with no seed, so neither engine can pass
/// this by answering a constant.
#[test]
fn both_engines_read_the_seeded_slot_and_report_it_as_the_old_value() {
    // slot = r1 (0), store 9 over it, load it back into r3, return r3.
    let bytes = module_with_code(&[
        0x18, 0x01, 0x00, // LoadImm r1, 0
        0x18, 0x02, 0x09, // LoadImm r2, 9
        0xB4, 0x01, 0x02, // EvmSstore slot=r1 val=r2
        0xB3, 0x03, 0x01, // EvmSload dst=r3 slot=r1
        0x05, 0x03, // Ret r3
    ]);

    // `"X3EVM_SL"` little-endian, then the slot number — the derivation both engines share.
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&0x5833_4556_4D5F_534Cu64.to_le_bytes());
    let mut payload_seven = [0u8; 32];
    payload_seven[0] = 1;
    payload_seven[1] = 8;
    payload_seven[2..10].copy_from_slice(&7i64.to_le_bytes());
    let mut payload_nine = [0u8; 32];
    payload_nine[0] = 1;
    payload_nine[1] = 8;
    payload_nine[2..10].copy_from_slice(&9i64.to_le_bytes());
    let seeds = [(key, payload_seven)];

    let module = BytecodeModule::from_bytes(&bytes).expect("the std VM loads it");
    let mut vm = VM::with_config_and_seeds(module, x3_vm::VMConfig::default(), &seeds)
        .expect("the std VM accepts the seed");
    let result = vm.call_function(0, &[]).expect("the std VM runs it");
    assert_eq!(
        result.value,
        Some(x3_vm::Value::I64(9)),
        "the std VM must read the chain's slot and write over it"
    );
    let journal = vm.drain_storage_journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(
        journal[0].old_value,
        Some(payload_seven),
        "the std VM must report the chain's value as the old value"
    );
    assert_eq!(journal[0].new_value, Some(payload_nine));

    let runtime = execute_x3bc_with_slots(&bytes, 100_000, &seeds)
        .expect("the runtime interpreter accepts the seed");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(9)
    );
    assert_eq!(runtime.storage_writes.len(), 1);
    assert_eq!(
        runtime.storage_writes[0].old_value,
        Some(payload_seven),
        "the on-chain interpreter must report the same old value"
    );
    assert_eq!(
        runtime.storage_writes[0].key.as_bytes(),
        &journal[0].key,
        "and name the same slot"
    );

    // The control: with no seed, the same program writes over nothing on both engines — so the
    // `Some(payload_seven)` above is the seed's doing and not a constant in either engine.
    let mut unseeded_vm = VM::with_config_and_seeds(
        BytecodeModule::from_bytes(&bytes).expect("loads"),
        x3_vm::VMConfig::default(),
        &[],
    )
    .expect("no seeds is valid");
    unseeded_vm.call_function(0, &[]).expect("runs");
    assert_eq!(
        unseeded_vm.drain_storage_journal()[0].old_value,
        None,
        "without the seed there is nothing to have replaced"
    );
    assert_eq!(
        execute_x3bc_with_slots(&bytes, 100_000, &[])
            .expect("runs")
            .storage_writes[0]
            .old_value,
        None
    );
}

// Divergences that have been closed, kept so the reasoning is not lost:
//
// * `0x26`/`0x27` (`inc`/`dec`) and `0x34` (`mod_f`) are refused by both engines (measured
//   2026-09-28: `InvalidOpcode` / `ForbiddenOnChain` in the runtime, `UnimplementedOpcode` in the
//   `std` VM). The numeric conversions `0x60`-`0x68` and `0x84` (`ctx_gas`) executed in the runtime
//   interpreter only; they are refused at intake by both now (TICKET-149) and are in the table
//   above.
// * `0x90`/`0x91` (`AtomicBegin`/`AtomicCommit`) execute in the `std` VM and, before this change,
//   were silently skipped by the runtime one; the runtime interpreter now implements the window
//   over globals, so both engines have a real implementation.
// * The runtime interpreter refuses the whole aggregate family and every cross-vm/gpu intrinsic
//   (`AGENTS.md` §5, §18), and `crates/x3-vm` refuses them too. `0xB3`/`0xB4`
//   (`EvmSload`/`EvmSstore`) used to be the one exception, in the wrong direction — the `std` VM
//   implemented them and the runtime refused them. Both implement them now, and the test above
//   compares the two journals rather than assuming they agree.
