#!/usr/bin/env bash
# ProofForge - Security Gates Runner (S0 & S1)
# Executes security verification gates
# Usage: ./scripts/run-security-gates.sh [gate_level]
#   gate_level: all (default), s0, s1

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROOF_BINARY="${REPO_ROOT}/target/release/x3-proof"
RESULTS_DIR="${REPO_ROOT}/.proof-results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Functions
log_header() {
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}════════════════════════════════════════${NC}"
}

log_gate() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

log_pass() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_fail() {
    echo -e "${RED}✗ $1${NC}"
}

# S0 SECURITY GATE
run_s0_gate() {
    log_gate "S0 Security Gate - Basic Verification"
    
    mkdir -p "$RESULTS_DIR"
    local results_file="${RESULTS_DIR}/s0-security-gate.txt"
    
    {
        echo "=== S0 Security Gate Execution ==="
        echo "Timestamp: $(date -u)"
        echo ""
        echo "Running basic security verification..."
        echo ""
    } > "$results_file"
    
    # Run security gate
    if "$PROOF_BINARY" security-gate -v >> "$results_file" 2>&1; then
        log_pass "S0 gate passed"
    else
        log_fail "S0 gate failed - see $results_file"
        return 1
    fi
    
    {
        echo ""
        echo "=== Blockers Check ==="
    } >> "$results_file"
    
    if "$PROOF_BINARY" explain-blockers all >> "$results_file" 2>&1; then
        log_pass "No critical blockers detected"
    fi
    
    return 0
}

# S1 SECURITY GATE
run_s1_gate() {
    log_gate "S1 Security Gate - Extended Verification"
    
    mkdir -p "$RESULTS_DIR"
    local results_file="${RESULTS_DIR}/s1-security-gate.txt"
    
    {
        echo "=== S1 Security Gate Execution ==="
        echo "Timestamp: $(date -u)"
        echo ""
    } > "$results_file"
    
    # Critical modules to verify
    local critical_modules=(
        "consensus:P7:Consensus Mechanism"
        "bridge:P7:Cross-Chain Bridge"
        "runtime:P7:Runtime Environment"
        "asset_kernel:P7:Asset Kernel"
        "custody:P7:Custody System"
    )
    
    echo "Verifying critical modules with strict validation:" >> "$results_file"
    echo "" >> "$results_file"
    
    local all_passed=true
    for entry in "${critical_modules[@]}"; do
        IFS=':' read -r module level name <<< "$entry"
        
        echo "→ Verifying $name ($module) as $level..." 
        echo "→ Verifying $name ($module) as $level..." >> "$results_file"
        
        if "$PROOF_BINARY" prove "$module" --strict -v >> "$results_file" 2>&1; then
            log_pass "$name verified successfully"
            echo "  ✓ PASSED" >> "$results_file"
        else
            log_fail "$name verification failed"
            echo "  ✗ FAILED" >> "$results_file"
            all_passed=false
        fi
        echo "" >> "$results_file"
    done
    
    if $all_passed; then
        log_pass "S1 gate passed - all critical modules verified"
        return 0
    else
        log_fail "S1 gate failed - some modules did not verify"
        return 1
    fi
}

# MAIN
main() {
    log_header "ProofForge Security Gates"
    
    local gate_level="${1:-all}"
    local exit_code=0
    
    # Build if needed
    if [ ! -f "$PROOF_BINARY" ]; then
        echo "Building ProofForge..."
        cd "$REPO_ROOT"
        cargo build -p proof-forge --release 2>&1 | tail -3
    fi
    
    echo ""
    
    case "$gate_level" in
        all)
            run_s0_gate || exit_code=$?
            run_s1_gate || exit_code=$?
            ;;
        s0)
            run_s0_gate || exit_code=$?
            ;;
        s1)
            run_s1_gate || exit_code=$?
            ;;
        *)
            echo "Invalid gate level: $gate_level"
            echo "Valid options: all, s0, s1"
            exit 1
            ;;
    esac
    
    echo ""
    log_header "Security Gates Complete"
    
    exit $exit_code
}

trap 'echo -e "\n${RED}Interrupted${NC}"; exit 130' INT TERM

main "$@"
