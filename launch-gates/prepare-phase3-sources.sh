#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 PHASE 3 SOURCE EXTRACTOR - Prepare Code for AI Auditing
# ═══════════════════════════════════════════════════════════════════════════════
# Extracts targeted source code for each audit context
# Creates lean, focused code packs ready for AI analysis
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

REPO_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
SOURCE_DIR="${REPO_ROOT}/launch-gates/sources"
mkdir -p "${SOURCE_DIR}"
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 PHASE 3: SOURCE CODE EXTRACTION - $(date)"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# SOURCE PACK 1: WIRING VERIFICATION
# ═══════════════════════════════════════════════════════════════════════════════
echo "[1/5] Extracting Wiring Verification Sources..."
mkdir -p "${SOURCE_DIR}/pack-01-wiring"

# Extract runtime construct_runtime! macro
echo "# Runtime Construct Macro" > "${SOURCE_DIR}/pack-01-wiring/01-construct-runtime.rs"
grep -A 300 "construct_runtime!" runtime/src/lib.rs >> "${SOURCE_DIR}/pack-01-wiring/01-construct-runtime.rs" 2>/dev/null || echo "# Extracted construct_runtime macro"

# Extract all pallet lib.rs files
mkdir -p "${SOURCE_DIR}/pack-01-wiring/pallets"
for pallet_dir in pallets/*/; do
  pallet_name=$(basename "$pallet_dir")
  if [ -f "${pallet_dir}src/lib.rs" ]; then
    cp "${pallet_dir}src/lib.rs" "${SOURCE_DIR}/pack-01-wiring/pallets/${pallet_name}-lib.rs"
  fi
done

# Extract Cargo.toml for runtime
cp runtime/Cargo.toml "${SOURCE_DIR}/pack-01-wiring/runtime-Cargo.toml"

PACK1_SIZE=$(du -sh "${SOURCE_DIR}/pack-01-wiring" | cut -f1)
echo "✅ Wiring sources: $PACK1_SIZE"

# ═══════════════════════════════════════════════════════════════════════════════
# SOURCE PACK 2: MAINNET READINESS
# ═══════════════════════════════════════════════════════════════════════════════
echo "[2/5] Extracting Mainnet Readiness Sources..."
mkdir -p "${SOURCE_DIR}/pack-02-mainnet"

# Extract pallet implementations (first 500 lines each for size)
for pallet in pallets/*/src/lib.rs; do
  pallet_name=$(basename $(dirname $pallet))
  head -500 "$pallet" > "${SOURCE_DIR}/pack-02-mainnet/${pallet_name}-excerpt.rs"
done

# Extract runtime config
grep -A 100 "pub struct Runtime" runtime/src/lib.rs > "${SOURCE_DIR}/pack-02-mainnet/runtime-config.rs" 2>/dev/null || true

# Extract Cargo.lock first 1000 lines (dependency info)
head -1000 Cargo.lock > "${SOURCE_DIR}/pack-02-mainnet/dependencies-excerpt.lock"

PACK2_SIZE=$(du -sh "${SOURCE_DIR}/pack-02-mainnet" | cut -f1)
echo "✅ Mainnet sources: $PACK2_SIZE"

# ═══════════════════════════════════════════════════════════════════════════════
# SOURCE PACK 3: BRIDGE & ATOMIC SECURITY
# ═══════════════════════════════════════════════════════════════════════════════
echo "[3/5] Extracting Bridge & Atomic Security Sources..."
mkdir -p "${SOURCE_DIR}/pack-03-bridge-atomic"

# Critical pallets for bridge/atomic
CRITICAL_PALLETS=("pallet-bridge" "x3-atomic-router" "x3-cross-vm-router" "settlement")

for pallet_name in "${CRITICAL_PALLETS[@]}"; do
  PALLET_PATH="pallets/${pallet_name}"
  if [ -d "$PALLET_PATH" ]; then
    # Main implementation
    if [ -f "${PALLET_PATH}/src/lib.rs" ]; then
      cp "${PALLET_PATH}/src/lib.rs" "${SOURCE_DIR}/pack-03-bridge-atomic/${pallet_name}-lib.rs"
    fi
    
    # Tests
    if [ -f "${PALLET_PATH}/src/tests.rs" ]; then
      cp "${PALLET_PATH}/src/tests.rs" "${SOURCE_DIR}/pack-03-bridge-atomic/${pallet_name}-tests.rs"
    fi
    
    # Security-critical modules (replay, finality, etc)
    for module in "${PALLET_PATH}"/src/*.rs; do
      if [[ "$module" == *"replay"* ]] || [[ "$module" == *"security"* ]] || [[ "$module" == *"finality"* ]]; then
        cp "$module" "${SOURCE_DIR}/pack-03-bridge-atomic/$(basename $module)"
      fi
    done
  fi
done

PACK3_SIZE=$(du -sh "${SOURCE_DIR}/pack-03-bridge-atomic" | cut -f1)
echo "✅ Bridge/Atomic sources: $PACK3_SIZE"

# ═══════════════════════════════════════════════════════════════════════════════
# SOURCE PACK 4: INVARIANT HUNTING
# ═══════════════════════════════════════════════════════════════════════════════
echo "[4/5] Extracting Invariant Hunter Sources..."
mkdir -p "${SOURCE_DIR}/pack-04-invariant"

# Extract all test files (invariant tests)
find pallets -name "tests.rs" -o -name "*test*.rs" | while read test_file; do
  test_name=$(basename "$test_file")
  pallet=$(basename $(dirname $(dirname "$test_file")))
  mkdir -p "${SOURCE_DIR}/pack-04-invariant/${pallet}"
  cp "$test_file" "${SOURCE_DIR}/pack-04-invariant/${pallet}/${test_name}"
done

# Extract integration tests
if [ -d "integration-tests" ]; then
  cp -r integration-tests "${SOURCE_DIR}/pack-04-invariant/integration-tests" 2>/dev/null || true
fi

PACK4_SIZE=$(du -sh "${SOURCE_DIR}/pack-04-invariant" | cut -f1)
echo "✅ Invariant sources: $PACK4_SIZE"

# ═══════════════════════════════════════════════════════════════════════════════
# SOURCE PACK 5: TEST GAP ANALYSIS
# ═══════════════════════════════════════════════════════════════════════════════
echo "[5/5] Extracting Test Gap Analysis Sources..."
mkdir -p "${SOURCE_DIR}/pack-05-test-gap"

# All Rust source files (organized by category)
mkdir -p "${SOURCE_DIR}/pack-05-test-gap/runtime"
mkdir -p "${SOURCE_DIR}/pack-05-test-gap/pallets"
mkdir -p "${SOURCE_DIR}/pack-05-test-gap/tests"

# Runtime sources (first 1000 lines)
head -1000 runtime/src/lib.rs > "${SOURCE_DIR}/pack-05-test-gap/runtime/lib-excerpt.rs"

# Key pallet sources with test focus
for pallet_dir in pallets/*/; do
  pallet_name=$(basename "$pallet_dir")
  if [ -f "${pallet_dir}src/lib.rs" ]; then
    head -500 "${pallet_dir}src/lib.rs" > "${SOURCE_DIR}/pack-05-test-gap/pallets/${pallet_name}-excerpt.rs"
  fi
  if [ -f "${pallet_dir}src/tests.rs" ]; then
    cp "${pallet_dir}src/tests.rs" "${SOURCE_DIR}/pack-05-test-gap/pallets/${pallet_name}-tests.rs"
  fi
done

# Core test suites
if [ -d "tests" ]; then
  cp tests/*.rs "${SOURCE_DIR}/pack-05-test-gap/tests/" 2>/dev/null || true
fi

PACK5_SIZE=$(du -sh "${SOURCE_DIR}/pack-05-test-gap" | cut -f1)
echo "✅ Test gap sources: $PACK5_SIZE"

echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "✅ PHASE 3 SOURCES READY: 5 Code Packs Prepared"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# Summary
echo "Source Code Packs Created:"
echo "  pack-01-wiring/           $PACK1_SIZE - Runtime wiring + all pallets"
echo "  pack-02-mainnet/          $PACK2_SIZE - Mainnet readiness implementation"
echo "  pack-03-bridge-atomic/    $PACK3_SIZE - Bridge/Atomic/CrossVM security code"
echo "  pack-04-invariant/        $PACK4_SIZE - All tests for invariant checking"
echo "  pack-05-test-gap/         $PACK5_SIZE - Full test coverage analysis"
echo ""

TOTAL_SIZE=$(du -sh "${SOURCE_DIR}" | cut -f1)
echo "Total size: $TOTAL_SIZE"
echo ""

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "PHASE 3 READY: Use context + source pairs for AI auditing"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Example AI Audit Session:"
echo "  Context: launch-gates/audits/audit-01-wiring-context.json"
echo "  Sources: launch-gates/sources/pack-01-wiring/"
echo "  Prompt:  'Using the context, audit this codebase for wiring issues'"
echo ""
echo "Repeat for audits 2-5 with corresponding context and source pairs"
