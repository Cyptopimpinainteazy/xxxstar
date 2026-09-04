#!/bin/bash

# repomix-mainnet-pack.sh
# Generate targeted Repomix packs for mainnet audit

set -e

REPO_DIR="/home/lojak/Desktop/X3_ATOMIC_STAR"
PACKS_DIR="$REPO_DIR/launch-gates/packs"
REPORTS_DIR="$REPO_DIR/launch-gates/reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "🚀 Starting Repomix mainnet audit packs..."
echo ""

# Pack 1: Full Repo
echo "📦 Pack 1/5: Full repo audit..."
repomix \
  --output "$PACKS_DIR/full-repo-$TIMESTAMP.md" \
  --style markdown \
  "$REPO_DIR" 2>&1 | grep -E "(Total files|Tokens|Output)" || true
echo "✅ Full repo pack ready"
echo ""

# Pack 2: Runtime & Consensus
echo "📦 Pack 2/5: Runtime & consensus audit..."
repomix \
  --output "$PACKS_DIR/runtime-consensus-$TIMESTAMP.md" \
  --include "crates/**/*.rs,runtime/**/*.rs,pallets/**/*.rs,Cargo.toml,Cargo.lock,*.md" \
  --ignore "target/**,node_modules/**,.git/**,**/*.wasm,**/*.bin,apps/**" \
  --style markdown \
  "$REPO_DIR" 2>&1 | grep -E "(Total files|Tokens|Output)" || true
echo "✅ Runtime pack ready"
echo ""

# Pack 3: Bridge & Atomic
echo "📦 Pack 3/5: Bridge & atomic cross-VM audit..."
repomix \
  --output "$PACKS_DIR/bridge-atomic-$TIMESTAMP.md" \
  --include "crates/x3-bridge*/**/*.rs,crates/x3-atomic*/**/*.rs,crates/x3-cross*/**/*.rs,crates/x3-vm/**/*.rs,crates/x3-svm/**/*.rs,**/tests/**/*.rs,*.md,Cargo.toml" \
  --ignore "target/**,.git/**" \
  --style markdown \
  "$REPO_DIR" 2>&1 | grep -E "(Total files|Tokens|Output)" || true
echo "✅ Bridge pack ready"
echo ""

# Pack 4: Test Coverage
echo "📦 Pack 4/5: Test coverage audit..."
repomix \
  --output "$PACKS_DIR/tests-$TIMESTAMP.md" \
  --include "**/tests/**/*.rs,**/*test*.rs,**/*.spec.ts,**/*.test.ts,proptest*.rs,fuzz/**/*.rs,Cargo.toml" \
  --ignore "target/**,node_modules/**,.git/**" \
  --style markdown \
  "$REPO_DIR" 2>&1 | grep -E "(Total files|Tokens|Output)" || true
echo "✅ Tests pack ready"
echo ""

# Pack 5: Git Drift
echo "📦 Pack 5/5: Git drift audit..."
repomix \
  --output "$PACKS_DIR/git-drift-$TIMESTAMP.md" \
  --include-logs \
  --include-diffs \
  --style markdown \
  "$REPO_DIR" 2>&1 | grep -E "(Total files|Tokens|Output)" || true
echo "✅ Git drift pack ready"
echo ""

echo "✅ All packs generated in: $PACKS_DIR"
echo ""
echo "📋 Pack files:"
ls -lh "$PACKS_DIR"/*.md 2>/dev/null | tail -5 || echo "No packs found"
echo ""
echo "🎯 Next: Run audits with:"
echo "  1. Full repo wiring audit (01-wiring-audit.md)"
echo "  2. Mainnet launch gate (02-mainnet-launch-gate.md)"
echo "  3. Bridge safety (03-bridge-safety-audit.md)"
echo "  4. Invariant hunter (04-invariant-hunter.md)"
echo "  5. Test gaps (05-test-gap-audit.md)"
