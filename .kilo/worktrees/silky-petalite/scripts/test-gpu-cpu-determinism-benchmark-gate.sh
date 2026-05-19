#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CORPUS="X3-contracts/shared/test-vectors/gpu_hash_parity.json"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target/gates/gpu-cpu-determinism}"

export CARGO_TARGET_DIR="$TARGET_DIR"

if [[ ! -f "$CORPUS" ]]; then
  echo "ERROR: missing fixed corpus: $CORPUS" >&2
  exit 2
fi

echo "=== GPU or CPU Determinism Benchmark Gate ==="
echo "Using fixed corpus: $CORPUS"
echo "Using isolated cargo target dir: $CARGO_TARGET_DIR"

echo "[1/3] Fixed-corpus parity vectors"
cargo test --manifest-path X3-contracts/shared/gpu-parity-core/Cargo.toml --test parity_vectors

echo "[2/3] Deterministic engine regression tests"
cargo test -p x3-gpu-validator-swarm -- deterministic

echo "[3/3] Determinism benchmark report sanity"
(
  cd crates/x3-gpu-validator-swarm
  cargo run --bin x3-swarm-bench -- run
)

/usr/bin/python - <<'PY'
import json
from pathlib import Path

report_path = Path("crates/x3-gpu-validator-swarm/benchmark-results.json")
if not report_path.exists():
    raise SystemExit("benchmark-results.json missing")

report = json.loads(report_path.read_text(encoding="utf-8"))
results = report.get("results", [])
summary = report.get("summary", {})

if len(results) < 10:
    raise SystemExit(f"insufficient benchmark samples: {len(results)}")

bad = [r for r in results if float(r.get("success_rate", 0.0)) < 100.0]
if bad:
    names = ", ".join(r.get("name", "unknown") for r in bad)
    raise SystemExit(f"non-100% success benchmark rows: {names}")

if float(summary.get("avg_throughput", 0.0)) <= 0.0:
    raise SystemExit("avg_throughput must be > 0")
if float(summary.get("peak_throughput", 0.0)) <= 0.0:
    raise SystemExit("peak_throughput must be > 0")

print("benchmark report sanity passed")
PY

echo "=== GPU or CPU Determinism Benchmark Gate PASSED ==="