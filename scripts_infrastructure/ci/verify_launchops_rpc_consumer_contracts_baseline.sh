#!/usr/bin/env bash
# Gate 14: pin .launchops/rpc_consumer_contracts.json against a fingerprint baseline.
# Each method is fingerprinted over its bucket, consumer modes, registration count,
# runtime trait hints, node-local signals, and notes. Any change to a method's
# contract (e.g. bucket flip from runtime_backed to placeholder, or a consumer
# mode change from direct_read_candidate to mock_only) fails CI with exit 1 and
# requires an accepted_reason in the baseline.
#
# Totals (frontend_safe_count, sidecar_only_count, mock_only_count) are pinned
# as hard numbers — they must match the baseline exactly.
#
# Run with UPDATE_BASELINE=1 to regenerate while preserving accepted_reason
# where the fingerprint for a given method is unchanged.
set -euo pipefail

ART=".launchops/rpc_consumer_contracts.json"
BASE=".launchops/rpc_consumer_contracts.baseline.json"

if [[ ! -f "$ART" ]]; then
  echo "::error::missing $ART"
  exit 1
fi
if [[ ! -f "$BASE" ]]; then
  echo "::error::missing $BASE (seed with UPDATE_BASELINE=1)"
  exit 1
fi

python3 - "$ART" "$BASE" <<'PY'
import hashlib
import json
import os
import sys

art_path, base_path = sys.argv[1], sys.argv[2]
art = json.load(open(art_path))
base = json.load(open(base_path))

def fp(method_obj):
    payload = {
        "method": method_obj["method"],
        "bucket": method_obj["bucket"],
        "frontend_consumer_mode": method_obj["frontend_consumer_mode"],
        "sidecar_consumer_mode": method_obj["sidecar_consumer_mode"],
        "registration_count": method_obj["registration_count"],
        "runtime_trait_hints": sorted(method_obj.get("runtime_trait_hints", [])),
        "node_local_signals": sorted(method_obj.get("node_local_signals", [])),
        "notes": method_obj.get("notes", []),
    }
    s = json.dumps(payload, sort_keys=True).encode()
    return hashlib.sha256(s).hexdigest()[:16]

def all_methods(art):
    out = []
    for group in ("frontend_safe_methods", "sidecar_only_methods", "mock_only_methods"):
        for m in art.get(group, []):
            out.append((m["method"], m, group))
    return out

cur_methods = all_methods(art)
cur_keys = {m[0] for m in cur_methods}
cur_by_key = {m[0]: m[1] for m in cur_methods}
cur_group = {m[0]: m[2] for m in cur_methods}

cur_totals = {
    "frontend_safe_count": art.get("frontend_safe_count", 0),
    "sidecar_only_count": art.get("sidecar_only_count", 0),
    "mock_only_count": art.get("mock_only_count", 0),
}

if os.environ.get("UPDATE_BASELINE") == "1":
    old_by_key = {e["method"]: e for e in base.get("methods", [])}
    new_methods = []
    for method_name, obj, group in sorted(cur_methods):
        f = fp(obj)
        old = old_by_key.get(method_name, {})
        accepted = old.get("accepted_reason", "")
        if old.get("fingerprint") and old.get("fingerprint") != f:
            accepted = ""
        new_methods.append({
            "method": method_name,
            "group": group,
            "bucket": obj["bucket"],
            "frontend_consumer_mode": obj["frontend_consumer_mode"],
            "sidecar_consumer_mode": obj["sidecar_consumer_mode"],
            "fingerprint": f,
            "accepted_reason": accepted,
        })
    out = {
        "_comment": (
            "Gate 14: per-method contract fingerprints for "
            ".launchops/rpc_consumer_contracts.json. Any change to a method's "
            "bucket, consumer modes, trait hints, node-local signals, or notes "
            "fails CI. Totals are pinned exactly. Regenerate with "
            "UPDATE_BASELINE=1 and fill accepted_reason for each change."
        ),
        "totals": cur_totals,
        "methods": new_methods,
    }
    json.dump(out, open(base_path, "w"), indent=2, sort_keys=True)
    print(f"baseline rewritten: {len(new_methods)} methods pinned "
          f"(frontend_safe={cur_totals['frontend_safe_count']}, "
          f"sidecar_only={cur_totals['sidecar_only_count']}, "
          f"mock_only={cur_totals['mock_only_count']})")
    sys.exit(0)

# verify
base_totals = base.get("totals", {})
for k in ("frontend_safe_count", "sidecar_only_count", "mock_only_count"):
    if cur_totals[k] != base_totals.get(k):
        print(f"::error::{k} drifted: baseline={base_totals.get(k)} current={cur_totals[k]}")
        print("hint: review the changes, then regenerate with UPDATE_BASELINE=1 "
              "and fill in accepted_reason for each changed method.")
        sys.exit(1)

base_by_key = {e["method"]: e for e in base.get("methods", [])}
base_keys = set(base_by_key)

added = sorted(cur_keys - base_keys)
removed = sorted(base_keys - cur_keys)
drifted = []
for k in cur_keys & base_keys:
    if fp(cur_by_key[k]) != base_by_key[k]["fingerprint"]:
        drifted.append(k)
drifted.sort()

failed = False
if added:
    print(f"::error::{len(added)} new rpc_consumer_contracts method(s) not in baseline: "
          + ", ".join(added[:3]) + ("..." if len(added) > 3 else ""))
    failed = True
if removed:
    print(f"::error::{len(removed)} rpc_consumer_contracts method(s) disappeared: "
          + ", ".join(removed[:3]) + ("..." if len(removed) > 3 else ""))
    failed = True
if drifted:
    print(f"::error::{len(drifted)} rpc_consumer_contracts method(s) drifted: "
          + ", ".join(drifted[:3]) + ("..." if len(drifted) > 3 else ""))
    failed = True

if failed:
    print("hint: review the changes, then regenerate with UPDATE_BASELINE=1 "
          "and fill in accepted_reason for each changed method.")
    sys.exit(1)

print(f"rpc_consumer_contracts.json matches baseline "
      f"({len(cur_methods)} methods, totals pinned)")
PY
