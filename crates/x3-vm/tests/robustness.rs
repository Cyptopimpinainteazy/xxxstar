//! Property tests: the verifier and the interpreter against arbitrary input.
//!
//! The chain executes whatever bytes a submitter sends, so the verifier and the VM are an attack
//! surface: a panic in either is a crashed block producer, and a nondeterministic result is a fork.
//! These properties hold for *every* input, not just the fixtures the unit tests pick:
//!
//! 1. reading or verifying arbitrary bytes never panics;
//! 2. an admitted module, executed under a gas limit, never panics, never charges more than the
//!    limit, and runs no more instructions than it was charged gas (the 1-gas floor);
//! 3. executing an admitted module twice gives the same result, gas, instruction count, slot
//!    journal and events: the engine is deterministic.
//!
//! Generated programs are built from real opcodes with operand bytes of the right width, so a large
//! share of them pass verification and reach the interpreter; arbitrary byte soup alone would be
//! refused at the first byte and test only the decoder.

use proptest::prelude::*;
use x3_backend::bc_format::{BytecodeModule, FunctionEntry};
use x3_backend::opcode::Opcode;
use x3_vm::{VMConfig, Verifier, VerifyOptions, VM};

const GAS: u64 = 20_000;

/// Every defined opcode, with the operand width the verifier decodes for it (zero operands).
///
/// Kept to opcodes a block can actually run: each must pass on-chain verification on its own (with
/// zero operands), which drops debug, GPU, float and unimplemented opcodes and unbalanced atomic
/// markers. Those are covered by the refusal tests; here the point is to reach the interpreter.
fn opcode_table() -> Vec<(u8, usize)> {
    (0u8..=255)
        .filter(|b| Opcode::from_byte(*b).is_some())
        .filter(|b| {
            let mut alone = vec![*b];
            alone.extend_from_slice(&[0u8; 16]);
            let sized = Verifier::decode_all_instructions(&alone)
                .ok()
                .and_then(|i| i.first().map(|i| i.size));
            sized.is_some_and(|size| {
                let mut code = alone[..size].to_vec();
                code.push(Opcode::RetVoid as u8);
                Verifier::verify_module_bytes(&module_bytes(code), &VerifyOptions::on_chain())
                    .is_ok()
            })
        })
        .filter_map(|b| {
            let mut probe = vec![b];
            probe.extend_from_slice(&[0u8; 16]);
            Verifier::decode_all_instructions(&probe)
                .ok()
                .or_else(|| {
                    // A variable-length instruction with zero counts decodes alone.
                    let mut one = vec![b];
                    one.extend_from_slice(&[0u8; 8]);
                    Verifier::decode_all_instructions(&one).ok()
                })
                .and_then(|instrs| instrs.first().map(|i| (b, i.size)))
        })
        .collect()
}

/// One instruction: a runnable opcode and random operand bytes of its fixed width. Operand bytes
/// are small, so registers land inside the 16-register frame and constant and jump indices stay
/// near the start of their tables, and still vary enough to reach type mismatches, bad registers,
/// out-of-range indices and the like.
fn instruction() -> impl Strategy<Value = Vec<u8>> {
    let table = opcode_table();
    (0..table.len(), proptest::collection::vec(0u8..20, 16)).prop_map(move |(i, operands)| {
        let (opcode, size) = table[i];
        let mut bytes = vec![opcode];
        bytes.extend_from_slice(&operands[..size - 1]);
        bytes
    })
}

fn program() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(instruction(), 1..40).prop_map(|instrs| {
        let mut code: Vec<u8> = instrs.into_iter().flatten().collect();
        code.push(Opcode::RetVoid as u8);
        code
    })
}

fn module_bytes(code: Vec<u8>) -> Vec<u8> {
    let mut module = BytecodeModule::new();
    module.const_pool.add_integer(7).expect("const");
    module.const_pool.add_bytes(b"evt".to_vec()).expect("const");
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 1,
    });
    module.code = code;
    module.to_bytes()
}

/// Everything an execution can be observed by, for comparing two runs.
type Observation = (String, u64, u64, String, String);

fn execute(bytes: &[u8]) -> Observation {
    let module = BytecodeModule::from_bytes(bytes).expect("admitted modules parse");
    let mut vm = VM::with_config(
        module,
        VMConfig {
            gas_limit: GAS,
            ..VMConfig::default()
        },
    );
    let result = vm.call_function(0, &[]);
    let gas = vm.gas_used();
    let instructions = vm.instruction_count();
    (
        format!("{result:?}"),
        gas,
        instructions,
        format!("{:?}", vm.drain_storage_journal()),
        format!("{:?}", vm.drain_events()),
    )
}

/// 2_000 cases per property by default; `PROPTEST_CASES` raises it for a long hunt
/// (`PROPTEST_CASES=200000 cargo test -p x3-vm --test robustness --release`).
fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: cases(), .. ProptestConfig::default() })]

    #[test]
    fn arbitrary_bytes_never_panic_the_reader_or_the_verifier(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = BytecodeModule::from_bytes(&bytes);
        let _ = Verifier::verify_module_bytes(&bytes, &VerifyOptions::on_chain());
        let _ = Verifier::verify_module_bytes(&bytes, &VerifyOptions::default());
        let _ = Verifier::decode_all_instructions(&bytes);
    }

    #[test]
    fn arbitrary_code_in_a_valid_envelope_never_panics_the_verifier(code in proptest::collection::vec(any::<u8>(), 1..256)) {
        let bytes = module_bytes(code);
        let _ = Verifier::verify_module_bytes(&bytes, &VerifyOptions::on_chain());
    }

    #[test]
    fn an_admitted_program_runs_within_its_gas_and_never_panics(code in program()) {
        let bytes = module_bytes(code);
        if Verifier::verify_module_bytes(&bytes, &VerifyOptions::on_chain()).is_ok() {
            let (_, gas, instructions, _, _) = execute(&bytes);
            prop_assert!(gas <= GAS, "charged {gas} under a limit of {GAS}");
            prop_assert!(instructions <= gas, "{instructions} instructions for {gas} gas");
        }
    }

    #[test]
    fn an_admitted_program_is_deterministic(code in program()) {
        let bytes = module_bytes(code);
        if Verifier::verify_module_bytes(&bytes, &VerifyOptions::on_chain()).is_ok() {
            prop_assert_eq!(execute(&bytes), execute(&bytes));
        }
    }
}

/// The generator is not vacuous: a meaningful share of generated programs pass verification and
/// reach the interpreter, so the two execution properties above are about real executions.
#[test]
fn generated_programs_reach_the_interpreter() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = program();
    let admitted = (0..500)
        .filter(|_| {
            let code = strategy.new_tree(&mut runner).expect("generates").current();
            Verifier::verify_module_bytes(&module_bytes(code), &VerifyOptions::on_chain()).is_ok()
        })
        .count();
    assert!(
        admitted >= 100,
        "only {admitted} of 500 generated programs were admitted"
    );
}
