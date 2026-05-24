//! Bytecode function-name validation tests.
//!
//! These tests exercise `codegen::validate_function_name`, which enforces
//! that all function names emitted to bytecode are legal identifiers:
//!   - non-empty
//!   - first char: ASCII letter or `_`
//!   - remaining chars: ASCII alphanumeric or `_`

use x3_compiler::codegen::validate_function_name;

// ── valid names ───────────────────────────────────────────────────────────────

#[test]
fn valid_simple_name() {
    assert!(validate_function_name("main"));
}

#[test]
fn valid_name_with_underscores() {
    assert!(validate_function_name("transfer_funds"));
}

#[test]
fn valid_name_starting_with_underscore() {
    assert!(validate_function_name("_internal"));
}

#[test]
fn valid_name_with_digits_after_letter() {
    assert!(validate_function_name("fn123"));
}

#[test]
fn valid_single_char() {
    assert!(validate_function_name("x"));
}

// ── invalid names ─────────────────────────────────────────────────────────────

#[test]
fn invalid_empty_name() {
    assert!(!validate_function_name(""));
}

#[test]
fn invalid_starts_with_digit() {
    assert!(!validate_function_name("123bad"));
}

#[test]
fn invalid_contains_space() {
    assert!(!validate_function_name("has space"));
}

#[test]
fn invalid_contains_dash() {
    assert!(!validate_function_name("has-dash"));
}

#[test]
fn invalid_contains_bang() {
    assert!(!validate_function_name("has!bang"));
}

#[test]
fn invalid_contains_dot() {
    assert!(!validate_function_name("module.function"));
}
