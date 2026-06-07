#!/usr/bin/env bash
# ProofForge - CI/CD Deployment Verification
# Validates the complete CI/CD infrastructure is ready
# Usage: ./scripts/verify-deployment.sh [--fix]

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROOF_BINARY="${REPO_ROOT}/target/release/x3-proof"
FIX_ISSUES="${1:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Tracking
CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_WARNINGS=0

log_header() {
    echo -e "${CYAN}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════════════${NC}"
}

log_check() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

log_pass() {
    echo -e "${GREEN}✓ $1${NC}"
    ((CHECKS_PASSED++))
}

log_warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
    ((CHECKS_WARNINGS++))
}

log_fail() {
    echo -e "${RED}✗ $1${NC}"
    ((CHECKS_FAILED++))
}

fix_issue() {
    if [[ "$FIX_ISSUES" == "--fix" ]]; then
        echo -e "${YELLOW}  Attempting to fix: $1${NC}"
        return 0
    else
        echo -e "${YELLOW}  To fix: $1${NC}"
        return 1
    fi
}

# Check binary exists
check_binary() {
    log_check "Checking ProofForge binary..."
    
    if [[ ! -f "$PROOF_BINARY" ]]; then
        log_fail "Binary not found at: $PROOF_BINARY"
        
        if fix_issue "Building binary..."; then
            cd "$REPO_ROOT"
            cargo build -p proof-forge --release 2>&1 | tail -3
            log_pass "Binary built successfully"
        fi
    else
        local size=$(ls -lh "$PROOF_BINARY" | awk '{print $5}')
        log_pass "Binary found ($size)"
    fi
}

# Check binary works
check_binary_functionality() {
    log_check "Testing binary functionality..."
    
    if "$PROOF_BINARY" --version > /dev/null 2>&1; then
        local version=$("$PROOF_BINARY" --version)
        log_pass "Binary functional: $version"
    else
        log_fail "Binary execution failed"
    fi
}

# Check GitHub Actions files
check_github_actions() {
    log_check "Verifying GitHub Actions workflows..."
    
    local workflow_file="${REPO_ROOT}/.github/workflows/proof-gates.yml"
    
    if [[ ! -f "$workflow_file" ]]; then
        log_fail "Workflow file missing: $workflow_file"
    else
        log_pass "Workflow file found"
        
        # Check YAML syntax
        if command -v yamllint > /dev/null 2>&1; then
            if yamllint "$workflow_file" > /dev/null 2>&1; then
                log_pass "YAML syntax valid"
            else
                log_warn "YAML syntax check failed (install yamllint to verify)"
            fi
        else
            log_warn "yamllint not installed (install with: pip install yamllint)"
        fi
        
        # Check key sections
        if grep -q "jobs:" "$workflow_file"; then
            log_pass "Workflow contains jobs section"
        else
            log_fail "Workflow missing jobs section"
        fi
        
        if grep -q "s1-merge-gate" "$workflow_file"; then
            log_pass "S1 merge gate job defined"
        else
            log_fail "S1 merge gate job missing"
        fi
    fi
}

# Check pre-commit hook
check_pre_commit_hook() {
    log_check "Verifying pre-commit hook..."
    
    local hook_source="${REPO_ROOT}/.github/hooks/pre-commit"
    local hook_dest="${REPO_ROOT}/.git/hooks/pre-commit"
    
    if [[ ! -f "$hook_source" ]]; then
        log_fail "Pre-commit hook source missing: $hook_source"
    else
        log_pass "Hook source exists"
        
        if [[ ! -f "$hook_dest" ]]; then
            log_warn "Hook not installed in .git/hooks/"
            
            if fix_issue "Installing hook..."; then
                cp "$hook_source" "$hook_dest"
                chmod +x "$hook_dest"
                log_pass "Hook installed"
            fi
        else
            if [[ -x "$hook_dest" ]]; then
                log_pass "Hook installed and executable"
            else
                log_warn "Hook exists but not executable"
                
                if fix_issue "Making hook executable..."; then
                    chmod +x "$hook_dest"
                    log_pass "Hook is now executable"
                fi
            fi
        fi
    fi
}

# Check security gates scripts
check_security_gates() {
    log_check "Verifying security gates scripts..."
    
    local gates_script="${REPO_ROOT}/scripts/run-security-gates.sh"
    
    if [[ ! -f "$gates_script" ]]; then
        log_fail "Security gates script missing: $gates_script"
    else
        log_pass "Security gates script exists"
        
        if [[ -x "$gates_script" ]]; then
            log_pass "Security gates script is executable"
        else
            log_warn "Security gates script not executable"
            
            if fix_issue "Making executable..."; then
                chmod +x "$gates_script"
                log_pass "Script is now executable"
            fi
        fi
    fi
}

# Check documentation
check_documentation() {
    log_check "Verifying documentation files..."
    
    local docs=(
        "docs/DEVELOPMENT_SETUP.md"
        "docs/SECURITY_GATES.md"
        "docs/GITHUB_PAGES_SETUP.md"
    )
    
    for doc in "${docs[@]}"; do
        local path="${REPO_ROOT}/${doc}"
        if [[ -f "$path" ]]; then
            local lines=$(wc -l < "$path")
            log_pass "$doc ($lines lines)"
        else
            log_warn "$doc missing"
        fi
    done
}

# Check Cargo configuration
check_cargo_config() {
    log_check "Verifying Cargo configuration..."
    
    if [[ -f "${REPO_ROOT}/proof-forge/Cargo.toml" ]]; then
        log_pass "Proof-forge Cargo.toml found"
    else
        log_fail "Proof-forge Cargo.toml not found"
    fi
    
    if [[ -f "${REPO_ROOT}/Cargo.toml" ]]; then
        log_pass "Root Cargo.toml found"
    else
        log_fail "Root Cargo.toml not found"
    fi
}

# Test gates execution
test_gates_execution() {
    log_check "Testing gate execution..."
    
    if "$PROOF_BINARY" scan-claims > /dev/null 2>&1; then
        log_pass "S0 gate command available"
    else
        log_warn "S0 gate command failed"
    fi
    
    if "$PROOF_BINARY" security-gate --help > /dev/null 2>&1; then
        log_pass "S1 gate command available"
    else
        log_warn "S1 gate command failed"
    fi
    
    if "$PROOF_BINARY" testnet-gate --help > /dev/null 2>&1; then
        log_pass "Testnet gate command available"
    else
        log_warn "Testnet gate command failed"
    fi
    
    if "$PROOF_BINARY" mainnet-gate --help > /dev/null 2>&1; then
        log_pass "Mainnet gate command available"
    else
        log_warn "Mainnet gate command failed"
    fi
}

# Check Git configuration
check_git_config() {
    log_check "Verifying Git configuration..."
    
    if cd "$REPO_ROOT" && git rev-parse --git-dir > /dev/null 2>&1; then
        log_pass "Git repository initialized"
        
        # Check for .github directory
        if [[ -d ".github" ]]; then
            log_pass ".github directory exists"
        else
            log_fail ".github directory missing"
        fi
        
        # Check remote
        if git remote -v | grep -q "origin"; then
            log_pass "Git remote 'origin' configured"
            local remote_url=$(git remote get-url origin)
            log_pass "Remote URL: $remote_url"
        else
            log_warn "Git remote 'origin' not configured"
        fi
    else
        log_fail "Not a Git repository"
    fi
}

# Check file permissions
check_file_permissions() {
    log_check "Verifying file permissions..."
    
    local files=(
        ".github/hooks/pre-commit"
        "scripts/run-security-gates.sh"
        "scripts/publish-dashboard.sh"
    )
    
    for file in "${files[@]}"; do
        local path="${REPO_ROOT}/${file}"
        if [[ -f "$path" ]]; then
            if [[ -x "$path" ]]; then
                log_pass "$file is executable"
            else
                log_warn "$file is not executable"
                
                if fix_issue "Making $file executable..."; then
                    chmod +x "$path"
                    log_pass "$file is now executable"
                fi
            fi
        fi
    done
}

# Summary
print_summary() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  VERIFICATION SUMMARY${NC}"
    echo -e "${CYAN}════════════════════════════════════════════════════════${NC}"
    
    echo ""
    echo -e "${GREEN}✓ PASSED:${NC}   $CHECKS_PASSED"
    echo -e "${YELLOW}⚠ WARNINGS:${NC} $CHECKS_WARNINGS"
    echo -e "${RED}✗ FAILED:${NC}   $CHECKS_FAILED"
    
    local total=$((CHECKS_PASSED + CHECKS_FAILED + CHECKS_WARNINGS))
    local pass_rate=$((CHECKS_PASSED * 100 / total))
    echo ""
    echo -e "Status: ${CYAN}${pass_rate}%${NC} ($CHECKS_PASSED/$total checks passed)"
    
    if [[ $CHECKS_FAILED -eq 0 ]]; then
        echo ""
        echo -e "${GREEN}✅ All systems ready for CI/CD deployment${NC}"
        echo ""
        echo "Next steps:"
        echo "  1. git add .github/ scripts/ docs/"
        echo "  2. git commit -m 'Phase 3: CI/CD Infrastructure Complete'"
        echo "  3. git push origin main"
        echo "  4. Monitor: GitHub Actions tab in repository"
        echo ""
        return 0
    else
        echo ""
        echo -e "${YELLOW}⚠️  Issues detected - review above${NC}"
        echo ""
        echo "To auto-fix issues, run:"
        echo "  ./scripts/verify-deployment.sh --fix"
        echo ""
        return 1
    fi
}

# Main execution
main() {
    log_header "ProofForge CI/CD Deployment Verification"
    
    if [[ "$FIX_ISSUES" == "--fix" ]]; then
        echo -e "\n${YELLOW}Running in AUTO-FIX mode${NC}\n"
    fi
    
    check_binary
    check_binary_functionality
    check_github_actions
    check_pre_commit_hook
    check_security_gates
    check_file_permissions
    check_documentation
    check_cargo_config
    check_git_config
    test_gates_execution
    
    print_summary
}

# Run main
main "$@"
