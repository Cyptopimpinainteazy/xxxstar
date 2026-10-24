#!/usr/bin/env bash
set -euo pipefail

# Exit 1: an executed stage failed. Exit 2: required coverage was skipped.
# Keep collecting stage evidence after a failure, but never certify a partial run.

# Derived, not hardcoded. This script named `/home/lojak/Desktop/X3_ATOMIC_STAR`, a directory that
# does not exist on this box, so every step ran `cd` into nothing and the whole sequence failed
# instantly — which is exactly the FAIL that `reports/rc6/*` carried: the report was describing the
# script's own broken path, not the chain. Every other gate in this repository derives its root from
# `BASH_SOURCE`; so does this one now.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# Ensure Rust toolchain and local Node 20 are available for gate scripts.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi
if [ -d "$ROOT/.tools/node20/bin" ]; then
  export PATH="$ROOT/.tools/node20/bin:$PATH"
fi

mkdir -p reports/rc6
TS=$(date -u +%Y%m%dT%H%M%S%NZ)
OUT="reports/rc6/release_gate_sequence_${TS}.md"
FAILED=0
SKIPPED=0

run_step() {
  local name="$1"
  local log="$2"
  shift 2
  local code=0
  local status=PASS
  echo "## ${name}" >> "$OUT"
  if "$@" > "$log" 2>&1; then
    status=PASS
  else
    code=$?
    status=FAIL
    FAILED=$((FAILED + 1))
  fi
  echo "status=${status}" >> "$OUT"
  echo "exit_code=${code}" >> "$OUT"
  echo >> "$OUT"
  printf '[%s] %s (exit_code=%s)\n' "$status" "$name" "$code"
}

echo "# Release Gate Sequence Run (${TS})" > "$OUT"
echo >> "$OUT"

run_step "1) Build (cargo build -p x3-chain-node --release)" "reports/rc6/build_${TS}.log" cargo build -p x3-chain-node --release
# Step 2 used to run `rc2_internal_settlement_smoke.sh`, whose driver is JavaScript on the repo's
# pinned `@polkadot/api`. That driver cannot decode this chain any more: it fails during API init with
# `createType(ExtrinsicUnknown):: Unsupported unsigned extrinsic version 5` for every block, because
# the runtime's extrinsics are version 5 and the pinned polkadot-js only knows version 4 (measured
# 2026-09-27 against the tree's runtime, spec_version 20). It is not a chain defect and not something
# the gate can work around — it is a stale client. The cross-chain evidence now comes from step 3,
# whose live half (`tests/e2e --features real-chain --test live_internal_mainnet_e2e`) runs the same
# kind of checks on a real chain in Rust.
#
# The breadth is not equal, and that is a ticket rather than a silence: the JS smoke swept all six
# internal routes plus nine negative cases (external route, wrong recipient per domain, wrong sender
# type, duplicate message, duplicate nonce, refund-after-finalize, refund-before-expiry,
# completion-after-refund) with a supply-invariant check. The Rust suite has four tests. The
# retirement and the port are recorded in TESTNET_GAP_LEDGER.md.
echo "## 2) Cross-chain live smoke — retired (see TESTNET_GAP_LEDGER.md, legacy JS driver cannot decode extrinsic v5)" >> "$OUT"
echo "status=SKIPPED" >> "$OUT"
echo "reason=Required six-route live coverage has not been replaced; release certification is blocked." >> "$OUT"
echo >> "$OUT"
SKIPPED=$((SKIPPED + 1))
run_step "3) Mock+Live E2E gate (scripts/mainnet/rc2_mock_and_live_gate.sh)" "reports/rc6/mock_live_gate_${TS}.log" bash scripts/mainnet/rc2_mock_and_live_gate.sh
run_step "4) Invariant/Security suite (scripts/run-security-gates.sh all)" "reports/rc6/security_gates_${TS}.log" bash scripts/run-security-gates.sh all
run_step "5) RC6 readiness (scripts/mainnet/rc6_public_testnet_readiness.sh)" "reports/rc6/rc6_readiness_${TS}.log" bash scripts/mainnet/rc6_public_testnet_readiness.sh

echo "## Log Files" >> "$OUT"
echo "- reports/rc6/build_${TS}.log" >> "$OUT"
# No `rc2_smoke_${TS}.log`: step 2 is retired above, so listing a log for it would send the reader
# looking for a file that was never written.
echo "- reports/rc6/mock_live_gate_${TS}.log" >> "$OUT"
echo "- reports/rc6/security_gates_${TS}.log" >> "$OUT"
echo "- reports/rc6/rc6_readiness_${TS}.log" >> "$OUT"

STATUS=PASS
EXIT_CODE=0
if (( FAILED > 0 )); then
  STATUS=FAIL
  EXIT_CODE=1
elif (( SKIPPED > 0 )); then
  STATUS=BLOCKED
  EXIT_CODE=2
fi
{
  echo
  echo "## Overall result"
  echo "overall_status=$STATUS"
  echo "failed_stages=$FAILED"
  echo "skipped_required_stages=$SKIPPED"
} >> "$OUT"
printf '%s: %s\n' "$STATUS" "$ROOT/$OUT"
exit "$EXIT_CODE"
