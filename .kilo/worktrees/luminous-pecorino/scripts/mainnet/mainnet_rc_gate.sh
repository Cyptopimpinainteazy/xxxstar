#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_DIR="$ROOT_DIR/reports"
STATUS_REPORT="$REPORT_DIR/mainnet_rc_gate_status.md"
mkdir -p "$REPORT_DIR"

cd "$ROOT_DIR"

: > "$STATUS_REPORT"
{
  echo "# Mainnet RC Gate Status"
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
run_step "genesis_lint" bash "$ROOT_DIR/scripts/mainnet/genesis_lint.sh"
run_step "generate_mainnet_chain_spec" bash "$ROOT_DIR/scripts/mainnet/generate_mainnet_chain_spec.sh"
run_step "runtime_upgrade_rehearsal" bash "$ROOT_DIR/scripts/mainnet/runtime_upgrade_rehearsal.sh"

run_step "test_pallet_x3_cross_vm_router" cargo test -p pallet-x3-cross-vm-router
run_step "test_pallet_x3_supply_ledger" cargo test -p pallet-x3-supply-ledger
run_step "test_pallet_x3_atomic_kernel" cargo test -p pallet-x3-atomic-kernel
run_step "test_x3_ixl" cargo test -p x3-ixl
run_step "test_x3_proof" cargo test -p x3-proof

run_step "x3_proof_mainnet_rc_report" cargo run -p x3-proof -- mainnet-rc-report --out reports/mainnet_rc_report.md

echo "mainnet_rc_gate: PASS"
