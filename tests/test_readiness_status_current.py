#!/usr/bin/env python3
"""Regression tests for the status-freshness guard.

`scripts/check-readiness-status-current.py` recomputes the aggregate readiness
from `FEATURE_REGISTRY.toml` and fails when `CURRENT_MAINNET_STATUS.md` disagrees.

The gap it closes was measured on 2026-10-23. The published status document
claimed 55.29% across 21 entries while the registry held 23 averaging 62.74%, and
it described `atomic_kernel` as 35% with its authorization broken five weeks
after TICKET-097 closed by deleting the unsigned finalization extrinsic.
`scripts/check-readiness-consistency.sh` ran on that same tree and printed PASS,
because it only fails when a document claims *more* than the registry allows —
never when it has gone stale downward.

The trees these tests build are the failure modes seen in practice: a re-scored
feature, a dropped scoreboard row, a drifted mode, a phantom row, and the real
stale document from `50029cf14^`. Each is driven through the `X3_READINESS_REGISTRY`
and `X3_READINESS_STATUS` hooks, so no canonical file is modified.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
CHECK = REPO_ROOT / "scripts" / "check-readiness-status-current.py"
REGISTRY = REPO_ROOT / "FEATURE_REGISTRY.toml"
STATUS = REPO_ROOT / "CURRENT_MAINNET_STATUS.md"


def run_check(registry: Path, status: Path) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["X3_READINESS_REGISTRY"] = str(registry)
    env["X3_READINESS_STATUS"] = str(status)
    return subprocess.run(
        ["python3", str(CHECK)],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )


@pytest.fixture()
def copies(tmp_path: Path) -> tuple[Path, Path]:
    # Tracked inputs: their absence is a broken checkout, not an empty test.
    # Skipping here would report green for a tree whose guard had gone missing.
    assert CHECK.is_file(), f"status-freshness check is missing: {CHECK}"
    assert REGISTRY.is_file(), f"FEATURE_REGISTRY.toml is missing: {REGISTRY}"
    assert STATUS.is_file(), f"CURRENT_MAINNET_STATUS.md is missing: {STATUS}"
    reg = tmp_path / "FEATURE_REGISTRY.toml"
    doc = tmp_path / "CURRENT_MAINNET_STATUS.md"
    shutil.copyfile(REGISTRY, reg)
    shutil.copyfile(STATUS, doc)
    return reg, doc


def test_canonical_tree_is_current() -> None:
    result = run_check(REGISTRY, STATUS)
    assert result.returncode == 0, result.stdout + result.stderr


def test_rescored_feature_fails(copies: tuple[Path, Path]) -> None:
    reg, doc = copies
    text = reg.read_text(encoding="utf-8")
    mutated = text.replace(
        "[atomic_kernel]\n", "[atomic_kernel]\n# test marker\n", 1
    ).replace("readiness_score = 57", "readiness_score = 61", 1)
    assert "readiness_score = 61" in mutated
    reg.write_text(mutated, encoding="utf-8")

    result = run_check(reg, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "atomic_kernel" in result.stdout
    assert "61%" in result.stdout


def test_stale_average_line_fails(copies: tuple[Path, Path]) -> None:
    reg, doc = copies
    text = doc.read_text(encoding="utf-8")
    mutated = text.replace("**Average readiness: 62.74%**", "**Average readiness: 55.29%**", 1)
    assert mutated != text, "the average line the test mutates is absent from the document"
    doc.write_text(mutated, encoding="utf-8")

    result = run_check(reg, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "55.29%" in result.stdout


def test_dropped_scoreboard_row_fails(copies: tuple[Path, Path]) -> None:
    reg, doc = copies
    text = doc.read_text(encoding="utf-8")
    kept = [line for line in text.splitlines() if "Tauri OS (apps/tauri-os)" not in line]
    assert len(kept) == len(text.splitlines()) - 1, "the row the test drops is absent"
    doc.write_text("\n".join(kept) + "\n", encoding="utf-8")

    result = run_check(reg, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "tauri_os" in result.stdout


def test_mode_drift_fails(copies: tuple[Path, Path]) -> None:
    reg, doc = copies
    text = doc.read_text(encoding="utf-8")
    lines = []
    flipped = False
    for line in text.splitlines():
        if not flipped and "Atomic Kernel (pallets/x3-atomic-kernel)" in line:
            line = line.replace("LIVE_TESTNET", "GUARDED_TESTNET", 1)
            flipped = True
        lines.append(line)
    assert flipped, "the row the test mutates is absent"
    doc.write_text("\n".join(lines) + "\n", encoding="utf-8")

    result = run_check(reg, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "mode LIVE_TESTNET" in result.stdout


def test_phantom_row_fails(copies: tuple[Path, Path]) -> None:
    reg, doc = copies
    text = doc.read_text(encoding="utf-8")
    text += (
        "\nGhost Feature (pallets/does-not-exist)              "
        "\u2588\u2588\u2588\u2588\u2591\u2591\u2591\u2591\u2591\u2591   40%  LIVE_TESTNET \u2014 invented\n"
    )
    doc.write_text(text, encoding="utf-8")

    result = run_check(reg, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "pallets/does-not-exist" in result.stdout


@pytest.mark.parametrize("missing", ["registry", "status"])
def test_missing_input_is_an_error_not_a_pass(tmp_path: Path, missing: str) -> None:
    reg = REGISTRY if missing != "registry" else tmp_path / "absent.toml"
    doc = STATUS if missing != "status" else tmp_path / "absent.md"

    result = run_check(reg, doc)
    # 2, never 0: a check that cannot read its inputs has verified nothing.
    assert result.returncode == 2, result.stdout + result.stderr


def test_the_actual_2026_10_23_regression_is_caught(tmp_path: Path) -> None:
    """The stale document this guard exists for, replayed against today's registry."""
    stale = subprocess.run(
        ["git", "show", "50029cf14^:CURRENT_MAINNET_STATUS.md"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=120,
    )
    if stale.returncode != 0:
        pytest.skip("the pre-fix status document is not reachable from this clone")

    doc = tmp_path / "CURRENT_MAINNET_STATUS.md"
    doc.write_text(stale.stdout, encoding="utf-8")

    result = run_check(REGISTRY, doc)
    assert result.returncode == 1, result.stdout + result.stderr
    # The three figures the room quoted from it, all of them wrong.
    assert "atomic_kernel" in result.stdout
    assert "tauri_os" in result.stdout
    assert "Average readiness" in result.stdout
