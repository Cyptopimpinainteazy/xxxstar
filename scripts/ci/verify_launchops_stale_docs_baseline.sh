#!/usr/bin/env bash
# Pin .launchops/stale_docs.json against a signed baseline so any doc<->code
# drift change (new stale doc, removed stale doc, or altered linked_code /
# severity / reason for an existing stale doc) fails CI until reviewed and the
# baseline is deliberately regenerated.
#
# Set UPDATE_BASELINE=1 to rewrite the baseline, preserving any accepted_reason
# entries whose fingerprint still matches.
set -euo pipefail

CURRENT="${1:-.launchops/stale_docs.json}"
BASELINE="${2:-.launchops/stale_docs.baseline.json}"

if [[ ! -f "$CURRENT" ]]; then
  echo "::error::missing $CURRENT (run 'cargo run -p launchops -- scan')"
  exit 1
fi
if [[ ! -f "$BASELINE" && "${UPDATE_BASELINE:-0}" != "1" ]]; then
  echo "::error::missing $BASELINE (seed with UPDATE_BASELINE=1)"
  exit 1
fi

python3 - "$CURRENT" "$BASELINE" <<'PY'
import hashlib, json, os, sys

cur_path, base_path = sys.argv[1], sys.argv[2]

with open(cur_path) as fh:
    cur = json.load(fh)

if not isinstance(cur, list):
    print(f"::error::{cur_path} must be a JSON array")
    sys.exit(1)

def fingerprint(entry):
    payload = {
        "file": entry.get("file", ""),
        "severity": entry.get("severity", ""),
        "reason": entry.get("reason", ""),
        "linked_code": sorted(entry.get("linked_code") or []),
    }
    return hashlib.sha256(
        json.dumps(payload, sort_keys=True).encode("utf-8")
    ).hexdigest()[:16]

cur_map = {}
dupes = []
for entry in cur:
    f = entry.get("file")
    if not f:
        print("::error::stale_docs entry missing 'file'")
        sys.exit(1)
    if f in cur_map:
        dupes.append(f)
    cur_map[f] = fingerprint(entry)
if dupes:
    print(f"::error::duplicate file keys in {cur_path}: {sorted(set(dupes))[:10]}")
    sys.exit(1)

update = os.environ.get("UPDATE_BASELINE", "0") == "1"

prev_accept = {}
if os.path.exists(base_path):
    with open(base_path) as fh:
        prev = json.load(fh)
    for e in prev.get("stale_docs", []):
        prev_accept[e["file"]] = {
            "fingerprint": e.get("fingerprint", ""),
            "accepted_reason": e.get("accepted_reason", ""),
        }

if update:
    new_entries = []
    for f in sorted(cur_map):
        fp = cur_map[f]
        reason = ""
        prev = prev_accept.get(f)
        if prev and prev.get("fingerprint") == fp:
            reason = prev.get("accepted_reason", "")
        new_entries.append({
            "file": f,
            "fingerprint": fp,
            "accepted_reason": reason,
        })
    out = {
        "_comment": (
            "Pinned stale_docs.json fingerprints. Each entry captures sha256 "
            "over {file, severity, reason, sorted(linked_code)}. Any drift "
            "(new stale doc, removed stale doc, altered linked_code / severity "
            "/ reason) fails CI until this file is regenerated with "
            "UPDATE_BASELINE=1 and each accepted_reason is reviewed."
        ),
        "stale_docs": new_entries,
    }
    with open(base_path, "w") as fh:
        json.dump(out, fh, indent=2, sort_keys=False)
        fh.write("\n")
    print(f"baseline rewritten: {len(new_entries)} stale_docs entries")
    sys.exit(0)

with open(base_path) as fh:
    base = json.load(fh)
base_map = {e["file"]: e.get("fingerprint", "") for e in base.get("stale_docs", [])}

errors = []
added = sorted(set(cur_map) - set(base_map))
removed = sorted(set(base_map) - set(cur_map))
if added:
    errors.append(
        f"{len(added)} new stale_docs entry/entries not in baseline: "
        + ", ".join(added[:10])
        + (" ..." if len(added) > 10 else "")
    )
if removed:
    errors.append(
        f"{len(removed)} stale_docs entry/entries disappeared: "
        + ", ".join(removed[:10])
        + (" ..." if len(removed) > 10 else "")
    )

drifted = []
for f in sorted(set(cur_map) & set(base_map)):
    if cur_map[f] != base_map[f]:
        drifted.append(f)
if drifted:
    errors.append(
        f"{len(drifted)} stale_docs entry/entries drifted (linked_code/severity/reason changed): "
        + ", ".join(drifted[:10])
        + (" ..." if len(drifted) > 10 else "")
    )

if errors:
    for e in errors:
        print(f"::error::{e}")
    print("hint: review the changes, then regenerate with UPDATE_BASELINE=1 "
          "and fill in accepted_reason for each entry.")
    sys.exit(1)

print(f"stale_docs.json matches baseline ({len(cur_map)} entries)")
PY
