#!/usr/bin/env bash
set -euo pipefail

mkdir -p .cache .reports .repomix .traycer .x3

echo "== X3 AI LOOP: packing repo context =="
.scripts/x3_repomix_pack.sh

echo "== X3 AI LOOP: scanning file list =="
.scripts/x3_full_scan.sh

echo "== X3 AI LOOP: smell scan =="
.scripts/x3_smell_scan.sh >/tmp/x3_ai_loop_smell_scan.out
wc -l .reports/x3_smells.txt

echo "== X3 AI LOOP: git status =="
git status --short > .reports/git_status.txt || true

echo "== X3 AI LOOP: done =="
echo "Open Traycer spec:"
echo ".traycer/X3_DEEP_DIVE_SPEC.md"
echo
echo "Then send scoped tasks to Roo using:"
echo ".traycer/X3_TASK_CHAIN.md"
