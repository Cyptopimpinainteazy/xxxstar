#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# Simple Audit Pack Generator (no repomix)
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

REPO_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
PACK_DIR="${REPO_ROOT}/launch-gates/repomix"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

mkdir -p "${PACK_DIR}"
cd "${REPO_ROOT}"

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "X3 AUDIT PACK GENERATOR (Direct File Listing)"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Repository: ${REPO_ROOT}"
echo "Output directory: ${PACK_DIR}"
echo "Timestamp: ${TIMESTAMP}"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 1: Full Repository Manifest
# ═══════════════════════════════════════════════════════════════════════════════

echo "[1/5] Generating full-repository audit pack..."

FULL_PACK="${PACK_DIR}/x3-full-repo-${TIMESTAMP}.md"

cat > "${FULL_PACK}" << 'MANIFEST'
# X3 Atomic Star - Full Repository Manifest

## Repository Structure

MANIFEST

find crates pallets runtime node integration-tests -type f \( -name "*.rs" -o -name "*.toml" \) 2>/dev/null | head -100 | sed 's/^/- /' >> "${FULL_PACK}"

cat >> "${FULL_PACK}" << 'MANIFEST'

## Key Files

MANIFEST

ls -lh Cargo.lock *.toml *.md 2>/dev/null | awk '{print "- " $9 " (" $5 ")"}' >> "${FULL_PACK}"

echo "  ✅ Created: $(basename ${FULL_PACK}) ($(du -h ${FULL_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 2: Bridge & Atomic Critical Path
# ═══════════════════════════════════════════════════════════════════════════════

echo "[2/5] Generating bridge & atomic execution critical path pack..."

BRIDGE_PACK="${PACK_DIR}/x3-bridge-atomic-${TIMESTAMP}.md"

cat > "${BRIDGE_PACK}" << 'MANIFEST'
# X3 Bridge & Atomic Trade - Critical Path

## Source Files

MANIFEST

find crates/x3-bridge crates/x3-atomic-trade pallets/x3-atlas-kernel -type f -name "*.rs" 2>/dev/null | sed 's/^/- /' >> "${BRIDGE_PACK}"

echo "  ✅ Created: $(basename ${BRIDGE_PACK}) ($(du -h ${BRIDGE_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 3: Runtime & Consensus
# ═══════════════════════════════════════════════════════════════════════════════

echo "[3/5] Generating runtime & consensus pack..."

RUNTIME_PACK="${PACK_DIR}/x3-runtime-consensus-${TIMESTAMP}.md"

cat > "${RUNTIME_PACK}" << 'MANIFEST'
# X3 Runtime & Consensus

## Source Files

MANIFEST

find runtime pallets -type f -name "*.rs" 2>/dev/null | head -50 | sed 's/^/- /' >> "${RUNTIME_PACK}"

echo "  ✅ Created: $(basename ${RUNTIME_PACK}) ($(du -h ${RUNTIME_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 4: Tests
# ═══════════════════════════════════════════════════════════════════════════════

echo "[4/5] Generating test coverage pack..."

TESTS_PACK="${PACK_DIR}/x3-tests-${TIMESTAMP}.md"

cat > "${TESTS_PACK}" << 'MANIFEST'
# X3 Test Coverage

## Test Files

MANIFEST

find . -name "*test*.rs" -o -name "*tests.rs" 2>/dev/null | grep -v target | grep -v ".git" | head -50 | sed 's/^/- /' >> "${TESTS_PACK}"

echo "  ✅ Created: $(basename ${TESTS_PACK}) ($(du -h ${TESTS_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# PACK 5: Git History
# ═══════════════════════════════════════════════════════════════════════════════

echo "[5/5] Generating git history & documentation pack..."

GIT_PACK="${PACK_DIR}/x3-git-drift-${TIMESTAMP}.md"

cat > "${GIT_PACK}" << 'MANIFEST'
# X3 Git History & Documentation

## Recent Commits (last 30)

MANIFEST

git log --oneline -30 >> "${GIT_PACK}" 2>/dev/null || echo "No git history" >> "${GIT_PACK}"

cat >> "${GIT_PACK}" << 'MANIFEST'

## Documentation Files

MANIFEST

find . -name "*.md" -type f -not -path "./.git/*" -not -path "./target/*" -not -path "./node_modules/*" 2>/dev/null | head -50 | sed 's/^/- /' >> "${GIT_PACK}"

cat >> "${GIT_PACK}" << 'MANIFEST'

## Proof System

MANIFEST

head -50 launch-gates/proofs.yaml >> "${GIT_PACK}" 2>/dev/null

echo "  ✅ Created: $(basename ${GIT_PACK}) ($(du -h ${GIT_PACK} | cut -f1))"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# Generate Manifest
# ═══════════════════════════════════════════════════════════════════════════════

echo "Generating pack manifest..."

MANIFEST_FILE="${PACK_DIR}/pack-manifest-${TIMESTAMP}.txt"

cat > "${MANIFEST_FILE}" << EOF
# X3 Audit Packs - Manifest

Generated: ${TIMESTAMP}
Repository: ${REPO_ROOT}

## Packs Created

1. x3-full-repo-${TIMESTAMP}.md - Full repository inventory
2. x3-bridge-atomic-${TIMESTAMP}.md - Bridge and atomic execution critical path
3. x3-runtime-consensus-${TIMESTAMP}.md - Runtime and consensus components
4. x3-tests-${TIMESTAMP}.md - Test coverage inventory
5. x3-git-drift-${TIMESTAMP}.md - Git history and documentation

## SHA256 Hashes

EOF

sha256sum repomix/x3-*.md >> "${MANIFEST_FILE}" 2>/dev/null || echo "Generated: ${TIMESTAMP}" >> "${MANIFEST_FILE}"

echo "  ✅ Created: $(basename ${MANIFEST_FILE})"
echo ""

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "✅ All packs generated successfully!"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "Packs ready in: ${PACK_DIR}"
echo ""
ls -lh "${PACK_DIR}" | tail -10
