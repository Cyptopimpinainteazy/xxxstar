#!/usr/bin/env bash
# Gate 15: pin .launchops/runtime_rpc_inventory.json so the runtime-trait
# surface (what the runtime exposes to the node via sp_api / RuntimeApi impls)
# cannot change silently.
#
# Fingerprint per trait covers:
#   trait_name, source_file, cfg_guard, method_names (sorted), method_count
# Line numbers are EXCLUDED on purpose — they are noise; any unrelated edit
# in lib.rs would otherwise trip the gate.
#
# Totals pinned as hard numbers: total_traits, traits_with_methods,
# total_methods. A count mismatch errors before the per-trait diff runs.
#
# UPDATE_BASELINE=1 regenerates, preserving accepted_reason where the
# per-trait fingerprint is unchanged (keyed by trait_name).
set -euo pipefail

ART=".launchops/runtime_rpc_inventory.json"
BASE=".launchops/runtime_rpc_inventory.baseline.json"

if [[ ! -f "$ART" ]]; then
  echo "::error::missing $ART"; exit 1
fi
if [[ ! -f "$BASE" ]]; then
  echo "::error::missing $BASE"; exit 1
fi

python3 - "$ART" "$BASE" <<'PY'
import json, sys, os, hashlib

art_path, base_path = sys.argv[1], sys.argv[2]
art = json.load(open(art_path))
base = json.load(open(base_path))
update = os.environ.get("UPDATE_BASELINE") == "1"

def fingerprint(payload):
    blob = json.dumps(payload, sort_keys=True).encode()
    return hashlib.sha256(blob).hexdigest()[:16]

def norm_trait(t):
    methods = sorted(m["name"] for m in t.get("methods", []))
    return {
        "trait_name": t["trait_name"],
        "source_file": t.get("source_file"),
        "cfg_guard": t.get("cfg_guard"),
        "method_names": methods,
        "method_count": len(methods),
    }

traits = art.get("runtime_traits", [])
current = {}
for t in traits:
    n = norm_trait(t)
    current[n["trait_name"]] = {
        "trait_name": n["trait_name"],
        "source_file": n["source_file"],
        "cfg_guard": n["cfg_guard"],
        "method_count": n["method_count"],
        "fingerprint": fingerprint(n),
    }

totals_current = {
    "total_traits": len(traits),
    "traits_with_methods": sum(1 for t in traits if t.get("methods")),
    "total_methods": sum(len(t.get("methods", [])) for t in traits),
}

if update:
    prior = {e["trait_name"]: e for e in base.get("traits", [])}
    new_traits = []
    for name in sorted(current):
        cur = current[name]
        prev = prior.get(name)
        accepted = ""
        if prev and prev.get("fingerprint") == cur["fingerprint"]:
            accepted = prev.get("accepted_reason", "")
        new_traits.append({**cur, "accepted_reason": accepted})
    out = {
        "_comment": "Pin of runtime_rpc_inventory.json. Per-trait fingerprint over {trait_name, source_file, cfg_guard, method_names sorted, method_count}. Line numbers excluded.",
        "totals": totals_current,
        "traits": new_traits,
    }
    with open(base_path, "w") as f:
        json.dump(out, f, indent=2, sort_keys=False)
        f.write("\n")
    print(f"baseline rewritten: {len(new_traits)} traits pinned "
          f"(total_traits={totals_current['total_traits']}, "
          f"with_methods={totals_current['traits_with_methods']}, "
          f"total_methods={totals_current['total_methods']})")
    sys.exit(0)

# verify mode
totals_base = base.get("totals", {})
totals_drift = []
for k, v in totals_current.items():
    bv = totals_base.get(k)
    if bv != v:
        totals_drift.append(f"{k} drifted: baseline={bv} current={v}")
for line in totals_drift:
    print(f"::error::{line}")

prior = {e["trait_name"]: e for e in base.get("traits", [])}
cur_names = set(current)
base_names = set(prior)

added = sorted(cur_names - base_names)
removed = sorted(base_names - cur_names)
drifted = sorted(
    n for n in cur_names & base_names
    if current[n]["fingerprint"] != prior[n]["fingerprint"]
)

fail = False
if totals_drift:
    fail = True
if added:
    print(f"::error::{len(added)} new runtime trait(s) not in baseline: " + ", ".join(added[:3]))
    fail = True
if removed:
    print(f"::error::{len(removed)} runtime trait(s) disappeared: " + ", ".join(removed[:3]))
    fail = True
if drifted:
    print(f"::error::{len(drifted)} runtime trait(s) drifted: " + ", ".join(drifted[:3]))
    fail = True

if fail:
    print("hint: review the changes, then regenerate with UPDATE_BASELINE=1 and fill in accepted_reason for each changed trait.")
    sys.exit(1)

print(f"runtime_rpc_inventory.json matches baseline "
      f"({totals_current['total_traits']} traits, "
      f"{totals_current['traits_with_methods']} with methods, "
      f"{totals_current['total_methods']} methods pinned)")
PY
