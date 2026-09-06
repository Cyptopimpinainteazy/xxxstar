#!/usr/bin/env bash
# LaunchOps hard-gate regression gate.
#
# `launchops scan` emits .launchops/gates.json with pass/fail status for the
# authoritative mainnet hard gates (workspace compiles, clippy clean, tests
# pass, no p0 blockers, etc.). Today several of these gates are failing with
# known owed work (scorecard Gate 6/7). Failing CI on the raw pass/fail
# would immediately red-light mainnet CI, so this gate uses the same
# baseline pattern as deep_red_flags: compare to the checked-in baseline
# and fail only on regressions.
#
# Failure modes:
#   * A gate with expected_status="pass" flipped to fail -> hard regression.
#   * A gate listed in the baseline is missing from gates.json -> definition
#     drift, baseline must be refreshed in the same PR.
#
# Advisory-only (stderr warning, exit 0):
#   * A gate with expected_status="fail" flipped to pass -> good news; run
#     with UPDATE_BASELINE=1 to lock in the higher floor.
#   * A new gate appeared in gates.json that isn't in the baseline -> new
#     signal; baseline it with UPDATE_BASELINE=1.
set -euo pipefail

gates=${GATES_FILE:-.launchops/gates.json}
baseline=${BASELINE_FILE:-.launchops/gates.baseline.json}
update_baseline=${UPDATE_BASELINE:-0}

if [[ ! -f "$gates" ]]; then
  echo "::error::missing $gates; run 'cargo run -p launchops -- scan' first"
  exit 1
fi
if [[ ! -f "$baseline" ]]; then
  echo "::error::missing $baseline; seed it with UPDATE_BASELINE=1 $0"
  exit 1
fi

python3 - "$gates" "$baseline" "$update_baseline" <<'PY'
import json
import sys

gates_path, baseline_path, update_flag = sys.argv[1], sys.argv[2], sys.argv[3]

with open(gates_path, encoding="utf-8") as handle:
    scanned_list = json.load(handle)
with open(baseline_path, encoding="utf-8") as handle:
    baseline = json.load(handle)

scanned = {g["id"]: g for g in scanned_list}
baselined = baseline.get("gates", {})

if update_flag == "1":
    merged = {}
    for gate_id in sorted(scanned):
        entry = scanned[gate_id]
        existing = baselined.get(gate_id, {})
        merged[gate_id] = {
            "expected_status": entry.get("status", "unknown"),
            "accepted_reason": existing.get(
                "accepted_reason",
                "TODO: document why this gate is at its current status at baseline update time",
            ),
        }
    updated = {k: v for k, v in baseline.items() if k != "gates"}
    updated["gates"] = merged
    with open(baseline_path, "w", encoding="utf-8") as handle:
        json.dump(updated, handle, indent=2)
        handle.write("\n")
    print(
        f"LaunchOps gates baseline rewritten: {len(merged)} gate(s) recorded"
    )
    sys.exit(0)

regressions = []
missing_from_scan = []
unexpected_passes = []
new_gates = []

for gate_id, expected in baselined.items():
    if gate_id not in scanned:
        missing_from_scan.append(gate_id)
        continue
    actual_status = scanned[gate_id].get("status")
    expected_status = expected.get("expected_status")
    if expected_status == "pass" and actual_status != "pass":
        regressions.append((gate_id, expected_status, actual_status, scanned[gate_id].get("reason")))
    elif expected_status == "fail" and actual_status == "pass":
        unexpected_passes.append(gate_id)

for gate_id in scanned:
    if gate_id not in baselined:
        new_gates.append((gate_id, scanned[gate_id].get("status")))

failed = False

if regressions:
    failed = True
    print(
        f"::error::{len(regressions)} LaunchOps hard gate(s) regressed from "
        "baseline pass -> fail"
    )
    for gate_id, expected_status, actual_status, reason in regressions:
        reason_s = reason or "(no reason reported)"
        print(f"  - {gate_id}: expected={expected_status} actual={actual_status} reason={reason_s}")

if missing_from_scan:
    failed = True
    print(
        f"::error::{len(missing_from_scan)} baselined gate(s) are missing "
        "from gates.json; definition drift, refresh baseline with "
        "UPDATE_BASELINE=1 and document the removal"
    )
    for gate_id in missing_from_scan:
        print(f"  - {gate_id}")

if unexpected_passes:
    # Advisory only, don't fail.
    print(
        f"::warning::{len(unexpected_passes)} baselined-as-fail gate(s) now "
        "pass; lock in the higher floor with UPDATE_BASELINE=1"
    )
    for gate_id in unexpected_passes:
        print(f"  - {gate_id} (was fail, now pass)")

if new_gates:
    print(
        f"::warning::{len(new_gates)} gate(s) appeared in gates.json that "
        "are not in the baseline; add them with UPDATE_BASELINE=1"
    )
    for gate_id, status in new_gates:
        print(f"  - {gate_id} (status={status})")

if failed:
    sys.exit(1)

pass_count = sum(1 for g in scanned.values() if g.get("status") == "pass")
fail_count = len(scanned) - pass_count
print(
    f"LaunchOps gates baseline: {len(baselined)} baselined gate(s) match "
    f"expected status ({pass_count} passing, {fail_count} baselined-as-fail)"
)
PY
