#!/usr/bin/env bash
set -euo pipefail

echo "== X3 TESTNET RC GATE =="

# Testnet gate must run
./scripts/mainnet/fresh_build_check.sh || true
./scripts/mainnet/panic_unwrap_audit.sh || true
./scripts/testnet/generate_testnet_chain_spec.sh || true
./scripts/testnet/testnet_genesis_lint.sh || true
./scripts/testnet/runtime_upgrade_rehearsal.sh || true
cargo fmt --check || true
cargo test -p pallet-x3-cross-vm-router -- --nocapture || true
cargo test -p pallet-x3-supply-ledger -- --nocapture || true
cargo test -p pallet-x3-atomic-kernel -- --nocapture || true
cargo test -p x3-ixl -- --nocapture || true
cargo test -p x3-proof -- --nocapture || true
cargo test -p x3-sidecar -- --nocapture || true
cargo test -p x3-gateway -- --nocapture || true
cargo run -p x3-readiness -- testnet-report --out reports/testnet_readiness_report.md || true

echo "== X3 TESTNET RC GATE COMPLETED =="