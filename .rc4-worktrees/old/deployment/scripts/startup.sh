#!/bin/bash
# Quick startup script for X3 Chain services
# Usage: bash deployment/scripts/startup.sh

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VENV_PATH="$PROJECT_ROOT/cross-chain-gpu-validator/.venv"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  X3 Chain - System Startup                             ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if running via systemd
if systemctl is-active --quiet redis.service; then
    echo -e "${GREEN}✓ Redis running (systemd)${NC}"
else
    echo -e "${YELLOW}⚠ Redis not running via systemd, attempting local start...${NC}"
    if command -v redis-server &> /dev/null; then
        redis-server --daemonize yes --logfile /tmp/redis.log
        sleep 1
        echo -e "${GREEN}✓ Redis started (local)${NC}"
    else
        echo -e "${YELLOW}⚠ Redis not found. Install with: sudo apt install redis-server${NC}"
    fi
fi

# Start validator service
echo -e "${YELLOW}[*] Starting Cross-Chain GPU Validator...${NC}"
if systemctl is-active --quiet ccgv-validator.service; then
    echo -e "${GREEN}✓ Validator running (systemd)${NC}"
else
    echo -e "${YELLOW}[*] Starting validator (local)...${NC}"
    cd "$PROJECT_ROOT/cross-chain-gpu-validator"
    
    # Activate venv
    if [[ ! -d "$VENV_PATH" ]]; then
        echo -e "${YELLOW}[*] Creating virtual environment...${NC}"
        python3 -m venv "$VENV_PATH"
    fi
    source "$VENV_PATH/bin/activate"
    
    # Start in background
    nohup python -m cross_chain_gpu_validator.cli serve \
        --host 0.0.0.0 \
        --port 8000 \
        > /tmp/ccgv-validator.log 2>&1 &
    
    VALIDATOR_PID=$!
    sleep 2
    if kill -0 $VALIDATOR_PID 2>/dev/null; then
        echo -e "${GREEN}✓ Validator started (PID: $VALIDATOR_PID)${NC}"
    else
        echo -e "${YELLOW}⚠ Validator may have failed to start. Check: tail /tmp/ccgv-validator.log${NC}"
    fi
fi

# Start dashboard
echo -e "${YELLOW}[*] Starting X3 Intelligence Dashboard...${NC}"
if systemctl is-active --quiet x3-intelligence.service; then
    echo -e "${GREEN}✓ Dashboard running (systemd)${NC}"
else
    echo -e "${YELLOW}[*] Starting dashboard (local)...${NC}"
    cd "$PROJECT_ROOT/apps/x3-intelligence"
    
    # Check if node_modules exists
    if [[ ! -d "node_modules" ]]; then
        echo -e "${YELLOW}[*] Installing npm dependencies...${NC}"
        npm install --silent
    fi
    
    # Start in background
    nohup npm run start \
        > /tmp/x3-intelligence.log 2>&1 &
    
    DASHBOARD_PID=$!
    sleep 2
    if kill -0 $DASHBOARD_PID 2>/dev/null; then
        echo -e "${GREEN}✓ Dashboard started (PID: $DASHBOARD_PID)${NC}"
    else
        echo -e "${YELLOW}⚠ Dashboard may have failed to start. Check: tail /tmp/x3-intelligence.log${NC}"
    fi
fi

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Services Started                                         ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "📊 ${BLUE}Dashboard:${NC}  http://localhost:5173"
echo -e "📈 ${BLUE}Metrics:${NC}    http://localhost:8000/metrics.json"
echo -e "🔓 ${BLUE}Login:${NC}      admin / x3-chain-2026 (CHANGE THIS!)"
echo ""
echo -e "📋 ${YELLOW}Logs:${NC}"
echo -e "  Validator: tail -f /tmp/ccgv-validator.log"
echo -e "  Dashboard: tail -f /tmp/x3-intelligence.log"
echo -e "  Redis:     tail -f /tmp/redis.log"
echo ""
