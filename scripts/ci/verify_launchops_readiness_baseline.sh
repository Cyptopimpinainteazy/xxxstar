#!/usr/bin/env bash
# LaunchOps readiness regression gate.
#
# `launchops scan` emits .launchops/readiness.json with a weighted scan_score,
# per-module breakdown, and a BLOCKED/READY status. Today we are BLOCKED with
# scan_score 60.63 (canonical hard-gate readiness 20.8% per scorecard
# 2026-04-22). Failing CI on status=BLOCKED directly would red-light every
# run, so this gate uses the same baseline pattern as gates_baseline and
# deep_red_flags: enforce no regression, not perfection.
#
# Failure modes:
#   * scan_score drops by more than READINESS_TOLERANCE (default 0.5) from
#     the baselined scan_score_floor.
#   * any baselined module drops by more than READINESS_MODULE_TOLERANCE
#     (default 1.0) from its baselined score.
#   * status regresses from non-BLOCKED -> BLOCKED.
#   * a baselined module disappears from the scan (definition drift).
#   * X3_REQUIRE_READINESS_UNBLOCKED=1 and scan status is BLOCKED.
#
# Advisory-only (stderr warning, exit 0):
#   * scan_score improves by more than READINESS_TOLERANCE -> lock in with
#     UPDATE_BASELINE=1.
#   * any module improves by more than READINESS_MODULE_TOLERANCE.
#   * status flips BLOCKED -> READY.
#   * a new module appears in readiness.json that isn't in the baseline.
set -euo pipefail

readiness=${READINESS_FILE:-.launchops/readiness.json}
baseline=${BASELINE_FILE:-.launchops/readiness.baseline.json}
update_baseline=${UPDATE_BASELINE:-0}
require_unblocked=${X3_REQUIRE_READINESS_UNBLOCKED:-0}
score_tolerance=${READINESS_TOLERANCE:-0.5}
module_tolerance=${READINESS_MODULE_TOLERANCE:-1.0}

if [[ ! -f "$readiness" ]]; then
  echo "::error::missing $readiness; run 'cargo run -p launchops -- scan' first"
  exit 1
fi
if [[ ! -f "$baseline" ]]; then
  echo "::error::missing $baseline; seed it with UPDATE_BASELINE=1 $0"
  exit 1
fi

export READINESS_PATH="$readiness"
export BASELINE_PATH="$baseline"
export UPDATE_FLAG="$update_baseline"
export REQUIRE_UNBLOCKED="$require_unblocked"
export SCORE_TOL="$score_tolerance"
export MODULE_TOL="$module_tolerance"

python3 - <<'PY'
import json
import os
import sys

readiness_path = os.environ["READINESS_PATH"]
baseline_path = os.environ["BASELINE_PATH"]
update_flag = os.environ["UPDATE_FLAG"]
require_unblocked = os.environ["REQUIRE_UNBLOCKED"]
score_tol = float(os.environ["SCORE_TOL"])
module_tol = float(os.environ["MODULE_TOL"])

with open(readiness_path, encoding="utf-8") as handle:
    scanned = json.load(handle)
with open(baseline_path, encoding="utf-8") as handle:
    baseline = json.load(handle)

scan_score = float(scanned.get("scan_score", 0.0))
scan_status = scanned.get("status", "UNKNOWN")
scan_modules = {k: float(v) for k, v in scanned.get("module_breakdown", {}).items()}

baseline_score = float(baseline.get("scan_score_floor", 0.0))
baseline_status = baseline.get("status", "UNKNOWN")
baseline_modules = {k: float(v) for k, v in baseline.get("module_breakdown_floor", {}).items()}

if update_flag == "1":
    updated = {k: v for k, v in baseline.items() if k not in ("scan_score_floor", "module_breakdown_floor", "status")}
    updated["status"] = scan_status
    updated["scan_score_floor"] = scan_score
    updated["module_breakdown_floor"] = dict(sorted(scan_modules.items()))
    with open(baseline_path, "w", encoding="utf-8") as handle:
        json.dump(updated, handle, indent=2)
        handle.write("\n")
    print(
        f"LaunchOps readiness baseline rewritten: scan_score={scan_score} "
        f"status={scan_status} modules={len(scan_modules)}"
    )
    sys.exit(0)

failures = []
warnings = []

# Score regression.
if scan_score + score_tol < baseline_score:
    failures.append(
        f"scan_score regressed: baseline_floor={baseline_score:.2f} "
        f"actual={scan_score:.2f} tolerance={score_tol:.2f}"
    )
elif scan_score > baseline_score + score_tol:
    warnings.append(
        f"scan_score improved: baseline_floor={baseline_score:.2f} -> "
        f"actual={scan_score:.2f}; lock in with UPDATE_BASELINE=1"
    )

# Status regression: only non-BLOCKED -> BLOCKED is a failure.
if baseline_status != "BLOCKED" and scan_status == "BLOCKED":
    failures.append(
        f"status regressed: baseline={baseline_status} -> actual=BLOCKED"
    )
elif baseline_status == "BLOCKED" and scan_status != "BLOCKED":
    warnings.append(
        f"status improved: baseline=BLOCKED -> actual={scan_status}; "
        "lock in with UPDATE_BASELINE=1"
    )
elif baseline_status != scan_status:
    warnings.append(
        f"status changed: baseline={baseline_status} actual={scan_status}"
    )

# Optional strict mode.
if require_unblocked == "1" and scan_status == "BLOCKED":
    failures.append(
        "X3_REQUIRE_READINESS_UNBLOCKED=1 and status=BLOCKED; "
        "this build requires readiness to be unblocked"
    )

# Module-by-module.
for module, baseline_value in baseline_modules.items():
    if module not in scan_modules:
        failures.append(
            f"module '{module}' baselined at {baseline_value:.2f} is missing "
            "from readiness.json; definition drift, refresh baseline"
        )
        continue
    actual_value = scan_modules[module]
    if actual_value + module_tol < baseline_value:
        failures.append(
            f"module '{module}' regressed: baseline_floor={baseline_value:.2f} "
            f"actual={actual_value:.2f} tolerance={module_tol:.2f}"
        )
    elif actual_value > baseline_value + module_tol:
        warnings.append(
            f"module '{module}' improved: baseline_floor={baseline_value:.2f} "
            f"-> actual={actual_value:.2f}; lock in with UPDATE_BASELINE=1"
        )

# New modules.
for module in scan_modules:
    if module not in baseline_modules:
        warnings.append(
            f"new module '{module}' appeared with score "
            f"{scan_modules[module]:.2f}; baseline it with UPDATE_BASELINE=1"
        )

if failures:
    print(f"::error::{len(failures)} LaunchOps readiness regression(s)")
    for msg in failures:
        print(f"  - {msg}")
    sys.exit(1)

for msg in warnings:
    print(f"::warning::{msg}")

print(
    f"LaunchOps readiness baseline: scan_score={scan_score:.2f} "
    f"(floor {baseline_score:.2f}), status={scan_status} "
    f"(baseline {baseline_status}), {len(scan_modules)} module(s) match or "
    "exceed baseline floor"
)
PY
