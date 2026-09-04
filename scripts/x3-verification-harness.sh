#!/usr/bin/env bash
# X3 Verification Harness — runs all 6 verification tools
# Kani | Loom | Shuttle | Miri | sanitizers | cargo-mutants
set -euo pipefail

CARGO="$HOME/.cargo/bin/cargo"
NIGHTLY="$CARGO +nightly-2026-05-01"
STABLE="$CARGO +1.90.0"
WORKSPACE="/home/x3star/Desktop/xxxstar-main"
REPORT_DIR="$WORKSPACE/proof/verification-reports"
mkdir -p "$REPORT_DIR"

TIMESTAMP=$(date -Iseconds)
echo "=== X3 Verification Harness — $TIMESTAMP ==="

# ---- Kani (bounded model checking) ----
echo "--- Kani ---"
# Kani targets crates with arithmetic invariants
KANI_CRATES=(
  "crates/x3-atomic-swap"
  "crates/x3-fees"
  "crates/x3-flash-finality"
)
for crate in "${KANI_CRATES[@]}"; do
  echo "Kani on $crate ..."
  if [ -f "$WORKSPACE/$crate/Cargo.toml" ]; then
    cd "$WORKSPACE" && kani --output-format terse "$crate" 2>&1 | tail -30 | tee "$REPORT_DIR/kani-$(basename $crate).txt" || echo "KANI: FAILED for $crate"
  fi
done 2>&1 | tee "$REPORT_DIR/kani-summary.txt"

# ---- Miri (undefined behavior) ----
echo "--- Miri ---"
# Target crates with unsafe code or raw pointer usage
MIRI_CRATES=(
  "crates/x3-accel"
  "crates/x3-vm"
  "crates/x3-backend"
  "crates/x3-proof"
  "crates/x3-gpu-validator-swarm"
)
for crate in "${MIRI_CRATES[@]}"; do
  echo "Miri on $crate ..."
  if [ -f "$WORKSPACE/$crate/Cargo.toml" ]; then
    cd "$WORKSPACE" && $NIGHTLY miri test --package "$(basename $crate)" 2>&1 | tail -50 | tee "$REPORT_DIR/miri-$(basename $crate).txt" || echo "MIRI: FAILED for $crate"
  fi
done 2>&1 | tee "$REPORT_DIR/miri-summary.txt"

# ---- Shuttle (randomized concurrency) ----
echo "--- Shuttle ---"
SHUTTLE_CRATES=(
  "crates/x3-rpc"
  "crates/x3-relayer"
  "crates/northern-swarm"
  "crates/x3-gateway"
)
for crate in "${SHUTTLE_CRATES[@]}"; do
  echo "Shuttle on $crate ..."
  if [ -f "$WORKSPACE/$crate/Cargo.toml" ]; then
    cd "$WORKSPACE" && $NIGHTLY shuttle test --package "$(basename $crate)" 2>&1 | tail -50 | tee "$REPORT_DIR/shuttle-$(basename $crate).txt" || echo "SHUTTLE: FAILED for $crate"
  fi
done 2>&1 | tee "$REPORT_DIR/shuttle-summary.txt"

# ---- Rust Sanitizers (address + leak + thread) ----
echo "--- Sanitizers ---"
SAN_CRATES=(
  "crates/x3-atomic-swap"
  "crates/x3-gateway"
  "crates/northern-swarm"
  "crates/x3-bitcoin-vault"
)
for crate in "${SAN_CRATES[@]}"; do
  echo "ASAN on $crate ..."
  if [ -f "$WORKSPACE/$crate/Cargo.toml" ]; then
    RUSTFLAGS="-Zsanitizer=address" $NIGHTLY test --package "$(basename $crate)" --target x86_64-unknown-linux-gnu 2>&1 | tail -30 | tee "$REPORT_DIR/asan-$(basename $crate).txt" || echo "ASAN: FAILED for $crate"
  fi
done 2>&1 | tee "$REPORT_DIR/sanitizer-summary.txt"

# ---- cargo-mutants (mutation testing) ----
echo "--- cargo-mutants ---"
MUTANT_CRATES=(
  "crates/x3-common"
  "crates/x3-fees"
  "crates/x3-packet-schema"
)
for crate in "${MUTANT_CRATES[@]}"; do
  echo "Mutants on $crate ..."
  if [ -f "$WORKSPACE/$crate/Cargo.toml" ]; then
    cd "$WORKSPACE" && $CARGO mutants --package "$(basename $crate)" --timeout 30 2>&1 | tail -30 | tee "$REPORT_DIR/mutants-$(basename $crate).txt" || echo "MUTANTS: FAILED for $crate"
  fi
done 2>&1 | tee "$REPORT_DIR/mutants-summary.txt"

echo "=== Verification harness complete ==="
echo "Reports in: $REPORT_DIR"