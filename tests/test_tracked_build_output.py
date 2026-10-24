from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def tracked_files() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.splitlines()


def test_no_generated_dist_tree_is_tracked() -> None:
    assert [path for path in tracked_files() if "/dist/" in f"/{path}"] == []


def test_no_crawler_state_is_tracked() -> None:
    assert [
        path for path in tracked_files() if path.endswith("/crawler_state.json")
    ] == []

