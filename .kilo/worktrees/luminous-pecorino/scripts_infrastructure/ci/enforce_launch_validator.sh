#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

mkdir -p target

cargo build -p x3-chain-node --release

report_path="target/launch-validator-report.json"

if ! cargo run -p x3-launch-validator --bin x3-launch-check -- --json > "$report_path"; then
  echo "::error::x3-launch-check reported blocking launch checklist failures"
  exit 1
fi

python3 - <<'PY'
import json
from pathlib import Path

report = Path('target/launch-validator-report.json')
content = json.loads(report.read_text())

def normalize_result(result):
  if isinstance(result, str):
    lower = result.strip().lower()
    if lower == "pass":
      return "pass"
    if lower == "fail":
      return "fail"
    return "skip"
  if isinstance(result, dict):
    keys = {k.strip().lower() for k in result.keys()}
    if "pass" in keys:
      return "pass"
    if "fail" in keys or "failed" in keys:
      return "fail"
    return "skip"
  return "skip"

if isinstance(content, dict):
  summary = content.get('summary', {})
  passed = int(summary.get('pass', 0))
  failed = int(summary.get('fail', 0))
  skipped = int(summary.get('skip', 0))
elif isinstance(content, list):
  passed = 0
  failed = 0
  skipped = 0
  for item in content:
    status = normalize_result(item.get("result"))
    if status == "pass":
      passed += 1
    elif status == "fail":
      failed += 1
    else:
      skipped += 1
else:
  raise SystemExit("Unexpected launch-validator JSON format")

print(f"launch-validator summary: pass={passed} fail={failed} skip={skipped}")

if failed > 0:
  raise SystemExit("launch-validator contains failing checks")
PY

echo "Launch validator gate passed"
