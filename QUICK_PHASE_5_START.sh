#!/bin/bash

# X3_ATOMIC_STAR - PHASE 5+ QUICK LAUNCH REFERENCE
# Run this to continue development from where we left off
# Date: April 25, 2026

PROJECT_ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"
cd "$PROJECT_ROOT"

echo "═══════════════════════════════════════════════════════════"
echo "🚀 X3_ATOMIC_STAR - Phase 5+ Quick Start"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}📊 CURRENT STATUS${NC}"
echo "────────────────────────────────────────────────────────"

# Check validators running
VAL1=$(pgrep -f 'x3-chain-node.*Validator-1' | wc -l)
VAL2=$(pgrep -f 'x3-chain-node.*Validator-2' | wc -l)
VAL3=$(pgrep -f 'x3-chain-node.*Validator-3' | wc -l)

echo -e "Validators Running:"
echo -e "  Validator-1: $([ $VAL1 -gt 0 ] && echo -e "${GREEN}✅ RUNNING${NC}" || echo "⏹️  STOPPED")"
echo -e "  Validator-2: $([ $VAL2 -gt 0 ] && echo -e "${GREEN}✅ RUNNING${NC}" || echo "⏹️  STOPPED")"
echo -e "  Validator-3: $([ $VAL3 -gt 0 ] && echo -e "${GREEN}✅ RUNNING${NC}" || echo "⏹️  STOPPED")"

echo ""
echo -e "${BLUE}🎯 NEXT ACTIONS${NC}"
echo "────────────────────────────────────────────────────────"
echo ""
echo "1️⃣  LAUNCH VALIDATOR 3 (if not running)"
echo "   Command:"
echo "   ./target/release/x3-chain-node \\"
echo "     --chain ./deployment/chain-specs/x3-testnet-raw.json \\"
echo "     --validator --name 'Validator-3' \\"
echo "     --port 30335 --rpc-port 9935 --unsafe-rpc-external \\"
echo "     --prometheus-port 9618 \\"
echo "     --bootnodes \"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWP1XsE2tRWDVyAMyCxeDUqsCvCGFKt7ZoCZk7Wn8BKWjU\" \\"
echo "     --tmp"
echo ""
echo "2️⃣  MONITOR BLOCK PRODUCTION"
echo "   Command:"
echo "   watch -n 2 'curl -s http://127.0.0.1:9933 -X POST \\'"
echo "     -H \"Content-Type: application/json\" \\'"
echo "     -d \"{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"method\\\":\\\"chain_getBlockNumber\\\",\\\"params\\\":[],\\\"id\\\":1}\" | jq .result'"
echo ""
echo "3️⃣  EXECUTE SETTLEMENT FLOW"
echo "   Command:"
echo "   cargo test --release settlement_flow -- --nocapture"
echo ""
echo "4️⃣  DEPLOY INDEXER"
echo "   Command:"
echo "   cd crates/x3-indexer && cargo run --release -- --listen 0.0.0.0:4000"
echo ""

echo -e "${BLUE}📋 DOCUMENTATION${NC}"
echo "────────────────────────────────────────────────────────"
echo "Phase 1-4 Summary: ./PHASE_1B_4_COMPLETION_REPORT.md"
echo "Phase 5+ Roadmap: ./PHASE_5_ROADMAP.md"
echo "Session Summary:  ./SESSION_PHASE_5_LAUNCH_SUMMARY.md"
echo ""

echo -e "${BLUE}🔍 MONITORING${NC}"
echo "────────────────────────────────────────────────────────"
echo "Logs:"
echo "  Val1: tail -f /tmp/x3-testnet-logs/validator1.log"
echo "  Val2: tail -f /tmp/x3-testnet-logs/validator2.log"
echo "  Val3: tail -f /tmp/x3-testnet-logs/validator3.log"
echo ""
echo "RPC Health Checks:"
echo "  Val1: http://127.0.0.1:9933"
echo "  Val2: http://127.0.0.1:9934"
echo "  Val3: http://127.0.0.1:9935"
echo ""

echo -e "${BLUE}🧹 CLEANUP${NC}"
echo "────────────────────────────────────────────────────────"
echo "Stop all validators: pkill -9 x3-chain-node"
echo "Clean state:         rm -rf /tmp/x3-testnet* /tmp/substrate*"
echo ""

echo "═══════════════════════════════════════════════════════════"
echo -e "${GREEN}✅ System ready for Phase 5+ continuation${NC}"
echo "═══════════════════════════════════════════════════════════"
