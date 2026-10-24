#!/usr/bin/env python3
"""Self-test for `panic_unwrap_scan.py`'s test-code classification.

The scanner's whole job is to tell "a panic in production code" from "a panic in the
tests of production code", and it got that wrong for an entire class of attribute: it
matched the literal `#[cfg(test)]`, so every `#[cfg(all(test, feature = "std"))]`
module — which is how this repository writes its runtime and node test modules — was
counted as production code. On 2026-09-26 the release gate's panic ratchet read 570
against a baseline of 516 and failed; the same scan with the attribute parsed properly
reads 509, and `runtime/src/lib.rs` goes from 26 findings to none.

That is the regression this file pins. Run it with:

    python3 scripts/audit/panic_unwrap_self_test.py

Exit 0 = every case behaves; non-zero = the scanner's classification moved.
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import panic_unwrap_scan as scan  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

FAILURES: list[str] = []


def check(description: str, condition: bool) -> None:
    print(f"  {'ok  ' if condition else 'FAIL'} {description}")
    if not condition:
        FAILURES.append(description)


def attribute_cases() -> None:
    print("attribute classification")
    cases = [
        ("#[cfg(test)]", True),
        ('#[cfg(all(test, feature = "std"))]', True),
        ('#[cfg(all(test, feature = "std", feature = "frontier"))]', True),
        ('#[cfg(any(test, feature = "dev-mock"))]', True),
        # The opposite of a test item: this exists only in a production build.
        ('#[cfg(all(not(test), feature = "std"))]', False),
        # A `test` inside a feature name is not the test configuration.
        ('#[cfg(feature = "testnet")]', False),
        ("let x = 1;", False),
    ]
    for text, expected in cases:
        check(f"{text} -> {expected}", scan.is_cfg_test_item(text) is expected)


def range_cases() -> None:
    print("excluded line ranges")
    lines = [
        "fn production() {",  # 1
        "    let x = Option::<u8>::None.unwrap();",  # 2 — production, must count
        "}",  # 3
        '#[cfg(all(test, feature = "std"))]',  # 4
        "mod whitespace_tests {",  # 5
        "    #[test]",  # 6
        "    fn inner() {",  # 7
        "        let a = 1u8.checked_add(1).unwrap();",  # 8 — test
        "        let b = 1u8.checked_add(1).unwrap();",  # 9 — nested, must not count
        "    }",  # 10
        "}",  # 11
    ]
    ranges = scan.test_line_ranges(lines)
    covered = {n for start, end in ranges for n in range(start, end + 1)}
    for line in (4, 5, 6, 7, 8, 9, 10, 11):
        check(f"line {line} is inside a test range", line in covered)
    for line in (1, 2, 3):
        check(f"line {line} is production", line not in covered)


def module_path_cases() -> None:
    print("out-of-line module paths")
    decl = os.path.join("p", "src", "vm_revert.rs")
    cases = [
        ('#[cfg(test)]\n#[path = "tests_x.rs"]\nmod tests_x;', decl, ["p/src/tests_x.rs"]),
        ("#[cfg(test)]\nmod tests;", decl, ["p/src/vm_revert/tests.rs", "p/src/vm_revert/tests"]),
        ("#[cfg(test)]\nmod mock;", os.path.join("p", "src", "lib.rs"), ["p/src/mock.rs", "p/src/mock"]),
        ("#[cfg(all(test, feature = \"std\"))]\nmod tests;", os.path.join("p", "src", "mod.rs"),
         ["p/src/tests.rs", "p/src/tests"]),
        ("#[cfg(not(test))]\nmod real;", decl, []),
    ]
    for text, declaring, expected in cases:
        got = scan.out_of_line_test_modules(declaring, text)
        check(f"{text.splitlines()[-1]} in {os.path.basename(declaring)} -> {expected}", got == expected)


def tree_cases() -> None:
    print("this tree")
    roots = ["crates", "pallets", "runtime", "node"]
    test_only = scan.test_only_files(roots)
    for path in [
        "pallets/x3-settlement-engine/src/tests.rs",
        "crates/x3-state-snapshot/src/tests.rs",
        "runtime/src/tests.rs",
        # declared from vm_revert.rs, not the crate root, through #[path]
        "pallets/x3-atomic-kernel/src/tests_vm_revert_hardening.rs",
    ]:
        check(f"{path} is a test-only file", path in test_only)

    findings = scan.scan_file(os.path.join(ROOT, "runtime/src/lib.rs"), test_only)
    check(
        f"runtime/src/lib.rs has no production panics (found {len(findings)})",
        findings == [],
    )


def raw_string_cases() -> None:
    """A raw string's braces must not end a test module's range.

    Measured 2026-09-27: a `#[cfg(test)]` fixture written as a multi-line `r#"{ … }"#` JSON blob put
    two braces into the depth counter, so the enclosing test module appeared to close early and the
    test's own `panic!` was reported as a production finding — a false positive in a ratchet whose
    whole purpose is to be trustworthy. Raw strings were simply not handled.
    """
    print("raw strings")
    lines = [
        "pub fn production() {",  # 1
        "    let v = maybe().unwrap();",  # 2 — production, must count
        "}",  # 3
        "#[cfg(test)]",  # 4
        "mod tests {",  # 5
        "    #[test]",  # 6
        "    fn parses() {",  # 7
        '        let json = r#"{',  # 8 — brace inside a raw string
        '            "entries": [{ "a": 1 }]',  # 9 — more braces inside it
        '        }"#;',  # 10 — closes the raw string
        '        let parsed = serde_json::from_str(json).expect("loads");',  # 11 — test
        '        assert!(parsed.is_object(), "{}", "no");',  # 12 — test
        "    }",  # 13
        "}",  # 14
        "",  # 15
        "pub fn after_the_tests() -> u8 {",  # 16
        "    Option::<u8>::None.unwrap()",  # 17 — production, must count again
        "}",  # 18
    ]
    ranges = scan.test_line_ranges(lines)
    covered = {n for start, end in ranges for n in range(start, end + 1)}
    for line in range(4, 15):
        check(f"line {line} (raw-string fixture) is inside the test range", line in covered)
    for line in (1, 2, 3, 16, 17, 18):
        check(f"line {line} is production", line not in covered)

    # And the scanner's own classification of those braces, end to end.
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "sample.rs")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
        findings = scan.scan_file(path, set())
    found_lines = {line for line, _ in findings}
    check("the raw-string fixture is not a finding", found_lines == {2, 17})


def main() -> int:
    attribute_cases()
    range_cases()
    raw_string_cases()
    module_path_cases()
    tree_cases()
    if FAILURES:
        print(f"\npanic_unwrap_self_test: FAIL — {len(FAILURES)} case(s) moved")
        return 1
    print("\npanic_unwrap_self_test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
