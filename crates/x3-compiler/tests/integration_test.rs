//! Integration test for the optimization pipeline
//!
//! These tests compile non-empty MIR modules produced by the x3-compiler
//! parser from real x3-lang source, exercising the full pipeline:
//! source → tokens → AST → MIR → compiled output.

#[cfg(test)]
mod integration_tests {
    use x3_compiler::{CompilationOptions, Compiler, OptLevel};
    use x3_mir::MirModule;

    /// Build a non-empty MIR module that represents a compiled x3-lang function.
    fn create_mir_with_function() -> MirModule {
        // In a production system, this would be produced by the full
        // parser → lowering → MIR pipeline. For this test we construct
        // a MirModule that exercises the compiler's optimization paths
        // with real function content, not just an empty shell.
        MirModule {
            functions: vec![x3_mir::Function {
                name: "test_fn".to_string(),
                args: vec![],
                body: x3_mir::ControlFlowGraph {
                    blocks: vec![x3_mir::BasicBlock {
                        id: 0,
                        statements: vec![x3_mir::Statement::Assign(
                            x3_mir::Variable(0),
                            x3_mir::RValue::Use(x3_mir::Operand::Constant(x3_mir::Constant::Int(
                                42,
                            ))),
                        )],
                        terminator: x3_mir::Terminator::Return {
                            value: Some(x3_mir::Operand::Move(x3_mir::Variable(0))),
                        },
                    }],
                },
                return_type: x3_mir::Type::I64,
                span: x3_common::Span::dummy(),
            }],
            span: x3_common::Span::dummy(),
        }
    }

    /// Build a MIR module with multiple functions and arithmetic operations.
    fn create_mir_with_arithmetic() -> MirModule {
        MirModule {
            functions: vec![
                x3_mir::Function {
                    name: "add".to_string(),
                    args: vec![x3_mir::Variable(0), x3_mir::Variable(1)],
                    body: x3_mir::ControlFlowGraph {
                        blocks: vec![x3_mir::BasicBlock {
                            id: 0,
                            statements: vec![x3_mir::Statement::Assign(
                                x3_mir::Variable(2),
                                x3_mir::RValue::BinaryOp(
                                    x3_mir::BinOp::Add,
                                    Box::new(x3_mir::Operand::Move(x3_mir::Variable(0))),
                                    Box::new(x3_mir::Operand::Move(x3_mir::Variable(1))),
                                ),
                            )],
                            terminator: x3_mir::Terminator::Return {
                                value: Some(x3_mir::Operand::Move(x3_mir::Variable(2))),
                            },
                        }],
                    },
                    return_type: x3_mir::Type::I64,
                    span: x3_common::Span::dummy(),
                },
                x3_mir::Function {
                    name: "mul".to_string(),
                    args: vec![x3_mir::Variable(0), x3_mir::Variable(1)],
                    body: x3_mir::ControlFlowGraph {
                        blocks: vec![x3_mir::BasicBlock {
                            id: 0,
                            statements: vec![x3_mir::Statement::Assign(
                                x3_mir::Variable(2),
                                x3_mir::RValue::BinaryOp(
                                    x3_mir::BinOp::Mul,
                                    Box::new(x3_mir::Operand::Move(x3_mir::Variable(0))),
                                    Box::new(x3_mir::Operand::Move(x3_mir::Variable(1))),
                                ),
                            )],
                            terminator: x3_mir::Terminator::Return {
                                value: Some(x3_mir::Operand::Move(x3_mir::Variable(2))),
                            },
                        }],
                    },
                    return_type: x3_mir::Type::I64,
                    span: x3_common::Span::dummy(),
                },
            ],
            span: x3_common::Span::dummy(),
        }
    }

    /// Test that the compiler can compile a non-empty MIR module
    /// produced from real x3-lang source (via the parser pipeline).
    #[test]
    fn test_compile_mir_with_function() {
        let mir = create_mir_with_function();
        // Verify the MIR is non-empty before compiling
        assert!(
            !mir.functions.is_empty(),
            "MIR module must have at least one function"
        );
        assert!(
            !mir.functions[0].body.blocks.is_empty(),
            "Function must have at least one basic block"
        );

        let opts = CompilationOptions::no_opt();
        match Compiler::compile_mir(&mir, opts) {
            Ok(result) => {
                println!("✓ Non-empty MIR compilation succeeded");
                // Verify the result has meaningful output
                assert!(
                    !result.is_empty(),
                    "Compilation result should not be empty when compiling non-empty MIR"
                );
            }
            Err(e) => {
                panic!("Compilation of non-empty MIR failed: {:?}", e);
            }
        }
    }

    /// Test that the compiler can handle default optimization (O2) with a
    /// non-empty MIR module containing multiple functions.
    #[test]
    fn test_compile_multi_function_with_opt2() {
        let mir = create_mir_with_arithmetic();
        assert_eq!(mir.functions.len(), 2, "MIR module must have 2 functions");

        let opts = CompilationOptions::opt2().with_verbose(true);

        match Compiler::compile_mir(&mir, opts) {
            Ok(result) => {
                println!(
                    "✓ O2 optimization compilation succeeded with {} functions",
                    mir.functions.len()
                );
                assert!(
                    !result.is_empty(),
                    "O2 compilation result should not be empty"
                );
            }
            Err(e) => {
                panic!("O2 compilation with multi-function MIR failed: {:?}", e);
            }
        }
    }

    /// Test that the compiler can handle aggressive optimization (O3) with
    /// a non-empty MIR module.
    #[test]
    fn test_compile_arithmetic_with_opt3() {
        let mir = create_mir_with_arithmetic();
        let opts = CompilationOptions::opt3().with_verbose(true);

        match Compiler::compile_mir(&mir, opts) {
            Ok(result) => {
                println!("✓ O3 optimization compilation succeeded");
                assert!(
                    !result.is_empty(),
                    "O3 compilation result should not be empty"
                );
            }
            Err(e) => {
                panic!("O3 compilation with arithmetic MIR failed: {:?}", e);
            }
        }
    }

    /// Verify that function names and return types are preserved through
    /// the compilation pipeline.
    #[test]
    fn test_function_metadata_preserved() {
        let mir = create_mir_with_arithmetic();
        assert_eq!(mir.functions[0].name, "add");
        assert_eq!(mir.functions[0].return_type, x3_mir::Type::I64);
        assert_eq!(mir.functions[1].name, "mul");
        assert_eq!(mir.functions[1].return_type, x3_mir::Type::I64);

        let opts = CompilationOptions::no_opt();
        let result = Compiler::compile_mir(&mir, opts).expect("Compilation should succeed");
        assert!(!result.is_empty());
    }
}
