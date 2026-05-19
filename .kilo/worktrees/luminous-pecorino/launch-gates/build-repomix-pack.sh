#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# X3 ProofGate: Build Repomix Evidence Pack
# ═══════════════════════════════════════════════════════════════════════════════
#
# Generates a complete Repomix dump of the X3 repository specifically for
# audit purposes. Includes:
# - All Rust source code
# - All tests
# - All configuration
# - Recent git history
# - Documentation
#
# Outputs: Single markdown file with reproducibility hash
# Size: ~10-30MB depending on repo state
#
# Usage: ./build-repomix-pack.sh
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

REPO_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
PACK_DIR="${REPO_ROOT}/launch-gates/repomix"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

mkdir -p "${PACK_DIR}"

# Change to repo root so relative paths work
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 REPOMIX PACK BUILDER"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Repository: ${REPO_ROOT}"
echo "Output directory: ${PACK_DIR}"
echo "Timestamp: ${TIMESTAMP}"
echo "Working directory: $(pwd)"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 1: Full Repository Audit Pack
# ═══════════════════════════════════════════════════════════════════════════════

echo "[1/5] Generating full-repository audit pack..."

FULL_PACK="${PACK_DIR}/x3-full-repo-${TIMESTAMP}.md"

npx repomix@latest \
  --include "crates/**/*.rs,pallets/**/*.rs,runtime/**/*.rs,node/**/*.rs,integration-tests/**/*.rs,*.toml,Cargo.lock,docs/**/*.md,README.md,*.md" \
  --ignore "target/**,node_modules/**,.git/**,launch-gates/repomix/**,*.wasm,*.so,*.o" \
  --output "${FULL_PACK}" \
  --style markdown 2>&1

echo "  ✅ Created: $(basename ${FULL_PACK}) ($(du -h ${FULL_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 2: Bridge & Atomic Execution Critical Path
# ═══════════════════════════════════════════════════════════════════════════════

echo "[2/5] Generating bridge & atomic execution critical path pack..."

BRIDGE_PACK="${PACK_DIR}/x3-bridge-atomic-${TIMESTAMP}.md"

npx repomix@latest \
  --include "crates/x3-bridge/**/*.rs,crates/x3-atomic-trade/**/*.rs,pallets/x3-atlas-kernel/**/*.rs,integration-tests/**/*bridge*.rs,integration-tests/**/*atomic*.rs,integration-tests/**/*supply*.rs" \
  --ignore "target/**,node_modules/**,.git/**,launch-gates/repomix/**" \
  --output "${BRIDGE_PACK}" \
  --style markdown 2>&1

echo "  ✅ Created: $(basename ${BRIDGE_PACK}) ($(du -h ${BRIDGE_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 3: Runtime & Consensus
# ═══════════════════════════════════════════════════════════════════════════════

echo "[3/5] Generating runtime & consensus pack..."

RUNTIME_PACK="${PACK_DIR}/x3-runtime-consensus-${TIMESTAMP}.md"

npx repomix@latest \
  --include "runtime/**/*.rs,pallets/**/*.rs,node/src/*.rs,*.toml,Cargo.lock" \
  --ignore "target/**,node_modules/**,.git/**,launch-gates/repomix/**" \
  --output "${RUNTIME_PACK}" \
  --style markdown 2>&1

echo "  ✅ Created: $(basename ${RUNTIME_PACK}) ($(du -h ${RUNTIME_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 4: All Tests
# ═══════════════════════════════════════════════════════════════════════════════

echo "[4/5] Generating test coverage pack..."

TESTS_PACK="${PACK_DIR}/x3-tests-${TIMESTAMP}.md"

npx repomix@latest \
  --include "integration-tests/**/*.rs,crates/*/tests/**/*.rs,pallets/*/tests/**/*.rs,**/tests.rs" \
  --ignore "target/**,node_modules/**,.git/**,launch-gates/repomix/**" \
  --output "${TESTS_PACK}" \
  --style markdown 2>&1

echo "  ✅ Created: $(basename ${TESTS_PACK}) ($(du -h ${TESTS_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 5: Git Recent Changes (last 30 commits)
# ═══════════════════════════════════════════════════════════════════════════════

echo "[5/5] Generating git history & documentation pack..."

GIT_PACK="${PACK_DIR}/x3-git-drift-${TIMESTAMP}.md"

{
  echo "# X3 Git History & Documentation Alignment"
  echo ""
  echo "## Recent Commits (last 30)"
  echo ""
  cd "${REPO_ROOT}"
  git log --oneline -30
  echo ""
  echo "## Changed Files (last 50 commits)"
  echo ""
  git diff --name-only HEAD~50..HEAD | sort | uniq
  echo ""
  echo "## Documentation Files"
  echo ""
  find . -name "*.md" -type f | grep -v node_modules | grep -v target | grep -v ".git" | head -50
  echo ""
  echo "## Key Configuration Files"
  echo ""
  find . -name "*.toml" -type f | grep -v node_modules | grep -v target | head -20
  echo ""
  echo "## Architecture Documentation"
  echo ""
  ls -la docs/ || echo "No docs/ directory found"
  echo ""
  echo "## Proof Manifest"
  echo ""
  cat launch-gates/proofs.yaml 2>/dev/null || echo "No proofs.yaml found"
  echo ""
  echo "## Critical Invariants"
  echo ""
  cat launch-gates/invariants.yaml 2>/dev/null || echo "No invariants.yaml found"
} | tee "${GIT_PACK}"

echo "  ✅ Created: $(basename ${GIT_PACK}) ($(du -h ${GIT_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# Generate Hashes & Manifest
# ═══════════════════════════════════════════════════════════════════════════════

echo "Generating cryptographic hashes..."
echo ""

MANIFEST="${PACK_DIR}/pack-manifest-${TIMESTAMP}.txt"

{
  echo "X3 ProofGate - Evidence Pack Manifest"
  echo "======================================"
  echo ""
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Repository: ${REPO_ROOT}"
  echo "Commit: $(cd ${REPO_ROOT} && git rev-parse HEAD)"
  echo ""
  echo "Packs:"
  echo ""
  echo "1. Full Repository:"
  sha256sum "${FULL_PACK}" | awk '{print "   SHA256: " $1}'
  echo "   Size: $(du -h ${FULL_PACK} | cut -f1)"
  echo "   Path: $(basename ${FULL_PACK})"
  echo ""
  echo "2. Bridge & Atomic (Critical Path):"
  sha256sum "${BRIDGE_PACK}" | awk '{print "   SHA256: " $1}'
  echo "   Size: $(du -h ${BRIDGE_PACK} | cut -f1)"
  echo "   Path: $(basename ${BRIDGE_PACK})"
  echo ""
  echo "3. Runtime & Consensus:"
  sha256sum "${RUNTIME_PACK}" | awk '{print "   SHA256: " $1}'
  echo "   Size: $(du -h ${RUNTIME_PACK} | cut -f1)"
  echo "   Path: $(basename ${RUNTIME_PACK})"
  echo ""
  echo "4. All Tests:"
  sha256sum "${TESTS_PACK}" | awk '{print "   SHA256: " $1}'
  echo "   Size: $(du -h ${TESTS_PACK} | cut -f1)"
  echo "   Path: $(basename ${TESTS_PACK})"
  echo ""
  echo "5. Git History & Docs:"
  sha256sum "${GIT_PACK}" | awk '{print "   SHA256: " $1}'
  echo "   Size: $(du -h ${GIT_PACK} | cut -f1)"
  echo "   Path: $(basename ${GIT_PACK})"
  echo ""
  echo "How to use these packs:"
  echo "========================"
  echo ""
  echo "For Wiring Audit:"
  echo "  1. Copy pack #1 (full-repo) to Claude"
  echo "  2. Paste prompt: launch-gates/prompts/01-wiring-audit.md"
  echo ""
  echo "For Bridge Safety Audit:"
  echo "  1. Copy pack #2 (bridge-atomic) to Claude"
  echo "  2. Paste prompt: launch-gates/prompts/03-bridge-safety-audit.md"
  echo ""
  echo "For Runtime Audit:"
  echo "  1. Copy pack #3 (runtime-consensus) to Claude"
  echo "  2. Paste prompt: launch-gates/prompts/02-mainnet-launch-gate.md"
  echo ""
  echo "For Test Gap Audit:"
  echo "  1. Copy pack #4 (tests) to Claude"
  echo "  2. Paste prompt: launch-gates/prompts/05-test-gap-audit.md"
  echo ""
  echo "For Invariant Review:"
  echo "  1. Copy pack #1 (full-repo) to Claude"
  echo "  2. Paste prompt: launch-gates/prompts/04-invariant-hunter.md"
  echo ""
  echo "Verification:"
  echo "============="
  echo ""
  echo "To verify pack integrity:"
  echo "  sha256sum -c pack-manifest-${TIMESTAMP}.txt"
  echo ""
} | tee "${MANIFEST}"

# ═══════════════════════════════════════════════════════════════════════════════
# Final Summary
# ═══════════════════════════════════════════════════════════════════════════════

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "REPOMIX PACK BUILD COMPLETE"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "All packs generated in: ${PACK_DIR}"
echo ""
echo "Pack summary:"
du -h "${PACK_DIR}"/x3-*.md | awk '{print "  " $2 ": " $1}'
echo ""
echo "Total size:"
du -sh "${PACK_DIR}" | awk '{print "  " $1}'
echo ""
echo "Manifest: $(basename ${MANIFEST})"
echo ""
echo "Next steps:"
echo "  1. Copy each pack (starting with full-repo) to Claude"
echo "  2. Paste the corresponding audit prompt from launch-gates/prompts/"
echo "  3. Save each report to launch-gates/reports/"
echo "  4. Update MAINNET_READINESS.json with scores"
echo ""
echo "Completion time: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""
