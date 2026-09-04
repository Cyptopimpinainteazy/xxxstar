#!/usr/bin/env bash
# X3 Tool Run Script — runs all installed tools and generates reports
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="$PROJECT_DIR/reports"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SUMMARY="$REPORT_DIR/TOOL_RUN_SUMMARY.md"

mkdir -p "$REPORT_DIR/security" "$REPORT_DIR/invariants" "$REPORT_DIR/fuzzing" "$REPORT_DIR/substrate"

echo "# X3 Tool Run — $TIMESTAMP" > "$SUMMARY"
echo "" >> "$SUMMARY"
echo "| Tool | Result |" >> "$SUMMARY"
echo "|------|--------|" >> "$SUMMARY"

run_and_record() {
    local tool="$1"
    local cmd="$2"
    local outfile="$REPORT_DIR/${3:-$tool}.txt"
    echo "" >> "$SUMMARY"
    echo "--- Running $tool ---" 
    if eval "$cmd" > "$outfile" 2>&1; then
        echo "  ✓ $tool PASSED"
        echo "| $tool | ✅ PASS |" >> "$SUMMARY"
    else
        local rc=$?
        echo "  ✗ $tool FAILED (exit $rc)"
        echo "| $tool | ❌ FAIL ($rc) |" >> "$SUMMARY"
    fi
    echo "  Log: $outfile"
}

# Tool 1: cargo-audit
run_and_record "cargo-audit" "cargo audit --no-fetch" "security/cargo-audit"

# Tool 2: cargo-deny
run_and_record "cargo-deny" "cargo deny check advisories" "security/cargo-deny-advisories"
run_and_record "cargo-deny-licenses" "cargo deny check licenses" "security/cargo-deny-licenses"

# Tool 3: cargo-geiger (unsafe usage audit)
run_and_record "cargo-geiger" "cargo geiger --output-format Json 2>&1 | head -100" "security/cargo-geiger"

# Tool 4: cargo-nextest
run_and_record "cargo-nextest" "cargo nextest run --workspace --no-tests=warn 2>&1 | tail -20" "cargo-nextest"

# Tool 5: cargo-fuzz (check setup)
if command -v cargo-fuzz &>/dev/null; then
    if [ -d "$PROJECT_DIR/fuzz" ]; then
        run_and_record "cargo-fuzz" "cargo fuzz list 2>&1" "fuzzing/fuzz-targets"
    else
        echo "  cargo-fuzz: no fuzz directory found (run 'cargo fuzz init' first)"
        echo "| cargo-fuzz | ⏳ not configured |" >> "$SUMMARY"
    fi
fi

# Tool 6: slither (Solidity static analysis)
if command -v slither &>/dev/null && [ -d "$PROJECT_DIR/X3-contracts/evm" ]; then
    run_and_record "slither" "slither $PROJECT_DIR/X3-contracts/evm/ --print human-summary 2>&1 | tail -40" "security/slither-report"
elif command -v slither &>/dev/null && [ -d "$PROJECT_DIR/contracts" ]; then
    run_and_record "slither" "slither $PROJECT_DIR/contracts/ --print human-summary 2>&1 | tail -40" "security/slither-report"
fi

# Tool 7: proptest (property-based tests — run via cargo test)
run_and_record "proptest-asset-kernel" "cargo test -p x3-asset-kernel proptest 2>&1 | tail -20" "invariants/proptest-asset-kernel"
run_and_record "proptest-atomic-trade" "cargo test -p x3-atomic-trade proptest 2>&1 | tail -20" "invariants/proptest-atomic-trade"

# Tool 8: cargo-llvm-cov (coverage)
run_and_record "cargo-llvm-cov" "cargo llvm-cov nextest --workspace --lcov --output-path lcov.info 2>&1 | tail -20" "coverage"

# Tool 9: cargo-mutants (mutation testing on core crates)
for crate in x3-asset-kernel x3-atomic-trade; do
    run_and_record "cargo-mutants-$crate" "cargo mutants -p $crate 2>&1 | tail -20" "mutants-$crate"
done

echo "" >> "$SUMMARY"
echo "---" >> "$SUMMARY"
echo "Full logs in: $REPORT_DIR/" >> "$SUMMARY"

echo ""
echo "=== X3 Tool Run Complete ==="
echo "Summary: $SUMMARY"
cat "$SUMMARY"