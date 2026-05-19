#!/usr/bin/env bash
# Verify .launchops/red_flags.json against pinned baseline.
#
# Shape of red_flags.json: flat list of 500 entries, each:
#   { file, line, pattern, reason, severity }
#
# Gate: every entry must be present in .launchops/red_flags.baseline.json
# with an unchanged fingerprint. A fingerprint is sha256 over
# sort_keys JSON of {file, line, pattern, reason, severity}, truncated
# to 16 hex chars. Each baseline entry also carries accepted_reason
# which a human fills in during review.
#
# UPDATE_BASELINE=1 regenerates the baseline, preserving accepted_reason
# when the fingerprint for a given (file, line, pattern, severity) key
# is unchanged.
#
# Fails CI with ::error:: + exit 1 on any:
#   - new red_flag not in baseline (review required)
#   - red_flag disappeared from scan (baseline must be re-pinned)
#   - drifted entry (reason changed for existing key)
#
# Line shifts are NOT ignored: if a flagged line number changes, that
# counts as drift. Re-run with UPDATE_BASELINE=1 after reviewing the
# move.
set -euo pipefail

SCAN_FILE="${1:-.launchops/red_flags.json}"
BASELINE_FILE="${2:-.launchops/red_flags.baseline.json}"

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

REQ = {"file", "line", "pattern", "reason", "severity"}

def key_of(e):
    return f"{e['file']}::{e['line']}::{e['pattern']}::{e['severity']}"

def fingerprint(e):
    payload = {k: e[k] for k in sorted(REQ)}
    return hashlib.sha256(
        json.dumps(payload, sort_keys=True).encode("utf-8")
    ).hexdigest()[:16]

# Pre-check scan shape
for i, e in enumerate(scan):
    if not isinstance(e, dict) or not REQ.issubset(e.keys()):
        print(
            f"::error::scan entry {i} missing required fields {REQ - set(e.keys() if isinstance(e, dict) else [])}",
            file=sys.stderr,
        )
        sys.exit(1)

scan_index = {}
for e in scan:
    k = key_of(e)
    if k in scan_index:
        print(
            f"::error::duplicate key in {scan_path}: {k}",
            file=sys.stderr,
        )
        sys.exit(1)
    scan_index[k] = e

if os.environ.get("UPDATE_BASELINE") == "1":
    # Preserve accepted_reason for keys whose fingerprint is unchanged
    old_by_key = {}
    if isinstance(baseline, dict) and isinstance(baseline.get("red_flags"), list):
        for b in baseline["red_flags"]:
            if isinstance(b, dict) and "key" in b:
                old_by_key[b["key"]] = b

    out = []
    for k in sorted(scan_index):
        e = scan_index[k]
        fp = fingerprint(e)
        prev = old_by_key.get(k, {})
        accepted = (
            prev.get("accepted_reason", "")
            if prev.get("fingerprint") == fp
            else ""
        )
        out.append(
            {
                "key": k,
                "file": e["file"],
                "line": e["line"],
                "pattern": e["pattern"],
                "severity": e["severity"],
                "fingerprint": fp,
                "accepted_reason": accepted,
            }
        )

    baseline_path.write_text(
        json.dumps(
            {
                "_comment": (
                    "Pinned fingerprints for .launchops/red_flags.json. "
                    "Regenerate with UPDATE_BASELINE=1. Fill accepted_reason "
                    "for each entry during review. Any new/removed/drifted "
                    "entry fails CI until this baseline is updated."
                ),
                "red_flags": out,
            },
            indent=2,
            sort_keys=False,
        )
        + "\n"
    )
    print(f"baseline rewritten: {len(out)} red_flags entries")
    sys.exit(0)

if not isinstance(baseline, dict) or not isinstance(baseline.get("red_flags"), list):
    print(
        f"::error::{baseline_path} must be an object with a 'red_flags' list; "
        "run UPDATE_BASELINE=1 to seed.",
        file=sys.stderr,
    )
    sys.exit(1)

base_by_key = {b["key"]: b for b in baseline["red_flags"] if isinstance(b, dict) and "key" in b}

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
        f"::error::{len(new_keys)} new red_flags entry/entries not in baseline: "
        + ", ".join(new_keys[:5])
        + (" ..." if len(new_keys) > 5 else ""),
        file=sys.stderr,
    )
    errors += 1
if removed_keys:
    print(
        f"::error::{len(removed_keys)} red_flags entry/entries disappeared: "
        + ", ".join(removed_keys[:5])
        + (" ..." if len(removed_keys) > 5 else ""),
        file=sys.stderr,
    )
    errors += 1
if drifted:
    print(
        f"::error::{len(drifted)} red_flags entry/entries drifted (reason changed): "
        + ", ".join(drifted[:5])
        + (" ..." if len(drifted) > 5 else ""),
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

print(f"red_flags.json matches baseline ({len(scan)} entries)")
PY
