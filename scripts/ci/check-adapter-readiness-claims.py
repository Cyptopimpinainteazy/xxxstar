#!/usr/bin/env python3
"""Do the swap adapters claim capabilities their own code contradicts?

`X3VmAdapter::readiness_score()` is a *self-declaration*: each flag it sets true adds 10 points, and
`AdapterScoreboard` — the "mandatory scoreboard" the crate's docs describe — reports that number as
the adapter's readiness. So a flag set true is a claim, and a claim that chain evidence exists is
false when the same file fabricates it.

The evidence this checks is in the same file: an adapter that builds its lock/claim/refund proofs
with `mock_tx_id`, a "Simulated block number", a literal "mock proof" payload or a placeholder
cannot honestly declare

    event_proof_extraction      there is no event and nothing is extracted
    finality_proof              there is no finality to prove
    rpc_indexer_support         there is no node to ask
    proof_ledger_integration    nothing verifiable is written to a ledger

`x3vm_htlc.rs` is the model: it declares those four false, and its non-simulation branch declares
everything false. Fourteen other adapters declared them true while their own tests assert on the
mock transaction ids — measured 2026-09-26.

This is a ratchet, like the fuzz and bootnode baselines: `security/adapter-readiness-claims-baseline.txt`
lists the adapters that still over-claim and may only shrink. Fixing one (by telling the truth in
`readiness_score`, or by giving the adapter a real evidence path) means removing its line here.

Usage:
    scripts/ci/check-adapter-readiness-claims.py           # check against the baseline
    scripts/ci/check-adapter-readiness-claims.py --list    # print every violation found
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "crates/x3-atomic-swap/src"
BASELINE = ROOT / "security/adapter-readiness-claims-baseline.txt"

#: Markers that mean "this file fabricates the evidence a real chain would supply".
MOCK_MARKERS = ("mock_tx_id", "Simulated block", "mock proof", "placeholder")

#: The four flags that assert chain evidence exists.
CLAIM_FLAGS = (
    "event_proof_extraction",
    "finality_proof",
    "rpc_indexer_support",
    "proof_ledger_integration",
)


def readiness_body(text: str) -> str | None:
    """Return the body of `fn readiness_score`, brace-matched from its opening brace."""
    match = re.search(r"fn\s+readiness_score\s*\(", text)
    if not match:
        return None
    start = text.find("{", match.end())
    if start < 0:
        return None
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start : i + 1]
    return None


def violations() -> list[str]:
    found: list[str] = []
    for path in sorted(SRC.glob("*_htlc.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        if not any(marker in text for marker in MOCK_MARKERS):
            continue
        body = readiness_body(text)
        if body is None:
            continue
        claimed = [flag for flag in CLAIM_FLAGS if re.search(rf"{flag}\s*:\s*true", body)]
        if claimed:
            rel = path.relative_to(ROOT)
            found.append(
                f"FAIL  {rel}: claims {' , '.join(claimed)} while fabricating chain evidence"
            )
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--list", action="store_true", help="print every violation")
    args = parser.parse_args()

    current = sorted(violations())

    if args.list or not BASELINE.exists():
        for line in current:
            print(line)
        if not args.list:
            print(f"no baseline at {BASELINE.relative_to(ROOT)}", file=sys.stderr)
            return 0 if not current else 2
        return 0

    recorded = sorted(
        line.strip() for line in BASELINE.read_text(encoding="utf-8").splitlines()
        if line.startswith("FAIL")
    )

    new = [line for line in current if line not in recorded]
    fixed = [line for line in recorded if line not in current]

    if new:
        print("check-adapter-readiness-claims: FAIL: over-claiming adapters not on the baseline:", file=sys.stderr)
        for line in new:
            print(f"  {line}", file=sys.stderr)
    if fixed:
        print(
            "check-adapter-readiness-claims: FAIL: baseline entries that are gone; remove them from "
            f"{BASELINE.relative_to(ROOT)}:",
            file=sys.stderr,
        )
        for line in fixed:
            print(f"  {line}", file=sys.stderr)

    if new or fixed:
        return 1

    print(
        f"check-adapter-readiness-claims: OK - no new over-claims; "
        f"{len(recorded)} known, all on the shrinking list"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
