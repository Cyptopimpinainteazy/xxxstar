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


def tree_cases() -> None:
    print("this tree")
    roots = ["crates", "pallets", "runtime", "node"]
    test_only = scan.test_only_files(roots)
    for path in [
        "pallets/x3-settlement-engine/src/tests.rs",
        "crates/x3-state-snapshot/src/tests.rs",
        "runtime/src/tests.rs",
    ]:
        check(f"{path} is a test-only file", path in test_only)

    findings = scan.scan_file(os.path.join(ROOT, "runtime/src/lib.rs"), test_only)
    check(
        f"runtime/src/lib.rs has no production panics (found {len(findings)})",
        findings == [],
    )


def main() -> int:
    attribute_cases()
    range_cases()
    tree_cases()
    if FAILURES:
        print(f"\npanic_unwrap_self_test: FAIL — {len(FAILURES)} case(s) moved")
        return 1
    print("\npanic_unwrap_self_test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
