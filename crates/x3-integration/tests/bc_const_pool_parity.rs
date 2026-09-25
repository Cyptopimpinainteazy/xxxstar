//! Cross-decoder parity for the X3BC body: the constant pool, and the sections after it.
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

/// Two functions, one call with two arguments: the *function table* has to frame identically for
/// both readers, or the callee's entry point and parameter count are read from the wrong bytes.
///
/// `main` is function 0 because that is the ABI both readers implement — the entry is the function
/// at index 0, which is what the compiler arranges (TICKET-130) and what `execute_x3bc` runs. The
/// callee is index 1 with a deliberately longer name: the name is length-prefixed and skipped, so a
/// reader that got its width wrong would land mid-name and read the rest of the table as garbage.
#[test]
fn a_call_with_arguments_means_the_same_thing_to_both_readers() {
    let mut module = BytecodeModule::new();

    let mut code = Vec::new();
    // main (entry, function 0): two immediates, call function 1 with both, return its result
    code.extend_from_slice(&[0x18, 0, 20]); // LoadImm r0, 20
    code.extend_from_slice(&[0x18, 1, 22]); // LoadImm r1, 22
    code.push(0x04); // Call dst r2, func 1, argc 2, args [r0, r1]
    code.push(2);
    code.extend_from_slice(&1u32.to_le_bytes());
    code.extend_from_slice(&2u16.to_le_bytes());
    code.extend_from_slice(&[0, 1]);
    code.extend_from_slice(&[0x05, 2]); // Ret r2
    let callee_entry = code.len() as u32;
    // callee(r0, r1) -> r0 + r1
    code.extend_from_slice(&[0x20, 2, 0, 1]); // AddI r2, r0, r1
    code.extend_from_slice(&[0x05, 2]); // Ret r2

    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 3,
        max_stack: 3,
        return_type_tag: 1,
    });
    module.functions.push(FunctionEntry {
        name: "a_callee_with_a_long_name".to_string(),
        entry_point: callee_entry,
        param_count: 2,
        local_count: 1,
        max_stack: 3,
        return_type_tag: 1,
    });
    module.code = code;
    let bytes = module.to_bytes();

    let std_module = BytecodeModule::from_bytes(&bytes).expect("the std reader must accept it");
    assert_eq!(
        std_module
            .functions
            .iter()
            .map(|f| (f.name.clone(), f.entry_point, f.param_count))
            .collect::<Vec<_>>(),
        vec![
            ("main".to_string(), 0, 0),
            ("a_callee_with_a_long_name".to_string(), callee_entry, 2)
        ],
        "the std reader must recover the table the writer wrote"
    );

    let runtime = x3_x3_integration::mini_x3::execute_x3bc(&bytes, 10_000)
        .expect("the runtime's reader must execute the same module");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(42),
        "and the call must reach the callee the table names, with its two arguments"
    );
}

/// A global is stored and loaded back: the *global table* frames identically for both readers
/// (length-prefixed name, type tag, mutability byte, initial constant index).
#[test]
fn a_global_stored_and_loaded_means_the_same_thing_to_both_readers() {
    let mut module = BytecodeModule::new();
    let seven = module
        .const_pool
        .add_integer(7)
        .expect("the writer must accept an integer constant")
        .0;
    module.globals.push(x3_backend::GlobalEntry {
        name: "global_counter_with_a_long_name".to_string(),
        type_tag: 1,
        mutable: true,
        init_const: x3_backend::opcode::ConstIdx(seven),
    });

    let mut code = Vec::new();
    code.extend_from_slice(&[0x18, 0, 7]); // LoadImm r0, 7
    code.push(0x13); // StoreGlobal idx 0, r0
    code.extend_from_slice(&0u32.to_le_bytes());
    code.push(0);
    code.push(0x12); // LoadGlobal dst r1, idx 0
    code.push(1);
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&[0x05, 1]); // Ret r1
    module.code = code;
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 2,
        max_stack: 2,
        return_type_tag: 1,
    });
    let bytes = module.to_bytes();

    let std_module = BytecodeModule::from_bytes(&bytes).expect("the std reader must accept it");
    assert_eq!(std_module.globals.len(), 1);
    assert_eq!(
        std_module.globals[0].name,
        "global_counter_with_a_long_name"
    );
    assert!(std_module.globals[0].mutable);

    let runtime = x3_x3_integration::mini_x3::execute_x3bc(&bytes, 10_000)
        .expect("the runtime's reader must execute the same module");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(7),
        "the global the module stored must be the value it loads back"
    );
}

/// Debug info and metadata are written after the code section. The runtime's reader stops at the
/// code — it has no use for either — so the property that matters is that their presence changes
/// nothing it executes, and that the checksum still covers them for the reader that does parse them.
#[test]
fn trailing_debug_info_and_metadata_do_not_change_what_runs() {
    let mut module = BytecodeModule::new();
    module.code = vec![0x18, 0, 9, 0x05, 0]; // LoadImm r0, 9; Ret r0
    module.functions.push(FunctionEntry {
        name: "main".to_string(),
        entry_point: 0,
        param_count: 0,
        local_count: 1,
        max_stack: 1,
        return_type_tag: 1,
    });
    module.debug_info = Some(x3_backend::DebugInfo {
        source_map: vec![x3_backend::SourceMapEntry {
            code_offset: 0,
            source_line: 12,
            source_column: 3,
        }],
        symbol_names: [(0u32, "r0".to_string())].into_iter().collect(),
    });
    module.metadata = Some(x3_backend::ModuleMetadata {
        compiler: "x3-test".to_string(),
        compiler_version: "0.1.0".to_string(),
        compiled_at: 1_700_000_000,
        source_file: Some("parity.x3".to_string()),
        source_hash: None,
        opt_level: 2,
        annotations: [("k".to_string(), "v".to_string())].into_iter().collect(),
    });
    let bytes = module.to_bytes();

    let std_module = BytecodeModule::from_bytes(&bytes)
        .expect("the std reader must parse the trailing sections it wrote");
    assert!(
        std_module.debug_info.is_some(),
        "debug info must survive the round trip"
    );
    assert!(
        std_module.metadata.is_some(),
        "metadata must survive the round trip"
    );

    let runtime = x3_x3_integration::mini_x3::execute_x3bc(&bytes, 10_000)
        .expect("the runtime's reader must execute the module with trailing sections present");
    assert_eq!(
        runtime.return_val,
        x3_x3_integration::mini_x3::MiniValue::I64(9),
        "the trailing sections must not shift what the runtime executes"
    );

    // A byte flipped inside the debug section is part of the body: the checksum covers it, and
    // both readers must refuse rather than execute a module whose evidence was edited.
    let mut edited = bytes.clone();
    let last_debug_byte = edited.len() - 2;
    edited[last_debug_byte] ^= 0xFF;
    assert!(
        BytecodeModule::from_bytes(&edited).is_err(),
        "the std reader must refuse a module whose trailing section was edited"
    );
    assert!(
        x3_x3_integration::mini_x3::execute_x3bc(&edited, 10_000).is_err(),
        "and so must the runtime's reader"
    );
}

/// The envelope's version fields are enforced by *both* readers, which is what makes them evidence
/// rather than a comment: a module written by a newer compiler, or one that says it needs a newer
/// loader, is refused by the std reader and by the runtime's reader alike.
#[test]
fn a_module_from_a_newer_compiler_or_needing_a_newer_loader_is_refused_by_both_readers() {
    fn envelope(version: u32, min_version: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(x3_common::bytecode::MAGIC);
        b.extend_from_slice(&version.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // flags
        b.extend_from_slice(&0u32.to_le_bytes()); // checksum (patched below)
        b.extend_from_slice(&min_version.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // features
        b.extend_from_slice(&0u32.to_le_bytes()); // const pool: empty
        b.extend_from_slice(&1u32.to_le_bytes()); // one function
        b.extend_from_slice(&0u16.to_le_bytes()); // name
        b.extend_from_slice(&0u32.to_le_bytes()); // entry
        b.push(0u8); // params
        b.extend_from_slice(&1u16.to_le_bytes()); // locals
        b.extend_from_slice(&1u16.to_le_bytes()); // max_stack
        b.push(1u8); // return type tag
        b.extend_from_slice(&0u32.to_le_bytes()); // globals: empty
        b.extend_from_slice(&5u32.to_le_bytes()); // code: LoadImm r0, 1; Ret r0 (5 bytes)
        b.extend_from_slice(&[0x18, 0, 1, 0x05, 0]);
        b.push(0u8); // no debug
        b.push(0u8); // no metadata
        let checksum = x3_common::bytecode::checksum(&b[x3_common::bytecode::HEADER_LEN..]);
        b[x3_common::bytecode::CHECKSUM_OFFSET..x3_common::bytecode::CHECKSUM_OFFSET + 4]
            .copy_from_slice(&checksum.to_le_bytes());
        b
    }

    let current = x3_common::bytecode::VERSION;
    // The field layout is major << 16 | minor << 8 | patch, so `+ 1` is a patch and `+ 0x100` a minor.
    let newer_minor = current + 0x100;

    assert!(
        BytecodeModule::from_bytes(&envelope(
            current,
            x3_common::bytecode::MIN_SUPPORTED_VERSION
        ))
        .is_ok(),
        "the fixture must be a module both readers accept, or the refusals below prove nothing"
    );

    // A patch bump is compatible by the format's own semantic versioning: both readers must accept
    // it. They used to disagree here — `x3-backend` accepted `1.0.1` while the shared rule the
    // no-std reader uses refused it (TICKET-137).
    let patch_bump = envelope(current + 1, x3_common::bytecode::MIN_SUPPORTED_VERSION);
    assert!(
        BytecodeModule::from_bytes(&patch_bump).is_ok(),
        "a patch bump is readable, per the format's semantic versioning"
    );
    assert!(
        x3_x3_integration::mini_x3::validate_x3bc(&patch_bump).is_ok(),
        "and the runtime's reader must agree — it used to refuse this one"
    );

    for (label, bytes) in [
        (
            "a module from a newer compiler",
            envelope(newer_minor, x3_common::bytecode::MIN_SUPPORTED_VERSION),
        ),
        (
            "a module needing a newer loader",
            envelope(current, newer_minor),
        ),
    ] {
        assert!(
            BytecodeModule::from_bytes(&bytes).is_err(),
            "the std reader must refuse {label}"
        );
        assert!(
            x3_x3_integration::mini_x3::validate_x3bc(&bytes).is_err(),
            "and the runtime's reader must refuse {label} too"
        );
    }
}
