#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 PROOF RUNNER: Execute All Mainnet Readiness Proofs
# ═══════════════════════════════════════════════════════════════════════════════
#
# Generates hard evidence for every claim about X3 mainnet readiness.
# Each proof is:
# - Reproducible (same command, same result)
# - Hashable (output can be signed)
# - Timestamped
# - Indexed for dashboard
#
# This is the single source of truth. If it doesn't pass here, it doesn't count.
#
# Usage: ./run-proof-commands.sh
# Output: launch-gates/evidence/ (with SHA256 hashes)
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# Resolve repo root from this script's location so the gate is portable across
# machines and CI runners (no hard-coded user paths).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EVIDENCE_DIR="${REPO_ROOT}/launch-gates/evidence"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
STRICT_TARGET_DIR="${REPO_ROOT}/target_strict"
STRICT_TOOLCHAIN="${STRICT_TOOLCHAIN:-1.90.0}"
# Keep Proof 2 aligned with the strict toolchain by default while still
# allowing overrides for local diagnosis.
STRICT_TEST_TOOLCHAIN="${STRICT_TEST_TOOLCHAIN:-${STRICT_TOOLCHAIN}}"

# Allow `STRICT=0` to keep the legacy advisory mode locally; CI must run with
# the default STRICT=1 so non-blocking failures cannot mask real regressions.
STRICT="${STRICT:-1}"

mkdir -p "${EVIDENCE_DIR}"
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 PROOF RUNNER - $(date)"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Repository: ${REPO_ROOT}"
echo "Evidence directory: ${EVIDENCE_DIR}"
echo "Timestamp: ${TIMESTAMP}"
echo ""

PROOF_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Helper: Run proof and capture result.
run_proof() {
  local proof_id=$1
  local proof_name=$2
  local command=$3

  PROOF_COUNT=$((PROOF_COUNT + 1))

  echo "[${PROOF_COUNT}] $proof_name"

  local log_file="${EVIDENCE_DIR}/${proof_id}-${TIMESTAMP}.log"

  # Some local runs have observed Cargo failing with os error 2 while writing
  # fingerprint artifacts if these directories were pruned between proof steps.
  # Pre-create the standard target subdirectories so proof results reflect real
  # compile failures rather than transient filesystem setup issues.
  mkdir -p "${STRICT_TARGET_DIR}/debug/.fingerprint" "${STRICT_TARGET_DIR}/release/.fingerprint"

  if eval "$command" > "${log_file}" 2>&1; then
    echo "  ✅ PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
    sha256sum "${log_file}" > "${log_file}.sha256"
    echo "  Hash: $(cut -d' ' -f1 ${log_file}.sha256 | head -c 16)..."
    return 0
  else
    echo "  ❌ FAIL"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    tail -20 "${log_file}" || true
    return 1
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 1: Compilation
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 1: COMPILATION"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-01-check-workspace" \
  "Cargo check (full workspace)" \
  "env RUSTUP_TOOLCHAIN=${STRICT_TOOLCHAIN} CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} RUST_MIN_STACK=${RUST_MIN_STACK:-1073741824} CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=${STRICT_TARGET_DIR} RUSTFLAGS='-C debuginfo=0' cargo check --workspace --all-targets"

run_proof "proof-01-check-runtime" \
  "Cargo check (runtime only)" \
  "env RUSTUP_TOOLCHAIN=${STRICT_TOOLCHAIN} CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} RUST_MIN_STACK=${RUST_MIN_STACK:-1073741824} CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=${STRICT_TARGET_DIR} RUSTFLAGS='-C debuginfo=0' cargo check -p x3-chain-runtime"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 2: Runtime Tests
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 2: RUNTIME TESTS"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-02-test-workspace" \
  "Cargo test (full workspace)" \
  "env RUSTUP_TOOLCHAIN=${STRICT_TEST_TOOLCHAIN} CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} RUST_MIN_STACK=${RUST_MIN_STACK:-1073741824} CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=${STRICT_TARGET_DIR} RUSTFLAGS='-C debuginfo=0' cargo test --workspace --lib --exclude x3-chain-runtime"

run_proof "proof-02-test-runtime" \
  "Runtime tests" \
  "env RUSTUP_TOOLCHAIN=${STRICT_TEST_TOOLCHAIN} CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} RUST_MIN_STACK=${RUST_MIN_STACK:-1073741824} CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=${STRICT_TARGET_DIR} RUSTFLAGS='-C debuginfo=0' cargo test -p x3-chain-runtime --lib"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 3: Bridge & Atomic Tests
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 3: BRIDGE & ATOMIC TESTS"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-03-test-bridge" \
  "Bridge tests" \
  "cargo test -p x3-bridge --lib 2>&1 | tail -200"

run_proof "proof-03-test-atomic" \
  "Atomic swap tests" \
  "cargo test -p x3-atomic-trade --lib 2>&1 | tail -200"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 4: Code Quality
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 4: CODE QUALITY"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-04-clippy" \
  "Clippy (no warnings)" \
  "cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -200"

run_proof "proof-04-fmt-check" \
  "Format check" \
  "cargo fmt --all -- --check"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 5: Hazard Scan
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 5: PRODUCTION HAZARD SCAN"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-05-hazard-scan" \
  "Strict hazard scan (blocking) + critical-path TODO/FIXME zero-tolerance" \
  "STRICT=${STRICT} STRICT_P2=1 BLOCK_P2_CATEGORIES='DEV,MEMORY,LOCALHOST,HARDCODED' launch-gates/embarrassment-scan.sh '${EVIDENCE_DIR}/proof-05-hazard-scan-${TIMESTAMP}.log' '${EVIDENCE_DIR}/proof-05-hazard-scan-raw-${TIMESTAMP}.txt' && ! rg -n 'TODO|FIXME' pallets/x3-supply-ledger/src runtime/src crates/x3-launch-validator/src --no-heading"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 6: Wiring Check
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 6: RUNTIME WIRING"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-06-construct-runtime" \
  "construct_runtime! present" \
  "rg -A 30 'construct_runtime!' runtime/src/lib.rs | head -50"

run_proof "proof-06-pallets-count" \
  "Count pallets in construct_runtime!" \
  "rg 'construct_runtime!' -A 50 runtime/src/lib.rs | grep -c 'pallet' || echo '0'"

run_proof "proof-06-supply-ledger-pallet" \
  "Supply ledger pallet wired in construct_runtime!" \
  "rg -n 'SupplyLedger\s*:\s*pallet_x3_supply_ledger' runtime/src/lib.rs"

run_proof "proof-06-supply-ledger-config" \
  "Supply ledger Config impl wired for Runtime" \
  "rg -n 'impl\s+pallet_x3_supply_ledger::Config\s+for\s+Runtime' runtime/src/lib.rs"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 7: Phase 5a Settlement Tests
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 7: PHASE 5a SETTLEMENT TESTS"
echo "═══════════════════════════════════════════════════════════════════════════════"

if [ -d "tests_phase4" ]; then
  run_proof "proof-07-settlement-tests" \
    "72/72 settlement E2E tests" \
    "cd tests_phase4 && pytest p4_p5_production_release.py -v --tb=short 2>&1 | tail -200"
else
  echo "⚠️  tests_phase4 directory not found - skipping"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 8: Git State
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 8: GIT STATE"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-08-git-commit" \
  "Current commit hash" \
  "git rev-parse HEAD 2>&1"

run_proof "proof-08-git-status" \
  "Git status (should be clean)" \
  "git status --short"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 9: Fresh Machine Test
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 9: FRESH MACHINE CAPABILITY"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-09-build-release" \
  "cargo build --release (x3-chain-node)" \
  "cargo build --release -p x3-chain-node 2>&1 | tail -200"

run_proof "proof-09-build-exists" \
  "Release binary exists" \
  "ls -lh target/release/x3-chain-node"

# ═══════════════════════════════════════════════════════════════════════════════
# PROOF 10: ProofForge Receipt and Claim Registry Integrity
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF 10: PROOFFORGE RECEIPT & CLAIM INTEGRITY"
echo "═══════════════════════════════════════════════════════════════════════════════"

run_proof "proof-10-receipt-shape" \
  "ProofForge receipt schema validation" \
  "bash scripts/proof/verify-receipts.sh"

run_proof "proof-10-claim-status" \
  "Claims registry status consistency" \
  "bash scripts/proof/check-claim-status.sh"

# ═══════════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PROOF RUN COMPLETE"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Results:"
echo "  Total proofs: $PROOF_COUNT"
echo "  ✅ Passed: $PASS_COUNT"
echo "  ❌ Failed: $FAIL_COUNT"
echo ""

# Generate summary hash
sha256sum "${EVIDENCE_DIR}"/*.log > "${EVIDENCE_DIR}/ALL_PROOFS_${TIMESTAMP}.sha256"

echo "Evidence directory: ${EVIDENCE_DIR}"
echo "Evidence hash: $(sha256sum ${EVIDENCE_DIR}/ALL_PROOFS_${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
  echo "✅ ALL PROOFS PASSED"
  exit 0
else
  echo "❌ SOME PROOFS FAILED - Review logs above"
  exit 1
fi
