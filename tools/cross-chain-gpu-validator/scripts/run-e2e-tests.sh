#!/bin/bash
# E2E test runner for cross-chain GPU validator with security & debugging
# Supports: live RPC, mock RPC, containerized Docker environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DIR="$PROJECT_ROOT/tests"
RESULTS_DIR="${TEST_RESULTS_DIR:-/tmp/ccgv-test-results}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test mode: "docker", "mock", "live"
TEST_MODE="${1:-docker}"

# Validate test mode
if [[ ! "$TEST_MODE" =~ ^(docker|mock|live)$ ]]; then
    echo "Usage: $0 {docker|mock|live}"
    echo ""
    echo "Modes:"
    echo "  docker   - Run tests in containerized environment (default, recommended)"
    echo "  mock     - Run tests with mock RPC responses"
    echo "  live     - Run tests against live testnets (requires testnet RPC URLs)"
    exit 1
fi

mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Cross-Chain GPU Validator - E2E Security Testing         ║${NC}"
echo -e "${BLUE}║  Mode: ${TEST_MODE}                                                    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# ============================================================================
# Docker Mode: Containerized E2E Testing
# ============================================================================
if [[ "$TEST_MODE" == "docker" ]]; then
    echo -e "${YELLOW}[*] Starting containerized E2E test environment...${NC}"
    
    # Check Docker availability
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}[!] Docker not found. Install docker and try again.${NC}"
        exit 1
    fi
    
    # Check docker-compose or docker compose
    if command -v docker-compose &> /dev/null; then
        COMPOSE_CMD="docker-compose"
    elif docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
    else
        echo -e "${RED}[!] Docker Compose not found. Install docker-compose and try again.${NC}"
        exit 1
    fi
    
    # Build and start services
    echo -e "${YELLOW}[*] Building Docker images...${NC}"
    cd "$PROJECT_ROOT"
    $COMPOSE_CMD -f docker-compose.testnet.yml build --no-cache
    
    echo -e "${YELLOW}[*] Starting test environment...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml up -d
    
    # Wait for services to be healthy
    echo -e "${YELLOW}[*] Waiting for services to be healthy...${NC}"
    sleep 10
    
    # Check service health
    for service in ethereum-testnet solana-testnet redis ccgv-validator; do
        echo -n "  Checking $service... "
        if $COMPOSE_CMD -f docker-compose.testnet.yml ps $service | grep -q "healthy\|running"; then
            echo -e "${GREEN}✓${NC}"
        else
            echo -e "${RED}✗${NC}"
            echo -e "${RED}[!] Service $service failed to start${NC}"
            echo ""
            echo "Logs:"
            $COMPOSE_CMD -f docker-compose.testnet.yml logs $service | tail -20
            exit 1
        fi
    done
    
    echo -e "${BLUE}[*] Running security E2E tests...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml run --rm test-runner \
        test_security_e2e.py::TestInputValidationSanitization \
        test_security_e2e.py::TestPrivateKeySignatureHandling \
        test_security_e2e.py::TestRpcEndpointSecurity \
        test_security_e2e.py::TestTransactionAtomicityGuarantees
    
    echo -e "${BLUE}[*] Running dashboard E2E tests...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml run --rm test-runner \
        test_dashboard_e2e.py
    
    echo -e "${BLUE}[*] Running RPC endpoint tests...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml run --rm test-runner \
        test_rpc_endpoints_e2e.py
    
    # Collect results
    echo -e "${YELLOW}[*] Collecting test results...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml cp test-runner:/results/coverage "$RESULTS_DIR/" || true
    
    # Generate summary
    echo -e "${YELLOW}[*] Generating test summary...${NC}"
    $COMPOSE_CMD -f docker-compose.testnet.yml logs ccgv-validator > "$RESULTS_DIR/validator.log" || true
    
    echo -e "${GREEN}[✓] Containerized tests completed${NC}"
    echo -e "Results:${NC} $RESULTS_DIR"
    
    # Cleanup
    CLEANUP="${2:-auto}"
    if [[ "$CLEANUP" == "auto" ]] || [[ "$CLEANUP" == "--cleanup" ]]; then
        echo -e "${YELLOW}[*] Cleaning up Docker resources...${NC}"
        $COMPOSE_CMD -f docker-compose.testnet.yml down -v
        echo -e "${GREEN}[✓] Cleanup complete${NC}"
    else
        echo -e "${YELLOW}[*] Keeping containers running. Run 'docker-compose -f docker-compose.testnet.yml down' to cleanup${NC}"
    fi

# ============================================================================
# Mock Mode: Deterministic Mock RPC Testing
# ============================================================================
elif [[ "$TEST_MODE" == "mock" ]]; then
    echo -e "${YELLOW}[*] Running E2E tests with mock RPC responses...${NC}"
    
    # Set mock RPC environment variables
    export CCGV_USE_MOCK_RPC=true
    export CCGV_EVM_RPC="mock://ethereum"
    export CCGV_SVM_RPC="mock://solana"
    
    cd "$PROJECT_ROOT"
    
    # Run tests
    python -m pytest \
        -v \
        --tb=short \
        --cov=cross_chain_gpu_validator \
        --cov-report=html:"$RESULTS_DIR/coverage" \
        --cov-report=term \
        --junit-xml="$RESULTS_DIR/junit.xml" \
        "$TEST_DIR/test_security_e2e.py" \
        "$TEST_DIR/test_dashboard_e2e.py" \
        "$TEST_DIR/test_rpc_endpoints_e2e.py"
    
    echo -e "${GREEN}[✓] Mock RPC tests completed${NC}"
    echo -e "Results: $RESULTS_DIR"

# ============================================================================
# Live Mode: Testing Against Actual Testnets
# ============================================================================
elif [[ "$TEST_MODE" == "live" ]]; then
    echo -e "${YELLOW}[*] Running E2E tests against live testnet RPC endpoints...${NC}"
    
    # Check required environment variables
    if [[ -z "$CCGV_EVM_RPC" ]] || [[ -z "$CCGV_SVM_RPC" ]]; then
        echo -e "${RED}[!] Error: Live mode requires testnet RPC URLs${NC}"
        echo -e "Set environment variables:${NC}"
        echo "  export CCGV_EVM_RPC='https://sepolia.infura.io/v3/YOUR_KEY'"
        echo "  export CCGV_SVM_RPC='https://api.testnet.solana.com'"
        exit 1
    fi
    
    echo -e "${BLUE}Testing against:${NC}"
    echo "  EVM RPC: $CCGV_EVM_RPC"
    echo "  SVM RPC: $CCGV_SVM_RPC"
    echo ""
    
    cd "$PROJECT_ROOT"
    
    # Run with increased timeouts and retries for live endpoints
    python -m pytest \
        -v \
        --tb=short \
        --cov=cross_chain_gpu_validator \
        --cov-report=html:"$RESULTS_DIR/coverage" \
        --cov-report=term \
        --junit-xml="$RESULTS_DIR/junit.xml" \
        --timeout=30 \
        -m "not requires_gpu" \
        "$TEST_DIR/test_rpc_endpoints_e2e.py" \
        "$TEST_DIR/test_security_e2e.py::TestRpcEndpointSecurity"
    
    echo -e "${GREEN}[✓] Live testnet tests completed${NC}"
    echo -e "Results: $RESULTS_DIR"
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  E2E Testing Complete                                     ║${NC}"
echo -e "${GREEN}║  Mode: $TEST_MODE                                                    ║${NC}"
echo -e "${GREEN}║  Results: $RESULTS_DIR${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"

exit 0
