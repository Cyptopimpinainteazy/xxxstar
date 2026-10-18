//! The two X3BC readers must fail closed on malformed input, and never panic.
//!
//! There is one bytecode format and two readers of it: `x3-backend::bc_format` (the `std`
//! reader the compiler and the `std` VM use) and `x3-integration::mini_x3`, which
//! re-implements the reader for `no_std` — and *that* is the one `pallets/x3-kernel`
//! executes on chain, through `WasmX3Adapter`. A panic in either is a chain-level denial
//! of service on input an attacker chooses, and a divergence between them is worse: a
//! module the compiler accepted and the chain then reads differently.
//!
//! `bc_const_pool_parity.rs` compares what the two see on *well-formed* inputs.
//! This file is the other half: it drives both readers over deliberately damaged ones.
//!
//! Every mutation that keeps the header honest recomputes the envelope checksum first, so
//! the reader underneath is exercised instead of the checksum gate. That distinction is
//! not academic — TICKET-108 was a reader that never checked the checksum, hidden by a
//! fixture whose checksum was already zero.

#![cfg(feature = "std")]

use x3_backend::{BytecodeModule, FunctionEntry};
use x3_common::bytecode as bc;
use x3_x3_integration::mini_x3;

/// Header offsets, in the order the writer emits them and both readers check them.
const VERSION_OFFSET: usize = 4;
const MIN_VERSION_OFFSET: usize = 16;

/// Recompute the envelope checksum over the body so a damaged body still reaches the reader.
fn reseal(bytes: &mut [u8]) {
    if bytes.len() < bc::HEADER_LEN {
        return;
    }
    let sum = bc::checksum(&bytes[bc::HEADER_LEN..]);
    bytes[bc::CHECKSUM_OFFSET..bc::CHECKSUM_OFFSET + 4].copy_from_slice(&sum.to_le_bytes());
}

/// Build a real module with the real writer: a constant pool entry and a `main` that loads it
/// and returns. Every byte of the envelope, including the checksum, comes from `to_bytes`.
fn module_with_integer_const() -> Vec<u8> {
    let mut module = BytecodeModule::new();
    let idx = module
        .const_pool
        .add_integer(7)
        .expect("the writer must accept an integer constant")
        .0;
    let mut code = vec![0x10, 0x00]; // LoadConst r0
    code.extend_from_slice(&idx.to_le_bytes());
    code.extend_from_slice(&[0x05, 0x00]); // Ret r0
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

fn module_with_string_const() -> Vec<u8> {
    let mut module = BytecodeModule::new();
    let idx = module
        .const_pool
        .add_string("a string long enough to occupy several body bytes".to_string())
        .expect("the writer must accept a string constant")
        .0;
    let mut code = vec![0x10, 0x00];
    code.extend_from_slice(&idx.to_le_bytes());
    code.extend_from_slice(&[0x05, 0x00]);
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 4,
    });
    module.code = code;
    module.to_bytes()
}

fn corpus() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("integer const", module_with_integer_const()),
        ("string const", module_with_string_const()),
    ]
}

/// What each reader decided, without unwrapping anything: a panic inside is the failure mode
/// this whole file exists to catch, so the readers are called for their verdict only.
///
/// `mini` is the runtime's *format* reader. Its on-chain validator (`validate_x3bc`) also checks
/// the code, so it refuses modules both readers can read; it is called here too, for the panic
/// check and for `the_on_chain_validator_never_admits_what_the_std_reader_refuses`.
fn verdicts(bytes: &[u8]) -> (bool, bool, bool) {
    let mini = mini_x3::read_x3bc(bytes).is_ok();
    let _ = mini_x3::validate_x3bc(bytes);
    let backend = BytecodeModule::from_bytes(bytes).is_ok();
    let verifier =
        x3_vm::Verifier::verify_module_bytes(bytes, &x3_vm::VerifyOptions::on_chain()).is_ok();
    (mini, backend, verifier)
}

#[test]
fn the_on_chain_validator_never_admits_what_the_std_reader_refuses() {
    let mut admitted_unreadable = Vec::new();
    for (name, bytes) in corpus() {
        for offset in 0..bytes.len() {
            for value in [0x00u8, 0xFF, 0x7F, 0x80, 0x01] {
                let mut damaged = bytes.clone();
                if damaged[offset] == value {
                    continue;
                }
                damaged[offset] = value;
                reseal(&mut damaged);
                if mini_x3::validate_x3bc(&damaged).is_ok()
                    && BytecodeModule::from_bytes(&damaged).is_err()
                {
                    admitted_unreadable.push(format!("{name}: byte[{offset}] = {value:#04x}"));
                }
            }
        }
    }
    assert!(
        admitted_unreadable.is_empty(),
        "the runtime would run modules the std toolchain cannot read: {admitted_unreadable:?}"
    );
}

#[test]
fn the_corpus_is_valid_to_begin_with() {
    // Without this, every assertion below would pass on a corpus that is garbage already.
    for (name, bytes) in corpus() {
        let (mini, backend, verifier) = verdicts(&bytes);
        assert!(
            mini && backend && verifier,
            "{name}: the fixture must be valid before it is damaged (mini={mini} backend={backend} verifier={verifier})"
        );
    }
}

#[test]
fn every_truncation_is_refused_by_every_reader() {
    for (name, bytes) in corpus() {
        for len in 0..bytes.len() {
            let truncated = &bytes[..len];
            let (mini, backend, verifier) = verdicts(truncated);
            assert!(
                !mini,
                "{name}: a {len}-byte prefix was accepted by the on-chain reader"
            );
            assert!(
                !backend,
                "{name}: a {len}-byte prefix was accepted by the std reader"
            );
            assert!(!verifier, "{name}: a {len}-byte prefix passed verification");
        }
    }
}

#[test]
fn the_header_fields_are_gates_in_every_reader() {
    for (name, bytes) in corpus() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("a wrong magic", {
                let mut b = bytes.clone();
                b[0] = b'Y';
                b
            }),
            ("a future major version", {
                let mut b = bytes.clone();
                b[VERSION_OFFSET..VERSION_OFFSET + 4].copy_from_slice(&(2u32 << 16).to_le_bytes());
                b
            }),
            ("a newer minor version", {
                let mut b = bytes.clone();
                b[VERSION_OFFSET..VERSION_OFFSET + 4]
                    .copy_from_slice(&(bc::READER_VERSION + 0x100).to_le_bytes());
                b
            }),
            ("a loader demand from the future", {
                let mut b = bytes.clone();
                b[MIN_VERSION_OFFSET..MIN_VERSION_OFFSET + 4]
                    .copy_from_slice(&(2u32 << 16).to_le_bytes());
                b
            }),
            ("a body that does not match its checksum", {
                let mut b = bytes.clone();
                let last = b.len() - 1;
                b[last] ^= 0xFF;
                b
            }),
        ];

        for (label, damaged) in cases {
            let (mini, backend, verifier) = verdicts(&damaged);
            assert!(!mini, "{name}: {label} was accepted by the on-chain reader");
            assert!(!backend, "{name}: {label} was accepted by the std reader");
            assert!(!verifier, "{name}: {label} passed verification");
        }
    }
}

#[test]
fn no_re_sealed_body_mutation_panics_either_reader() {
    // The sweep that is meant to find something: every byte of the body, four replacement
    // values each, with the checksum recomputed so the readers actually run.
    let replacements: [u8; 4] = [0x00, 0xFF, 0x7F, 0x80];

    for (name, bytes) in corpus() {
        let body_start = bc::HEADER_LEN;
        for offset in body_start..bytes.len() {
            for value in replacements {
                let mut damaged = bytes.clone();
                if damaged[offset] == value {
                    continue;
                }
                damaged[offset] = value;
                reseal(&mut damaged);

                // A panic in here is the failure; the verdict itself is only recorded.
                let _ = verdicts(&damaged);
            }
        }

        // And the same sweep with every header field set to each replacement value, so the
        // version/flag handling is exercised rather than just the body.
        for offset in 0..bc::HEADER_LEN {
            for value in replacements {
                let mut damaged = bytes.clone();
                if damaged[offset] == value {
                    continue;
                }
                damaged[offset] = value;
                reseal(&mut damaged);
                let _ = verdicts(&damaged);
            }
        }
        let _ = name;
    }
}

#[test]
fn the_two_readers_agree_about_re_sealed_body_mutations() {
    // The property the parity file asserts for well-formed modules, extended to damaged ones:
    // whatever the bytes are, the reader that runs on chain and the reader that runs off it
    // must not disagree about whether the module is readable. A disagreement means a module
    // the toolchain accepted could be executed with different semantics on chain.
    let mut divergences: Vec<String> = Vec::new();

    for (name, bytes) in corpus() {
        for offset in bc::HEADER_LEN..bytes.len() {
            for value in [0x00u8, 0xFF, 0x7F, 0x80] {
                let mut damaged = bytes.clone();
                if damaged[offset] == value {
                    continue;
                }
                damaged[offset] = value;
                reseal(&mut damaged);

                let (mini, backend, _verifier) = verdicts(&damaged);
                if mini != backend {
                    divergences.push(format!(
                        "{name}: body[{offset}] = {value:#04x} -> on-chain reader says {}, std reader says {}",
                        if mini { "readable" } else { "refused" },
                        if backend { "readable" } else { "refused" },
                    ));
                }
            }
        }
    }

    assert!(
        divergences.is_empty(),
        "the two readers disagree about {} damaged module(s); the first few are:\n{}",
        divergences.len(),
        divergences
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Found by the `x3bc_engines` fuzz target: x3-backend unpacked the version fields eight bits at a
/// time, so a version of `0x4301_0000` (a high byte the format does not have) read as 1.0.0 there
/// and was refused by the runtime's reader, and a `min_version` with a high byte set passed the
/// std loader's gate. Both readers now apply `x3-common`'s gate to the full `u32`.
#[test]
fn a_version_field_with_a_high_byte_is_refused_by_both_readers() {
    let crash: &[u8] = &[
        0x58, 0x33, 0x42, 0x43, 0x00, 0x00, 0x01, 0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x5b, 0x00, 0x00, 0x42, 0x29, 0x36, 0xff, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let (mini, backend, _) = verdicts(crash);
    assert_eq!(
        mini, backend,
        "the two readers must agree on the fuzzer's input"
    );

    for (name, offset) in [
        ("version", VERSION_OFFSET),
        ("min_version", MIN_VERSION_OFFSET),
    ] {
        let mut bytes = module_with_integer_const();
        bytes[offset + 3] = 0x43; // the high byte of the little-endian u32
        reseal(&mut bytes);
        let (mini, backend, _) = verdicts(&bytes);
        assert!(
            !mini && !backend,
            "{name} with a high byte: mini={mini} backend={backend}"
        );
    }
}
