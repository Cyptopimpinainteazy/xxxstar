//! Cross-decoder parity for the X3BC constant pool.
//!
//! There is one bytecode format and two readers of it: `x3-backend::bc_format` (the `std`
//! writer/reader used by the compiler and the `std` VM) and `x3-integration::mini_x3`, which
//! re-implements the reader for `no_std` — and that `no_std` reader is the one
//! `pallets/x3-kernel` executes on chain. `x3-common::bytecode` pins the shared envelope
//! (magic, version, checksum), so the two cannot disagree about the header, but the body
//! encoding is still written twice.
//!
//! These tests drive both readers over the *same* bytes and compare what each of them sees.
//! They are the regression that keeps a constant from meaning one thing at compile time and
//! another thing at execution time.

#![cfg(feature = "std")]

use x3_backend::{BytecodeModule, FunctionEntry};

/// `LoadConst r<dest>, <idx>` is `0x10, dest:u8, idx:u32` and `Ret r<dest>` is `0x05, dest:u8`.
fn code_load_const_then_return(idx: u32) -> Vec<u8> {
    let mut code = vec![0x10, 0x00];
    code.extend_from_slice(&idx.to_le_bytes());
    code.extend_from_slice(&[0x05, 0x00]);
    code
}

/// Build a real module with the real writer: constant pool `[String(value)]`, one `main` that
/// loads that constant into r0 and returns it. Every byte of the envelope, including the
/// checksum, comes from `to_bytes`.
fn module_with_string_const(value: &str) -> Vec<u8> {
    let mut module = BytecodeModule::new();
    let idx = module
        .const_pool
        .add_string(value.to_string())
        .expect("the writer must accept a string constant")
        .0;
    assert_eq!(idx, 0, "the string should be pool entry 0 in this fixture");
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 16,
        max_stack: 16,
        return_type_tag: 4, // "other": the return value is not a scalar
    });
    module.code = code_load_const_then_return(idx);
    module.to_bytes()
}

#[test]
fn a_string_constant_means_the_same_thing_to_both_readers() {
    let bytes = module_with_string_const("X3/USD");

    // The std reader: what the compiler intended.
    let std_module = BytecodeModule::from_bytes(&bytes)
        .expect("the writer's own output must round-trip through the std reader");
    assert_eq!(
        std_module
            .get_const(x3_backend::opcode::ConstIdx(0))
            .and_then(|c| c.as_string()),
        Some("X3/USD"),
        "std reader must recover the string constant"
    );

    // The no-std runtime reader: what actually executes on chain.
    let result =
        x3_x3_integration::mini_x3::execute_x3bc(&bytes, 10_000).expect("the module must execute");
    assert_eq!(
        result.return_val,
        x3_x3_integration::mini_x3::MiniValue::Bytes(b"X3/USD".to_vec()),
        "the no-std runtime must see the same string the compiler emitted, not an empty value"
    );
}

#[test]
fn an_unknown_constant_tag_fails_closed() {
    // Hand-build a valid envelope around a constant pool whose single entry has a tag no
    // reader knows (9). The checksum is recomputed over the body, so this is a well-formed
    // envelope carrying a malformed body — exactly what a corrupted or hostile module is.
    let mut b = Vec::new();
    b.extend_from_slice(x3_common::bytecode::MAGIC);
    b.extend_from_slice(&x3_common::bytecode::VERSION.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes()); // flags
    b.extend_from_slice(&0u32.to_le_bytes()); // checksum (patched below)
    b.extend_from_slice(&x3_common::bytecode::MIN_SUPPORTED_VERSION.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes()); // features
                                              // Const pool: one entry with an impossible tag.
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(9u8);
    // Function table: one `main` at entry 0.
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&0u16.to_le_bytes()); // name_len
    b.extend_from_slice(&0u32.to_le_bytes()); // entry_point
    b.push(0u8); // param_count
    b.extend_from_slice(&16u16.to_le_bytes()); // local_count
    b.extend_from_slice(&16u16.to_le_bytes()); // max_stack
    b.push(1u8); // return_type_tag
                 // Global table (empty)
    b.extend_from_slice(&0u32.to_le_bytes());
    // Code: RetVoid
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(0x06);
    b.push(0u8); // no debug info
    b.push(0u8); // no metadata

    let checksum = x3_common::bytecode::checksum(&b[x3_common::bytecode::HEADER_LEN..]);
    let at = x3_common::bytecode::CHECKSUM_OFFSET;
    b[at..at + 4].copy_from_slice(&checksum.to_le_bytes());

    let err = x3_x3_integration::mini_x3::execute_x3bc(&b, 10_000)
        .expect_err("an unknown constant tag must be rejected");
    assert_eq!(
        err,
        x3_x3_integration::mini_x3::X3Error::InvalidConstTag(9),
        "a malformed constant tag is not an EOF; the error must name what was wrong"
    );
}
