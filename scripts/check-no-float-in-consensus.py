#!/usr/bin/env python3
"""Refuse float arithmetic in the code whose decisions reach consensus.

PHASE 43 (fixed-point financial math) and PHASE 42 (determinism) are prohibitions rather than
features: a value that becomes a balance, a reward, a slash or a settlement amount must be computed
exactly, because a float's rounding is not a decision anyone wrote down. `x3-lang` already has this
check for its own two crates (`compiler/tests/test_computation_discipline.rs`), and TICKET-094 measured
that the *root* workspace's consensus surface was never checked at all: its 551-file worklist was never
classified, so "no native float in a consensus-sensitive path" was a claim about one directory rather
than about the repository.

Measured when this gate was written: `pallets/*/src` and `runtime/src` contain **zero** `f64`/`f32`
occurrences. The six in `pallets/x3-dex/fuzz/` are fuzz-target tolerance comparisons and are out of this
scan's scope by directory, not by exemption — a fuzz target is not a decision path.

A line may carry `// float-exemption: <reason>` if it is a float this rule is not about — a wire format
that mirrors an upstream layout, for instance, which is what `x3-lang/vm/src/bridge.rs`'s
`warmup_cooldown_rate` is. The reason is required: an exemption without one is a blank cheque.

Exit status: 0 clean, 1 offenders, 2 the scan found no files (a gate that reads nothing is not a gate).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# Production code of the consensus surface. `tests/`, `benches/` and `fuzz/` are excluded by name
# rather than by exemption: a test's tolerance or a fuzz target's comparison is not a decision path.
ROOTS = sorted(REPO.glob("pallets/*/src")) + [REPO / "runtime" / "src"]
MARKER = "// float-exemption:"
# Word boundaries, not substrings: `ProofNotMultipleOf32` contains `f32` and is not a float.
FLOAT = re.compile(r"\b(f64|f32)\b")
# A pallet keeps its test module in `src/tests.rs` and its mock runtime in `src/mock.rs`, so these are
# test code that happens to live under `src/` — excluded by name, with `fuzz/` and `benches/`, because a
# test's tolerance is not a decision path either.
TEST_SUPPORT = {"tests.rs", "mock.rs", "test_helpers.rs", "benchmarking.rs"}


def code_lines(path: Path):
    for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        # A trailing comment is stripped so that a doc comment mentioning `f64` is not an offender.
        code = line.split("//", 1)[0]
        yield number, line, code


def main() -> int:
    offenders: list[str] = []
    exemptions = 0
    files = 0
    for root in ROOTS:
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.rs")):
            if path.name in TEST_SUPPORT or any(part in ("tests", "benches", "fuzz") for part in path.parts):
                continue
            files += 1
            for number, line, code in code_lines(path):
                if not FLOAT.search(code):
                    continue
                if MARKER in line:
                    exemptions += 1
                    continue
                offenders.append(f"{path.relative_to(REPO)}:{number}: {line.strip()}")

    if files == 0:
        print("no-float-in-consensus: the scan found no files, which is a misconfiguration", file=sys.stderr)
        return 2
    if offenders:
        print("no-float-in-consensus: these lines use a float in consensus-sensitive code:", file=sys.stderr)
        for offender in offenders:
            print(f"  {offender}", file=sys.stderr)
        print(
            f"\ncomputed exactly, or add `{MARKER} <reason>` on the line if the float is not a "
            "decision (a wire format, say).",
            file=sys.stderr,
        )
        return 1
    print(f"no-float-in-consensus: {files} files, 0 offenders, {exemptions} exemption(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
