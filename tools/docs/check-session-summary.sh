#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

shopt -s nullglob
files=(.ai/SESSION_UPDATE_SUMMARY_*.md)

if [[ ${#files[@]} -eq 0 ]]; then
  echo "No session summary files found under .ai/SESSION_UPDATE_SUMMARY_*.md"
  exit 1
fi

required=(
  "DOCUMENTATION UPDATE REPORT"
  "Task:"
  "Timestamp:"
  "Test Results:"
  "Build Result:"
  "Metrics:"
  "Files Updated:"
  "Consistency Check:"
  "Next Action:"
)

failed=0
for f in "${files[@]}"; do
  echo "Checking $f"
  for token in "${required[@]}"; do
    if ! grep -Fq "$token" "$f"; then
      echo "  MISSING: $token"
      failed=1
    fi
  done
done

if [[ $failed -ne 0 ]]; then
  echo "Session summary format check FAILED"
  exit 1
fi

echo "Session summary format check PASSED"
