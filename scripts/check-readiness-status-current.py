#!/usr/bin/env python3
"""Fail when `CURRENT_MAINNET_STATUS.md` no longer agrees with `FEATURE_REGISTRY.toml`.

## Why this exists

`scripts/check-readiness-consistency.sh` guards one direction only: it fails when a
status document claims **more** than the registry allows. It has never noticed a
document that has gone stale **downward**, and on 2026-10-23 that gap was not
theoretical — the published status file said 55.29% across 21 entries while the
registry held 23 entries averaging 62.74%, and it described `atomic_kernel` as 35%
with its authorization broken five weeks after TICKET-097 closed by deleting the
unsigned finalization path. The consistency gate ran on the same tree and printed
PASS throughout.

Every other ratchet in this repository detects something getting *worse*. None
detected something that had stopped being *true*. A stored average is a copy of a
derivable value, and a copy rots; this check recomputes the derivation and fails on
any gap.

## What it checks

The full scoreboard in `CURRENT_MAINNET_STATUS.md`:

1. The header's `<pct>% registry-scored readiness across <n> entries` matches.
2. The `**Average readiness: <pct>%** — <n> tracked features.` line matches.
3. Every registry entry's `readiness_score` appears on a scoreboard line carrying
   that entry's `crate_or_service` path — so a re-scored feature cannot hide behind
   a matching total.
4. No scoreboard line cites a score for a path that the registry does not hold at
   that value.

Plus every other document that restates the aggregate, each entry in `LIVE_CLAIMS`
below. `RELEASE_GATES.md` carried two of them and both were stale on 2026-10-23
(58.5% across 22 entries, and 55.5% further down), which is what widening this
beyond a single file is for.

### Dated statements are deliberately out of scope

A banner reading "on 2026-10-23 the registry held 62.74%" stays true forever; a
line reading "registry now 62.74%" rots the moment a score moves. Only the second
kind is gated. If you want a figure to survive without maintenance, date it.

`docs/audit/X3_FEATURE_COMPLETION_MATRIX.md` also quotes the mean, min and max, but
it is generated and already gated by `scripts/x3_audit_matrix.py --check`, which
fails on a source-digest mismatch. Two gates over one file would report the same
cause twice.

Tests vary the registry or the documents through `X3_READINESS_REGISTRY` and
`X3_READINESS_STATUS` rather than mutating the canonical files, matching the
existing hook in `check-readiness-consistency.sh`.

Exit 0 when current, 1 on any disagreement, 2 when an input is missing or unparsable
(a check that cannot read its inputs has verified nothing, and must not report PASS).
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REGISTRY = pathlib.Path(
    os.environ.get("X3_READINESS_REGISTRY", ROOT / "FEATURE_REGISTRY.toml")
)
STATUS = pathlib.Path(
    os.environ.get("X3_READINESS_STATUS", ROOT / "CURRENT_MAINNET_STATUS.md")
)

# A scoreboard line: `Name (crate/path)   ██████░░░░   57%  MODE — note`.
# The bar is exactly ten cells of FULL BLOCK / LIGHT SHADE, which is what keeps the
# label group from swallowing the bar.
SCORE_LINE = re.compile(
    r"^(?P<label>.+?)\s+[\u2588\u2591]{10}\s+(?P<score>\d{1,3})%\s+(?P<mode>[A-Z_]+)",
    re.M,
)
HEADER = re.compile(
    r"(?P<pct>\d+(?:\.\d+)?)%\s+registry-scored readiness across\s+(?P<n>\d+)\s+entries",
    re.I,
)
AVERAGE = re.compile(
    r"\*\*Average readiness:\s*(?P<pct>\d+(?:\.\d+)?)%\*\*\s*[—-]+\s*(?P<n>\d+)\s+tracked features",
    re.I,
)


class RegistryError(Exception):
    pass


# Every document that restates the aggregate as a *current* value. Each pattern
# must carry named groups `pct` and `n`; `RELEASE_GATES.md` appears twice because it
# states the figure in two places and both had gone stale independently.
LIVE_CLAIMS = [
    {
        "path": "docs/current/README.md",
        "what": "registry-scored readiness line",
        "pattern": r"Registry-scored readiness: (?P<pct>\d+(?:\.\d+)?)%\*\*\s+across (?P<n>\d+) entries",
    },
    {
        "path": "x3-lang/spec/INDEX.md",
        "what": "registry line",
        "pattern": r"(?P<n>\d+) features tracked in `FEATURE_REGISTRY\.toml` \(average readiness (?P<pct>\d+(?:\.\d+)?)%",
    },
    {
        "path": "RELEASE_GATES.md",
        "what": "overall readiness claim",
        "pattern": r"Overall readiness: (?P<pct>\d+(?:\.\d+)?)%\*\*.*?across the (?P<n>\d+) scored entries",
    },
    {
        "path": "RELEASE_GATES.md",
        "what": "mainnet-ready-claims section",
        "pattern": r"Currently: (?P<pct>\d+(?:\.\d+)?)% registry-scored readiness",
        "count_optional": True,
    },
]


def parse_registry(text: str) -> list[dict[str, object]]:
    """Each `[feature]` table with a `readiness_score`, plus mode and crate."""
    entries: list[dict[str, object]] = []
    for match in re.finditer(r"^\[([a-z0-9_]+)\]$(.*?)(?=^\[|\Z)", text, re.S | re.M):
        key, body = match.group(1), match.group(2)
        score = re.search(r"^readiness_score\s*=\s*(\d+)", body, re.M)
        if not score:
            continue
        mode = re.search(r'^mode\s*=\s*"([A-Z_]+)"', body, re.M)
        crate = re.search(r'^crate_or_service\s*=\s*"([^"]+)"', body, re.M)
        entries.append(
            {
                "key": key,
                "score": int(score.group(1)),
                "mode": mode.group(1) if mode else "UNKNOWN",
                "crate": crate.group(1) if crate else "",
            }
        )
    if not entries:
        raise RegistryError("no feature table with a readiness_score was found")
    return entries


def crate_of(label: str) -> str:
    """The parenthesised path at the end of a scoreboard label."""
    found = re.search(r"\(([^()]+)\)\s*$", label)
    return found.group(1).strip() if found else ""


def main() -> int:
    if not REGISTRY.is_file():
        print(f"FAIL: registry not found at {REGISTRY}", file=sys.stderr)
        return 2
    if not STATUS.is_file():
        print(f"FAIL: status document not found at {STATUS}", file=sys.stderr)
        return 2

    try:
        entries = parse_registry(REGISTRY.read_text(encoding="utf-8"))
    except RegistryError as err:
        print(f"FAIL: {err} in {REGISTRY}", file=sys.stderr)
        return 2

    status = STATUS.read_text(encoding="utf-8")
    count = len(entries)
    average = sum(int(e["score"]) for e in entries) / count

    print(f"--- Recomputing from {REGISTRY.name} ---")
    print(f"  {count} entries, average {average:.2f}%")

    violations: list[str] = []

    # 1. Header: "<pct>% registry-scored readiness across <n> entries".
    header = HEADER.search(status)
    if not header:
        violations.append(
            "the header carries no '<pct>% registry-scored readiness across <n> entries' claim"
        )
    else:
        if int(header.group("n")) != count:
            violations.append(
                f"header claims {header.group('n')} entries, registry holds {count}"
            )
        if abs(float(header.group("pct")) - average) > 0.005:
            violations.append(
                f"header claims {header.group('pct')}%, registry average is {average:.2f}%"
            )

    # 2. Aggregate line.
    avg_line = AVERAGE.search(status)
    if not avg_line:
        violations.append("the '**Average readiness: ...** — N tracked features.' line is absent")
    else:
        if int(avg_line.group("n")) != count:
            violations.append(
                f"'Average readiness' line claims {avg_line.group('n')} features, registry holds {count}"
            )
        if abs(float(avg_line.group("pct")) - average) > 0.005:
            violations.append(
                f"'Average readiness' line claims {avg_line.group('pct')}%, registry average is {average:.2f}%"
            )

    # 3 and 4. Per-feature scores, matched by crate path.
    claimed: dict[str, list[tuple[int, str, str]]] = {}
    for line in SCORE_LINE.finditer(status):
        label = line.group("label").strip()
        crate = crate_of(label)
        claimed.setdefault(crate, []).append(
            (int(line.group("score")), line.group("mode"), label)
        )

    for entry in entries:
        key, score, mode, crate = (
            str(entry["key"]),
            int(entry["score"]),
            str(entry["mode"]),
            str(entry["crate"]),
        )
        rows = claimed.get(crate, [])
        if not rows:
            violations.append(
                f"{key} ({crate or 'no crate path'}) has no scoreboard line at all"
            )
            continue
        if not any(row[0] == score for row in rows):
            shown = ", ".join(f"{row[0]}%" for row in rows)
            violations.append(
                f"{key} ({crate}) is {score}% in the registry but the document shows {shown}"
            )
            continue
        matching_mode = [row for row in rows if row[0] == score and row[1] == mode]
        if not matching_mode:
            shown = ", ".join(f"{row[0]}% {row[1]}" for row in rows if row[0] == score)
            violations.append(
                f"{key} ({crate}) is mode {mode} in the registry but the document shows {shown}"
            )

    # A scoreboard line whose path the registry does not know at all is a phantom row.
    known_crates = {str(entry["crate"]) for entry in entries}
    for crate, rows in claimed.items():
        if crate and crate not in known_crates:
            for score, _mode, label in rows:
                violations.append(
                    f"scoreboard row '{label}' cites {score}% for {crate}, which is not a registry entry"
                )

    print(f"--- Checking {STATUS.name} ---")

    # Every other document that restates the aggregate.
    for claim in LIVE_CLAIMS:
        rel = str(claim["path"])
        doc_path = ROOT / rel
        if not doc_path.is_file():
            # The claim table names this file; its absence means the guard's own
            # configuration is wrong, which is not a pass.
            violations.append(f"{rel} is named by the claim table but does not exist")
            continue
        text = doc_path.read_text(encoding="utf-8")
        found = re.search(str(claim["pattern"]), text)
        if not found:
            violations.append(
                f"{rel} no longer carries its {claim['what']} in a form this check can read"
            )
            continue
        pct = float(found.group("pct"))
        if abs(pct - average) > 0.005:
            violations.append(
                f"{rel} ({claim['what']}) claims {found.group('pct')}%, registry average is {average:.2f}%"
            )
        groups = found.groupdict()
        if not claim.get("count_optional") and groups.get("n"):
            if int(groups["n"]) != count:
                violations.append(
                    f"{rel} ({claim['what']}) claims {groups['n']} entries, registry holds {count}"
                )

    for violation in violations:
        print(f"  VIOLATION: {violation}")

    print()
    print("=" * 40)
    if violations:
        print(
            f"FAIL: {len(violations)} disagreement(s). Regenerate the status document from "
            f"FEATURE_REGISTRY.toml, or revert the registry change."
        )
        return 1

    print(f"PASS: {STATUS.name} and {len(LIVE_CLAIMS)} other claim(s) are current with FEATURE_REGISTRY.toml.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
