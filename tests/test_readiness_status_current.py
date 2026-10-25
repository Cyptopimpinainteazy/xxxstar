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
import re
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


# --- The other documents that restate the aggregate -------------------------
#
# These are read from the working tree rather than a temp copy, because the claim
# patterns are keyed to repo-relative paths. Each test edits, asserts, and restores
# in a finally block so a failure cannot leave a stale figure behind.


def _mutate_and_check(rel_path: str, old: str, new: str) -> subprocess.CompletedProcess:
    target = REPO_ROOT / rel_path
    original = target.read_text(encoding="utf-8")
    assert old in original, f"{rel_path} does not contain the text this test mutates: {old!r}"
    try:
        target.write_text(original.replace(old, new, 1), encoding="utf-8")
        return run_check(REGISTRY, STATUS)
    finally:
        target.write_text(original, encoding="utf-8")


def test_docs_current_readme_is_gated() -> None:
    result = _mutate_and_check(
        "docs/current/README.md",
        "Registry-scored readiness: 62.74%** across 23 entries",
        "Registry-scored readiness: 53.74%** across 19 entries",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert "docs/current/README.md" in result.stdout
    assert "19 entries" in result.stdout


def test_x3_lang_spec_index_is_gated() -> None:
    result = _mutate_and_check(
        "x3-lang/spec/INDEX.md",
        "(average readiness 62.74%",
        "(average readiness 36%",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert "x3-lang/spec/INDEX.md" in result.stdout


def test_release_gates_overall_claim_is_gated() -> None:
    result = _mutate_and_check(
        "RELEASE_GATES.md",
        "Overall readiness: 62.74%**",
        "Overall readiness: 58.5%**",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert "overall readiness claim" in result.stdout


def test_release_gates_second_claim_is_gated() -> None:
    """The second figure in this file went stale independently of the first."""
    result = _mutate_and_check(
        "RELEASE_GATES.md",
        "Currently: 62.74% registry-scored",
        "Currently: 55.5% registry-scored",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert "mainnet-ready-claims section" in result.stdout


def test_registry_rescore_reaches_every_document() -> None:
    """One score change must fail all five claims, not only the scoreboard."""
    result = _mutate_and_check(
        "FEATURE_REGISTRY.toml",
        "readiness_score = 57",
        "readiness_score = 60",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    for expected in (
        "docs/current/README.md",
        "x3-lang/spec/INDEX.md",
        "RELEASE_GATES.md (overall readiness claim)",
        "RELEASE_GATES.md (mainnet-ready-claims section)",
    ):
        assert expected in result.stdout, f"{expected} was not reported as stale"


def test_a_named_document_that_vanishes_is_an_error(tmp_path: Path) -> None:
    """The claim table naming a file means its absence is a defect, not a skip."""
    target = REPO_ROOT / "docs/current/README.md"
    original = target.read_text(encoding="utf-8")
    moved = tmp_path / "README.md"
    try:
        target.rename(moved)
        result = run_check(REGISTRY, STATUS)
    finally:
        moved.rename(target)
    assert result.returncode == 1, result.stdout + result.stderr
    assert "does not exist" in result.stdout


def test_every_counted_violation_is_also_printed() -> None:
    """A violation that is counted but not shown is a defect in the check itself.

    The first version of the claims loop appended to `violations` after the print
    loop had already run, so a stale `RELEASE_GATES.md` produced `FAIL: 7
    disagreement(s)` above three visible lines. The count and the printed list have
    to agree.
    """
    result = _mutate_and_check(
        "RELEASE_GATES.md",
        "Currently: 62.74% registry-scored",
        "Currently: 55.5% registry-scored",
    )
    assert result.returncode == 1, result.stdout + result.stderr
    printed = result.stdout.count("VIOLATION:")
    counted = int(re.search(r"FAIL: (\d+) disagreement", result.stdout).group(1))
    assert printed == counted, f"{counted} counted but {printed} printed:\n{result.stdout}"
