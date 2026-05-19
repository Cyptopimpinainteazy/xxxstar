#!/bin/bash
# Quick start script for X3 Chain - "The Beast"

set -e

PROJECT_ROOT="/home/lojak/Desktop/x3-chain-master"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  🦾 X3 Chain - Starting 'The Beast'                    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Kill any existing processes (cleanup)
echo -e "${YELLOW}[*] Cleaning up old processes...${NC}"
pkill -f "npm run dev" || true
pkill -f "cross_chain_gpu_validator" || true
pkill -f "redis-server" || true
sleep 1

# Start Redis
echo -e "${BLUE}[*] Starting Redis...${NC}"
redis-server --port 6379 --daemonize yes --logfile /tmp/redis.log > /dev/null 2>&1
sleep 1
if redis-cli ping > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Redis started${NC}"
else
    echo -e "${YELLOW}⚠ Redis failed to start (may already be running)${NC}"
fi
echo ""

# Start X3 Intelligence Dashboard
echo -e "${BLUE}[*] Starting X3 Intelligence Dashboard...${NC}"
cd "$PROJECT_ROOT/apps/x3-intelligence"
npm install --silent > /dev/null 2>&1 || true
npm run dev -- --port 5173 --host 0.0.0.0 > /tmp/x3-intelligence.log 2>&1 &
X3_PID=$!
echo -e "${GREEN}✓ X3 Intelligence started (PID: $X3_PID)${NC}"
echo "   URL: http://localhost:5173"
echo "   Logs: tail -f /tmp/x3-intelligence.log"
echo ""

# Start X3 Intelligence Backend API Server
echo -e "${BLUE}[*] Starting X3 Intelligence API Server...${NC}"
cd "$PROJECT_ROOT/apps/x3-intelligence"
npm install --silent > /dev/null 2>&1 || true
node server.js > /tmp/x3-api.log 2>&1 &
API_PID=$!
echo -e "${GREEN}✓ API Server started (PID: $API_PID)${NC}"
echo "   URL: http://localhost:8001/api/v1"
echo "   Logs: tail -f /tmp/x3-api.log"
echo ""

# Start Cross-Chain GPU Validator
echo -e "${BLUE}[*] Starting Cross-Chain GPU Validator...${NC}"
KERNELS_PATH="$PROJECT_ROOT/infra-structure/validator/kernels"
(
  cd "$PROJECT_ROOT/infra-structure/validator"
  source .venv/bin/activate
  export CCGV_KERNEL_DIR="$KERNELS_PATH"
  export CCGV_USE_MOCK_RPC=true
  export CCGV_REQUIRE_GPU=false
  python -m cross_chain_gpu_validator.cli orchestrator > /tmp/ccgv-validator.log 2>&1
) &
CCGV_PID=$!
echo -e "${GREEN}✓ GPU Validator started (PID: $CCGV_PID)${NC}"
echo "   URL: http://localhost:8000/metrics.json"
echo "   Logs: tail -f /tmp/ccgv-validator.log"
echo ""

# Save PIDs for later cleanup
cat > /tmp/x3-chain-pids.txt << EOF
X3_INTELLIGENCE_PID=$X3_PID
API_SERVER_PID=$API_PID
CCGV_VALIDATOR_PID=$CCGV_PID
EOF

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  🦾 The Beast is Running!                                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Services:"
echo "  📊 X3 Intelligence:    http://localhost:5173"
echo "  🔌 API Server:         http://localhost:8001/api/v1"
echo "  ⚙️  GPU Validator:       http://localhost:8000/metrics.json"
echo ""
echo "Login:"
echo "  Username: admin"
echo "  Password: x3-chain-2026"
echo ""
echo "PIDs saved to: /tmp/x3-chain-pids.txt"
echo ""
echo "To stop all services:"
echo "  bash $PROJECT_ROOT/scripts/stop-beast.sh"
echo ""
echo "To view logs:"
echo "  tail -f /tmp/x3-intelligence.log"
echo "  tail -f /tmp/x3-api.log"
echo "  tail -f /tmp/ccgv-validator.log"
echo ""

# Wait for services to be ready
sleep 3

# Check services are running
echo -e "${BLUE}[*] Verifying services...${NC}"
if ps -p $X3_PID > /dev/null; then
    echo -e "${GREEN}✓ X3 Intelligence is running${NC}"
else
    echo -e "${YELLOW}⚠ X3 Intelligence failed to start${NC}"
fi

if ps -p $API_PID > /dev/null; then
    echo -e "${GREEN}✓ API Server is running${NC}"
else
    echo -e "${YELLOW}⚠ API Server failed to start${NC}"
fi

if ps -p $CCGV_PID > /dev/null; then
    echo -e "${GREEN}✓ GPU Validator is running${NC}"
else
    echo -e "${YELLOW}⚠ GPU Validator failed to start${NC}"
fi

echo ""
echo -e "${GREEN}✅ Setup complete! Point your browser to http://localhost:5173${NC}"
echo ""

# Keep script running
wait
