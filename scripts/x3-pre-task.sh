#!/usr/bin/env bash
set -euo pipefail
# x3-pre-task.sh — snapshot current state before work starts

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo ""
echo "=========================================="
echo "X3 PRE-TASK SNAPSHOT — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "=========================================="

# Branch
if git -C "$REPO_ROOT" rev-parse --git-dir &>/dev/null; then
    BRANCH=$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo "detached")
    echo "Branch: $BRANCH"
    
    echo ""
    echo "--- Dirty Files ---"
    DIRTY=$(git -C "$REPO_ROOT" status --short 2>/dev/null | head -20)
    if [ -n "$DIRTY" ]; then
        echo "$DIRTY"
    else
        echo "(clean)"
    fi
else
    echo "Branch: (not a git repo)"
    echo "Dirty Files: UNKNOWN"
fi

echo ""
echo "--- Detected Languages ---"
[ -f "$REPO_ROOT/Cargo.toml" ] && echo "Rust"
[ -f "$REPO_ROOT/package.json" ] && echo "Node/TypeScript"
find "$REPO_ROOT" -name '*.py' -not -path '*/.venv/*' -not -path '*/node_modules/*' -not -path '*/target/*' 2>/dev/null | head -1 &>/dev/null && echo "Python"
[ -f "$REPO_ROOT/hardhat.config.ts" ] || [ -f "$REPO_ROOT/hardhat.config.js" ] && echo "Solidity/Hardhat"
[ -f "$REPO_ROOT/foundry.toml" ] && echo "Solidity/Foundry"

echo ""
echo "--- Available Proof Commands ---"
echo "Primary: bash scripts/x3-proof-check.sh"
[ -f "$REPO_ROOT/Cargo.toml" ] && echo "  cargo check --workspace"
[ -f "$REPO_ROOT/Cargo.toml" ] && echo "  cargo test --workspace"
[ -f "$REPO_ROOT/package.json" ] && echo "  npm test (if configured)"

echo ""
echo "--- Current Status ---"
if [ -f "$REPO_ROOT/docs/X3_COMPLETION_STATUS.md" ]; then
    head -20 "$REPO_ROOT/docs/X3_COMPLETION_STATUS.md" 2>/dev/null
else
    echo "docs/X3_COMPLETION_STATUS.md not found."
fi

echo ""
echo "--- Current Next 10 Tasks ---"
if [ -f "$REPO_ROOT/docs/X3_NEXT_TASKS.md" ]; then
    grep '^### [0-9]' "$REPO_ROOT/docs/X3_NEXT_TASKS.md" 2>/dev/null | head -10 || echo "  (no tasks found)"
else
    echo "docs/X3_NEXT_TASKS.md not found."
fi

echo ""
echo "--- Last Proof Result ---"
if [ -f "$REPO_ROOT/.x3/proof/latest-proof.log" ]; then
    tail -5 "$REPO_ROOT/.x3/proof/latest-proof.log" 2>/dev/null
else
    echo "No proof log found."
fi

echo ""
echo "--- Suspicious Stubs (quick scan) ---"
if [ -f "$REPO_ROOT/scripts/x3-detect-stubs.sh" ]; then
    bash "$REPO_ROOT/scripts/x3-detect-stubs.sh" 2>/dev/null | head -10
else
    echo "Stub detector not available."
fi

echo ""