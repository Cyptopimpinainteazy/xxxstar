#!/usr/bin/env bash
set -euo pipefail
# x3-install-git-hooks.sh — install real git hooks for pre-commit and pre-push

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if ! git -C "$REPO_ROOT" rev-parse --git-dir &>/dev/null 2>&1; then
    echo "Not a git repository. Cannot install hooks."
    exit 1
fi

GIT_DIR=$(git -C "$REPO_ROOT" rev-parse --git-dir 2>/dev/null)
HOOKS_DIR="$REPO_ROOT/$GIT_DIR/hooks"
mkdir -p "$HOOKS_DIR"

# pre-commit hook
PRE_COMMIT="$HOOKS_DIR/pre-commit"
if [ -f "$PRE_COMMIT" ]; then
    echo "Backing up existing pre-commit hook to $PRE_COMMIT.bak"
    cp "$PRE_COMMIT" "$PRE_COMMIT.bak"
fi

cat > "$PRE_COMMIT" << 'HOOKEOF'
#!/usr/bin/env bash
# X3 pre-commit hook — run stub detector and test-cheat detector
if [ "${SKIP_X3_CHECKS:-}" = "1" ]; then
    echo "[X3] Checks skipped via SKIP_X3_CHECKS=1"
    exit 0
fi

REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$REPO_ROOT" ]; then
    echo "[X3] Not in a git repo, skipping checks."
    exit 0
fi

STUB_SCRIPT="$REPO_ROOT/scripts/x3-detect-stubs.sh"
CHEAT_SCRIPT="$REPO_ROOT/scripts/x3-detect-test-cheats.sh"

FAILED=0

if [ -x "$STUB_SCRIPT" ]; then
    echo "[X3] Running stub detector..."
    if ! bash "$STUB_SCRIPT"; then
        echo "[X3] STUB DETECTOR FAILED — critical-path stubs found."
        echo "Fix stubs before committing or override with SKIP_X3_CHECKS=1 (must be justified)."
        FAILED=1
    fi
else
    echo "[X3] Stub detector not found at $STUB_SCRIPT — skipping."
fi

if [ -x "$CHEAT_SCRIPT" ]; then
    echo "[X3] Running test-cheat detector..."
    if ! bash "$CHEAT_SCRIPT"; then
        echo "[X3] TEST-CHEAT DETECTOR WARNED — review findings before committing."
    fi
else
    echo "[X3] Test-cheat detector not found at $CHEAT_SCRIPT — skipping."
fi

if [ "$FAILED" -eq 1 ]; then
    echo "[X3] Pre-commit checks FAILED. Commit blocked."
    exit 1
fi
echo "[X3] Pre-commit checks PASSED."
HOOKEOF

chmod +x "$PRE_COMMIT"
echo "Installed pre-commit hook at $PRE_COMMIT"

# pre-push hook
PRE_PUSH="$HOOKS_DIR/pre-push"
if [ -f "$PRE_PUSH" ]; then
    echo "Backing up existing pre-push hook to $PRE_PUSH.bak"
    cp "$PRE_PUSH" "$PRE_PUSH.bak"
fi

cat > "$PRE_PUSH" << 'HOOKEOF'
#!/usr/bin/env bash
# X3 pre-push hook — run proof check
if [ "${SKIP_X3_CHECKS:-}" = "1" ]; then
    echo "[X3] Checks skipped via SKIP_X3_CHECKS=1"
    exit 0
fi

REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$REPO_ROOT" ]; then
    echo "[X3] Not in a git repo, skipping checks."
    exit 0
fi

PROOF_SCRIPT="$REPO_ROOT/scripts/x3-proof-check.sh"

if [ -x "$PROOF_SCRIPT" ]; then
    echo "[X3] Running proof check..."
    if ! bash "$PROOF_SCRIPT"; then
        echo "[X3] PROOF CHECK FAILED — push blocked."
        echo "Fix failures or override with SKIP_X3_CHECKS=1 (must be justified in proof ledger)."
        exit 1
    fi
else
    echo "[X3] Proof check script not found at $PROOF_SCRIPT — skipping."
fi
echo "[X3] Pre-push checks PASSED."
HOOKEOF

chmod +x "$PRE_PUSH"
echo "Installed pre-push hook at $PRE_PUSH"

echo ""
echo "Git hooks installed. Verify with: ls -la $HOOKS_DIR/pre-commit $HOOKS_DIR/pre-push"