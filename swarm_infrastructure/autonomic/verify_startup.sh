#!/bin/bash
# X3 Autonomic Control Plane - Startup Verification
# Quick health check to verify the autonomic control plane is operational

set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080/api/autonomic}"
TIMEOUT=30

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${CYAN}${BOLD}"
echo "╔════════════════════════════════════════════════╗"
echo "║   X3 AUTONOMIC CONTROL PLANE VERIFICATION     ║"
echo "╚════════════════════════════════════════════════╝"
echo -e "${NC}"

# Wait for API to be ready
echo -e "${BLUE}[1/5]${NC} Waiting for autonomic API..."
for i in $(seq 1 $TIMEOUT); do
    if curl -sf "$API_URL/health" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} API is ready"
        break
    fi
    if [ $i -eq $TIMEOUT ]; then
        echo -e "${RED}✗${NC} API failed to respond after ${TIMEOUT}s"
        echo -e "${YELLOW}TIP:${NC} Is the swarm server running? Try: python3 -m swarm.api_server"
        exit 1
    fi
    sleep 1
done

# Check health score
echo -e "${BLUE}[2/5]${NC} Checking system health..."
HEALTH=$(curl -sf "$API_URL/health" 2>/dev/null)
SCORE=$(echo "$HEALTH" | jq -r '.score' 2>/dev/null || echo "0")
STATE=$(echo "$HEALTH" | jq -r '.state' 2>/dev/null || echo "unknown")

if (( $(echo "$SCORE >= 60" | bc -l) )); then
    echo -e "${GREEN}✓${NC} System health: ${GREEN}${SCORE}${NC}/100 (${STATE})"
else
    echo -e "${YELLOW}⚠${NC} System health: ${YELLOW}${SCORE}${NC}/100 (${STATE})"
fi

# Check sentinels
echo -e "${BLUE}[3/5]${NC} Checking sentinels..."
STATUS=$(curl -sf "$API_URL/status" 2>/dev/null)

GPU_RUNNING=$(echo "$STATUS" | jq -r '.gpu_guard.running' 2>/dev/null || echo "false")
if [ "$GPU_RUNNING" == "true" ]; then
    GPU_COUNT=$(echo "$STATUS" | jq -r '.gpu_guard.gpu_count' 2>/dev/null || echo "0")
    echo -e "${GREEN}✓${NC} GPU Guard: ${GPU_COUNT} GPUs detected"
else
    echo -e "${YELLOW}⚠${NC} GPU Guard: not running (may be no nvidia-smi)"
fi

RES_RUNNING=$(echo "$STATUS" | jq -r '.resource_monitor.running' 2>/dev/null || echo "false")
if [ "$RES_RUNNING" == "true" ]; then
    RAM_PCT=$(echo "$STATUS" | jq -r '.resource_monitor.ram_pct' 2>/dev/null || echo "N/A")
    echo -e "${GREEN}✓${NC} Resource Monitor: running (RAM ${RAM_PCT}%)"
else
    echo -e "${RED}✗${NC} Resource Monitor: not running"
fi

LOG_RUNNING=$(echo "$STATUS" | jq -r '.log_watcher.running' 2>/dev/null || echo "false")
if [ "$LOG_RUNNING" == "true" ]; then
    echo -e "${GREEN}✓${NC} Log Watcher: running"
else
    echo -e "${RED}✗${NC} Log Watcher: not running"
fi

# Check circuit breakers
echo -e "${BLUE}[4/5]${NC} Checking circuit breakers..."
BREAKERS=$(curl -sf "$API_URL/circuit-breakers" 2>/dev/null)
OPEN_COUNT=0
CLOSED_COUNT=0

for breaker in $(echo "$BREAKERS" | jq -r 'keys[]' 2>/dev/null); do
    STATE=$(echo "$BREAKERS" | jq -r ".[\"$breaker\"]" 2>/dev/null)
    if [ "$STATE" == "open" ]; then
        OPEN_COUNT=$((OPEN_COUNT + 1))
        echo -e "${RED}✗${NC} $breaker: OPEN"
    else
        CLOSED_COUNT=$((CLOSED_COUNT + 1))
    fi
done

if [ $OPEN_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓${NC} All circuit breakers closed ($CLOSED_COUNT total)"
else
    echo -e "${YELLOW}⚠${NC} $OPEN_COUNT breakers open, $CLOSED_COUNT closed"
fi

# Check recent actions
echo -e "${BLUE}[5/5]${NC} Checking recent activity..."
AUDIT=$(curl -sf "$API_URL/audit" 2>/dev/null)
AUDIT_COUNT=$(echo "$AUDIT" | jq 'length' 2>/dev/null || echo "0")
echo -e "${GREEN}✓${NC} Audit trail: $AUDIT_COUNT entries"

# Show last entry if exists
if [ "$AUDIT_COUNT" -gt 0 ]; then
    LAST_ENTRY=$(echo "$AUDIT" | jq -r '.[-1] | "\(.actor) → \(.action) → \(.target)"' 2>/dev/null)
    echo -e "  ${CYAN}Last:${NC} $LAST_ENTRY"
fi

# Summary
echo ""
echo -e "${CYAN}${BOLD}════════════════════════════════════════════════${NC}"
if (( $(echo "$SCORE >= 75" | bc -l) )) && [ "$RES_RUNNING" == "true" ] && [ $OPEN_COUNT -eq 0 ]; then
    echo -e "${GREEN}${BOLD}✓ AUTONOMIC CONTROL PLANE: OPERATIONAL${NC}"
    echo -e "${CYAN}${BOLD}════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${BOLD}Dashboard:${NC} firefox swarm/autonomic/dashboard.html"
    echo -e "${BOLD}API:${NC}       $API_URL/status"
    echo -e "${BOLD}Score:${NC}     ${GREEN}${SCORE}${NC}/100"
    echo -e "${BOLD}State:${NC}     ${STATE}"
    echo ""
    exit 0
else
    echo -e "${YELLOW}${BOLD}⚠ AUTONOMIC CONTROL PLANE: DEGRADED${NC}"
    echo -e "${CYAN}${BOLD}════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}Some components may need attention.${NC}"
    echo -e "Check full status: curl $API_URL/status | jq"
    echo ""
    exit 1
fi
