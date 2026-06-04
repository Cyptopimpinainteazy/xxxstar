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

    assert_eq!(vm.state.registers[0], 42);
    assert!(vm.state.call_stack.is_empty());
    assert_eq!(vm.state.pc, 4);
}
