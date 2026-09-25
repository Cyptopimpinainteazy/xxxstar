#!/usr/bin/env python3
"""Regression tests for the readiness consistency guard.

`scripts/check-readiness-consistency.sh` cross-checks
TESTNET_FEATURE_FLAGS.toml against FEATURE_REGISTRY.toml. It used to skip,
silently, every flags key that has no registry section of its own: the loop key
regex `^([a-z_]+)` never matched a key containing a digit, `get_mode` returned
nothing for an unknown key, and a key on the file's final line was dropped
because the file has no trailing newline. That is how
`external_bridges_mainnet = "GUARDED_TESTNET"` sat next to the registry's
"external-bridge path disabled at genesis" record while the guard reported PASS.

These tests pin the repaired behaviours, plus the canonical tree's own posture.
The guard is driven through its documented `X3_READINESS_FLAGS` test hook, so no
canonical file is modified.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
GUARD = REPO_ROOT / "scripts" / "check-readiness-consistency.sh"
FLAGS = REPO_ROOT / "TESTNET_FEATURE_FLAGS.toml"
REGISTRY = REPO_ROOT / "FEATURE_REGISTRY.toml"


def run_guard(flags_path: Path) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["X3_READINESS_FLAGS"] = str(flags_path)
    env["X3_READINESS_REGISTRY"] = str(REGISTRY)
    return subprocess.run(
        ["bash", str(GUARD)],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=300,
    )


@pytest.fixture()
def flags_fixture(tmp_path: Path) -> Path:
    if not GUARD.is_file():
        pytest.skip("readiness consistency guard script is not present")
    if not FLAGS.is_file() or not REGISTRY.is_file():
        pytest.skip("readiness source documents are not present")
    copy = tmp_path / "TESTNET_FEATURE_FLAGS.toml"
    shutil.copyfile(FLAGS, copy)
    return copy


def test_canonical_tree_satisfies_the_guard() -> None:
    result = run_guard(FLAGS)
    assert result.returncode == 0, result.stdout + result.stderr


def test_bridge_exposure_cannot_contradict_the_genesis_gate(flags_fixture: Path) -> None:
    text = flags_fixture.read_text(encoding="utf-8")
    mutated = "\n".join(
        'external_bridges_mainnet = "GUARDED_TESTNET"'
        if line.startswith("external_bridges_mainnet")
        else line
        for line in text.splitlines()
    ) + "\n"
    assert 'external_bridges_mainnet = "GUARDED_TESTNET"' in mutated
    flags_fixture.write_text(mutated, encoding="utf-8")

    result = run_guard(flags_fixture)
    assert result.returncode != 0
    assert "external_bridges_mainnet" in result.stdout
    assert "DISABLED_BLOCKED" in result.stdout


def test_last_line_key_without_trailing_newline_is_still_checked(flags_fixture: Path) -> None:
    """The canonical file has no final newline; a key appended there must not escape."""
    text = flags_fixture.read_text(encoding="utf-8").rstrip("\n")
    flags_fixture.write_text(
        text + '\nunregistered_subsystem = "LIVE_TESTNET"', encoding="utf-8"
    )

    result = run_guard(flags_fixture)
    assert result.returncode != 0
    assert "unregistered_subsystem" in result.stdout


def test_digit_key_mode_mismatch_is_detected(flags_fixture: Path) -> None:
    """`x3_forge` and friends contain a digit and were skipped by the old regex."""
    text = flags_fixture.read_text(encoding="utf-8")
    mutated = "\n".join(
        'x3_forge = "LIVE_TESTNET"' if line.startswith("x3_forge") else line
        for line in text.splitlines()
    ) + "\n"
    flags_fixture.write_text(mutated, encoding="utf-8")

    result = run_guard(flags_fixture)
    assert result.returncode != 0
    assert "x3_forge" in result.stdout
