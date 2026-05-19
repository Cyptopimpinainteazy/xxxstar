#!/bin/bash
#
# health-check.sh - Phase 5 System Health Verification
# Run this continuously during deployment and operations
#

TIMEOUT=5
RPC_URL="${RPC_URL:-http://localhost:9944}"
JURY_URL="${JURY_URL:-http://localhost:8080}"
ANCHORER_URL="${ANCHORER_URL:-http://localhost:8081}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

check_http() {
    local name=$1
    local url=$2
    local expected_status=${3:-200}
    
    local status=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout $TIMEOUT "$url" 2>/dev/null)
    
    if [ "$status" = "$expected_status" ]; then
        echo -e "${GREEN}✓${NC} $name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $name (status: $status)"
        ((FAILED++))
    fi
}

check_rpc_method() {
    local method=$1
    local expected_field=$2
    
    local response=$(curl -s -X POST "$RPC_URL" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":[],\"id\":1}" \
        --connect-timeout $TIMEOUT 2>/dev/null)
    
    if echo "$response" | grep -q "$expected_field"; then
        echo -e "${GREEN}✓${NC} RPC: $method"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} RPC: $method"
        ((FAILED++))
    fi
}

check_docker_container() {
    local name=$1
    
    if docker ps | grep -q "$name"; then
        echo -e "${GREEN}✓${NC} Container: $name"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} Container: $name (not running)"
        ((FAILED++))
    fi
}

main() {
    echo ""
    echo "================================"
    echo "  Phase 5 Health Check"
    echo "================================"
    echo ""
    
    echo "Network Connectivity:"
    check_http "Jury Service" "$JURY_URL" "200"
    
    echo ""
    echo "Blockchain Status:"
    check_rpc_method "system_health" "\"result\""
    check_rpc_method "system_health" "isSyncing"
    check_rpc_method "system_chain" "\"result\""
    
    echo ""
    echo "Jury Service Status:"
    check_http "Jury API Health" "$JURY_URL/health" "200"
    check_http "Jury Decisions" "$JURY_URL/api/jury/decisions" "200"
    
    echo ""
    echo "Docker Containers:"
    check_docker_container "blockchain-node"
    check_docker_container "jury-service"
    check_docker_container "postgres"
    check_docker_container "jury-anchorer"
    
    echo ""
    echo "Database:"
    if docker ps | grep -q postgres; then
        local count=$(docker exec $(docker ps -q -f "ancestor=postgres:15") \
            psql -U jury_admin -d jury_db -c "SELECT COUNT(*) FROM sessions;" 2>/dev/null | tail -1)
        if [ ! -z "$count" ]; then
            echo -e "${GREEN}✓${NC} Database: Connected ($count sessions)"
            ((PASSED++))
        else
            echo -e "${RED}✗${NC} Database: Connection failed"
            ((FAILED++))
        fi
    fi
    
    echo ""
    echo "================================"
    echo "  Summary: $PASSED passed, $FAILED failed"
    echo "================================"
    echo ""
    
    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}All checks passed!${NC}"
        return 0
    else
        echo -e "${RED}Some checks failed. Review above.${NC}"
        return 1
    fi
}

main
