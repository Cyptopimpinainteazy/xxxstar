#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 ProofGate: Build 5 Targeted Repomix Audit Packs
# ═══════════════════════════════════════════════════════════════════════════════
#
# Creates targeted audit packs for different audit profiles:
# 1. FULL REPO        - architecture review, missing wiring
# 2. RUNTIME/CONSENSUS - pallet composition, construct_runtime!, benchmarks
# 3. BRIDGE/ATOMIC     - cross-VM wiring, replay protection, rollback paths
# 4. TEST COVERAGE     - test suite analysis, missing tests
# 5. GIT DRIFT         - code vs docs, recent changes
#
# Each pack is a complete markdown file ready for AI audit.
# Each pack includes a reproducibility hash for traceability.
#
# Usage: ./build-audit-packs.sh
# Output: launch-gates/repomix/audit-pack-*.txt (with hashes)
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

REPO_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
PACK_DIR="${REPO_ROOT}/launch-gates/repomix"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

mkdir -p "${PACK_DIR}"
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 AUDIT PACK BUILDER - $(date)"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 1: FULL REPOSITORY
# ═══════════════════════════════════════════════════════════════════════════════
echo "[1/5] Building FULL REPOSITORY pack..."
PACK_FILE="${PACK_DIR}/audit-pack-01-full-repo-${TIMESTAMP}.txt"

npx repomix@latest \
  --include "crates/**/*.rs,pallets/**/*.rs,runtime/**/*.rs,node/**/*.rs,web/**/*,*.toml,*.md,scripts/**/*.sh,*.json" \
  --ignore "target/**,node_modules/**,.git/**,**/*.wasm,**/*.bin,**/.DS_Store" \
  --output "${PACK_FILE}" \
  2>&1 | head -20

sha256sum "${PACK_FILE}" > "${PACK_DIR}/audit-pack-01-full-repo-${TIMESTAMP}.sha256"

echo "✅ FULL REPO pack: $(du -h ${PACK_FILE} | cut -f1)"
echo "   Hash: $(cat ${PACK_DIR}/audit-pack-01-full-repo-${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 2: RUNTIME & CONSENSUS
# ═══════════════════════════════════════════════════════════════════════════════
echo "[2/5] Building RUNTIME/CONSENSUS pack..."
PACK_FILE="${PACK_DIR}/audit-pack-02-runtime-consensus-${TIMESTAMP}.txt"

npx repomix@latest \
  --include "runtime/**/*.rs,pallets/**/*.rs,crates/x3-runtime/**/*.rs,crates/x3-finality-oracle/**/*.rs,crates/x3-consensus/**/*.rs,benchmarks/**/*.rs,Cargo.toml,Cargo.lock,*.md" \
  --ignore "target/**,.git/**,node_modules/**" \
  --output "${PACK_FILE}" \
  2>&1 | head -20

sha256sum "${PACK_FILE}" > "${PACK_DIR}/audit-pack-02-runtime-consensus-${TIMESTAMP}.sha256"

echo "✅ RUNTIME/CONSENSUS pack: $(du -h ${PACK_FILE} | cut -f1)"
echo "   Hash: $(cat ${PACK_DIR}/audit-pack-02-runtime-consensus-${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 3: BRIDGE & ATOMIC CROSS-VM
# ═══════════════════════════════════════════════════════════════════════════════
echo "[3/5] Building BRIDGE/ATOMIC CROSS-VM pack..."
PACK_FILE="${PACK_DIR}/audit-pack-03-bridge-atomic-${TIMESTAMP}.txt"

npx repomix@latest \
  --include "crates/x3-bridge/**/*.rs,crates/x3-atomic-trade/**/*.rs,crates/evm-integration/**/*.rs,crates/svm-integration/**/*.rs,crates/**/tests/**/*.rs,crates/**/benches/**/*.rs,*.md,Cargo.toml" \
  --ignore "target/**,.git/**,node_modules/**" \
  --output "${PACK_FILE}" \
  2>&1 | head -20

sha256sum "${PACK_FILE}" > "${PACK_DIR}/audit-pack-03-bridge-atomic-${TIMESTAMP}.sha256"

echo "✅ BRIDGE/ATOMIC pack: $(du -h ${PACK_FILE} | cut -f1)"
echo "   Hash: $(cat ${PACK_DIR}/audit-pack-03-bridge-atomic-${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 4: TEST COVERAGE
# ═══════════════════════════════════════════════════════════════════════════════
echo "[4/5] Building TEST COVERAGE pack..."
PACK_FILE="${PACK_DIR}/audit-pack-04-test-coverage-${TIMESTAMP}.txt"

npx repomix@latest \
  --include "**/tests/**/*.rs,**/*test*.rs,**/*.spec.ts,**/*.test.ts,proptest*.rs,fuzz/**/*.rs,Cargo.toml,foundry.toml,hardhat.config.*,README.md" \
  --ignore "target/**,node_modules/**,.git/**" \
  --output "${PACK_FILE}" \
  2>&1 | head -20

sha256sum "${PACK_FILE}" > "${PACK_DIR}/audit-pack-04-test-coverage-${TIMESTAMP}.sha256"

echo "✅ TEST COVERAGE pack: $(du -h ${PACK_FILE} | cut -f1)"
echo "   Hash: $(cat ${PACK_DIR}/audit-pack-04-test-coverage-${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 5: GIT DRIFT (Recent changes + diffs)
# ═══════════════════════════════════════════════════════════════════════════════
echo "[5/5] Building GIT DRIFT pack..."
PACK_FILE="${PACK_DIR}/audit-pack-05-git-drift-${TIMESTAMP}.txt"

npx repomix@latest \
  --include "crates/**/*.rs,pallets/**/*.rs,runtime/**/*.rs,*.toml,*.md" \
  --ignore "target/**,.git/**,node_modules/**" \
  --include-logs \
  --include-diffs \
  --output "${PACK_FILE}" \
  2>&1 | head -20

sha256sum "${PACK_FILE}" > "${PACK_DIR}/audit-pack-05-git-drift-${TIMESTAMP}.sha256"

echo "✅ GIT DRIFT pack: $(du -h ${PACK_FILE} | cut -f1)"
echo "   Hash: $(cat ${PACK_DIR}/audit-pack-05-git-drift-${TIMESTAMP}.sha256 | cut -d' ' -f1 | head -c 16)..."
echo ""

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "✅ ALL 5 AUDIT PACKS GENERATED"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Packs are ready for AI audit:"
echo ""
echo "1. FULL REPO        → $(ls -1 ${PACK_DIR}/audit-pack-01* 2>/dev/null | head -1 | xargs -I{} basename {})"
echo "   Use for: Architecture review, missing wiring, repo structure"
echo ""
echo "2. RUNTIME/CONSENSUS → $(ls -1 ${PACK_DIR}/audit-pack-02* 2>/dev/null | head -1 | xargs -I{} basename {})"
echo "   Use for: Pallet composition, construct_runtime!, benchmark verification"
echo ""
echo "3. BRIDGE/ATOMIC     → $(ls -1 ${PACK_DIR}/audit-pack-03* 2>/dev/null | head -1 | xargs -I{} basename {})"
echo "   Use for: Cross-VM wiring, replay protection, atomicity guarantees"
echo ""
echo "4. TEST COVERAGE     → $(ls -1 ${PACK_DIR}/audit-pack-04* 2>/dev/null | head -1 | xargs -I{} basename {})"
echo "   Use for: Missing tests, coverage gaps, invariant testing"
echo ""
echo "5. GIT DRIFT         → $(ls -1 ${PACK_DIR}/audit-pack-05* 2>/dev/null | head -1 | xargs -I{} basename {})"
echo "   Use for: Code/docs alignment, recent changes, release risk"
echo ""
echo "Next: Feed each pack to an AI auditor with the prompts in:"
echo "  launch-gates/prompts/"
echo ""
