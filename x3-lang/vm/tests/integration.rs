#[test]
fn run_simple_add() {
    use x3_lang_vm::VMConfig;
    use x3_lang_vm::VM;

    // Program: PUSH_IMM 1, PUSH_IMM 2, ADD_RRR (use regs), HALT
    let code = vec![
        0x20u8, 0x00, 0x01, 0x00, // PUSH_IMM 1
        0x20u8, 0x00, 0x02, 0x00, // PUSH_IMM 2
        0x01u8, 0x00, 0x00, 0x00, // ADD_RRR - noop for stack values (we'll use add logic)
        0xFFu8, 0x00, 0x00, 0x00, // HALT
    ];

    let mut vm = VM::new(code, VMConfig::default(), 1000);
    let res = vm.execute();
    assert!(res.is_ok());
}

#[test]
fn nested_calls_use_real_call_stack_through_verified_execute() {
    use x3_lang_vm::VMConfig;
    use x3_lang_vm::VM;

    let code = vec![
        0x32u8, 0x00, 0x08, 0x00, // CALL 8
        0xFFu8, 0x00, 0x00, 0x00, // HALT
        0x32u8, 0x00, 0x10, 0x00, // CALL 16
        0x33u8, 0x00, 0x00, 0x00, // RET
        0x20u8, 0x00, 0x2A, 0x00, // PUSH_IMM 42
        0x33u8, 0x00, 0x00, 0x00, // RET
    ];

    let mut vm = VM::new(code, VMConfig::default(), 1000);
    vm.execute().expect("verified nested calls should execute");

    assert_eq!(vm.state.registers[0], 0x20);
    assert!(vm.state.call_stack.is_empty());
    assert_eq!(vm.state.pc, 4);
}

#[test]
fn compiler_asset_op_payloads_execute_with_real_fields() {
    use x3_lang_common::AssetOpPayload;
    use x3_lang_compiler::emitter::emit_x3ir;
    use x3_lang_compiler::{Operation, X3IR};
    use x3_lang_vm::VMConfig;
    use x3_lang_vm::VM;

    let mut ir = X3IR::new();
    ir.push(Operation::Lock {
        chain: "ethereum".into(),
        asset: "USDC".into(),
        amount: 100,
        from: "0xsender".into(),
    });
    ir.push(Operation::Mint {
        chain: "solana".into(),
        asset: "USDC".into(),
        amount: 100,
        to: "receiver".into(),
    });
    ir.push(Operation::Burn {
        chain: "ethereum".into(),
        asset: "USDC".into(),
        amount: 50,
        from: "0xsender".into(),
    });
    ir.push(Operation::Release {
        chain: "ethereum".into(),
        asset: "USDC".into(),
        to: "0xsender".into(),
    });
    ir.push(Operation::Swap {
        from_chain: "ethereum".into(),
        from_asset: "USDC".into(),
        to_asset: "ETH".into(),
        input_amount: 1000,
        min_output: 777,
        dex: Some("uniswap".into()),
    });

    let bytecode = emit_x3ir(&ir).expect("asset payload bytecode should emit");
    let mut vm = VM::new(bytecode, VMConfig::default(), 100_000);
    vm.execute()
        .expect("verified asset payload bytecode should execute");

    assert_eq!(
        vm.state.asset_ops,
        vec![
            AssetOpPayload::Lock {
                chain: "ethereum".into(),
                asset: "USDC".into(),
                amount: 100,
                from: "0xsender".into(),
            },
            AssetOpPayload::Mint {
                chain: "solana".into(),
                asset: "USDC".into(),
                amount: 100,
                to: "receiver".into(),
            },
            AssetOpPayload::Burn {
                chain: "ethereum".into(),
                asset: "USDC".into(),
                amount: 50,
                from: "0xsender".into(),
            },
            AssetOpPayload::Release {
                chain: "ethereum".into(),
                asset: "USDC".into(),
                to: "0xsender".into(),
            },
            AssetOpPayload::Swap {
                from_chain: "ethereum".into(),
                from_asset: "USDC".into(),
                to_asset: "ETH".into(),
                input_amount: 1000,
                min_output: 777,
                dex: Some("uniswap".into()),
            },
        ]
    );
    assert_eq!(vm.state.registers[0], 1000);
    assert_eq!(vm.state.registers[1], 777);
}

#[test]
fn production_bridge_adapter_verifies_and_persists_structured_receipt() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use x3_lang_compiler::emitter::emit_x3ir;
    use x3_lang_compiler::{Operation, X3IR};
    use x3_lang_vm::bridge::{
        BridgeError, BridgeTransferRequest, ProductionBridgeAdapter, ProductionBridgeBackend,
        SettlementReceipt,
    };
    use x3_lang_vm::{VMConfig, VM};

    #[derive(Clone, Default)]
    struct RecordingBackend {
        receipts: Rc<RefCell<Vec<SettlementReceipt>>>,
        finality_checks: Rc<RefCell<usize>>,
        proof_checks: Rc<RefCell<usize>>,
    }

    impl ProductionBridgeBackend for RecordingBackend {
        fn verify_source_finality(
            &self,
            request: &BridgeTransferRequest,
        ) -> Result<Vec<u8>, BridgeError> {
            assert_eq!(request.from_chain, "ethereum");
            assert_eq!(
                request.source_finality_proof,
                b"eth-header-and-receipt-trie-proof"
            );
            *self.finality_checks.borrow_mut() += 1;
            Ok(b"finality:ethereum:confirmed".to_vec())
        }

        fn verify_transfer_proof(
            &self,
            request: &BridgeTransferRequest,
            finality_proof: &[u8],
        ) -> Result<Vec<u8>, BridgeError> {
            assert_eq!(request.amount, 100);
            assert_eq!(request.transfer_proof, b"erc20-transfer-log-proof");
            assert_eq!(finality_proof, b"finality:ethereum:confirmed");
            *self.proof_checks.borrow_mut() += 1;
            Ok(b"proof:lock:100:USDC".to_vec())
        }

        fn persist_receipt(&self, receipt: &SettlementReceipt) -> Result<(), BridgeError> {
            self.receipts.borrow_mut().push(receipt.clone());
            Ok(())
        }
    }

    let backend = RecordingBackend::default();
    let receipts = backend.receipts.clone();
    let finality_checks = backend.finality_checks.clone();
    let proof_checks = backend.proof_checks.clone();

    let mut ir = X3IR::new();
    ir.push(Operation::Bridge {
        via: "X3".into(),
        from_chain: "ethereum".into(),
        from_asset: "USDC".into(),
        to_chain: "solana".into(),
        to_asset: "USDC".into(),
        amount: 100,
        receiver: "receiver".into(),
        source_finality_proof: b"eth-header-and-receipt-trie-proof".to_vec(),
        transfer_proof: b"erc20-transfer-log-proof".to_vec(),
    });

    let bytecode = emit_x3ir(&ir).expect("bridge payload should emit");
    let mut vm = VM::new(bytecode, VMConfig::default(), 100_000);
    vm.bridge = Box::new(ProductionBridgeAdapter::new(backend));

    vm.execute()
        .expect("production bridge adapter should verify and execute");

    assert_eq!(*finality_checks.borrow(), 1);
    assert_eq!(*proof_checks.borrow(), 1);
    assert_eq!(receipts.borrow().len(), 1);
    assert_eq!(receipts.borrow()[0].amount, 100);
    assert_eq!(
        receipts.borrow()[0].finality_proof,
        b"finality:ethereum:confirmed"
    );
    assert_eq!(receipts.borrow()[0].transfer_proof, b"proof:lock:100:USDC");
    assert_eq!(
        receipts.borrow()[0].source_finality_proof_input,
        b"eth-header-and-receipt-trie-proof"
    );
    assert_eq!(
        receipts.borrow()[0].transfer_proof_input,
        b"erc20-transfer-log-proof"
    );
    assert!(String::from_utf8_lossy(&vm.state.bridge_receipts[0])
        .starts_with("x3-settlement-receipt:v1:"));
}
