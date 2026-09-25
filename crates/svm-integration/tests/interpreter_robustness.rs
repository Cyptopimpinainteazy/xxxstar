//! The SVM interpreter must fail closed on malformed programs, and account for compute.
//!
//! `interp_execute_bpf` is the interpreter `pallet-x3-kernel` reaches on chain: the pallet's
//! `WasmSvmAdapter` calls it with the account/instruction bytes a transaction carries, and
//! `WasmSvmAdapter::validate` calls `interp_validate_program` on the same bytes before that.
//! Both sides of that boundary are attacker input, so this file damages a real program and
//! checks what comes back.
//!
//! The interpreter's own ELF parser and memory helpers use `checked_add` / `checked_mul` and
//! `slice::get` throughout, so this is a case where the expectation is "nothing found" — the
//! tests exist to keep it that way, and one of them is a compute-accounting assertion that
//! does not hold yet.

use x3_svm_integration::{interp_execute_bpf, interp_validate_program, SvmConfig};

/// `MOV64 r0, `val`; `EXIT` — two instructions, the smallest program that returns.
fn program_returning(val: i32) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&[0xb7, 0x00, 0x00, 0x00]);
    p.extend_from_slice(&val.to_le_bytes());
    p.extend_from_slice(&[0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    p
}

/// `MOV64 r0, 0` then `JA -1` — and **no EXIT**, because an EXIT before the jump would
/// return before the loop was ever entered. This is the shape the interpreter's own
/// `test_compute_unit_enforcement` uses.
fn program_that_never_terminates() -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&[0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    p.extend_from_slice(&[0x05, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00]);
    p
}

fn config(limit: u64) -> SvmConfig {
    SvmConfig {
        compute_unit_limit: limit,
        ..SvmConfig::default()
    }
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_fixture_is_valid_and_reports_the_fuel_it_burned() {
    let program = program_returning(0);
    assert!(interp_validate_program(&program).is_ok());
    let result = interp_execute_bpf(&program, &[], &config(200_000)).expect("must execute");
    assert!(result.success, "r0 = 0 is Solana's success convention");
    assert!(
        result.compute_units_used <= 10,
        "two instructions should cost a handful of units, not {}",
        result.compute_units_used
    );
}

#[test]
fn a_limit_above_the_interpreter_cap_does_not_inflate_the_units_reported() {
    // `execute_bpf` caps its internal fuel at MAX_INSN_FUEL (1_000_000) but then reports
    // `config.compute_unit_limit - vm.fuel`, which counts the gap between the caller's limit
    // and the cap as if it had been burned. The pallet charges the number that comes back, so
    // this is a real over-charge for any caller whose limit is above the cap.
    let program = program_returning(0);
    for limit in [1_000_000u64, 2_000_000, 50_000_000] {
        let result = interp_execute_bpf(&program, &[], &config(limit)).expect("must execute");
        assert!(
            result.compute_units_used <= 10,
            "two instructions under a {limit} unit limit reported {} units used",
            result.compute_units_used
        );
    }
}

#[test]
fn the_compute_limit_is_what_stops_a_loop() {
    let program = program_that_never_terminates();
    for limit in [10u64, 1_000, 200_000] {
        let err = interp_execute_bpf(&program, &[], &config(limit))
            .expect_err("an unterminated loop must not succeed");
        assert_eq!(err, x3_svm_integration::SvmError::OutOfComputeUnits);
    }
}

#[test]
fn every_truncation_of_a_program_is_handled_without_panicking() {
    let program = program_returning(0);
    for len in 0..program.len() {
        let prefix = &program[..len];
        // Either answer is fine for a prefix; a panic is not.
        let _ = interp_validate_program(prefix);
        let _ = interp_execute_bpf(prefix, &[], &config(1_000));
    }
}

#[test]
fn no_single_byte_mutation_panics_the_validator_or_the_interpreter() {
    let program = program_returning(0);
    for offset in 0..program.len() {
        for value in [0x00u8, 0x01, 0x05, 0x7F, 0x80, 0x95, 0xFF] {
            let mut damaged = program.clone();
            if damaged[offset] == value {
                continue;
            }
            damaged[offset] = value;

            let validated = interp_validate_program(&damaged).is_ok();
            // A 1_000 unit limit keeps a mutated jump from running away, and the interpreter
            // must return either way.
            let executed = interp_execute_bpf(&damaged, &[], &config(1_000));
            if let Ok(result) = executed {
                assert!(
                    result.compute_units_used <= 1_000,
                    "{}: a mutated program reported {} units under a 1_000 unit limit",
                    hex(&damaged),
                    result.compute_units_used
                );
            }
            // Only that both paths returned is asserted; the verdicts themselves are not a
            // promise this interpreter makes about arbitrary bytes.
            let _ = validated;
        }
    }
}

/// A minimal but real ELF64 with one `.text` section holding `program`.
fn elf_wrapping(program: &[u8]) -> Vec<u8> {
    const EHDR: usize = 64;
    const SHDR: usize = 64;
    const SHNUM: usize = 3;
    let shoff = EHDR;
    let strtab_off = shoff + SHNUM * SHDR;
    let strtab: &[u8] = b"\0.text\0";
    let text_off = strtab_off + strtab.len();

    let mut elf = vec![0u8; text_off];
    elf[0..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2; // ELF64
    elf[16..18].copy_from_slice(&2u16.to_le_bytes()); // e_type = ET_EXEC
    elf[18..20].copy_from_slice(&0xf7u16.to_le_bytes()); // e_machine = BPF
    elf[40..48].copy_from_slice(&(shoff as u64).to_le_bytes());
    elf[58..60].copy_from_slice(&(SHDR as u16).to_le_bytes());
    elf[60..62].copy_from_slice(&(SHNUM as u16).to_le_bytes());
    elf[62..64].copy_from_slice(&1u16.to_le_bytes()); // e_shstrndx = 1

    // section 1: .shstrtab
    let s1 = shoff + SHDR;
    // sh_name = 0 (the null name). Giving this section the same name index as `.text` was
    // the first version of this fixture, and the parser — which returns the first section
    // whose name is `.text` — then read the 9-byte string table as the program.
    elf[s1..s1 + 4].copy_from_slice(&0u32.to_le_bytes());
    elf[s1 + 4..s1 + 8].copy_from_slice(&3u32.to_le_bytes()); // sh_type = STRTAB
    elf[s1 + 24..s1 + 32].copy_from_slice(&(strtab_off as u64).to_le_bytes());
    elf[s1 + 32..s1 + 40].copy_from_slice(&(strtab.len() as u64).to_le_bytes());

    // section 2: .text
    let s2 = shoff + 2 * SHDR;
    elf[s2..s2 + 4].copy_from_slice(&1u32.to_le_bytes()); // sh_name -> "\0.text\0"[1]
    elf[s2 + 4..s2 + 8].copy_from_slice(&1u32.to_le_bytes()); // sh_type = PROGBITS
    elf[s2 + 24..s2 + 32].copy_from_slice(&(text_off as u64).to_le_bytes());
    elf[s2 + 32..s2 + 40].copy_from_slice(&(program.len() as u64).to_le_bytes());

    // The string table belongs at `strtab_off`, not wherever the vector happens to end:
    // the section header above points at it by offset, and the parser reads it from there.
    elf[strtab_off..strtab_off + strtab.len()].copy_from_slice(strtab);
    elf.extend_from_slice(program);
    assert_eq!(elf.len(), text_off + program.len());
    elf
}

#[test]
fn an_elf_wrapped_program_runs_and_damaged_elf_headers_do_not_panic() {
    let program = program_returning(0);
    let elf = elf_wrapping(&program);

    assert!(
        interp_validate_program(&elf).is_ok(),
        "the fixture ELF must be accepted, or the sweep below proves nothing"
    );
    let result = interp_execute_bpf(&elf, &[], &config(200_000)).expect("must execute");
    assert!(result.success);

    // Damage every byte of the ELF header and of the section headers: `elf_find_text` reads
    // offsets and counts out of exactly these, and it must answer rather than panic.
    for offset in 0..(64 + 3 * 64) {
        for value in [0x00u8, 0xFF, 0x7F] {
            let mut damaged = elf.clone();
            if damaged[offset] == value {
                continue;
            }
            damaged[offset] = value;
            let _ = interp_validate_program(&damaged);
            let _ = interp_execute_bpf(&damaged, &[], &config(1_000));
        }
    }
}

#[test]
fn a_text_section_that_is_not_whole_instructions_is_refused_by_both_paths() {
    // Validation is the pallet's precondition for execution, so the two must not disagree
    // about a payload execution can never run: an ELF whose `.text` is empty, or is not a
    // whole number of 8-byte instructions, is refused by `validate_program` as well.
    for program in [vec![0u8; 0], vec![0u8; 9], vec![0x95, 0x00, 0x00]] {
        let elf = elf_wrapping(&program);
        assert!(
            interp_validate_program(&elf).is_err(),
            "a {}-byte .text section validated",
            program.len()
        );
        assert!(
            interp_execute_bpf(&elf, &[], &config(1_000)).is_err(),
            "a {}-byte .text section executed",
            program.len()
        );
    }
}
