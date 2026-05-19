#!/bin/bash
# Stop script for X3 Chain - "The Beast"

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║  Stopping X3 Chain - 'The Beast'                       ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check for PID file
if [[ -f /tmp/x3-chain-pids.txt ]]; then
    source /tmp/x3-chain-pids.txt
    
    echo "[*] Stopping X3 Intelligence (PID: $X3_INTELLIGENCE_PID)..."
    kill -TERM $X3_INTELLIGENCE_PID 2>/dev/null || true
    sleep 1
    kill -KILL $X3_INTELLIGENCE_PID 2>/dev/null || true
    echo "✓ X3 Intelligence stopped"
    
    echo "[*] Stopping GPU Validator (PID: $CCGV_VALIDATOR_PID)..."
    kill -TERM $CCGV_VALIDATOR_PID 2>/dev/null || true
    sleep 1
    kill -KILL $CCGV_VALIDATOR_PID 2>/dev/null || true
    echo "✓ GPU Validator stopped"
    
    rm /tmp/x3-chain-pids.txt
else
    echo "[*] PID file not found. Killing all remaining processes..."
    pkill -f "npm run dev" || true
    pkill -f "cross_chain_gpu_validator" || true
fi

echo ""
echo "✅ All services stopped"
echo ""
