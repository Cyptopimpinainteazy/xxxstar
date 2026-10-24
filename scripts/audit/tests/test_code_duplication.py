from __future__ import annotations

import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_DIR))

from audit_code_duplication import scan  # noqa: E402


def _git_repo(root: Path) -> None:
    import subprocess

    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(
        ["git", "add", "."],
        cwd=root,
        check=True,
    )


def test_scan_finds_exact_duplicate_source(tmp_path: Path) -> None:
    body = "\n".join(
        [
            "pub fn first() -> u64 { 1 }",
            "pub fn second() -> u64 { 2 }",
            "pub fn third() -> u64 { 3 }",
            "pub fn fourth() -> u64 { 4 }",
        ]
    )
    (tmp_path / "a.rs").write_text(body, encoding="utf-8")
    (tmp_path / "b.rs").write_text(body, encoding="utf-8")
    (tmp_path / "c.rs").write_text(
        "\n".join(
            [
                "pub fn alpha() -> u64 { 10 }",
                "pub fn beta() -> u64 { 20 }",
                "pub fn gamma() -> u64 { 30 }",
                "pub fn delta() -> u64 { 40 }",
            ]
        ),
        encoding="utf-8",
    )
    _git_repo(tmp_path)

    report = scan(tmp_path, minimum_lines=4)

    assert report["summary"]["exact_duplicate_groups"] == 1
    assert report["exact_duplicate_groups"][0]["files"] == ["a.rs", "b.rs"]


def test_scan_ignores_whitespace_and_comments_for_clones(tmp_path: Path) -> None:
    shared = [
        "function stepOne() { return 1; }",
        "function stepTwo() { return 2; }",
        "function stepThree() { return 3; }",
        "function stepFour() { return 4; }",
        "function stepFive() { return 5; }",
    ]
    (tmp_path / "a.ts").write_text(
        "\n".join(f"  {line} // implementation" for line in shared),
        encoding="utf-8",
    )
    (tmp_path / "b.ts").write_text(
        "\n".join(f"{line}  " for line in shared),
        encoding="utf-8",
    )
    _git_repo(tmp_path)

    report = scan(tmp_path, minimum_lines=4)

    assert report["summary"]["structural_clone_groups"] >= 1
    assert report["structural_clone_groups"][0]["lines"] >= 4


def test_default_policy_prunes_vendor_tree(tmp_path: Path) -> None:
    source = "\n".join(f"line_{index}();" for index in range(5))
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "main.ts").write_text(source, encoding="utf-8")
    vendor = tmp_path / "node_modules" / "pkg"
    vendor.mkdir(parents=True)
    (vendor / "copy.ts").write_text(source, encoding="utf-8")
    _git_repo(tmp_path)

    report = scan(tmp_path, minimum_lines=4)

    assert report["summary"]["exact_duplicate_groups"] == 0


def test_default_policy_prunes_hidden_worktree(tmp_path: Path) -> None:
    source = "\n".join(f"line_{index}();" for index in range(5))
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "main.ts").write_text(source, encoding="utf-8")
    hidden = tmp_path / ".rc4-worktree"
    hidden.mkdir()
    (hidden / "copy.ts").write_text(source, encoding="utf-8")
    _git_repo(tmp_path)

    report = scan(tmp_path, minimum_lines=4)

    assert report["summary"]["exact_duplicate_groups"] == 0
