#!/usr/bin/env bash
# Evidence-first atomic trading test runner. Never treats an unexecuted test as a pass.
set -Eeuo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
OUT="${X3_ATOMIC_EVIDENCE_DIR:-$ROOT/target/x3-atomic-evidence}"
mkdir -p "$OUT"
git rev-parse HEAD > "$OUT/head.txt"
git status --short > "$OUT/worktree.txt"
printf 'repo=%s\nhead=%s\n' "$ROOT" "$(cat "$OUT/head.txt")"
run() {
  local label="$1"; shift
  printf '\n>>> %s\n' "$label"
  set +e
  "$@" > "$OUT/$label.log" 2>&1
  local code=$?
  set -e
  printf '%s %s\n' "$label" "$code" | tee -a "$OUT/status.txt"
  tail -n 12 "$OUT/$label.log"
  return "$code"
}
: > "$OUT/status.txt"
fail=0
# The nested X3Lang workspace owns x3-lang-vm.
(
 cd "$ROOT/x3-lang"
 run vm-all cargo test -p x3-lang-vm --locked
) || fail=1
(
 cd "$ROOT/x3-lang"
 run vm-trading cargo test -p x3-lang-vm --test trading_execution --locked
) || fail=1
(
 cd "$ROOT/x3-lang"
 run vm-receipt cargo test -p x3-lang-vm --test trading_execution receipt --locked
) || fail=1
# Root-workspace DEX rollback regression against real FRAME pallet storage.
(
 cd "$ROOT"
 run dex-two-pool-rollback cargo test -p pallet-x3-dex two_pool_second_leg_slippage_rolls_back_first_leg_and_events --locked
) || fail=1
# Inventory concrete test names for a later instruction-level trace adapter.
(
 cd "$ROOT/x3-lang"
 run vm-test-inventory cargo test -p x3-lang-vm --test trading_execution --locked -- --list
) || fail=1
if ((fail)); then echo 'FAIL: see evidence logs'; exit 1; fi
echo 'PASS: commands completed; review logs for test coverage before claiming atomic settlement.'
