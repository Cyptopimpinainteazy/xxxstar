#!/usr/bin/env bash
# Evidence-first atomic trading test runner. Never treats an unexecuted test as a pass.
set -Eeuo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
if [[ -n "${X3_ATOMIC_EVIDENCE_DIR:-}" ]]; then
  OUT="$X3_ATOMIC_EVIDENCE_DIR"
else
  mkdir -p "$ROOT/target/x3-atomic-evidence"
  OUT="$(mktemp -d "$ROOT/target/x3-atomic-evidence/$(date -u +%Y%m%dT%H%M%SZ)-XXXXXX")"
fi
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
if [[ -e "$OUT/status.txt" ]]; then
  echo "Refusing to overwrite existing evidence: $OUT" >&2
  exit 1
fi
git rev-parse HEAD > "$OUT/head.txt"
git status --short > "$OUT/worktree.txt"
{ rustc --version; cargo --version; } > "$OUT/toolchain.txt"
printf 'repo=%s\nhead=%s\nevidence=%s\n' "$ROOT" "$(cat "$OUT/head.txt")" "$OUT"
: > "$OUT/commands.txt"
run() {
  local label="$1"; shift
  printf '\n>>> %s\n' "$label"
  { printf '%s cwd=%q command=' "$label" "$PWD"; printf '%q ' "$@"; printf '\n'; } >> "$OUT/commands.txt"
  set +e
  "$@" > "$OUT/$label.log" 2>&1
  local code=$?
  set -e
  printf '%s %s\n' "$label" "$code" | tee -a "$OUT/status.txt"
  tail -n 12 "$OUT/$label.log"
  return "$code"
}
run_tests() {
  local label="$1"
  run "$@" || return "$?"
  # A successful Cargo exit can mean a stale filter selected no tests. Require
  # executed tests and reject ignored tests; empty doctest binaries are fine.
  if ! awk '
    /^test result: ok\./ {
      summaries++; passed += $4;
      if ($6 != 0 || $8 != 0) invalid = 1;
    }
    END { exit !(summaries > 0 && passed > 0 && !invalid) }
  ' "$OUT/$label.log"; then
    printf '%s-coverage 1\n' "$label" | tee -a "$OUT/status.txt"
    echo "FAIL: $label must execute tests without failures or ignored tests" >&2
    return 1
  fi
}
: > "$OUT/status.txt"
fail=0
# The nested X3Lang workspace owns x3-lang-vm.
(
 cd "$ROOT/x3-lang"
 run_tests vm-all cargo test -p x3-lang-vm --locked
) || fail=1
(
 cd "$ROOT/x3-lang"
 run_tests vm-trading cargo test -p x3-lang-vm --test trading_execution --locked
) || fail=1
(
 cd "$ROOT/x3-lang"
 run_tests vm-receipt cargo test -p x3-lang-vm --test trading_execution receipt --locked
) || fail=1
# Root-workspace DEX rollback regression against real FRAME pallet storage.
(
 cd "$ROOT"
 run_tests dex-two-pool-rollback cargo test -p pallet-x3-dex two_pool_second_leg_slippage_rolls_back_first_leg_and_events --locked
) || fail=1
# Production atomic trade engine pallet regressions.
(
 cd "$ROOT"
 run_tests atomic-engine-tests cargo test -p pallet-atomic-trade-engine --locked
) || fail=1
# Inventory concrete test names for a later instruction-level trace adapter.
(
 cd "$ROOT/x3-lang"
 run vm-test-inventory cargo test -p x3-lang-vm --test trading_execution --locked -- --list
) || fail=1
if ((fail)); then echo 'FAIL: see evidence logs'; exit 1; fi
echo 'PASS: commands completed; review logs for test coverage before claiming atomic settlement.'
