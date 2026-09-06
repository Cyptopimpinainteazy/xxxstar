#!/usr/bin/env bash
# Verify .launchops/requirement_conflicts.json against pinned baseline.
#
# Shape: flat list of 515 entries, each:
#   {
#     severity, conflict_type, reason,
#     requirement_a: {file, line, text, status, module},
#     requirement_b: {file, line, text, status, module}
#   }
#
# Scope: this gate covers only SHIPPED documentation conflicts. Entries
# whose requirement_a.file OR requirement_b.file lives under
# .kilo/worktrees/ are chat-transcript scratch from the AI assistant;
# they churn every session and are skipped here. That's 484 of the 515
# current entries. The remaining 31 are real "doc A says complete / doc
# B says incomplete" conflicts in crates/, PHASE_*, PROGRESS.md, etc.
#
# Fingerprint payload (order-stable, sides sorted by (file, line)):
#   {
#     conflict_type, severity, reason,
#     side_lo: {file, line, text, status, module},
#     side_hi: {file, line, text, status, module},
#   }
# Key for uniqueness: "lo_file:lo_line::hi_file:hi_line::conflict_type".
#
# UPDATE_BASELINE=1 regenerates the baseline, preserving accepted_reason
# when a key's fingerprint is unchanged.
#
# Fails CI with ::error:: + exit 1 on any:
#   - new real conflict not in baseline
#   - pinned conflict disappeared from scan
#   - drifted entry (text/status/reason changed for existing key)
set -euo pipefail

SCAN_FILE="${1:-.launchops/requirement_conflicts.json}"
BASELINE_FILE="${2:-.launchops/requirement_conflicts.baseline.json}"

if [[ ! -f "$SCAN_FILE" ]]; then
  echo "::error::missing $SCAN_FILE" >&2
  exit 1
fi
if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "::error::missing baseline $BASELINE_FILE" >&2
  exit 1
fi

python3 - "$SCAN_FILE" "$BASELINE_FILE" <<'PY'
import hashlib
import json
import os
import sys
from pathlib import Path

scan_path = Path(sys.argv[1])
baseline_path = Path(sys.argv[2])

scan = json.loads(scan_path.read_text())
baseline = json.loads(baseline_path.read_text())

if not isinstance(scan, list):
    print(f"::error::{scan_path} must be a JSON list", file=sys.stderr)
    sys.exit(1)

SIDE_FIELDS = ("file", "line", "text", "status", "module")
TOP_FIELDS = ("conflict_type", "severity", "reason", "requirement_a", "requirement_b")


def is_kilo(e):
    a = e.get("requirement_a", {}).get("file", "")
    b = e.get("requirement_b", {}).get("file", "")
    return ".kilo/" in a or ".kilo/" in b


def canonicalize(e):
    """Return (key, payload) with sides sorted by (file, line) so A/B
    ordering does not perturb the fingerprint."""
    a = {k: e["requirement_a"].get(k) for k in SIDE_FIELDS}
    b = {k: e["requirement_b"].get(k) for k in SIDE_FIELDS}
    lo, hi = sorted([a, b], key=lambda x: (str(x.get("file", "")), x.get("line", 0)))
    key = (
        f"{lo.get('file')}:{lo.get('line')}"
        f"::{hi.get('file')}:{hi.get('line')}"
        f"::{e.get('conflict_type')}"
    )
    payload = {
        "conflict_type": e.get("conflict_type"),
        "severity": e.get("severity"),
        "reason": e.get("reason"),
        "side_lo": lo,
        "side_hi": hi,
    }
    return key, payload


def fingerprint(payload):
    return hashlib.sha256(
        json.dumps(payload, sort_keys=True).encode("utf-8")
    ).hexdigest()[:16]


# Pre-check scan shape
for i, e in enumerate(scan):
    if not isinstance(e, dict):
        print(f"::error::scan entry {i} not an object", file=sys.stderr)
        sys.exit(1)
    for f in ("conflict_type", "severity", "requirement_a", "requirement_b"):
        if f not in e:
            print(f"::error::scan entry {i} missing field {f!r}", file=sys.stderr)
            sys.exit(1)

real = [e for e in scan if not is_kilo(e)]
kilo_count = len(scan) - len(real)

scan_index = {}
for e in real:
    k, p = canonicalize(e)
    if k in scan_index:
        print(f"::error::duplicate key in {scan_path}: {k}", file=sys.stderr)
        sys.exit(1)
    scan_index[k] = p

if os.environ.get("UPDATE_BASELINE") == "1":
    old_by_key = {}
    if isinstance(baseline, dict) and isinstance(baseline.get("conflicts"), list):
        for b in baseline["conflicts"]:
            if isinstance(b, dict) and "key" in b:
                old_by_key[b["key"]] = b

    out = []
    for k in sorted(scan_index):
        p = scan_index[k]
        fp = fingerprint(p)
        prev = old_by_key.get(k, {})
        accepted = (
            prev.get("accepted_reason", "")
            if prev.get("fingerprint") == fp
            else ""
        )
        out.append(
            {
                "key": k,
                "conflict_type": p["conflict_type"],
                "severity": p["severity"],
                "side_lo_file": p["side_lo"].get("file"),
                "side_lo_line": p["side_lo"].get("line"),
                "side_hi_file": p["side_hi"].get("file"),
                "side_hi_line": p["side_hi"].get("line"),
                "fingerprint": fp,
                "accepted_reason": accepted,
            }
        )

    baseline_path.write_text(
        json.dumps(
            {
                "_comment": (
                    "Pinned fingerprints for .launchops/requirement_conflicts.json. "
                    "Only SHIPPED-doc conflicts are gated; .kilo/worktrees/* entries "
                    "are skipped as chat-transcript noise. Regenerate with "
                    "UPDATE_BASELINE=1 and fill accepted_reason during review."
                ),
                "ignored_kilo_entries": kilo_count,
                "conflicts": out,
            },
            indent=2,
            sort_keys=False,
        )
        + "\n"
    )
    print(
        f"baseline rewritten: {len(out)} real conflicts pinned "
        f"({kilo_count} .kilo entries ignored)"
    )
    sys.exit(0)

if not isinstance(baseline, dict) or not isinstance(baseline.get("conflicts"), list):
    print(
        f"::error::{baseline_path} must be an object with a 'conflicts' list; "
        "run UPDATE_BASELINE=1 to seed.",
        file=sys.stderr,
    )
    sys.exit(1)

base_by_key = {
    b["key"]: b for b in baseline["conflicts"] if isinstance(b, dict) and "key" in b
}

scan_keys = set(scan_index)
base_keys = set(base_by_key)

new_keys = sorted(scan_keys - base_keys)
removed_keys = sorted(base_keys - scan_keys)
drifted = []
for k in sorted(scan_keys & base_keys):
    if fingerprint(scan_index[k]) != base_by_key[k].get("fingerprint"):
        drifted.append(k)

errors = 0
if new_keys:
    print(
        f"::error::{len(new_keys)} new requirement_conflicts entry/entries not in baseline: "
        + ", ".join(new_keys[:3])
        + (" ..." if len(new_keys) > 3 else ""),
        file=sys.stderr,
    )
    errors += 1
if removed_keys:
    print(
        f"::error::{len(removed_keys)} requirement_conflicts entry/entries disappeared: "
        + ", ".join(removed_keys[:3])
        + (" ..." if len(removed_keys) > 3 else ""),
        file=sys.stderr,
    )
    errors += 1
if drifted:
    print(
        f"::error::{len(drifted)} requirement_conflicts entry/entries drifted: "
        + ", ".join(drifted[:3])
        + (" ..." if len(drifted) > 3 else ""),
        file=sys.stderr,
    )
    errors += 1

if errors:
    print(
        "hint: review the changes, then regenerate with UPDATE_BASELINE=1 "
        "and fill in accepted_reason for each entry.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"requirement_conflicts.json matches baseline "
    f"({len(scan_index)} real conflicts, {kilo_count} .kilo entries ignored)"
)
PY
