#!/usr/bin/env bash
# Hard-invariant gate for .launchops/feature_matrix.json
#
# The feature matrix contains >12k per-feature rows — too large for a
# per-item fingerprint. Instead we pin the GOVERNANCE-RELEVANT AGGREGATES:
#
#   1. total feature count
#   2. status histogram (tested / implemented / missing / specified_only)
#   3. risk histogram (low / medium / high / critical)
#   4. module histogram (per-module feature count)
#   5. status x risk cross-tab (catches silent risk escalations on
#      incomplete features — e.g., a "critical" feature silently flipping
#      to "missing" without review)
#
# Any bucket delta errors with an actionable message. UPDATE_BASELINE=1
# regenerates the baseline.
set -euo pipefail

CURRENT=".launchops/feature_matrix.json"
BASELINE=".launchops/feature_matrix.baseline.json"

if [[ ! -f "$CURRENT" ]]; then
  echo "::error::$CURRENT missing"
  exit 1
fi

if [[ "${UPDATE_BASELINE:-0}" != "1" && ! -f "$BASELINE" ]]; then
  echo "::error::$BASELINE missing (run with UPDATE_BASELINE=1 to seed)"
  exit 1
fi

python3 - "$CURRENT" "$BASELINE" <<'PY'
import json, os, sys
from collections import Counter

current_path, baseline_path = sys.argv[1], sys.argv[2]

with open(current_path) as f:
    matrix = json.load(f)

if not isinstance(matrix, list):
    print(f"::error::{current_path} must be a JSON array")
    sys.exit(1)

# build distributions
total = len(matrix)
status = Counter()
risk = Counter()
module = Counter()
status_risk = Counter()

for i, row in enumerate(matrix):
    if not isinstance(row, dict):
        print(f"::error::row {i} is not an object")
        sys.exit(1)
    for k in ("feature", "status", "risk", "module"):
        if k not in row:
            print(f"::error::row {i} missing key '{k}'")
            sys.exit(1)
    s = row["status"]; r = row["risk"]; m = row["module"]
    status[s] += 1
    risk[r] += 1
    module[m] += 1
    status_risk[f"{s}|{r}"] += 1

def sorted_dict(c):
    return {k: c[k] for k in sorted(c)}

current = {
    "total": total,
    "status": sorted_dict(status),
    "risk": sorted_dict(risk),
    "module": sorted_dict(module),
    "status_risk": sorted_dict(status_risk),
}

update = os.environ.get("UPDATE_BASELINE") == "1"

if update:
    prior = {}
    if os.path.exists(baseline_path):
        try:
            prior = json.load(open(baseline_path))
        except Exception:
            prior = {}
    prior_reasons = prior.get("accepted_reasons", {}) if isinstance(prior, dict) else {}

    payload = {
        "_comment": (
            "Governance baseline for .launchops/feature_matrix.json. "
            "Pins total feature count and distributions across status, risk, "
            "module, and status x risk. Any bucket delta requires review. "
            "Add a key under accepted_reasons keyed by '<dimension>:<bucket>' "
            "when the delta is intentional. Regenerate with UPDATE_BASELINE=1."
        ),
        "total": current["total"],
        "status": current["status"],
        "risk": current["risk"],
        "module": current["module"],
        "status_risk": current["status_risk"],
        "accepted_reasons": prior_reasons,
    }
    with open(baseline_path, "w") as f:
        json.dump(payload, f, indent=2, sort_keys=False)
        f.write("\n")
    print(
        f"baseline rewritten: total={total}, "
        f"status_buckets={len(status)}, risk_buckets={len(risk)}, "
        f"module_buckets={len(module)}, status_risk_cells={len(status_risk)}"
    )
    sys.exit(0)

baseline = json.load(open(baseline_path))

errors = []

# total
if baseline.get("total") != current["total"]:
    errors.append(
        f"total drifted: baseline={baseline.get('total')} current={current['total']}"
    )

# per-dimension diffs
for dim in ("status", "risk", "module", "status_risk"):
    base_d = baseline.get(dim, {}) or {}
    cur_d = current[dim]
    all_keys = sorted(set(base_d) | set(cur_d))
    for k in all_keys:
        bv = base_d.get(k)
        cv = cur_d.get(k)
        if bv is None and cv is not None:
            errors.append(f"{dim}:{k} new bucket appeared (count={cv})")
        elif cv is None and bv is not None:
            errors.append(f"{dim}:{k} bucket disappeared (baseline count={bv})")
        elif bv != cv:
            errors.append(f"{dim}:{k} drifted: baseline={bv} current={cv}")

if errors:
    for e in errors:
        print(f"::error::{e}")
    print(
        "hint: review the feature matrix changes, then regenerate with "
        "UPDATE_BASELINE=1 and add an entry under accepted_reasons "
        "keyed by '<dimension>:<bucket>' explaining each delta."
    )
    sys.exit(1)

print(
    f"feature_matrix.json matches baseline "
    f"(total={total}, status={len(status)} buckets, risk={len(risk)} buckets, "
    f"module={len(module)} buckets, status_risk={len(status_risk)} cells)"
)
PY
