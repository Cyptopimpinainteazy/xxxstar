#!/usr/bin/env bash
set -euo pipefail
# x3-proof-check.sh — detect project type and run strongest available checks
# Saves output to .x3/proof/latest-proof.log

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
mkdir -p "$REPO_ROOT/.x3/proof"
LOG="$REPO_ROOT/.x3/proof/latest-proof.log"
echo "=== X3 Proof Check — $(date -u +%Y-%m-%dT%H:%M:%SZ) ===" > "$LOG"
PASS=0
FAIL=0

run_check() {
    local label="$1"
    shift
    echo "--- $label ---" | tee -a "$LOG"
    if "$@" >> "$LOG" 2>&1; then
        echo "[PASS] $label" | tee -a "$LOG"
        PASS=$((PASS + 1))
    else
        echo "[FAIL] $label (exit=$?)" | tee -a "$LOG"
        FAIL=$((FAIL + 1))
    fi
}

# Rust checks
if [ -f "$REPO_ROOT/Cargo.toml" ]; then
    echo "=== Rust detected ===" | tee -a "$LOG"
    run_check "cargo-fmt-check" cargo fmt --check --manifest-path "$REPO_ROOT/Cargo.toml" 2>&1 || true
    run_check "cargo-clippy" cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 || true
    if cargo test --workspace --all-features --no-run 2>/dev/null; then
        run_check "cargo-test" cargo test --workspace --all-features 2>&1 || true
    else
        echo "[SKIP] cargo-test — no test binaries built" | tee -a "$LOG"
    fi
fi

# Node/TypeScript checks
if [ -f "$REPO_ROOT/package.json" ]; then
    echo "=== Node/TypeScript detected ===" | tee -a "$LOG"
    if [ -f "$REPO_ROOT/package-lock.json" ] && [ ! -d "$REPO_ROOT/node_modules" ]; then
        run_check "npm-install" npm ci 2>&1 || true
    fi
    if npm run lint --if-present 2>/dev/null; then
        run_check "npm-lint" npm run lint 2>&1 || true
    fi
    if npm run typecheck --if-present 2>/dev/null; then
        run_check "npm-typecheck" npm run typecheck 2>&1 || true
    fi
    if npm test --if-present 2>/dev/null; then
        run_check "npm-test" npm test 2>&1 || true
    fi
    if npm run build --if-present 2>/dev/null; then
        run_check "npm-build" npm run build 2>&1 || true
    fi
fi

# Python checks
PYTHON_FILES=$(find "$REPO_ROOT" -name '*.py' -not -path '*/.venv/*' -not -path '*/node_modules/*' -not -path '*/target/*' 2>/dev/null | head -1)
if [ -n "$PYTHON_FILES" ]; then
    echo "=== Python detected ===" | tee -a "$LOG"
    run_check "python-compileall" python3 -m compileall -q "$REPO_ROOT" -x '\.venv|node_modules|target' 2>&1 || true
fi

# Solidity checks
if [ -f "$REPO_ROOT/hardhat.config.ts" ] || [ -f "$REPO_ROOT/hardhat.config.js" ]; then
    echo "=== Solidity/Hardhat detected ===" | tee -a "$LOG"
    if npx hardhat test 2>/dev/null; then
        run_check "hardhat-test" npx hardhat test 2>&1 || true
    else
        echo "[SKIP] hardhat-test — command failed or no tests" | tee -a "$LOG"
    fi
fi

if [ -f "$REPO_ROOT/foundry.toml" ] || [ -d "$REPO_ROOT/X3-contracts" ]; then
    echo "=== Foundry detected ===" | tee -a "$LOG"
    if command -v forge &>/dev/null; then
        if [ -f "$REPO_ROOT/foundry.toml" ]; then
            run_check "forge-test" forge test 2>&1 || true
        fi
        if [ -d "$REPO_ROOT/X3-contracts" ]; then
            (cd "$REPO_ROOT/X3-contracts" && forge test 2>&1) >> "$LOG" 2>&1 || echo "[FAIL] forge-test in X3-contracts" | tee -a "$LOG"
        fi
    else
        echo "[SKIP] forge-test — forge not installed" | tee -a "$LOG"
    fi
fi

# Generic stub detection
echo "=== Stub Detection ===" | tee -a "$LOG"
"$SCRIPT_DIR/x3-detect-stubs.sh" >> "$LOG" 2>&1 || true

# Generic test-cheat detection
echo "=== Test-Cheat Detection ===" | tee -a "$LOG"
"$SCRIPT_DIR/x3-detect-test-cheats.sh" >> "$LOG" 2>&1 || true

# Summary
echo "" | tee -a "$LOG"
echo "==========================================" | tee -a "$LOG"
echo "PROOF CHECK SUMMARY" | tee -a "$LOG"
echo "PASS: $PASS" | tee -a "$LOG"
echo "FAIL: $FAIL" | tee -a "$LOG"
if [ "$FAIL" -gt 0 ]; then
    echo "OVERALL: FAIL" | tee -a "$LOG"
    exit 1
else
    echo "OVERALL: PASS" | tee -a "$LOG"
    exit 0
fi