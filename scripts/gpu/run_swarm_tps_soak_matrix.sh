#!/usr/bin/env bash
# Run the pinned x3-gpu-validator-swarm TPS soak and emit a JSON comparison.
#
# Intended for a second GPU host or self-hosted GPU CI runner.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${X3_SWARM_SOAK_OUT_DIR:-$ROOT_DIR/target/swarm-tps-soak}"
SOAK_SECS="${X3_SWARM_SOAK_SECS:-60}"
TASKS="${X3_SWARM_SOAK_TASKS:-128}"
BATCH="${X3_SWARM_SOAK_BATCH:-1024}"
BACKEND="${X3_ACCEL:-wgpu}"
BASELINE_SLIDING_TPS="${X3_BASELINE_SLIDING_TPS:-58.5}"
BASELINE_HASHES_PER_SEC="${X3_BASELINE_HASHES_PER_SEC:-63823.33}"
MIN_SLIDING_RATIO="${X3_MIN_SLIDING_RATIO:-0.80}"
MIN_HASH_RATIO="${X3_MIN_HASH_RATIO:-0.80}"

mkdir -p "$OUT_DIR"

RAW_LOG="$OUT_DIR/swarm_tps_${BACKEND}_${TASKS}x${BATCH}_${SOAK_SECS}s.log"
REPORT_JSON="$OUT_DIR/swarm_tps_${BACKEND}_${TASKS}x${BATCH}_${SOAK_SECS}s.json"

echo "Running swarm TPS soak:"
echo "  backend=$BACKEND"
echo "  tasks_per_round=$TASKS"
echo "  batch_size=$BATCH"
echo "  soak_secs=$SOAK_SECS"
echo "  baseline_sliding_tps=$BASELINE_SLIDING_TPS"
echo "  baseline_hashes_per_sec=$BASELINE_HASHES_PER_SEC"

(
  cd "$ROOT_DIR"
  X3_ACCEL="$BACKEND" \
  X3_SWARM_SOAK_SECS="$SOAK_SECS" \
  X3_SWARM_SOAK_TASKS="$TASKS" \
  X3_SWARM_SOAK_BATCH="$BATCH" \
    cargo bench --target-dir /tmp/x3-swarm-soak-target \
      -p x3-gpu-validator-swarm --features wgpu --bench swarm_tps \
      -- sustained --nocapture
) | tee "$RAW_LOG"

python3 - "$RAW_LOG" "$REPORT_JSON" "$BASELINE_SLIDING_TPS" "$BASELINE_HASHES_PER_SEC" "$MIN_SLIDING_RATIO" "$MIN_HASH_RATIO" <<'PY'
import json
import re
import sys
from pathlib import Path

raw_path = Path(sys.argv[1])
report_path = Path(sys.argv[2])
baseline_sliding_tps = float(sys.argv[3])
baseline_hashes_per_sec = float(sys.argv[4])
min_sliding_ratio = float(sys.argv[5])
min_hash_ratio = float(sys.argv[6])
text = raw_path.read_text()

summary_re = re.search(
    r"sustained_secs=(?P<soak_secs>\d+)\s+"
    r"tasks_per_round=(?P<tasks>\d+)\s+"
    r"batch_size=(?P<batch>\d+)\s+"
    r"processed=(?P<processed>\d+)\s+"
    r"task_tps=(?P<task_tps>[0-9.]+)\s+"
    r"hash_tps=(?P<hash_tps>[0-9.]+)",
    text,
)
if not summary_re:
    raise SystemExit("failed to parse sustained swarm_tps summary line")

def metric(name: str, default: float = 0.0) -> float:
    pattern = rf"{re.escape(name)}(?:\{{[^}}]*\}})?\s+([0-9.]+)"
    match = re.search(pattern, text)
    return float(match.group(1)) if match else default

def label_metric(name: str) -> tuple[str | None, float | None]:
    pattern = rf"{re.escape(name)}\{{backend=\"([^\"]+)\"[^}}]*\}}\s+([0-9.]+)"
    match = re.search(pattern, text)
    return (match.group(1), float(match.group(2))) if match else (None, None)

backend, sliding_tps = label_metric("x3_swarm_sliding_window_tps")
_, lifetime_tps = label_metric("x3_swarm_tps")
_, fallbacks = label_metric("x3_swarm_accelerator_fallbacks_total")
_, mismatches = label_metric("x3_swarm_accelerator_parity_mismatches_total")

hash_tps = float(summary_re.group("hash_tps"))
sliding_ratio = (sliding_tps or 0.0) / baseline_sliding_tps if baseline_sliding_tps else None
hash_ratio = hash_tps / baseline_hashes_per_sec if baseline_hashes_per_sec else None

checks = {
    "backend_selected": backend == "wgpu",
    "no_accelerator_fallbacks": (fallbacks or 0.0) == 0.0,
    "no_parity_mismatches": (mismatches or 0.0) == 0.0,
    "sliding_tps_within_ratio": sliding_ratio is not None and sliding_ratio >= min_sliding_ratio,
    "hash_tps_within_ratio": hash_ratio is not None and hash_ratio >= min_hash_ratio,
}

report = {
    "backend": backend,
    "soak_secs": int(summary_re.group("soak_secs")),
    "tasks_per_round": int(summary_re.group("tasks")),
    "batch_size": int(summary_re.group("batch")),
    "processed_tasks": int(summary_re.group("processed")),
    "task_tps_summary": float(summary_re.group("task_tps")),
    "task_tps_prometheus": lifetime_tps,
    "sliding_window_tps": sliding_tps,
    "hashes_per_sec": hash_tps,
    "accelerator_fallbacks": fallbacks,
    "accelerator_parity_mismatches": mismatches,
    "baseline": {
        "sliding_window_tps": baseline_sliding_tps,
        "hashes_per_sec": baseline_hashes_per_sec,
    },
    "ratios": {
        "sliding_window_tps": sliding_ratio,
        "hashes_per_sec": hash_ratio,
    },
    "checks": checks,
    "raw_log": str(raw_path),
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print(json.dumps(report, indent=2, sort_keys=True))

if not all(checks.values()):
    raise SystemExit("swarm TPS soak comparison failed")
PY

echo "Wrote $REPORT_JSON"
