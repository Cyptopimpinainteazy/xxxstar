#!/usr/bin/env bash
set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_FILE="${1:-.artifacts/apps-quality-baseline-after.txt}"
APPS=(dex wallet x3-desktop x3-intelligence validators inferstructor-dashboard)
CMDS=(lint typecheck type-check test build)

mkdir -p "$(dirname "$OUT_FILE")"
: > "$OUT_FILE"

total=0
pass=0
fail=0

for app in "${APPS[@]}"; do
  if [ ! -f "apps/$app/package.json" ]; then
    continue
  fi

  echo "=== $app ===" | tee -a "$OUT_FILE"
  scripts=$(node -e "const p=require('./apps/$app/package.json'); console.log(Object.keys(p.scripts||{}).join(' '))")

  for cmd in "${CMDS[@]}"; do
    if echo " $scripts " | grep -q " $cmd "; then
      total=$((total + 1))
      echo "-> (cd apps/$app && npm run $cmd)" | tee -a "$OUT_FILE"
      if timeout 180 bash -lc "cd apps/$app && npm run $cmd" >> "$OUT_FILE" 2>&1; then
        pass=$((pass + 1))
        echo "RESULT $cmd: PASS" | tee -a "$OUT_FILE"
      else
        exit_code=$?
        fail=$((fail + 1))
        echo "RESULT $cmd: FAIL (exit $exit_code)" | tee -a "$OUT_FILE"
      fi
    fi
  done

  echo "" | tee -a "$OUT_FILE"
done

echo "=== SUMMARY ===" | tee -a "$OUT_FILE"
echo "Total gates: $total" | tee -a "$OUT_FILE"
echo "Pass: $pass" | tee -a "$OUT_FILE"
echo "Fail: $fail" | tee -a "$OUT_FILE"

if [ "$fail" -eq 0 ]; then
  echo "All app quality gates passed." | tee -a "$OUT_FILE"
  exit 0
fi

echo "Some app quality gates failed." | tee -a "$OUT_FILE"
exit 1
