#!/usr/bin/env bash
# Deep red flag regression gate.
#
# `launchops scan` emits .launchops/deep_red_flags.json, a list of critical
# anti-patterns (mainnet config changed without review evidence, assertions
# removed from tests, etc.). These are real release blockers, but today's
# scorecard carries some of them as accepted pending Wave 0 work, so failing
# CI on the raw list would immediately red-light mainnet CI.
#
# Instead, this gate fingerprints each flag and compares against a checked-in
# baseline (.launchops/deep_red_flags.baseline.json). CI fails only on:
#   * fingerprints present in the scan but missing from the baseline (new
#     regressions), or
#   * baseline fingerprints missing from the scan when UPDATE_BASELINE is not
#     set (accepted risks that silently disappeared -- the baseline is stale).
#
# To accept a new flag or clear a fixed one, run:
#     UPDATE_BASELINE=1 bash scripts/ci/verify_launchops_deep_red_flags_baseline.sh
# and commit the updated baseline in the same PR; that puts the human review
# moment into the PR diff.
set -euo pipefail

flags=${FLAGS_FILE:-.launchops/deep_red_flags.json}
baseline=${BASELINE_FILE:-.launchops/deep_red_flags.baseline.json}
update_baseline=${UPDATE_BASELINE:-0}

if [[ ! -f "$flags" ]]; then
  echo "::error::missing $flags; run 'cargo run -p launchops -- scan' first"
  exit 1
fi
if [[ ! -f "$baseline" ]]; then
  echo "::error::missing $baseline; seed it with UPDATE_BASELINE=1 $0"
  exit 1
fi

python3 - "$flags" "$baseline" "$update_baseline" <<'PY'
import hashlib
import json
import sys

flags_path, baseline_path, update_flag = sys.argv[1], sys.argv[2], sys.argv[3]


def fingerprint(entry):
    canonical = {
        "severity": entry.get("severity"),
        "flag_type": entry.get("flag_type"),
        "files": sorted(entry.get("files", [])),
    }
    blob = json.dumps(canonical, sort_keys=True).encode()
    return hashlib.sha256(blob).hexdigest()[:16]


with open(flags_path, encoding="utf-8") as handle:
    flags = json.load(handle)
with open(baseline_path, encoding="utf-8") as handle:
    baseline = json.load(handle)

scanned = {fingerprint(entry): entry for entry in flags}
baselined = {
    item["fingerprint"]: item for item in baseline.get("fingerprints", [])
}

new_flags = sorted(set(scanned) - set(baselined))
cleared_flags = sorted(set(baselined) - set(scanned))

if update_flag == "1":
    merged = []
    for fp, entry in scanned.items():
        existing = baselined.get(fp, {})
        merged.append({
            "fingerprint": fp,
            "severity": entry.get("severity"),
            "flag_type": entry.get("flag_type"),
            "files": sorted(entry.get("files", [])),
            "accepted_reason": existing.get(
                "accepted_reason",
                "TODO: document why this flag is accepted at baseline update time",
            ),
        })
    merged.sort(key=lambda item: item["fingerprint"])
    # Preserve every top-level baseline key other than `fingerprints`
    # (e.g. `_comment`) so human context is not lost on refresh.
    updated = {k: v for k, v in baseline.items() if k != "fingerprints"}
    updated["fingerprints"] = merged
    with open(baseline_path, "w", encoding="utf-8") as handle:
        json.dump(updated, handle, indent=2)
        handle.write("\n")
    print(
        f"LaunchOps deep_red_flags baseline rewritten: {len(merged)} "
        f"fingerprint(s) recorded"
    )
    sys.exit(0)

failed = False

if new_flags:
    failed = True
    print(
        f"::error::{len(new_flags)} new LaunchOps deep_red_flag(s) not in "
        "baseline; accept them by re-running with UPDATE_BASELINE=1 and "
        "committing the updated baseline with review evidence"
    )
    for fp in new_flags:
        entry = scanned[fp]
        print(
            f"  - {fp} {entry.get('severity','?')}/"
            f"{entry.get('flag_type','?')} files={entry.get('files', [])}"
        )

if cleared_flags:
    failed = True
    print(
        f"::error::{len(cleared_flags)} LaunchOps deep_red_flag baseline "
        "fingerprint(s) no longer present in scan; the baseline is stale, "
        "refresh it with UPDATE_BASELINE=1"
    )
    for fp in cleared_flags:
        entry = baselined[fp]
        print(
            f"  - {fp} {entry.get('severity','?')}/"
            f"{entry.get('flag_type','?')} files={entry.get('files', [])}"
        )

if failed:
    sys.exit(1)

print(
    f"LaunchOps deep_red_flags baseline: {len(scanned)} fingerprint(s) "
    "match baseline"
)
PY
