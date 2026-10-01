#!/usr/bin/env python3
"""Bounded stub and test-cheat detectors, with a ratchet.

`AGENTS.md` names `scripts/x3-detect-stubs.sh` and
`scripts/x3-detect-test-cheats.sh` under "Forbidden". Until 2026-09-27 neither
had ever completed: both walked build output and vendored trees
(`x3fronend/out/_next/...`, `*/node_modules`, a `.wt-*` worktree's vendored
`tauri-vendor/cc`) and hit the 240 s timeout, so a mandated check read as
satisfied while it never ran. This script walks the source tree with those
directories pruned and finishes in seconds.

Two modes, one per detector. Both print a readable summary by default and JSON
with `--json`; both refuse to fail on *pre-existing* findings and fail only on
growth (see "The ratchet" below).

`stubs` — shapes it detects:
  explicit-stub   `todo!(`, `unimplemented!(`, `panic!("... not implemented")`,
                  `panic!("stub"` — a call that exists only to be replaced.
  critical-marker `TODO`/`FIXME`/`STUB`/`placeholder`/`dummy`/`no-op`/`fake` as a
                  word on a line under runtime/, pallets/, node/, bridges/,
                  adapters/ or crates/.
  marker          the same words anywhere else in the tree.

`stubs` deliberately does NOT decide whether a `TODO` is legitimate. There are
thousands, most are years old, and the ratchet — not the detector — is what
keeps the number from growing.

`cheats` — shapes it detects (and only these):
  constant-assert a test asserts a literal against itself: `assert_eq!(1, 1)`,
                  `assert!(true)`, `expect(1).toBe(1)`, `assert 1 == 1`.
  noop-test       a `#[test]`/`#[tokio::test]` function whose body is empty.
  skip            `#[ignore]`, `it.skip(`, `describe.skip(`, `test.skip(`,
                  `#[ignore = "..."]`.
  prod-mock       a `mock`/`fake`/`stub` identifier in a file with no
                  `#[cfg(test)]` that is not under a `tests/`/`test/` directory:
                  a mock of the thing under test on a production path.

`cheats` deliberately does NOT detect: weakened or deleted assertions, deleted
test files, mocks inside `#[cfg(test)]` modules, or a skip produced by a build
step. Those are the `scripts/test_cheat_guard.py` diff compared against a base
revision; this script only reads the tree as it is.

The ratchet
-----------
Each mode has a baseline under `docs/reports/` (measured with one scanner, whose
SHA-256 is recorded). `--check` (the gate's mode) fails when

  * the scanner changed since the baseline was taken (a different scanner can
    move a count in either direction without any code changing), or
  * any class grew past its baseline count, or
  * the total did not shrink but the set of finding identities changed — a fix
    that removes one finding and introduces another must not pass as no change.

It passes when the counts are at or below the baseline, and says so when the
counts dropped, because that is the moment to refresh the baseline deliberately.

`--self-test` builds one synthetic tree carrying every pruning shape (a
`node_modules` entry whose directory name looks like source, both forge
dependencies, build output, a `.wt-*` checkout, a self-exclude, hidden paths,
plain source) and asserts that the rg glob set and the `os.walk` fallback
report exactly the same file list. The two enumerations must agree, or a
machine with rg reads different findings than one without it for the same
commit.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

SCANNER_PATH = Path(__file__).resolve()
REPO_ROOT = SCANNER_PATH.parent.parent

# Build output and vendored trees. The previous shell detectors pruned only
# `.git`, `target`, `node_modules`, `.venv`, `forge-std` and `vendor`, which is
# why `x3fronend/out/_next`, a `.wt-*` checkout and `dist/` were all scanned.
PRUNE_DIR_NAMES = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "out",
    ".next",
    "dist",
    "build",
    "forge-std",
    # `forge install` checks out both dependencies under X3-contracts/evm/lib;
    # `forge-std` was pruned from the start, openzeppelin-contracts was missed,
    # so a working checkout read ~16 findings above a clean extraction.
    "openzeppelin-contracts",
    "vendor",
    # The desktop app vendors whole crates under a compound name, so the
    # `vendor` entry above never matched it.
    "tauri-vendor",
    "libproto_lib",
    "__pycache__",
    "coverage",
}
PRUNE_DIR_PREFIXES = (".wt-",)
PRUNE_FILE_SUFFIXES = (".tar.gz", ".tgz", ".zip")

SOURCE_SUFFIXES = {
    ".rs",
    ".sol",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".py",
    ".move",
    ".cairo",
    ".sh",
}

CRITICAL_PREFIXES = (
    "runtime/",
    "pallets/",
    "node/",
    "bridges/",
    "adapters/",
    "crates/",
)

# A rules file names the words it looks for, so scanning it reports its own
# pattern list: one false finding per marker it documents, and any edit to a
# docstring would look like a regression. The detectors exclude themselves.
SELF_EXCLUDES = {
    "scripts/x3_fake_code_scan.py",
    "scripts/x3-detect-stubs.sh",
    "scripts/x3-detect-test-cheats.sh",
}

STUB_EXPLICIT_RES = (
    re.compile(r"\btodo!\s*\("),
    re.compile(r"\bunimplemented!\s*\("),
    re.compile(r"panic!\s*\(\s*\"[^\"]*not implemented", re.IGNORECASE),
    re.compile(r"panic!\s*\(\s*\"stub", re.IGNORECASE),
)
STUB_MARKER_RE = re.compile(
    r"\b(TODO|FIXME|STUB|placeholder|dummy|no-?op|fake)\b", re.IGNORECASE
)

CONSTANT_ASSERT_RUST_RE = re.compile(
    r"(?<![\w:])assert(?:_eq|_ne)?!\s*\(\s*(?P<a>-?\d[\d_]*(?:\.\d+)?|\"[^\"]*\"|true|false)"
    r"\s*,\s*(?P<b>-?\d[\d_]*(?:\.\d+)?|\"[^\"]*\"|true|false)\s*[,)]"
)
CONSTANT_ASSERT_JS_RE = re.compile(
    r"expect\s*\(\s*(?P<a>-?\d[\d_.]*|\"[^\"]*\")\s*\)\s*\.(?:toBe|toEqual|toStrictEqual)"
    r"\s*\(\s*(?P<b>-?\d[\d_.]*|\"[^\"]*\")\s*\)"
)
CONSTANT_ASSERT_PY_RE = re.compile(
    r"assert\s+(?P<a>-?\d[\d_]*(?:\.\d+)?|\"[^\"]*\"|True|False)\s*==\s*"
    r"(?P<b>-?\d[\d_]*(?:\.\d+)?|\"[^\"]*\"|True|False)\b"
)
# `assert!(false)` always panics, which is a legitimate way to mark an
# unreachable arm, and `debug_assert!`/`prop_assert!` are different macros. Only
# a bare `assert!(true)` is a test that cannot fail.
CONSTANT_ASSERT_BARE_RE = re.compile(r"(?<![\w:])assert!\s*\(\s*true\s*\)")

SKIP_RES = (
    re.compile(r"#\s*\[\s*ignore(\s*=|\(|\])"),
    re.compile(r"\b(?:it|test|describe)\.skip\s*\("),
    re.compile(r"\bxit\s*\("),
)

# Only a *definition or construction* of a mock counts. A bare word match fires
# on every sentence that mentions mocks — including this file's own header.
PROD_MOCK_RE = re.compile(
    r"(?i)(?:"
    r"\b(?:struct|enum|impl|type)\s+\w*(?:mock|fake|stub|dummy)\w*"
    r"|\bmod\s+(?:mocks?|fakes?|stubs?)\b"
    r"|\bMock[A-Z]\w*"
    r"|\bmock!\s*\("
    r")"
)

CFG_TEST_RE = r"#\s*\[\s*cfg\s*\(\s*(?:all\s*\(\s*)?test"
MOCK_REF_RE = r"\b(?:crate|super|self)::(?:mock|tests)\b"
TEST_ATTR_RE = r"#\s*\[\s*(?:tokio::)?test\s*\]"

# Supersets handed to `rg`, which finds *candidate* lines and files; the Python
# matchers above stay authoritative for classification. For the two paths to
# report the same findings, candidate patterns must be supersets of what the
# classifiers can match. `stubs` upholds that. `cheats` does not, for one
# class: `PROD_MOCK_RE` matches `struct XStub`/`MockThing` shapes that the
# `\b(mock|fake|stub|dummy)\b` candidate does not (no word boundary inside the
# identifier), so with rg the `prod-mock` class reads 0 and without rg it reads
# 239 on the same tree (`PATH=/usr/bin:/bin python3 scripts/x3_fake_code_scan.py
# cheats`). Deciding which of those 239 are real needs the `#[cfg(test)] mod
# mock;` question settled first; until then the ratchet records the rg-path
# count (the CI path) and this divergence stays tracked, not silently fixed.
STUB_CANDIDATE_PATTERNS = [
    r"\btodo!",
    r"\bunimplemented!",
    r"not implemented",
    r"(?i)\b(TODO|FIXME|STUB|placeholder|dummy|no-?op|fake)\b",
]
CHEAT_CANDIDATE_PATTERNS = [
    r"assert",
    r"#\s*\[\s*ignore",
    r"\.skip\s*\(",
    r"\bxit\s*\(",
    r"(?i)\b(mock|mockall|fake|stub|dummy)\b",
    r"#\s*\[\s*(?:tokio::)?test\s*\]",
]

TEST_DIR_PARTS = {"tests", "test", "__tests__", "spec"}


def identity(kind: str, rel_path: str, _lineno: int, text: str) -> str:
    """Identity of a finding: *what* it is, not where the line currently sits.

    The line number is deliberately excluded. Including it made every unrelated
    insertion above a finding change the identity digest, so the ratchet failed
    on edits that neither added nor removed anything — a false positive that
    invites the one behaviour the baseline must never see: re-baselining to make
    a red gate green. A finding that appears, disappears, moves file, changes
    kind or changes wording still changes the digest, which is the swap this
    ratchet exists to catch. Duplicate lines in one file still count twice,
    because `identity_digest` keeps one entry per finding rather than per
    distinct id.
    """
    payload = f"{kind}\0{rel_path}\0{text.strip()}"
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def _is_pruned_dir(name: str) -> bool:
    """Hidden directories are pruned wholesale.

    That is what `rg` does by default, and the two paths must agree: the same
    tree has to report the same findings whether or not `rg` is on the box.
    It also drops `.ai/` (1.2 GB of generated runlogs) and the `.wt-*`
    worktrees whose vendored `cc` crate is what made the old detectors hang.
    """
    return (
        name.startswith(".")
        or name in PRUNE_DIR_NAMES
        or any(name.startswith(prefix) for prefix in PRUNE_DIR_PREFIXES)
    )


def iter_source_files(root: Path = REPO_ROOT) -> list[tuple[str, Path]]:
    """Every scannable file, as `(root-relative path, absolute path)`, sorted."""
    found: list[tuple[str, Path]] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(name for name in dirnames if not _is_pruned_dir(name))
        for name in sorted(filenames):
            if name.startswith(".") or name.endswith(PRUNE_FILE_SUFFIXES):
                continue
            path = Path(dirpath) / name
            if path.suffix not in SOURCE_SUFFIXES:
                continue
            rel = path.relative_to(root).as_posix()
            if rel in SELF_EXCLUDES:
                continue
            found.append((rel, path))
    return sorted(found)


def _rg_args() -> list[str]:
    """rg arguments whose pruning matches `iter_source_files` exactly.

    Order matters: when several globs match one path, rg gives the *last* glob
    precedence. The suffix whitelists therefore come first and every exclusion
    comes after them; negatives-first re-included `node_modules/decimal.js`
    (a directory whose *name* matches `*.js`), the scanner itself (matches
    `*.py`) and every other excluded name that happens to look like a source
    file. `--self-test` pins the two enumerations together so that drift is a
    test failure, not a silently higher count.
    """
    args = ["rg", "--no-ignore", "--color", "never", "--no-heading"]
    for suffix in sorted(SOURCE_SUFFIXES):
        args += ["-g", f"*{suffix}"]
    for rel in sorted(SELF_EXCLUDES):
        args += ["-g", f"!{rel}"]
    for name in sorted(PRUNE_DIR_NAMES):
        args += ["-g", f"!**/{name}/**"]
    for prefix in PRUNE_DIR_PREFIXES:
        args += ["-g", f"!**/{prefix}*/**"]
    return args


def rg_lines(
    patterns: list[str], root: Path = REPO_ROOT
) -> list[tuple[str, int, str]] | None:
    """Candidate `(path, line, text)` matches, or `None` when rg is absent."""
    if shutil.which("rg") is None:
        return None
    args = _rg_args() + ["--json"]
    for pattern in patterns:
        args += ["-e", pattern]
    args.append(".")
    proc = subprocess.run(args, cwd=root, capture_output=True, text=True)
    if proc.returncode not in (0, 1):
        return None
    found: list[tuple[str, int, str]] = []
    for raw in proc.stdout.splitlines():
        try:
            obj = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if obj.get("type") != "match":
            continue
        data = obj["data"]
        path = data["path"]["text"]
        rel = path[2:] if path.startswith("./") else path
        found.append((rel, int(data["line_number"]), data["lines"]["text"].rstrip("\n")))
    return found


def rg_files(patterns: list[str], root: Path = REPO_ROOT) -> list[Path] | None:
    """Candidate files, or `None` when rg is absent."""
    if shutil.which("rg") is None:
        return None
    args = _rg_args() + ["--files-with-matches"]
    for pattern in patterns:
        args += ["-e", pattern]
    args.append(".")
    proc = subprocess.run(args, cwd=root, capture_output=True, text=True)
    if proc.returncode not in (0, 1):
        return None
    files = []
    for raw in proc.stdout.splitlines():
        rel = raw[2:] if raw.startswith("./") else raw
        files.append(root / rel)
    return sorted(files)


def rg_file_list(root: Path = REPO_ROOT) -> list[str] | None:
    """Every file rg would scan under `root`, as paths relative to it.

    This is `iter_source_files` in rg's glob language; `self_test` asserts the
    two agree shape by shape. `None` when rg is not installed.
    """
    if shutil.which("rg") is None:
        return None
    args = _rg_args() + ["--files", "."]
    proc = subprocess.run(args, cwd=root, capture_output=True, text=True)
    if proc.returncode not in (0, 1):
        return None
    files: list[str] = []
    for raw in proc.stdout.splitlines():
        rel = raw[2:] if raw.startswith("./") else raw
        files.append(rel)
    return sorted(files)


def read_lines(path: Path) -> list[str]:
    try:
        return path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return []


def scan_stubs() -> list[dict[str, object]]:
    findings: list[dict[str, object]] = []
    candidates = rg_lines(STUB_CANDIDATE_PATTERNS)
    if candidates is None:
        candidates = []
        for rel, path in iter_source_files():
            for lineno, text in enumerate(read_lines(path), start=1):
                candidates.append((rel, lineno, text))

    for rel, lineno, text in candidates:
        critical_path = rel.startswith(CRITICAL_PREFIXES)
        kind = None
        if any(rx.search(text) for rx in STUB_EXPLICIT_RES):
            kind = "explicit-stub"
        elif STUB_MARKER_RE.search(text):
            kind = "critical-marker" if critical_path else "marker"
        if kind is None:
            continue
        findings.append(
            {
                "kind": kind,
                "path": rel,
                "line": lineno,
                "text": text.strip()[:160],
                "id": identity(kind, rel, lineno, text),
            }
        )
    return sorted(findings, key=lambda f: (str(f["kind"]), str(f["path"]), int(f["line"])))


def _looks_like_test_file(name: str) -> bool:
    """A filename that is test code by this repository's convention.

    Substring matching on the whole name would be wrong ("attestation.rs"
    contains "test"); the stems here are the shapes the tree actually uses.
    `benchmarking.rs` is included because Substrate compiles it only under
    `runtime-benchmarks`, next to the mock runtime it names.
    """
    stem = name.rsplit(".", 1)[0].lower()
    return (
        stem in {"tests", "test", "spec", "benchmarking"}
        or stem.startswith("test_")
        or stem.startswith("tests_")
        or stem.endswith("_test")
        or stem.endswith("_tests")
        or stem.endswith(".test")
        or stem.endswith(".spec")
    )


def _path_is_test(rel: str) -> bool:
    path = Path(rel)
    if any("test" in part.lower() for part in path.parts[:-1]):
        return True
    return _looks_like_test_file(path.name)


def _is_test_context(rel: str, text: str) -> bool:
    if _path_is_test(rel):
        return True
    if "#[cfg(test)]" in text or "#[cfg(any(test" in text:
        return True
    # A crate-local mock can only be named from a test: the `mock` module is
    # itself `#[cfg(test)]`, so a file that reaches it is compiled for tests
    # even when the gate sits in its parent module.
    return re.search(MOCK_REF_RE, text) is not None


def test_context_files() -> set[str]:
    """Files that are test context: under a tests/ dir, or carrying `#[cfg(test)]`.

    Used instead of reading every candidate file, which is what made the first
    version of this scanner take half a minute.
    """
    files = rg_files([CFG_TEST_RE, MOCK_REF_RE])
    by_path = {rel for rel, _ in iter_source_files() if _path_is_test(rel)}
    if files is None:
        return {
            rel
            for rel, path in iter_source_files()
            if _is_test_context(rel, "\n".join(read_lines(path)))
        }
    return by_path | {
        path.relative_to(REPO_ROOT).as_posix()
        for path in files
    }


def _constant_assertion(text: str) -> bool:
    if CONSTANT_ASSERT_BARE_RE.search(text):
        return True
    for rx in (CONSTANT_ASSERT_RUST_RE, CONSTANT_ASSERT_JS_RE, CONSTANT_ASSERT_PY_RE):
        match = rx.search(text)
        if match is not None and match.group("a") == match.group("b"):
            return True
    return False


def _is_cfg_test(line: str) -> bool:
    stripped = line.strip()
    return stripped.startswith(("#[cfg(test)]", "#[cfg(any(test", "#[cfg(all(test"))


def _has_cfg_test_attribute_above(lines: list[str], lineno: int) -> bool:
    """Whether the definition at `lineno` is gated by `#[cfg(test)]`.

    Substrate writes the gate and the path on separate attributes
    (`#[cfg(test)]` then `#[path = "mock.rs"]` then `mod mock;`), so looking at
    one line above reports the pallet's own test mock as a production one.
    """
    index = lineno - 2
    while index >= 0:
        stripped = lines[index].strip()
        if not stripped:
            index -= 1
            continue
        if not stripped.startswith("#["):
            break
        if _is_cfg_test(stripped):
            return True
        index -= 1
    return False


NOOP_TEST_RUST_RE = re.compile(
    r"#\s*\[\s*(?:tokio::)?test\s*\]\s*(?:#\[[^\]]*\]\s*)*"
    r"(?:async\s+)?fn\s+(?P<name>\w+)\s*\([^)]*\)\s*(?:->[^{]*)?\{\s*\}",
    re.DOTALL,
)


def scan_cheats() -> list[dict[str, object]]:
    findings: list[dict[str, object]] = []
    test_files = test_context_files()

    candidates = rg_lines(CHEAT_CANDIDATE_PATTERNS)
    if candidates is None:
        candidates = [
            (rel, lineno, text)
            for rel, path in iter_source_files()
            for lineno, text in enumerate(read_lines(path), start=1)
        ]

    # Only the files that reach the `prod-mock` branch are read, to look at the
    # line above the definition: `#[cfg(test)] mod mock;` is Substrate's normal
    # way to declare a test mock, and it is not a mock on a production path.
    prod_mock_lines: dict[str, list[str]] = {}

    for rel, lineno, line in candidates:
        if line.lstrip().startswith(("//", "/*", "*")):
            # A commented-out assertion is not an assertion. Rust `#[ignore]`
            # arrives as `#...`, so this does not hide the skip class.
            continue
        in_tests = rel in test_files
        kind = None
        if in_tests and _constant_assertion(line):
            kind = "constant-assert"
        elif any(rx.search(line) for rx in SKIP_RES):
            kind = "skip"
        elif (
            # A mock *definition* on a production path. `crate::mock::…` inside a
            # `#[cfg(test)]` module is a use, not a definition, and never reaches
            # here; a file whose name says it is a test is skipped by path so a
            # production file that also holds a test module is still checked.
            not _path_is_test(rel)
            and rel.startswith(CRITICAL_PREFIXES)
            and PROD_MOCK_RE.search(line)
        ):
            if rel not in prod_mock_lines:
                prod_mock_lines[rel] = read_lines(REPO_ROOT / rel)
            if not _has_cfg_test_attribute_above(prod_mock_lines[rel], lineno):
                kind = "prod-mock"
        if kind is None:
            continue
        findings.append(
            {
                "kind": kind,
                "path": rel,
                "line": lineno,
                "text": line.strip()[:160],
                "id": identity(kind, rel, lineno, line),
            }
        )

    # A no-op body spans lines, so only the files that declare a test are read.
    declared = rg_files([TEST_ATTR_RE])
    if declared is None:
        declared = [
            path
            for rel, path in iter_source_files()
            if path.suffix == ".rs" and TEST_ATTR_RE in "\n".join(read_lines(path))
        ]
    for path in declared:
        if path.suffix != ".rs":
            continue
        rel = path.relative_to(REPO_ROOT).as_posix()
        text = "\n".join(read_lines(path))
        for match in NOOP_TEST_RUST_RE.finditer(text):
            lineno = text.count("\n", 0, match.start()) + 1
            findings.append(
                {
                    "kind": "noop-test",
                    "path": rel,
                    "line": lineno,
                    "text": f"fn {match.group('name')}() {{}}",
                    "id": identity("noop-test", rel, lineno, match.group(0)),
                }
            )

    return sorted(findings, key=lambda f: (str(f["kind"]), str(f["path"]), int(f["line"])))


MODES = {"stubs": scan_stubs, "cheats": scan_cheats}
BASELINES = {
    "stubs": REPO_ROOT / "docs/reports/fake-code-baseline.json",
    "cheats": REPO_ROOT / "docs/reports/test-cheat-baseline.json",
}


def counts_of(findings: list[dict[str, object]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for finding in findings:
        kind = str(finding["kind"])
        counts[kind] = counts.get(kind, 0) + 1
    return dict(sorted(counts.items()))


def identity_digest(findings: list[dict[str, object]]) -> str:
    ids = sorted(str(finding["id"]) for finding in findings)
    return hashlib.sha256("\n".join(ids).encode("utf-8")).hexdigest()


def scanner_sha256() -> str:
    return hashlib.sha256(SCANNER_PATH.read_bytes()).hexdigest()


def load_baseline(mode: str) -> dict[str, object] | None:
    path = BASELINES[mode]
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def write_baseline(mode: str, findings: list[dict[str, object]]) -> None:
    import datetime

    path = BASELINES[mode]
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "generated_at": datetime.datetime.now(datetime.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "scanner_sha256": scanner_sha256(),
        "counts": counts_of(findings),
        "identity_digest": identity_digest(findings),
        "note": (
            "Ratchet for scripts/x3_fake_code_scan.py. `counts` may only go down "
            "and `identity_digest` must not change while the total is unchanged: "
            "a fix that removes one finding and adds another is a regression. "
            "`scanner_sha256` pins the scanner this was measured with; refresh "
            "with `--update-baseline` and say in the commit message what grew."
        ),
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"{mode}: baseline written to {path.relative_to(REPO_ROOT)}")


def check(mode: str, findings: list[dict[str, object]]) -> int:
    baseline = load_baseline(mode)
    if baseline is None:
        print(
            f"{mode}: no baseline at {BASELINES[mode].relative_to(REPO_ROOT)}; "
            "run with --update-baseline",
            file=sys.stderr,
        )
        return 2

    current_counts = counts_of(findings)
    baseline_counts = {str(k): int(v) for k, v in dict(baseline["counts"]).items()}
    current_total = sum(current_counts.values())
    baseline_total = sum(baseline_counts.values())

    if str(baseline["scanner_sha256"]) != scanner_sha256():
        print(
            f"{mode}: the baseline was measured with a different scanner "
            f"({baseline['scanner_sha256'][:12]} != {scanner_sha256()[:12]}); "
            "refresh it deliberately",
            file=sys.stderr,
        )
        return 1

    grew = {
        kind: (baseline_counts.get(kind, 0), count)
        for kind, count in current_counts.items()
        if count > baseline_counts.get(kind, 0)
    }
    if grew or current_total > baseline_total:
        for kind, (was, now) in sorted(grew.items()):
            print(f"{mode}: {kind} grew {was} -> {now}", file=sys.stderr)
            for finding in findings:
                if finding["kind"] == kind:
                    print(
                        f"  {finding['path']}:{finding['line']}: {finding['text']}",
                        file=sys.stderr,
                    )
        if current_total > baseline_total and not grew:
            print(
                f"{mode}: total grew {baseline_total} -> {current_total}",
                file=sys.stderr,
            )
        return 1

    if current_total == baseline_total and identity_digest(findings) != str(
        baseline["identity_digest"]
    ):
        print(
            f"{mode}: the count is unchanged ({current_total}) but the findings "
            "changed — a fix that adds a finding while removing one is still growth",
            file=sys.stderr,
        )
        return 1

    if current_total < baseline_total:
        print(
            f"{mode}: PASS at {current_total} (baseline {baseline_total}); "
            "the baseline can be refreshed to lock the improvement in"
        )
    else:
        print(f"{mode}: PASS at the baseline of {baseline_total}")
    return 0


def self_test() -> int:
    """Pin the rg enumeration to the os.walk enumeration on a synthetic tree.

    The tree carries one instance of every shape the pruning exists for, in
    the exact arrangement that broke when the globs were ordered
    exclusions-first: a directory literally named `decimal.js` inside
    `node_modules`, both forge dependencies, a source-looking file under
    build output, and the scanner itself. Anything the tests add or drop
    shows up as a set difference instead of a count nobody can trace.
    """
    import tempfile

    shapes = {
        # Plain source: both must include these.
        "src/keep.rs": "fn main() {}\n",
        "src/keep.js": "console.log(1);\n",
        "apps/dashboard/src/panel.tsx": "// TODO\n",
        "tools/ok.py": "# placeholder\n",
        # Not source by suffix: both must drop it.
        "src/notes.txt": "not source\n",
        # Vendored and build trees: both must drop everything inside.
        "node_modules/decimal.js/decimal.js": "// TODO\n",
        "node_modules/decimal.js/decimal.mjs": "// TODO\n",
        "node_modules/pkg/index.mjs": "// fake\n",
        "X3-contracts/evm/lib/openzeppelin-contracts/contracts/token.sol": "// TODO\n",
        "X3-contracts/evm/lib/forge-std/src/Test.sol": "// TODO\n",
        "target/debug/foo.rs": "// TODO\n",
        "out/_next/chunk.js": "// TODO\n",
        # Hidden paths and worktrees: both must drop them.
        ".hidden_dir/leak.rs": "// TODO\n",
        ".wt-abc/vendor/lib.rs": "// TODO\n",
        # The detector itself: both must drop it even though it is `*.py`.
        "scripts/x3_fake_code_scan.py": "# placeholder\n",
        "scripts/x3-detect-stubs.sh": "# stub\n",
        "scripts/x3-detect-test-cheats.sh": "# stub\n",
    }

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        for rel, text in shapes.items():
            path = tmp_path / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

        expected = sorted(rel for rel, _ in iter_source_files(tmp_path))
        actual = rg_file_list(tmp_path)
        if actual is None:
            print(
                "self-test: rg is not installed, so the enumerations cannot "
                "be compared; install ripgrep",
                file=sys.stderr,
            )
            return 1
        if actual != expected:
            print(
                "fake-code scanner self-test FAILED: the rg glob set and the "
                "os.walk fallback disagree",
                file=sys.stderr,
            )
            print(f"  only the walker scans: {sorted(set(expected) - set(actual))}", file=sys.stderr)
            print(f"  only rg scans:         {sorted(set(actual) - set(expected))}", file=sys.stderr)
            return 1

        included = {
            "src/keep.rs",
            "apps/dashboard/src/panel.tsx",
            "tools/ok.py",
        }
        if not included <= set(expected):
            print(
                f"self-test: the tree stopped exercising inclusion: {sorted(included - set(expected))}",
                file=sys.stderr,
            )
            return 1

    print(
        f"fake-code scanner self-test: PASS ({len(expected)} files scanned; "
        "rg glob set == os.walk fallback)"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("mode", nargs="?", choices=sorted(MODES))
    parser.add_argument("--json", action="store_true", help="emit JSON on stdout")
    parser.add_argument(
        "--update-baseline", action="store_true", help="record the current findings"
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="pin the rg glob set to the os.walk fallback on a synthetic tree",
    )
    args = parser.parse_args()

    if args.self_test:
        if args.mode is not None or args.json or args.update_baseline:
            parser.error("--self-test does not combine with a mode or other flags")
        return self_test()
    if args.mode is None:
        parser.error("a mode is required unless --self-test is given")

    findings = MODES[args.mode]()

    if args.json:
        json.dump(
            {
                "mode": args.mode,
                "counts": counts_of(findings),
                "identity_digest": identity_digest(findings),
                "findings": findings,
            },
            sys.stdout,
            indent=2,
        )
        sys.stdout.write("\n")
        return 0

    if args.update_baseline:
        write_baseline(args.mode, findings)
        return 0

    counts = counts_of(findings)
    print(f"=== x3 {args.mode} scan — {sum(counts.values())} finding(s) ===")
    for kind, count in counts.items():
        print(f"  {kind}: {count}")
    return check(args.mode, findings)


if __name__ == "__main__":
    raise SystemExit(main())
