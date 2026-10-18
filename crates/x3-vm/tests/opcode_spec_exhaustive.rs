//! One opcode table, and nothing in the engine that disagrees with it.
//!
//! `x3bc_core::spec` is the canonical description of every X3BC opcode: operand widths, static gas,
//! verifier class, on-chain admissibility and the trap it can raise. Each check below fails the
//! moment an opcode exists without decoder support, verifier rules, interpreter execution, a gas
//! cost, trap behaviour or differential coverage.

use x3_backend::opcode::Opcode;
use x3bc_core::spec::{self, Execution, OperandKind};

fn all_opcodes() -> Vec<Opcode> {
    (0u8..=255).filter_map(Opcode::from_byte).collect()
}

#[test]
fn every_opcode_has_a_spec_entry_and_every_spec_entry_is_an_opcode() {
    let opcodes = all_opcodes();
    assert_eq!(opcodes.len(), spec::ALL.len());
    for opcode in &opcodes {
        let entry = spec::of(*opcode);
        assert_eq!(entry.opcode, *opcode);
        assert_eq!(spec::ALL.iter().filter(|e| e.opcode == *opcode).count(), 1);
    }
}

#[test]
fn the_verifier_decodes_every_opcode_with_the_spec_operand_widths() {
    for opcode in all_opcodes() {
        let entry = spec::of(opcode);
        let width: usize = entry.operands.iter().map(OperandKind::width).sum();
        let mut code = vec![opcode as u8];
        code.extend(std::iter::repeat_n(0u8, width));
        // Variable-length operands carry a zero count, so the encoding is complete as written.
        let decoded = x3_vm::Verifier::decode_all_instructions(&code)
            .unwrap_or_else(|error| panic!("{opcode:?}: decoder refused {code:?}: {error:?}"));
        assert_eq!(decoded.len(), 1, "{opcode:?}");
        assert_eq!(
            decoded[0].size,
            1 + width,
            "{opcode:?}: decoder width disagrees with the spec"
        );
    }
}

#[test]
fn the_engine_charges_the_spec_gas_for_every_opcode() {
    for opcode in all_opcodes() {
        assert_eq!(
            x3_vm::verifier::opcode_gas_cost(opcode as u8),
            spec::of(opcode).gas,
            "{opcode:?}"
        );
        assert!(spec::of(opcode).gas > 0, "{opcode:?} must cost something");
    }
}

#[test]
fn on_chain_admissibility_matches_the_verifier() {
    for opcode in all_opcodes() {
        let entry = spec::of(opcode);
        assert_eq!(
            x3_vm::verifier::admitted_on_chain(opcode, &x3_vm::VerifyOptions::on_chain_trading()),
            entry.on_chain_with_trading(),
            "{opcode:?}"
        );
        assert_eq!(
            x3_vm::verifier::admitted_on_chain(opcode, &x3_vm::VerifyOptions::on_chain()),
            entry.on_chain_base(),
            "{opcode:?}"
        );
    }
}

#[test]
fn every_executable_opcode_is_executed_and_every_other_one_traps_explicitly() {
    for opcode in all_opcodes() {
        let entry = spec::of(opcode);
        assert_eq!(
            x3_vm::vm::engine_executes(opcode),
            entry.execution == Execution::Interpreted,
            "{opcode:?}"
        );
        assert!(
            !entry.traps.is_empty() || entry.execution == Execution::Interpreted,
            "{opcode:?}"
        );
    }
}

#[test]
fn every_on_chain_opcode_has_differential_coverage() {
    // Each opcode the chain admits names the differential test that holds the production engine to
    // a reference for it; the named test must exist in this repository.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for opcode in all_opcodes() {
        let entry = spec::of(opcode);
        if !entry.on_chain_with_trading() {
            continue;
        }
        let coverage = entry
            .differential
            .unwrap_or_else(|| panic!("{opcode:?} runs on chain without differential coverage"));
        let (file, test) = coverage.split_once("::").expect("file::test");
        let text = std::fs::read_to_string(root.join(file))
            .unwrap_or_else(|_| panic!("{opcode:?}: coverage file {file} missing"));
        assert!(
            text.contains(&format!("fn {test}(")),
            "{opcode:?}: {coverage} missing"
        );
    }
}

#[test]
fn the_dead_duplicate_tables_are_gone() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for dead in ["bytecode.rs", "gas.rs", "revert.rs"] {
        assert!(
            !src.join(dead).exists(),
            "{dead} duplicates the canonical tables"
        );
    }
}
