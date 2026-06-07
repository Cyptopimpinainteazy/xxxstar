#!/usr/bin/env bash
set -euo pipefail

echo "== X3 MAINNET RC GATE =="

# Increase stack to avoid rustc 1.88 LLVM ICE on cc crate.
export RUST_MIN_STACK=16777216

# Mainnet gate must be harsher
./scripts/mainnet/fresh_build_check.sh || { echo "FAILED: fresh build check"; exit 1; }
./scripts/mainnet/panic_unwrap_audit.sh || { echo "FAILED: panic unwrap audit"; exit 1; }
./scripts/testnet/generate_testnet_chain_spec.sh || { echo "FAILED: chain spec generation"; exit 1; }
./scripts/testnet/testnet_genesis_lint.sh || { echo "FAILED: genesis lint"; exit 1; }
./scripts/testnet/runtime_upgrade_rehearsal.sh || { echo "FAILED: runtime upgrade rehearsal"; exit 1; }
cargo fmt --check || { echo "FAILED: code formatting"; exit 1; }
cargo test -p pallet-x3-cross-vm-router -- --nocapture || { echo "FAILED: cross-vm-router tests"; exit 1; }
cargo test -p pallet-x3-supply-ledger -- --nocapture || { echo "FAILED: supply-ledger tests"; exit 1; }
cargo test -p pallet-x3-atomic-kernel -- --nocapture || { echo "FAILED: atomic-kernel tests"; exit 1; }
cargo test -p x3-ixl -- --nocapture || { echo "FAILED: x3-ixl tests"; exit 1; }
cargo test -p x3-proof -- --nocapture || { echo "FAILED: x3-proof tests"; exit 1; }
cargo test -p x3-sidecar -- --nocapture || { echo "FAILED: x3-sidecar tests"; exit 1; }
cargo test -p x3-gateway -- --nocapture || { echo "FAILED: x3-gateway tests"; exit 1; }
cargo run -p x3-readiness -- testnet-report --out reports/testnet_readiness_report.md || { echo "FAILED: testnet report"; exit 1; }

echo "== X3 MAINNET RC GATE PASSED =="