#!/usr/bin/env python3
"""Does the repository still make claims it cannot back with a command?

The audit registry (`feature-matrix/*.toml`, `docs/audit/*`, `audit-artifacts/*`) *records*
claims — that is its job — so it is out of scope here. This scans the surfaces a person
actually reads or copies: the operator status docs, the release notes, and the outbound
message templates in the desktop CRM. Those are the places a claim becomes a public
statement, and they are the places nothing used to check.

Measured 2026-09-26: `apps/x3-desktop/src-tauri/src/crm/outreach_system.rs` carried
"300ms cross-chain finality (vs 12s on Solana)", "Compute revenue per GPU: $1200/month
(current pilot)", "5,000 TPS baseline", "Live in 3 production networks" and
"Deterministic execution guarantees (no MEV/reorg risk)" — none of it produced by anything
in this tree (`X3-CLAIM-001` established there is no GPU or chain-level TPS benchmark at
all), and all of it text a human would paste into an outbound email. `X3-CLAIM-002`'s
"MEV-proof marketing claim" had been removed from `CURRENT_MAINNET_STATUS.md`, which is why
the row read as half-closed: the claim had moved here.

Two rule sets, because the two kinds of claim behave differently:

  * ABSOLUTE  — phrases that are false regardless of context ("MEV-proof", "no MEV",
                "no front-running", "guaranteed finality"). Scanned across the whole tree.
  * NUMERIC   — performance and traction figures. Only a claim when they are asserted as
                results, so these are scanned on the declared claim surfaces and skipped
                when the line itself qualifies the number (target, research, unverified,
                not measured, placeholder, ...).

    scripts/ci/check-claims-hygiene.py            # check
    scripts/ci/check-claims-hygiene.py --list     # print every claim surface scanned

Exit 0 -> no unqualified claim found. Exit 1 -> at least one line asserts something the
repository cannot show a command for.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Never descend into build output, vendored sources, or the audit registry/reports that
# exist to quote claims and measurements.
SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "tauri-vendor",
    "dist",
    "build",
    ".srtool",
    ".ai",
    "audit-artifacts",
    "feature-matrix",
    "reports",
    "docs/audit",
    "docs/superpowers",
    "launch-gates",
    ".wt-lang",
}

# The surfaces a human reads or copies. Segment-anchored: `*` is one path component, `**`
# is the rest of the path, so `("*.md",)` means root-level markdown only and does not
# quietly sweep every document in the repository.
SURFACES = [
    ("*.md",),
    ("docs", "testnet-config", "**"),
    ("apps", "*", "src-tauri", "src", "crm", "**"),
]

# Phrases that are false in every context: the capability does not exist, so no qualifier
# rescues them (`feature-matrix/mev-privacy.toml` scores the whole MEV family at 2-52%).
ABSOLUTE = [
    (re.compile(r"\bMEV[-\s]?proof\b", re.I), "MEV protection is not implemented (X3-MEV-001..008)"),
    (re.compile(r"\bno\s+MEV\b", re.I), "no MEV protection is implemented"),
    (re.compile(r"\bMEV[-\s]?free\b", re.I), "no MEV protection is implemented"),
    (re.compile(r"\bno\s+front[-\s]?running\b", re.I), "no fair-ordering protocol is implemented (X3-MEV-008)"),
    (re.compile(r"\bfront[-\s]?running[-\s]?proof\b", re.I), "no fair-ordering protocol is implemented (X3-MEV-008)"),
    (re.compile(r"\bguaranteed\s+(order|ordering|execution order)\b", re.I), "no ordering guarantee is implemented (X3-MEV-008)"),
    (re.compile(r"\breorg[-\s]?proof\b", re.I), "reorg resistance is a GRANDPA finality property, not a proof"),
    (
        re.compile(r"\bdeterministic\s+execution\s+guarantees?\b", re.I),
        "the compiler/VM verifies a subset of programs, not all execution",
    ),
]

# Figures that are only a claim when asserted as a current result. Deliberately narrow:
# a plain "guarantee" or a price list is ordinary engineering prose and is not scanned, or
# the linter would be noise and the next person would switch it off.
NUMERIC = [
    (re.compile(r"\b\d[\d,._]*\s*(?:k|m|thousand|million)?\s*TPS\b", re.I), "throughput figure"),
    (re.compile(r"\bsub[-\s]?\d+(?:\.\d+)?\s*ms\b", re.I), "latency figure"),
    (re.compile(r"\b\d+(?:\.\d+)?\s*ms\s*(?:p99|latency|finality|settlement|block)", re.I), "latency figure"),
    (re.compile(r"\b\d+\s*[x×]\s*(?:faster|speedup|improvement|better)", re.I), "speedup figure"),
    (re.compile(r"\$\s?\d[\d,.]*\s*(?:/|per\s+)\s*gpu\b", re.I), "per-GPU revenue figure"),
    (re.compile(r"\bzero\s+(?:risk|downtime|capex|cap\s?ex)\b", re.I), "absolute claim"),
    (re.compile(r"\blive\s+in\s+\d+\s+production\b", re.I), "traction claim"),
    (re.compile(r"\b\d+\s+production\s+networks?\b", re.I), "traction claim"),
    (re.compile(r"\bcurrent\s+pilot\b", re.I), "traction claim"),
    (re.compile(r"\bpartners?\s+already\s+live\b", re.I), "traction claim"),
    (re.compile(r"\bacross\s+\d+\s+(?:countries|continents|regions)\b", re.I), "traction claim"),
]

# A number is a claim only when nothing on the line marks it as not-yet-true. `claim` is in
# this list so that a sentence *about* a claim (the registry, a ticket, a report) is not one.
QUALIFIER = re.compile(
    r"\b(?:not|no|never|without|avoid|do not|don't|none|target|targets|targeted|planned|planning|"
    r"roadmap|under development|research|experimental|unverified|unproven|unreachable|dead|disabled|"
    r"blocked|ticket|todo|placeholder|aspirational|intended|proposed|proposal|aim|would|could|"
    r"claim|claims|claiming|report|reports|matrix|row|score|measured|verify|verification|test|tests|"
    r"gate|check|scan|fail|fails|refuses|refused|removed|removes|renamed|rename|baseline|honest|"
    r"inject|injects|injected|injection|simulate|simulated|withdrawn|dropped)\b",
    re.I,
)

TEXT_SUFFIXES = {".md", ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".sh", ".toml", ".json", ".txt", ".html"}


def tracked_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True
    ).stdout
    files = []
    for raw in out.split(b"\0"):
        if not raw:
            continue
        rel = raw.decode("utf-8", "replace")
        if any(part in SKIP_DIRS for part in rel.split("/")[:-1]):
            continue
        p = ROOT / rel
        if p.suffix in TEXT_SUFFIXES and p.is_file():
            files.append(Path(rel))
    return files


def _matches(parts: tuple[str, ...], pattern: tuple[str, ...]) -> bool:
    if not pattern:
        return not parts
    head, rest = pattern[0], pattern[1:]
    if head == "**":
        return True
    if not parts:
        return False
    # A segment may be a glob, but it only ever matches *one* component: `parts[0]` never
    # contains "/", so `fnmatch` cannot let a pattern cross a directory boundary.
    from fnmatch import fnmatchcase

    return fnmatchcase(parts[0], head) and _matches(parts[1:], rest)


def is_surface(rel: str) -> bool:
    parts = tuple(rel.split("/"))
    return any(_matches(parts, pattern) for pattern in SURFACES)


def scan_line(rel: str, lineno: int, line: str, surface: bool, absolute_only: bool) -> list[str]:
    hits = []
    qualified = bool(QUALIFIER.search(line))
    for pattern, why in ABSOLUTE:
        if pattern.search(line) and not qualified:
            hits.append(f"{rel}:{lineno}: ABSOLUTE ({why}): {line.strip()[:160]}")
    if surface and not absolute_only and not qualified:
        for pattern, why in NUMERIC:
            if pattern.search(line):
                hits.append(f"{rel}:{lineno}: unqualified {why}: {line.strip()[:160]}")
                break
    return hits


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list", action="store_true", help="print the claim surfaces")
    args = parser.parse_args()

    files = tracked_files()
    surfaces = [str(f) for f in files if is_surface(str(f))]

    if args.list:
        print(f"claim surfaces ({len(surfaces)}):")
        for s in surfaces:
            print(f"  {s}")
        return 0

    scanned = 0
    hits: list[str] = []
    for rel in files:
        rel_s = str(rel)
        surface = is_surface(rel_s)
        # ABSOLUTE patterns are cheap and unambiguous, so they are checked tree-wide;
        # NUMERIC patterns only on a declared surface.
        try:
            text = (ROOT / rel).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        scanned += 1
        if not surface and not any(p.search(text) for p, _ in ABSOLUTE):
            continue
        for i, line in enumerate(text.splitlines(), start=1):
            hits.extend(scan_line(rel_s, i, line, surface, absolute_only=not surface))

    if hits:
        print("claims-hygiene: FAIL")
        for h in hits:
            print(f"  {h}")
        print()
        print(f"{len(hits)} unqualified claim(s) across {scanned} scanned file(s).")
        print("Rewrite the line so it states what is real, or qualify it (target/planned/")
        print("unverified/research), or record it in the audit registry instead of the surface.")
        return 1

    print(f"claims-hygiene: OK - {scanned} file(s) scanned, {len(surfaces)} claim surface(s), no unqualified claim")
    return 0


if __name__ == "__main__":
    sys.exit(main())
