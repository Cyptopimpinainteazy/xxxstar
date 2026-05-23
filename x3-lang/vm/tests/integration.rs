#[test]
fn run_simple_add() {
    use x3_lang_common::Span;
    use x3_lang_vm::executor::execute;
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
