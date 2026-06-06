#!/bin/bash
# X3_ATOMIC_STAR Testnet Quick-Start Script
# Purpose: One-command testnet deployment
# Date: 2026-04-24

set -e

PROJECT_DIR="/home/lojak/Desktop/X3_ATOMIC_STAR"
BUILD_DIR="$PROJECT_DIR/target/release"
NODE_BIN="$BUILD_DIR/x3-chain-node"
TESTNET_DIR="$PROJECT_DIR/testnet"
LOG_DIR="$PROJECT_DIR/logs"
VALIDATOR_COUNT=${1:-1}

echo "🚀 X3_ATOMIC_STAR Testnet Quick-Start"
echo "======================================"
echo ""

# Step 1: Check prerequisites
echo "✅ STEP 1: Checking prerequisites..."
if [ ! -f "$NODE_BIN" ]; then
    echo "❌ Node binary not found: $NODE_BIN"
    echo "   Run: cargo build --release -p x3-chain-node"
    exit 1
fi
echo "   ✓ Node binary found"

if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found"
    exit 1
fi
RUSTC_VERSION=$(rustc --version | awk '{print $2}')
echo "   ✓ Rust $RUSTC_VERSION"
echo ""

# Step 2: Create testnet directories
echo "✅ STEP 2: Creating testnet directories..."
mkdir -p "$TESTNET_DIR" "$LOG_DIR"
echo "   ✓ Directories created"
echo ""

# Step 3: Generate keys and chain spec
echo "✅ STEP 3: Generating chain spec..."
if [ ! -f "$TESTNET_DIR/chain-spec.json" ]; then
    echo "   Creating fresh chain spec..."
    # For now, we'll note that this should be done via deployment scripts
    echo "   ℹ  Chain spec should be created via: ./deployment/key-gen-testnet.sh"
    echo "   Using default development chain-spec..."
fi
echo ""

# Step 4: Start node
echo "✅ STEP 4: Starting X3 node (${VALIDATOR_COUNT} validator mode)..."
echo ""
echo "Command:"
echo "   $NODE_BIN --chain dev --tmp"
echo ""
echo "Node logs will appear below:"
echo "─────────────────────────────────────────────────────────"
echo ""

# Run node in foreground with development chain
exec "$NODE_BIN" \
    --chain dev \
    --tmp \
    --rpc-methods Unsafe \
    --rpc-external \
    --ws-external \
    --ws-port 9944 \
    --rpc-port 9933 \
    --log runtime=debug 2>&1 | tee "$LOG_DIR/testnet-$(date +%s).log"
