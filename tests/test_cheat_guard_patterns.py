#!/usr/bin/env python3
"""Pin the test-cheat guard's patterns: the true positives *and* the false ones.

`scripts/test_cheat_guard.py` is the gate that stops a weakened test from landing.
It has been wrong in the false-positive direction once: the Jasmine x-it rule was
unanchored, so it also matched `sys.exit(main())` and every Python script that
exits through `sys.exit` was reported as a skipped test. A guard that cries wolf is
a guard people learn to route around, so both directions are pinned here:

* a standalone x-it call is still a skipped test;
* `sys.exit(` / `os._exit(` are not.
"""

from __future__ import annotations

import importlib.util
import re
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
GUARD = REPO_ROOT / "scripts" / "test_cheat_guard.py"


def _load_guard():
    spec = importlib.util.spec_from_file_location("x3_test_cheat_guard", GUARD)
    assert spec is not None and spec.loader is not None, f"cannot load {GUARD}"
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def guard():
    assert GUARD.is_file(), f"test-cheat guard script is missing: {GUARD}"
    return _load_guard()


def _matches(patterns, line: str) -> bool:
    return any(re.search(rx, line) for rx in patterns)


# The guard scans this file too, so the weakened-test fixtures are assembled from
# fragments: a literal one in the source would be reported by the very rule the
# test exercises, and the honest fix for that is to not write the literal, not to
# carve out an exception to the scan.
XIT = "x" + "it("
SKIP_CALL = ".sk" + "ip("
IGNORE_ATTR = "#[" + "ignore]"
WEAK_ASSERT = "assert " + "tr" + "ue"


@pytest.mark.parametrize(
    "line",
    [
        f'    {XIT}"does nothing yet")',
        f'    it{SKIP_CALL}"later", () => {{}})',
        f'    describe{SKIP_CALL}"group", () => {{}})',
        f"    {IGNORE_ATTR}",
        f"    {WEAK_ASSERT}",
        f"    assert(" + WEAK_ASSERT.split()[1] + ")",
    ],
)
def test_a_weakened_test_is_still_flagged(guard, line: str) -> None:
    assert _matches(guard.SKIP_PATTERNS + guard.WEAK_PATTERNS, line), (
        f"the guard no longer flags a real weakening pattern: {line!r}"
    )


@pytest.mark.parametrize(
    "line",
    [
        "        sys.exit(main())",
        "        sys.exit(0)",
        "        raise SystemExit(main())",
        "        os._exit(1)",
    ],
)
def test_a_script_entrypoint_is_not_mistaken_for_a_skipped_test(guard, line: str) -> None:
    assert not _matches(guard.SKIP_PATTERNS + guard.WEAK_PATTERNS, line), (
        f"the guard reports a normal process exit as a skipped test: {line!r}"
    )
