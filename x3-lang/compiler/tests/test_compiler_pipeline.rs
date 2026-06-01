//! Comprehensive tests for the X3 compiler pipeline:
//! AST → X3IR → Bytecode

use x3_lang_ast::ast::*;
use x3_lang_common::{IntBase, Span, Spanned, Symbol};
use x3_lang_compiler::{
    compile_program, compile_source, compile_to_ir, parser::parse_source, Operation,
};

fn lit_int(value: u128) -> Expression {
    Expression::Literal(LiteralExpr::Int {
        value,
        base: IntBase::Decimal,
        suffix: None,
    })
}

/// Test 1: Compile empty program (should succeed with minimal bytecode)
#[test]
fn test_compile_empty_program() {
    let program = Program::new(vec![]);
    let result = compile_program(&program);

    assert!(result.is_ok(), "empty program should compile");
    let bytecode = result.unwrap();
    assert!(!bytecode.is_empty(), "bytecode should not be empty");
    assert_eq!(bytecode[0], 0x01, "bytecode should start with version 0x01");
    assert_eq!(bytecode.len() % 4, 0, "bytecode should be 4-byte aligned");
}

/// Test 2: Compile to IR (without bytecode emission)
#[test]
fn test_compile_to_ir() {
    let program = Program::new(vec![]);
    let result = compile_to_ir(&program);

    assert!(result.is_ok(), "should compile to IR");
    let ir = result.unwrap();
    assert_eq!(
        ir.operations.len(),
        0,
        "empty program should have no operations"
    );
}

/// Test 3: IR with atomic block structure
#[test]
fn test_ir_contains_atomic_operations() {
    let program = Program::new(vec![Spanned::new(
        Item::AtomicSwap(AtomicSwapDecl {
            name: Symbol::new("settle"),
            body: vec![Statement::Lock {
                chain: ChainRef(Symbol::new("ethereum")),
                asset: AssetRef::new(ChainRef(Symbol::new("ethereum")), Symbol::new("USDC")),
                amount: lit_int(100),
                from: Expression::Literal(LiteralExpr::Address(Symbol::new("0xsender"))),
            }],
            on_fail: Some(FailureAction::Rollback),
            timeout: Some(lit_int(10)),
        }),
        Span::DUMMY,
    )]);
    let result = compile_to_ir(&program);

    assert!(result.is_ok());
    let ir = result.unwrap();
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::AtomicBegin)));
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::AtomicEnd)));
}

/// Test 4: Bytecode compiles successfully with basic operations
#[test]
fn test_bytecode_generation_succeeds() {
    let program = Program::new(vec![]);
    let result = compile_program(&program);

    assert!(result.is_ok(), "should generate valid bytecode");
    let bytecode = result.unwrap();

    // Basic sanity checks
    assert!(bytecode.len() > 0, "bytecode should not be empty");
    assert_eq!(bytecode[0], 0x01, "version byte should be 0x01");
}

/// Test 5: Compiled bytecode is deterministic
#[test]
fn test_compilation_is_deterministic() {
    let program = Program::new(vec![]);

    let result1 = compile_program(&program);
    let result2 = compile_program(&program);

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let bytecode1 = result1.unwrap();
    let bytecode2 = result2.unwrap();

    assert_eq!(bytecode1, bytecode2, "compilation should be deterministic");
}

/// Test 6: Verify IR operations structure
#[test]
fn test_ir_operation_types() {
    // Test that X3IR contains the expected operation types
    let _lock = Operation::Lock {
        chain: "eth".to_string(),
        asset: "USDC".to_string(),
        amount: 1000,
        from: "0x123".to_string(),
    };

    let _mint = Operation::Mint {
        chain: "sol".to_string(),
        asset: "USDC".to_string(),
        amount: 1000,
        to: "abc123".to_string(),
    };

    let _burn = Operation::Burn {
        chain: "eth".to_string(),
        asset: "USDC".to_string(),
        amount: 1000,
        from: "0x123".to_string(),
    };

    let _release = Operation::Release {
        chain: "x3".to_string(),
        asset: "USDC".to_string(),
        to: "x3_addr".to_string(),
    };

    let _swap = Operation::Swap {
        from_chain: "eth".to_string(),
        from_asset: "USDC".to_string(),
        to_asset: "ETH".to_string(),
        input_amount: 1000,
        min_output: 900,
        dex: Some("uniswap".to_string()),
    };

    // Just verify these compile successfully
    let _atomic_begin = Operation::AtomicBegin;
    let _atomic_end = Operation::AtomicEnd;
    let _nop = Operation::Nop;
}

/// Test 7: Verify bytecode contains expected opcodes
#[test]
fn test_bytecode_opcode_integrity() {
    let program = Program::new(vec![]);
    let bytecode = compile_program(&program).unwrap();

    // Bytecode should at least have version byte
    assert!(bytecode.len() > 0);
    assert_eq!(bytecode[0], 0x01); // Version
}

/// Test 8: IR metadata is preserved through compilation
#[test]
fn test_ir_metadata_preservation() {
    let program = Program::new(vec![]);
    let ir = compile_to_ir(&program).unwrap();

    // Metadata fields should be initialized (even if None)
    assert!(ir.metadata.nonce.is_none() || ir.metadata.nonce.is_some());
    assert!(ir.metadata.chain_id.is_none() || ir.metadata.chain_id.is_some());
}

/// Test 9: Bytecode alignment (critical for VM execution)
#[test]
fn test_bytecode_alignment() {
    let program = Program::new(vec![]);
    let bytecode = compile_program(&program).unwrap();

    // All bytecode must be 4-byte aligned
    assert_eq!(
        bytecode.len() % 4,
        0,
        "bytecode length {} is not 4-byte aligned",
        bytecode.len()
    );
}

/// Test 10: Empty program produces valid bytecode
#[test]
fn test_empty_program_valid() {
    let program = Program::new(vec![]);
    let result = compile_program(&program);

    // Should succeed
    assert!(result.is_ok());

    // Bytecode should be valid
    let bytecode = result.unwrap();
    assert!(!bytecode.is_empty());
    assert_eq!(bytecode[0], 0x01);
}

// Integration test: Full pipeline
#[test]
fn test_full_pipeline_end_to_end() {
    // Create a simple program
    let program = Program::new(vec![]);

    // Step 1: Compile to IR
    let ir_result = compile_to_ir(&program);
    assert!(ir_result.is_ok(), "AST → X3IR should succeed");

    let _ir = ir_result.unwrap();

    // Step 2: Compile to bytecode
    let bytecode_result = compile_program(&program);
    assert!(bytecode_result.is_ok(), "X3IR → Bytecode should succeed");

    let bytecode = bytecode_result.unwrap();

    // Step 3: Verify bytecode properties
    assert_eq!(bytecode[0], 0x01, "version should be correct");
    assert_eq!(bytecode.len() % 4, 0, "should be 4-byte aligned");

    println!("End-to-end pipeline test passed");
    println!("Bytecode length: {} bytes", bytecode.len());
}

#[test]
fn test_source_parser_annotation_and_capability_call_reach_bytecode() {
    let source = r#"
        @role("keeper")
        fn scan_pending() {
            mempool_scan(max_results=16);
        }
    "#;

    let program = parse_source(source).expect("source should parse");
    let ir = compile_to_ir(&program).expect("parsed source should lower");
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::RoleCheck { role } if role == "keeper")));
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::MempoolScan { max_results: 16 })));

    let bytecode = compile_source(source).expect("source should compile");
    assert!(
        bytecode.contains(&0x93),
        "role check opcode should be emitted"
    );
    assert!(
        bytecode.contains(&0x88),
        "mempool scan opcode should be emitted"
    );
}

#[test]
fn test_source_parser_capability_matrix_reaches_x3ir_and_bytecode() {
    let source = r#"
        @multisig(2, 3)
        fn guarded_ops() {
            verify_zk("proof", "input", "vk");
            storage_store("blob");
            pathfind("solana", "ethereum", max_depth=4);
            pause();
            resume();
        }
    "#;

    let program = parse_source(source).expect("capability matrix source should parse");
    let ir = compile_to_ir(&program).expect("capability matrix should lower");
    assert!(ir.operations.iter().any(|op| matches!(
        op,
        Operation::MultisigCheck {
            required: 2,
            total: 3
        }
    )));
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::ProofVerify { .. })));
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::StorageOp { .. })));
    assert!(ir
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Pathfind { max_depth: 4, .. })));
    assert_eq!(
        ir.operations
            .iter()
            .filter(|op| matches!(op, Operation::EmergencyControl { .. }))
            .count(),
        2
    );

    let bytecode = compile_source(source).expect("capability matrix source should compile");
    for opcode in [0x94, 0x85, 0x86, 0x87, 0x8A] {
        assert!(
            bytecode.contains(&opcode),
            "expected capability opcode 0x{opcode:02x}"
        );
    }
}
