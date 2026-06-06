#!/bin/bash
# Phase 5 Complete Execution Orchestrator
# Runs Settlement Testing, Indexer, and Monitoring in parallel

set -e

WORKSPACE="/home/lojak/Desktop/X3_ATOMIC_STAR"
LOG_DIR="/tmp/x3-testnet-logs"
mkdir -p "$LOG_DIR"

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║      PHASE 5 - UNIFIED EXECUTION ORCHESTRATOR                ║"
echo "║   Settlement Testing | Indexer Deployment | Live Monitoring  ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo

# ===== PHASE 5a: Settlement Flow E2E Tests =====
echo "📋 [Phase 5a] Launching Settlement Flow E2E Tests..."
echo "    Command: cargo test --lib tests_phase4 --release -- --nocapture"
echo "    Logging to: $LOG_DIR/settlement-tests.log"
echo
cd "$WORKSPACE"
timeout 600 cargo test --lib tests_phase4 --release -- --nocapture \
  2>&1 | tee "$LOG_DIR/settlement-tests.log" &
SETTLEMENT_PID=$!
echo "    PID: $SETTLEMENT_PID"
echo

# ===== PHASE 5b: Indexer Deployment =====
echo "📋 [Phase 5b] Building X3 Indexer..."
echo "    Location: crates/x3-indexer"
echo "    Target RPC: http://127.0.0.1:9933"
echo "    Listen: 0.0.0.0:4000"
echo
sleep 5  # Give settlement tests time to start
cd "$WORKSPACE/crates/x3-indexer"
timeout 300 cargo build --release 2>&1 | tee "$LOG_DIR/indexer-build.log" &
INDEXER_BUILD_PID=$!
echo "    Build PID: $INDEXER_BUILD_PID"
echo

# ===== PHASE 5c: Live Monitoring (Parallel) =====
echo "📋 [Phase 5c] Starting Real-Time Monitoring..."
echo "    Tracking: Validator-1 logs, block production, peer consensus"
echo
(
  while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$TIMESTAMP] ╔════ CONSENSUS STATUS ════╗"
    
    # Get latest validator status
    LATEST=$(tail -1 "$LOG_DIR/validator1.log" 2>/dev/null || echo "No logs yet")
    echo "[$TIMESTAMP] Val-1: $LATEST"
    
    LATEST2=$(tail -1 "$LOG_DIR/validator2.log" 2>/dev/null || echo "No logs yet")
    echo "[$TIMESTAMP] Val-2: $LATEST2"
    
    LATEST3=$(tail -1 "$LOG_DIR/validator3.log" 2>/dev/null || echo "No logs yet")
    echo "[$TIMESTAMP] Val-3: $LATEST3"
    
    # Check test progress
    if [ -f "$LOG_DIR/settlement-tests.log" ]; then
      TEST_COUNT=$(grep -c "test result:" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
      if [ "$TEST_COUNT" -gt 0 ]; then
        echo "[$TIMESTAMP] ✅ Settlement tests completed: $TEST_COUNT result(s)"
      else
        RUNNING=$(grep -c "^test " "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
        echo "[$TIMESTAMP] 🔄 Settlement tests running: $RUNNING test(s) executed"
      fi
    fi
    
    # Check indexer build
    if [ -f "$LOG_DIR/indexer-build.log" ]; then
      if grep -q "Finished.*indexer" "$LOG_DIR/indexer-build.log" 2>/dev/null; then
        echo "[$TIMESTAMP] ✅ Indexer build complete!"
      else
        BUILD_LINES=$(wc -l < "$LOG_DIR/indexer-build.log" 2>/dev/null || echo "0")
        echo "[$TIMESTAMP] 🔄 Indexer build in progress: $BUILD_LINES lines"
      fi
    fi
    
    echo "[$TIMESTAMP] ╚═══════════════════════════╝"
    echo
    sleep 5
  done
) &
MONITOR_PID=$!
echo "    Monitor PID: $MONITOR_PID"
echo

# ===== Wait for All Tasks =====
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏳ All Phase 5 tasks launched. Waiting for completion..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Function to monitor process status
check_tasks() {
  SETTLEMENT_DONE=0
  INDEXER_DONE=0
  
  if ! kill -0 $SETTLEMENT_PID 2>/dev/null; then
    SETTLEMENT_DONE=1
  fi
  
  if ! kill -0 $INDEXER_BUILD_PID 2>/dev/null; then
    INDEXER_DONE=1
  fi
  
  return 0
}

# Wait for settlement tests
echo "⏳ Waiting for Settlement Flow Tests..."
wait $SETTLEMENT_PID 2>/dev/null || true
echo "✅ Settlement Flow Tests complete!"
echo

# Wait for indexer build
echo "⏳ Waiting for Indexer Build..."
wait $INDEXER_BUILD_PID 2>/dev/null || true
echo "✅ Indexer Build complete!"
echo

# Kill monitor
kill $MONITOR_PID 2>/dev/null || true

# ===== Provide Results Summary =====
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║              PHASE 5 EXECUTION SUMMARY                       ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo

# Settlement test results
echo "📊 SETTLEMENT TESTS:"
if [ -f "$LOG_DIR/settlement-tests.log" ]; then
  PASS_COUNT=$(grep -c "test result: ok" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
  FAIL_COUNT=$(grep -c "test result: FAILED" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
  echo "  ✅ Passed: $PASS_COUNT"
  echo "  ❌ Failed: $FAIL_COUNT"
  tail -20 "$LOG_DIR/settlement-tests.log" | grep -E "test result:|passed|FAILED" | tail -5 || true
else
  echo "  ⚠️  No settlement test results found"
fi
echo

# Indexer build results
echo "🔨 INDEXER BUILD:"
if [ -f "$LOG_DIR/indexer-build.log" ]; then
  if grep -q "Finished.*release" "$LOG_DIR/indexer-build.log" 2>/dev/null; then
    echo "  ✅ Build succeeded"
    BINARY_SIZE=$(du -h "$WORKSPACE/crates/x3-indexer/target/release/x3-indexer" 2>/dev/null | cut -f1)
    echo "  📦 Binary size: $BINARY_SIZE"
    echo "  🚀 Ready for deployment on :4000"
  else
    echo "  ⚠️  Build status unclear - check logs"
  fi
else
  echo "  ⚠️  No indexer build logs found"
fi
echo

# Validator network state
echo "🌐 VALIDATOR NETWORK STATE:"
echo "  Validators Running: $(pgrep -f 'x3-chain-node.*--validator' | wc -l)/3"
for i in {1,2,3}; do
  LOG="$LOG_DIR/validator$i.log"
  if [ -f "$LOG" ]; then
    LATEST=$(tail -1 "$LOG")
    echo "  Validator-$i: $LATEST" | head -c 100
    echo
  fi
done
echo

# Next steps
echo "📋 NEXT STEPS:"
echo "1️⃣  Start Indexer (if build succeeded):"
echo "    ./crates/x3-indexer/target/release/x3-indexer \\"
echo "      --listen 0.0.0.0:4000 \\"
echo "      --rpc-urls http://127.0.0.1:9933"
echo
echo "2️⃣  Validate Cross-VM Bridge:"
echo "    Query RPC for bridge adapter status and proof validation"
echo
echo "3️⃣  Monitor Performance:"
echo "    watch -n 2 'tail -1 $LOG_DIR/validator1.log'"
echo
echo "📂 Log Files:"
echo "   Settlement Tests: $LOG_DIR/settlement-tests.log"
echo "   Indexer Build: $LOG_DIR/indexer-build.log"
echo "   Validator Logs: $LOG_DIR/validator{1,2,3}.log"
echo
echo "═══════════════════════════════════════════════════════════════"
echo "✨ Phase 5 Execution Complete!"
echo "═══════════════════════════════════════════════════════════════"
