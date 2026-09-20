#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# panic_unwrap_audit.sh — panic ratchet for consensus-critical code
#
# A panic in `on_initialize`/`on_finalize`/`offchain_worker` stops block
# production; a panic in a `#[pallet::call]` turns a user extrinsic into a
# block-inclusion failure. This script finds them and refuses to let the count
# grow.
#
# It replaced a grep-based version that classified by *file path*, so
# `#[cfg(test)] mod tests` — which lives in the same file as the code it tests —
# was counted as production. That reported ~5,170 "production hot path" findings,
# nearly all of them test assertions, and the script then **always exited 0**
# while its own report said "gate: FAIL". `mainnet_rc_gate.sh` guarded this with
# `|| exit 1`, which could never fire.
#
# What it does now:
#   * scans with scripts/audit/panic_unwrap_scan.py, which excludes `#[cfg(test)]`
#     items and commented lines, and classifies by the enclosing function
#   * compares against docs/reports/panic-unwrap-baseline.json
#   * exits non-zero when any class grows, when the baseline is missing, or when a
#     runtime-hook panic appears that the baseline does not list as allowed
#   * writes the human report to reports/panic_unwrap_audit.md
#
# Usage:
#   bash scripts/mainnet/panic_unwrap_audit.sh                # check (the gate)
#   bash scripts/mainnet/panic_unwrap_audit.sh --update-baseline
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCAN="$ROOT_DIR/scripts/audit/panic_unwrap_scan.py"
BASELINE="$ROOT_DIR/docs/reports/panic-unwrap-baseline.json"
REPORT="$ROOT_DIR/reports/panic_unwrap_audit.md"
UPDATE=0
if [ "${1:-}" = "--update-baseline" ]; then
  UPDATE=1
elif [ -n "${1:-}" ]; then
  echo "panic_unwrap_audit: unknown option '$1' (only --update-baseline is accepted)" >&2
  exit 2
fi

[ -f "$SCAN" ] || { echo "panic_unwrap_audit: missing $SCAN" >&2; exit 1; }
mkdir -p "$(dirname "$REPORT")" "$(dirname "$BASELINE")"

CURRENT="$(mktemp)"
trap 'rm -f "$CURRENT"' EXIT
python3 "$SCAN" >"$CURRENT"

python3 - "$CURRENT" "$BASELINE" "$REPORT" "$UPDATE" "$ROOT_DIR" <<'PY'
import json
import os
import subprocess
import sys
from datetime import datetime, timezone

current_path, baseline_path, report_path, update_flag, root = sys.argv[1:6]
# argv is a string: "0" is truthy in Python, so compare instead of testing.
update = update_flag == "1"
current = json.load(open(current_path))
counts = current["counts"]
hooks = current["runtime_hooks"]

baseline = None
if os.path.exists(baseline_path):
    baseline = json.load(open(baseline_path))

if update:
    try:
        commit = subprocess.run(
            ["git", "-C", root, "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
    except Exception:
        commit = "unknown"
    json.dump(
        {
            "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "commit": commit,
            "counts": counts,
            "files_scanned": current["files_scanned"],
            "runtime_hooks_allowed": hooks,
            "note": (
                "Panic/unwrap ratchet. `counts` may only go down; runtime_hooks_allowed "
                "is an explicit allowlist of KNOWN block-hook panics that are scheduled to "
                "be removed — it is not a target state, and it should trend to empty. "
                "Refresh with "
                "scripts/mainnet/panic_unwrap_audit.sh --update-baseline, and say why "
                "in the commit message if a count increases."
            ),
        },
        open(baseline_path, "w"),
        indent=2,
    )
    print(f"panic_unwrap_audit: baseline updated -> {baseline_path}")
    print(
        "  runtime-hook={runtime-hook} pallet-call={pallet-call} production={production}".format(
            **counts
        )
    )
    sys.exit(0)

# ── check mode ───────────────────────────────────────────────────────────────
failures: list[str] = []

if baseline is None:
    failures.append(
        f"no baseline at {baseline_path}: run "
        "scripts/mainnet/panic_unwrap_audit.sh --update-baseline and commit the result"
    )
    base_counts = {"runtime-hook": 0, "pallet-call": 0, "production": 0}
    allowed_hooks: list[str] = []
else:
    base_counts = baseline["counts"]
    allowed_hooks = baseline.get("runtime_hooks_allowed", [])

new_hooks = sorted(set(hooks) - set(allowed_hooks))
if new_hooks:
    failures.append(
        "new panic in a block hook (on_initialize/on_finalize/offchain_worker) — "
        "this stops block production: " + ", ".join(new_hooks)
    )

for kind, value in counts.items():
    allowed = base_counts.get(kind, 0)
    if value > allowed:
        failures.append(f"{kind} panics/unwraps grew: {allowed} -> {value}")

with open(report_path, "w", encoding="utf-8") as handle:
    handle.write("# Panic Unwrap Audit\n\n")
    handle.write(
        f"Generated: {datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')} "
        f"by scripts/mainnet/panic_unwrap_audit.sh\n\n"
    )
    handle.write("## Classification\n\n")
    handle.write("- **runtime-hook** — inside `on_initialize` / `on_finalize` / `offchain_worker`: a panic stops block production\n")
    handle.write("- **pallet-call** — inside a `#[pallet::call]` extrinsic body: a panic fails the block that includes it\n")
    handle.write("- **production** — every other line reachable in a production build\n\n")
    handle.write("`#[cfg(test)]` items and commented-out lines are excluded: they do not\n")
    handle.write("exist in the runtime or in a release node build.\n\n")
    handle.write("## Counts\n\n")
    handle.write("| class | current | baseline |\n| --- | --- | --- |\n")
    for kind in ("runtime-hook", "pallet-call", "production"):
        handle.write(f"| {kind} | {counts[kind]} | {base_counts.get(kind, 0)} |\n")
    handle.write(f"\nfiles scanned: {current['files_scanned']}\n\n")
    handle.write("## Block-hook panics\n\n")
    if hooks:
        for hook in hooks:
            marker = " (allowlisted)" if hook in allowed_hooks else " **NEW — FAILS THE GATE**"
            handle.write(f"- `{hook}`{marker}\n")
    else:
        handle.write("None. No panic is reachable from a block hook.\n")
    handle.write("\n## Production findings by file\n\n")
    per_file: dict[str, int] = {}
    for item in current.get("findings", {}).get("production", []):
        per_file[item["file"]] = per_file.get(item["file"], 0) + 1
    for path, count in sorted(per_file.items(), key=lambda kv: -kv[1])[:25]:
        rel = os.path.relpath(path, root)
        handle.write(f"- {count:4d}  {rel}\n")
    handle.write("\n## Verdict\n\n")
    if failures:
        handle.write("FAIL\n\n")
        for item in failures:
            handle.write(f"- {item}\n")
    else:
        handle.write("PASS — no new panic/unwrap in consensus-critical code.\n")

print(f"panic_unwrap_audit: wrote {report_path}")
print(
    "  runtime-hook={runtime-hook} pallet-call={pallet-call} production={production} "
    "(baseline {a}/{b}/{c})".format(
        a=base_counts.get("runtime-hook", 0),
        b=base_counts.get("pallet-call", 0),
        c=base_counts.get("production", 0),
        **counts,
    )
)
if failures:
    print("panic_unwrap_audit: FAIL")
    for item in failures:
        print(f"  - {item}")
    sys.exit(1)
print("panic_unwrap_audit: PASS")
PY
