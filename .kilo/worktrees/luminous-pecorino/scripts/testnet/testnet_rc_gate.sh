#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
STATUS_REPORT="$REPORT_DIR/testnet_rc_gate_status.md"
mkdir -p "$REPORT_DIR"

: > "$STATUS_REPORT"
{
  echo "# X3 Testnet RC Gate Status"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
} >> "$STATUS_REPORT"

run_step() {
  local name="$1"
  shift
  local log_file="$REPORT_DIR/${name// /_}.log"
  echo "Running: $name"
  "$@" >"$log_file" 2>&1
  echo "- $name: PASS" >> "$STATUS_REPORT"
}

run_step "fresh_build_check" bash "$ROOT_DIR/scripts/mainnet/fresh_build_check.sh"
run_step "panic_unwrap_audit" bash "$ROOT_DIR/scripts/mainnet/panic_unwrap_audit.sh"
run_step "genesis_lint" bash "$ROOT_DIR/scripts/testnet/testnet_genesis_lint.sh"
run_step "runtime_upgrade_rehearsal" bash "$ROOT_DIR/scripts/testnet/runtime_upgrade_rehearsal.sh"

run_step "test_pallet_x3_cross_vm_router" cargo test -p pallet-x3-cross-vm-router -- --nocapture
run_step "test_pallet_x3_supply_ledger" cargo test -p pallet-x3-supply-ledger -- --nocapture
run_step "test_pallet_x3_atomic_kernel" cargo test -p pallet-x3-atomic-kernel -- --nocapture
run_step "test_x3_ixl" cargo test -p x3-ixl -- --nocapture
run_step "test_x3_proof" cargo test -p x3-proof -- --nocapture
run_step "test_x3_sidecar" cargo test -p x3-sidecar -- --nocapture
run_step "test_x3_gateway" cargo test -p x3-gateway -- --nocapture

run_step "x3_readiness_report" cargo run -p x3-readiness -- testnet-report --out "$REPORT_DIR"

echo "TESTNET RC GATE: PASS" >> "$STATUS_REPORT"
echo "== X3 Testnet RC Gate PASSED =="
