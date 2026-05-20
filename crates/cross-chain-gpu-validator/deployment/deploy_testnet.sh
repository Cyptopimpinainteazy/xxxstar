#!/bin/bash
# Testnet deployment script for Solana + Ethereum

set -e

echo "Cross-Chain GPU Validator - Testnet Deployment"
echo "=============================================="

# Configuration
REDIS_URL=${REDIS_URL:-"redis://localhost:6379"}
SOLANA_RPC=${SOLANA_RPC:-"https://api.devnet.solana.com"}
ETH_RPC=${ETH_RPC:-"https://goerli.infura.io/v3/YOUR_KEY"}
VALIDATOR_PORT=${VALIDATOR_PORT:-"8080"}

echo "Configuration:"
echo "  Redis: $REDIS_URL"
echo "  Solana RPC: $SOLANA_RPC"
echo "  Ethereum RPC: $ETH_RPC"
echo "  Validator Port: $VALIDATOR_PORT"

# Check dependencies
echo -e "\nChecking dependencies..."
command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found"; exit 1; }
echo "✓ Rust toolchain found"

# Build validator
echo -e "\nBuilding validator service..."
cargo build --release --bin cross-chain-gpu-validator
echo "✓ Build complete: target/release/cross-chain-gpu-validator"

# Start Redis if not running
echo -e "\nChecking Redis availability..."
if ! timeout 2 bash -c "echo > /dev/tcp/localhost/6379" 2>/dev/null; then
    echo "⚠ Redis not accessible at localhost:6379"
    echo "  Start with: redis-server"
    exit 1
fi
echo "✓ Redis available"

# Deploy validator service
echo -e "\nDeploying validator service..."
export REDIS_URL=$REDIS_URL
export SOLANA_RPC=$SOLANA_RPC
export ETH_RPC=$ETH_RPC
export RUST_LOG=info

target/release/cross-chain-gpu-validator &
VALIDATOR_PID=$!
echo "✓ Validator service started (PID: $VALIDATOR_PID)"

echo -e "\nDeployment successful!"
echo "Monitor at: http://localhost:$VALIDATOR_PORT/metrics"
echo "Press Ctrl+C to stop"

wait $VALIDATOR_PID
