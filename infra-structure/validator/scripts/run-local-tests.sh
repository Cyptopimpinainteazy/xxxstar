#!/bin/bash
# Local E2E test runner for cross-chain GPU validator
# No Docker required - runs directly with mock RPC and local Redis

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DIR="$PROJECT_ROOT/tests"
RESULTS_DIR="${TEST_RESULTS_DIR:-./test-results}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Cross-Chain GPU Validator - Local E2E Tests (No Docker)   ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# ============================================================================
# Check Python & venv
# ============================================================================
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}[!] Python 3 not found. Install Python 3.10+ and try again.${NC}"
    exit 1
fi

PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
echo -e "${BLUE}[*] Using Python ${PYTHON_VERSION}${NC}"

# Create venv if needed
if [[ ! -d "$PROJECT_ROOT/.venv" ]]; then
    echo -e "${YELLOW}[*] Creating Python virtual environment...${NC}"
    python3 -m venv "$PROJECT_ROOT/.venv"
fi

# Activate venv
source "$PROJECT_ROOT/.venv/bin/activate"

# ============================================================================
# Install Dependencies
# ============================================================================
echo -e "${YELLOW}[*] Installing dependencies...${NC}"
cd "$PROJECT_ROOT"
pip install -q -e . 2>/dev/null || true
pip install -q pytest pytest-cov pytest-timeout pytest-xdist requests-mock 2>/dev/null || true

# ============================================================================
# Set Mock RPC Environment
# ============================================================================
echo -e "${YELLOW}[*] Configuring mock RPC mode...${NC}"
export CCGV_USE_MOCK_RPC=true
export CCGV_EVM_RPC="mock://ethereum"
export CCGV_SVM_RPC="mock://solana"
export CCGV_REQUIRE_GPU=false
export CCGV_GPU_PARITY_CHECK=false

# ============================================================================
# Run Tests
# ============================================================================
echo -e "${BLUE}[*] Running security tests...${NC}"
python -m pytest -v --tb=short \
    "$TEST_DIR/test_security_e2e.py" \
    -o addopts="" \
    --junit-xml="$RESULTS_DIR/security-results.xml" \
    || SECURITY_FAILED=true

echo ""
echo -e "${BLUE}[*] Running dashboard tests...${NC}"
python -m pytest -v --tb=short \
    "$TEST_DIR/test_dashboard_e2e.py" \
    -o addopts="" \
    --junit-xml="$RESULTS_DIR/dashboard-results.xml" \
    || DASHBOARD_FAILED=true

echo ""
echo -e "${BLUE}[*] Running RPC endpoint tests...${NC}"
python -m pytest -v --tb=short \
    "$TEST_DIR/test_rpc_endpoints_e2e.py" \
    -o addopts="" \
    --junit-xml="$RESULTS_DIR/rpc-results.xml" \
    || RPC_FAILED=true

# ============================================================================
# Coverage Report
# ============================================================================
echo ""
echo -e "${YELLOW}[*] Generating coverage report...${NC}"
python -m pytest \
    "$TEST_DIR/test_security_e2e.py" \
    "$TEST_DIR/test_dashboard_e2e.py" \
    "$TEST_DIR/test_rpc_endpoints_e2e.py" \
    --cov=cross_chain_gpu_validator \
    --cov-report=html:"$RESULTS_DIR/coverage" \
    --cov-report=term \
    -q 2>/dev/null || true

# ============================================================================
# Summary
# ============================================================================
echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Local E2E Tests Complete                                 ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Results directory: ${RESULTS_DIR}"
echo -e "Coverage report:   ${RESULTS_DIR}/coverage/index.html"
echo ""

if [[ -z "$SECURITY_FAILED" ]] && [[ -z "$DASHBOARD_FAILED" ]] && [[ -z "$RPC_FAILED" ]]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠ Some tests had failures - see above for details${NC}"
    exit 1
fi
