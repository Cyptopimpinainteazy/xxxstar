"""Regression tests for `scripts/agent_guard.py`'s secret scan.

The scan is a line-level heuristic, so it has to be tested from both sides: a
rule that misses real material is a hole, and one that flags prose is a gate
nobody can pass. Both have happened — the bip39 2.x constructor was allowed by
hand, and then the *general* rule still matched any Rust path-qualified name
because `[:=]` treats the first colon of `Mnemonic::from_phrase` as an
assignment.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]


def guard():
    spec = importlib.util.spec_from_file_location(
        "agent_guard", ROOT / "scripts" / "agent_guard.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def is_secret_like(line: str) -> bool:
    module = guard()
    if module.is_allowed_line(line):
        return False
    return any(re.search(pattern, line) for pattern in module.SECRET_PATTERNS)


def test_prose_naming_a_rust_path_is_not_secret_like():
    for line in [
        "  `bip39::Mnemonic::from_phrase` spelling. The live code uses",
        '  `is_allowed_line("... Mnemonic::parse_in(...)") == True`.',
        "// The Mnemonic::parse_in constructor parses a caller-supplied phrase.",
        "let key = ApiKey::from_bytes(raw);",
        "let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, phrase)?;",
    ]:
        assert not is_secret_like(line), line


def test_a_real_assignment_is_still_secret_like():
    # The values are assembled from pieces: this file is scanned by the very rule
    # it tests, so a literal assignment here is reported as the material it is
    # checking for. The concatenation happens before the matcher runs, so what is
    # tested is still a real assignment of a real-looking value.
    for line in [
        'let private_key = "' + "0xdeadbeef" + 'deadbeef";',
        'private_key: "' + "0xdeadbeefdeadbeef" + 'deadbeefdeadbeef"',
        'mnemonic = "' + "abandonability" + 'ableabout"',
        'api_key = "' + "sk_live_" + 'abcdefghijklmnop"',
        'rpc-key: "' + "abcdefghijkl" + 'mnop"',
    ]:
        assert is_secret_like(line), line


def test_known_key_shapes_are_still_secret_like():
    # Assembled rather than written out: a test for a secret scanner is itself
    # scanned, and a literal here reads as the material it is testing for. (The
    # guard's own rules say so — this file was reported the first time it was
    # written with the shapes inline.)
    for line in [
        "AKIA" + "IOSFODNN7EXAMPLE",
        "-----BEGIN " + "RSA PRIVATE KEY-----",
        "-----BEGIN " + "OPENSSH PRIVATE KEY-----",
    ]:
        assert is_secret_like(line), line
