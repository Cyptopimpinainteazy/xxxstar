#!/usr/bin/env bash
# LaunchOps test-results regression gate.
#
# `launchops scan` emits .launchops/test_results.json, the captured
# status/exit-code for each local preflight command (cargo_audit,
# cargo_check, cargo_clippy, cargo_deny, cargo_fmt, cargo_test). Today
# every entry is `failed`: cargo-audit and cargo-deny tooling is not
# installed in the scan environment (exit 101/5) and cargo_fmt has real
# drift. Failing CI on the raw list would be permanent red, but also
# silently tolerating it means the day cargo_clippy flips from
# "exit 101 / missing tool" to "exit 1 / real clippy error" we would
# never notice.
#
# This gate pins each test's (status, exit_code) to a checked-in
# baseline keyed by test name. CI fails on:
#   * a test name in the scan that is not in the baseline (new test --
#     document expected state),
#   * a test whose (status, exit_code) does not match the baseline
#     (regression or silent improvement that deserves a baseline
#     refresh), or
#   * a baseline test missing from the scan (baseline is stale) unless
#     UPDATE_BASELINE is set.
#
# To accept a change run:
#     UPDATE_BASELINE=1 bash scripts/ci/verify_launchops_test_results_baseline.sh
# and commit .launchops/test_results.baseline.json in the same PR.
set -euo pipefail

results=${TEST_RESULTS_FILE:-.launchops/test_results.json}
baseline=${BASELINE_FILE:-.launchops/test_results.baseline.json}
update_baseline=${UPDATE_BASELINE:-0}

if [[ ! -f "$results" ]]; then
  echo "::error::missing $results; run 'cargo run -p launchops -- scan' first"
  exit 1
fi
if [[ ! -f "$baseline" ]]; then
  echo "::error::missing $baseline; seed it with UPDATE_BASELINE=1 $0"
  exit 1
fi

python3 - "$results" "$baseline" "$update_baseline" <<'PY'
import json
import sys

results_path, baseline_path, update_flag = sys.argv[1], sys.argv[2], sys.argv[3]

with open(results_path, encoding="utf-8") as handle:
    results = json.load(handle)
with open(baseline_path, encoding="utf-8") as handle:
    baseline = json.load(handle)

scanned = {entry["name"]: entry for entry in results}
baselined = {item["name"]: item for item in baseline.get("tests", [])}

if update_flag == "1":
    merged = []
    for name, entry in scanned.items():
        existing = baselined.get(name, {})
        merged.append({
            "name": name,
            "status": entry.get("status"),
            "exit_code": entry.get("exit_code"),
            "accepted_reason": existing.get(
                "accepted_reason",
                "TODO: document why this test status is accepted at baseline update time",
            ),
        })
    merged.sort(key=lambda item: item["name"])
    updated = {k: v for k, v in baseline.items() if k != "tests"}
    updated["tests"] = merged
    with open(baseline_path, "w", encoding="utf-8") as handle:
        json.dump(updated, handle, indent=2)
        handle.write("\n")
    print(
        f"LaunchOps test_results baseline rewritten: {len(merged)} "
        f"test(s) recorded"
    )
    sys.exit(0)

failed = False

new_tests = sorted(set(scanned) - set(baselined))
if new_tests:
    failed = True
    print(
        f"::error::{len(new_tests)} new LaunchOps test(s) not in baseline; "
        "accept them by re-running with UPDATE_BASELINE=1 and committing "
        "the updated baseline"
    )
    for name in new_tests:
        entry = scanned[name]
        print(
            f"  - {name} status={entry.get('status')} "
            f"exit_code={entry.get('exit_code')}"
        )

removed_tests = sorted(set(baselined) - set(scanned))
if removed_tests:
    failed = True
    print(
        f"::error::{len(removed_tests)} LaunchOps baseline test(s) no "
        "longer present in scan; refresh with UPDATE_BASELINE=1"
    )
    for name in removed_tests:
        print(f"  - {name}")

drifted = []
for name in sorted(set(scanned) & set(baselined)):
    entry = scanned[name]
    base = baselined[name]
    current = (entry.get("status"), entry.get("exit_code"))
    expected = (base.get("status"), base.get("exit_code"))
    if current != expected:
        drifted.append((name, expected, current))

if drifted:
    failed = True
    print(
        f"::error::{len(drifted)} LaunchOps test(s) drifted from baseline "
        "(status or exit_code changed); refresh baseline with "
        "UPDATE_BASELINE=1 after documenting the change"
    )
    for name, expected, current in drifted:
        print(
            f"  - {name} expected status={expected[0]} exit={expected[1]} "
            f"got status={current[0]} exit={current[1]}"
        )

if failed:
    sys.exit(1)

print(
    f"LaunchOps test_results baseline: {len(scanned)} test(s) "
    "match baseline"
)
PY
