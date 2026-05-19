#!/usr/bin/env bash

# COMPREHENSIVE TEST FRAMEWORK SETUP SCRIPT
# This script validates all test infrastructure is in place

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  X3 INTELLIGENCE + L1 BLOCKCHAIN TEST FRAMEWORK VALIDATOR  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASS=0
FAIL=0
WARN=0

check_file() {
  local file="$1"
  local description="$2"
  
  if [ -f "$file" ]; then
    echo -e "${GREEN}✓${NC} Found: $description"
    ((PASS++))
    return 0
  else
    echo -e "${RED}✗${NC} Missing: $description at $file"
    ((FAIL++))
    return 1
  fi
}

check_command() {
  local cmd="$1"
  local description="$2"
  
  if command -v "$cmd" &> /dev/null; then
    echo -e "${GREEN}✓${NC} Found: $description"
    ((PASS++))
    return 0
  else
    echo -e "${YELLOW}⚠${NC} Missing: $description (command: $cmd)"
    ((WARN++))
    return 1
  fi
}

# ============ X3 INTELLIGENCE TESTS ============

echo ""
echo "${BLUE}═══ X3 INTELLIGENCE TESTS ═══${NC}"
echo ""

check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-intelligence/tests/__tests__/api.test.ts" "API Service Tests"
check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-intelligence/tests/__tests__/server.test.ts" "Server Endpoint Tests"
check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-intelligence/tests/__tests__/comprehensive.test.ts" "Comprehensive Test Suite"
check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-desktop/tests/e2e/smoke-tests.spec.ts" "E2E: Smoke Tests"
check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-desktop/tests/e2e/tauri-backend.spec.ts" "E2E: Tauri Backend"
check_file "/home/lojak/Desktop/x3-chain-master/apps/x3-desktop/tests/e2e/practical-integration.spec.ts" "E2E: Practical Integration"

# ============ BLOCKCHAIN TESTS ============

echo ""
echo "${BLUE}═══ LAYER 1 BLOCKCHAIN TESTS ═══${NC}"
echo ""

check_file "/home/lojak/Desktop/x3-chain-master/docs/tests/TESTING_STRATEGY.md" "Testing Strategy & Threat Model"
check_file "/home/lojak/Desktop/x3-chain-master/tests/L1_CONSENSUS_AND_ATOMICITY.test.ts" "Consensus & Atomicity Tests"
check_file "/home/lojak/Desktop/x3-chain-master/tests/L1_ISOLATION_AND_ATTACKS.test.ts" "Isolation & Attack Tests"
check_file "/home/lojak/Desktop/x3-chain-master/tests/L1_LOAD_AND_FORMAL.test.ts" "Load & Formal Specification Tests"

# ============ DOCUMENTATION ============

echo ""
echo "${BLUE}═══ DOCUMENTATION ═══${NC}"
echo ""

check_file "/home/lojak/Desktop/x3-chain-master/docs/tests/PRE_MAINNET_ROADMAP.md" "Pre-Mainnet Roadmap (10 Phases)"
check_file "/home/lojak/Desktop/x3-chain-master/docs/tests/TEST_IMPLEMENTATION_GUIDE.md" "Test Implementation Patterns"
check_file "/home/lojak/Desktop/x3-chain-master/VALIDATION_CHECKLIST.md" "Comprehensive Validation Checklist"

# ============ ENVIRONMENT CHECKS ============

echo ""
echo "${BLUE}═══ ENVIRONMENT CHECKS ═══${NC}"
echo ""

check_command "node" "Node.js"
check_command "npm" "NPM"
check_command "npx" "NPX"
check_command "cargo" "Rust Cargo"
check_command "docker" "Docker"

# ============ SERVICE CHECKS ============

echo ""
echo "${BLUE}═══ RUNNING SERVICE CHECKS ═══${NC}"
echo ""

# Check if API server is running
if curl -s http://localhost:8001/health > /dev/null 2>&1; then
  echo -e "${GREEN}✓${NC} API Server running on localhost:8001"
  ((PASS++))
else
  echo -e "${YELLOW}⚠${NC} API Server not running (expected during test phase)"
  ((WARN++))
fi

# Check if Redis is running
if redis-cli ping > /dev/null 2>&1; then
  echo -e "${GREEN}✓${NC} Redis running on localhost:6379"
  ((PASS++))
else
  echo -e "${YELLOW}⚠${NC} Redis not running (optional service)"
  ((WARN++))
fi

# ============ SUMMARY ============

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                        SUMMARY                             ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo -e "${GREEN}Passed:${NC}  $PASS"
echo -e "${RED}Failed:${NC}  $FAIL"
echo -e "${YELLOW}Warned:${NC}  $WARN"
echo ""

# Calculate progress
TOTAL=$((PASS + FAIL + WARN))
if [ $TOTAL -gt 0 ]; then
  PROGRESS=$((PASS * 100 / TOTAL))
  echo "Overall Progress: $PROGRESS% ($PASS/$TOTAL items)"
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""

# Next steps
echo "${BLUE}NEXT STEPS:${NC}"
echo ""
echo "1. ${YELLOW}Web Dashboard Testing${NC}"
echo "   npm test api.test.ts              # Fix URL mismatch"
echo "   npm test server.test.ts           # Validate API endpoints"
echo "   npm run test:e2e                  # Run all E2E tests"
echo ""
echo "2. ${YELLOW}Blockchain Testing${NC}"
echo "   Read: /docs/tests/TESTING_STRATEGY.md"
echo "   Implement: L1_CONSENSUS_AND_ATOMICITY.test.ts"
echo "   Implement: L1_ISOLATION_AND_ATTACKS.test.ts"
echo ""
echo "3. ${YELLOW}Validation${NC}"
echo "   Review: docs/runbooks/testing/VALIDATION_CHECKLIST.md"
echo "   Track: PRE_MAINNET_ROADMAP.md"
echo ""
echo "4. ${YELLOW}Documentation${NC}"
echo "   Read: TEST_IMPLEMENTATION_GUIDE.md"
echo "   Review: All test specification files"
echo ""

if [ $FAIL -eq 0 ]; then
  echo -e "${GREEN}✓ All required files present!${NC}"
  exit 0
else
  echo -e "${RED}✗ Some files missing. Please check above.${NC}"
  exit 1
fi

