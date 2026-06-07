#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/ok_case.rs");
    t.compile_fail("tests/ui/fail_invalid_format.rs");
}
