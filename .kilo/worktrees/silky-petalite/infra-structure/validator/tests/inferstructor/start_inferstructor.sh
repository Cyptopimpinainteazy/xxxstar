#!/usr/bin/env bash
# One-command start for all Inferstructor services with authentication

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🚀 Starting Inferstructor Superhighway..."
echo ""

# Check Python dependencies
if ! python3 -c "import aiohttp" 2>/dev/null; then
    echo "📦 Installing Python dependencies..."
    pip3 install aiohttp pyjwt prometheus_client pyyaml >/dev/null 2>&1 || {
        echo "❌ Failed to install dependencies"
        echo "Run: pip3 install aiohttp pyjwt prometheus_client pyyaml"
        exit 1
    }
fi

# Kill any existing services
echo "🧹 Cleaning up old processes..."
pkill -f "validator_registry.py" 2>/dev/null || true
pkill -f "tps_bridge.py" 2>/dev/null || true
pkill -f "metrics_dashboard.py" 2>/dev/null || true
pkill -f "lane_orchestrator.py" 2>/dev/null || true
sleep 2

# Start Validator Registry (Port 7001)
echo "🔐 Starting Validator Registry on port 7001..."
python3 validator_registry.py > logs/validator_registry.log 2>&1 &
REGISTRY_PID=$!
echo "   PID: $REGISTRY_PID"

sleep 2

# Check if registry started
if ! curl -sf http://localhost:7001/health > /dev/null 2>&1; then
    echo "❌ Validator Registry failed to start"
    cat logs/validator_registry.log
    exit 1
fi

# Start TPS Bridge (Port 9999)
echo "🌉 Starting TPS Bridge on port 9999..."
python3 tps_bridge.py > logs/tps_bridge.log 2>&1 &
BRIDGE_PID=$!
echo "   PID: $BRIDGE_PID"

sleep 2

if ! curl -sf http://localhost:9999/health > /dev/null 2>&1; then
    echo "❌ TPS Bridge failed to start"
    cat logs/tps_bridge.log
    exit 1
fi

# Start Metrics Dashboard (Port 8080)
echo "📊 Starting Metrics Dashboard on port 8080..."
python3 metrics_dashboard.py > logs/metrics_dashboard.log 2>&1 &
DASHBOARD_PID=$!
echo "   PID: $DASHBOARD_PID"

sleep 2

if ! curl -sf http://localhost:8080 > /dev/null 2>&1; then
    echo "⚠️  Metrics Dashboard may not have started (port 8080)"
fi

# Start Lane Orchestrator (background)
echo "🎯 Starting Lane Orchestrator..."
python3 lane_orchestrator.py > logs/lane_orchestrator.log 2>&1 &
ORCHESTRATOR_PID=$!
echo "   PID: $ORCHESTRATOR_PID"

sleep 1

# Save PIDs for later cleanup
cat > .inferstructor.pids <<EOF
$REGISTRY_PID
$BRIDGE_PID
$DASHBOARD_PID
$ORCHESTRATOR_PID
EOF

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Inferstructor is LIVE!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📡 Services:"
echo "   🔐 Validator Registry:  http://localhost:7001"
echo "   🌉 TPS Bridge:          http://localhost:9999"
echo "   📊 Metrics Dashboard:   http://localhost:8080"
echo "   🎯 Lane Orchestrator:   Running in background"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎯 QUICK START FOR VALIDATORS:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Step 1: Register your validator"
echo "   ./register_validator.sh solana your-email@example.com pro"
echo ""
echo "Step 2: Test acceleration"
echo "   curl -X POST http://localhost:9999/accelerate \\"
echo "     -H 'X-API-Key: <your-api-key>' \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"tx_hash\":\"test\",\"tx_data\":\"48656c6c6f\",\"chain\":\"solana\"}'"
echo ""
echo "Step 3: Run performance test"
echo "   ./run_300x_test.sh --duration 5m"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📚 Documentation:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "   📖 Validator Quickstart:  VALIDATOR_QUICKSTART.md"
echo "   📖 Full Test Plan:        ../../docs/INFERSTRUCTOR_300X_TEST_PLAN.md"
echo "   📖 Quick Reference:       QUICKREF.md"
echo "   📖 README:                docs/root/README.md"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🛑 To stop all services:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "   ./stop_inferstructor.sh"
echo "   OR"
echo "   kill \$(cat .inferstructor.pids)"
echo ""

# Tail logs
echo "📋 Tailing logs (Ctrl+C to detach, services will keep running)..."
echo ""
sleep 2
tail -f logs/*.log
