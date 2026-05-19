#!/usr/bin/env bash
# X3 Chain Development Node Launcher
# This script launches a local X3 Chain blockchain node in development mode

set -e

# prevent rustup from trying to install components (e.g. clippy) each time
export RUSTUP_SKIP_UPDATE=1

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_ROOT"

echo "🌌 X3 Chain Development Node Launcher"
echo "==========================================="
echo ""

# Check if the binary is built
if [ ! -f "target/release/x3-chain-node" ]; then
    echo "⚠️  Binary not found. Building with: cargo build --release"
    cargo build --release
fi

# Function to kill processes using specific ports
kill_port_processes() {
    local port=$1
    local pids=$(lsof -ti:$port 2>/dev/null || netstat -tulpn 2>/dev/null | grep ":$port " | awk '{print $NF}' | cut -d'/' -f1)
    if [ ! -z "$pids" ]; then
        kill -9 $pids 2>/dev/null || true
    fi
}

# Default configuration
BASE_PATH="${BASE_PATH:-/tmp/x3-dev}"
RPC_PORT="${RPC_PORT:-9944}"
WS_PORT="${WS_PORT:-9945}"
P2P_PORT="${P2P_PORT:-30333}"
PROMETHEUS_PORT="${PROMETHEUS_PORT:-9615}"

SCRIPT_ARGS=()
KEEP_PORTS=false
PURGE_STATE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --purge|-p)
            PURGE_STATE=true
            shift
            ;;
        --keep-ports)
            KEEP_PORTS=true
            shift
            ;;
        *)
            SCRIPT_ARGS+=("$1")
            shift
            ;;
    esac
done

# Security: Determine RPC binding based on environment
# Production: bind only to localhost
# Development: can optionally bind externally with --unsafe-rpc-external
RPC_EXTERNAL=""
CORS_ORIGINS="http://localhost:3000,http://127.0.0.1:3000"

if [ "${X3_DEV_MODE:-}" = "external" ]; then
    echo "⚠️  WARNING: External RPC access enabled (development only!)"
    RPC_EXTERNAL="--rpc-external --unsafe-rpc-external"
    CORS_ORIGINS="*"
fi

# Clean previous state if requested
if [ "$PURGE_STATE" = "true" ]; then
    echo "🧹 Purging chain data at $BASE_PATH..."
    rm -rf "$BASE_PATH"
fi

# Always clean up old processes on ports (unless --keep-ports flag)
if [ "$KEEP_PORTS" != "true" ]; then
    echo "🧹 Cleaning up processes on ports $RPC_PORT, $P2P_PORT, $PROMETHEUS_PORT..."
    kill_port_processes "$RPC_PORT"
    kill_port_processes "$WS_PORT"  
    kill_port_processes "$P2P_PORT"
    kill_port_processes "$PROMETHEUS_PORT"
    sleep 1  # Give OS time to release ports
fi

echo ""
echo "Configuration:"
echo "  Base Path: $BASE_PATH"
echo "  RPC Port: $RPC_PORT"
echo "  P2P Port: $P2P_PORT"
echo "  Prometheus: http://127.0.0.1:$PROMETHEUS_PORT/metrics"
echo ""

# Check if desktop app should be started
# Default to false in CI/limited environments; allow override with env var
START_DESKTOP="${START_DESKTOP:-false}"
X3_PERF_MODE="${X3_PERF_MODE:-false}"

# Verify `npm` is available before attempting to start desktop app
if [ "$START_DESKTOP" = "true" ] && ! command -v npm >/dev/null 2>&1; then
    echo "⚠️  npm not found in PATH; disabling desktop app start"
    START_DESKTOP=false
fi

RPC_METHODS="Safe"
RPC_MAX_CONNECTIONS="100"
RPC_MAX_REQUEST_SIZE="10"
RPC_MAX_RESPONSE_SIZE="50"
RPC_MAX_SUBSCRIPTIONS_PER_CONNECTION="10"

if [ "$X3_PERF_MODE" = "true" ]; then
    echo "⚡ Performance mode enabled for stress testing"
    RPC_METHODS="Unsafe"
    RPC_MAX_CONNECTIONS="2000"
    RPC_MAX_REQUEST_SIZE="50"
    RPC_MAX_RESPONSE_SIZE="100"
    RPC_MAX_SUBSCRIPTIONS_PER_CONNECTION="20000"
fi

# Start the node with secure defaults in the background
./target/release/x3-chain-node \
    --dev \
    --base-path "$BASE_PATH" \
    --rpc-port "$RPC_PORT" \
    --port "$P2P_PORT" \
    --prometheus-port "$PROMETHEUS_PORT" \
    --rpc-cors "$CORS_ORIGINS" \
    --rpc-methods "$RPC_METHODS" \
    --rpc-max-connections "$RPC_MAX_CONNECTIONS" \
    --rpc-max-request-size "$RPC_MAX_REQUEST_SIZE" \
    --rpc-max-response-size "$RPC_MAX_RESPONSE_SIZE" \
    --rpc-max-subscriptions-per-connection "$RPC_MAX_SUBSCRIPTIONS_PER_CONNECTION" \
    --detailed-log-output \
    --log sync=debug,consensus=debug,grandpa=debug,runtime=info \
    $RPC_EXTERNAL \
    "${SCRIPT_ARGS[@]}" &

# Store the blockchain process PID
BLOCKCHAIN_PID=$!
echo "✅ Blockchain started (PID: $BLOCKCHAIN_PID)"
echo ""

# Set up signal handlers for clean shutdown
cleanup() {
    echo ""
    echo "🛑 Stopping blockchain..."
    kill -TERM $BLOCKCHAIN_PID 2>/dev/null || true
    wait $BLOCKCHAIN_PID 2>/dev/null || true
    echo "✅ Blockchain stopped"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Wait for blockchain to initialize (checking RPC port)
echo "⏳ Waiting for blockchain to be ready..."
sleep 2
for i in {1..30}; do
    READY=1
    if command -v nc >/dev/null 2>&1; then
        if nc -z localhost "$RPC_PORT" 2>/dev/null; then
            READY=0
        fi
    fi
    if [ $READY -ne 0 ] && command -v curl >/dev/null 2>&1; then
        if curl -s "http://localhost:$RPC_PORT/health" >/dev/null 2>&1; then
            READY=0
        fi
    fi
    if [ $READY -eq 0 ]; then
        echo "✅ Blockchain is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "⚠️  Blockchain may not be fully ready, but continuing..."
    fi
    sleep 1
done

# Start the desktop app if enabled
if [ "$START_DESKTOP" = "true" ]; then
    echo ""
    echo "🚀 Starting Tauri desktop app..."
    cd apps/x3-desktop
    npm run tauri dev &
    DESKTOP_PID=$!
    echo "✅ Desktop app started (PID: $DESKTOP_PID)"
    cd "$PROJECT_ROOT"
    echo ""
    echo "📍 Running services:"
    echo "   • Blockchain RPC: ws://127.0.0.1:$RPC_PORT"
    echo "   • Desktop App: Launching..."
    echo ""
    echo "Press Ctrl+C to stop both services"
    echo ""
    
    # Wait for both processes
    wait $BLOCKCHAIN_PID $DESKTOP_PID
else
    echo "🚀 Desktop app disabled (set START_DESKTOP=true to enable)"
    echo ""
    echo "📍 Blockchain RPC: ws://127.0.0.1:$RPC_PORT"
    echo ""
    echo "Press Ctrl+C to stop the blockchain"
    echo ""
    
    # Wait for blockchain and display block progress
    # Pipe logs through block visualizer if available
    if command -v python3 >/dev/null 2>&1 && [ -f "scripts/block_display.py" ]; then
        wait $BLOCKCHAIN_PID
    else
        wait $BLOCKCHAIN_PID
    fi
fi
