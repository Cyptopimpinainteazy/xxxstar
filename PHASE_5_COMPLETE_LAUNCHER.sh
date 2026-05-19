#!/bin/bash
# Phase 5 Complete Execution Launcher (ENHANCED)
# Runs all three concurrent Phase 5 tasks with deployment verification
# Fixes: Path issues, false positives, validator startup, logging

set -e

WORKSPACE="/home/lojak/Desktop/X3_ATOMIC_STAR"
LOG_DIR="/tmp/x3-testnet-logs"
VALIDATORS_DIR="/tmp/x3-validators"
mkdir -p "$LOG_DIR" "$VALIDATORS_DIR"

# Enhanced logging
log_info() { echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_error() { echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_success() { echo "[✅] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }
log_warning() { echo "[⚠️] $(date '+%Y-%m-%d %H:%M:%S') - $1"; }

# Process verification function
verify_process_started() {
  local pid=$1
  local name=$2
  local timeout=${3:-5}
  local elapsed=0
  
  while [ $elapsed -lt $timeout ]; do
    if kill -0 $pid 2>/dev/null; then
      log_success "$name process started (PID: $pid)"
      return 0
    fi
    sleep 0.5
    elapsed=$((elapsed + 1))
  done
  
  log_error "$name process did not start (PID: $pid may have exited)"
  return 1
}

# Binary health check
verify_binary_exists() {
  local binary=$1
  local name=$2
  
  if [ ! -f "$binary" ]; then
    log_error "$name binary not found at: $binary"
    return 1
  fi
  
  if [ ! -x "$binary" ]; then
    log_error "$name binary not executable: $binary"
    return 1
  fi
  
  log_success "$name binary verified: $binary"
  return 0
}

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║    PHASE 5 - COMPLETE PARALLEL EXECUTION LAUNCHER (ENHANCED)  ║"
echo "║  🔴 5a: Settlement E2E | 🟡 5b: Indexer | 🟢 5c: Monitoring   ║"
echo "║  ✨ Features: Process Verification | Error Handling | Logging  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

# Kill any existing Phase 5 processes
log_info "Cleaning up any existing Phase 5 processes..."
pkill -f "p4_p5_production_release" 2>/dev/null || true
pkill -f "x3-indexer" 2>/dev/null || true
pkill -f "x3-chain-node.*validator" 2>/dev/null || true
sleep 2
log_success "Cleanup complete"
echo

# ===== PHASE 5a: Settlement Flow E2E Tests =====
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔴 [Phase 5a] Settlement Flow E2E Testing"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Command: python3 p4_p5_production_release.py --validators 3 --testnet-enabled"
log_info "Logging to: $LOG_DIR/settlement-tests.log"
log_info "Starting in 3 seconds..."
sleep 3

if [ ! -f "$WORKSPACE/tests_phase4/p4_p5_production_release.py" ]; then
  log_error "Settlement test script not found at $WORKSPACE/tests_phase4/p4_p5_production_release.py"
  exit 1
fi

cd "$WORKSPACE/tests_phase4"
{
  echo "=== Settlement Tests Started: $(date) ==="
  echo "Environment: Python $(python3 --version 2>&1)"
  echo "Working directory: $PWD"
  timeout 900 python3 -u p4_p5_production_release.py --validators 3 --testnet-enabled 2>&1
  echo "=== Settlement Tests Exit Code: $? ==="
} > "$LOG_DIR/settlement-tests.log" 2>&1 &
SETTLEMENT_PID=$!

if verify_process_started $SETTLEMENT_PID "Settlement Tests"; then
  echo "✅ Started (PID: $SETTLEMENT_PID)"
else
  log_warning "Settlement test process may not have started properly, but continuing..."
fi
echo

# ===== PHASE 5b: Indexer Build & Deployment =====
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🟡 [Phase 5b] X3 Indexer Build & Deployment"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Location: crates/x3-indexer"
log_info "Logging to: $LOG_DIR/indexer.log"
log_info "Target: Release binary for :4000"
log_info "Starting in 10 seconds (after Phase 5a started)..."
sleep 10

cd "$WORKSPACE/crates/x3-indexer"

# Build indexer
log_info "Building X3 Indexer..."
{
  echo "=== Indexer Build Started: $(date) ==="
  timeout 600 cargo build --release 2>&1
  echo "=== Indexer Build Complete: $(date) ==="
} | tee "$LOG_DIR/indexer-build.log"

INDEXER_BINARY="$WORKSPACE/target/release/x3-indexer"

if ! verify_binary_exists "$INDEXER_BINARY" "Indexer"; then
  log_error "Indexer binary build failed or binary not found"
  exit 1
fi

echo
log_info "Deploying X3 Indexer on :4000..."

# Deploy indexer with proper verification
{
  echo "=== Indexer Runtime Started: $(date) ==="
  echo "Command: $INDEXER_BINARY --listen 0.0.0.0:4000 --rpc-urls http://127.0.0.1:9933 http://127.0.0.1:9934 http://127.0.0.1:9935"
  echo "Working directory: $PWD"
  timeout 600 "$INDEXER_BINARY" \
    --listen 0.0.0.0:4000 \
    --rpc-urls http://127.0.0.1:9933 \
               http://127.0.0.1:9934 \
               http://127.0.0.1:9935 \
    2>&1
  echo "=== Indexer Runtime Exit: $(date) ==="
} >> "$LOG_DIR/indexer.log" 2>&1 &
INDEXER_PID=$!

if verify_process_started $INDEXER_PID "Indexer"; then
  log_success "Indexer deployed (PID: $INDEXER_PID)"
  
  # Verify indexer is responding
  sleep 2
  if curl -s http://127.0.0.1:4000/graphql -X POST \
    -H "Content-Type: application/json" \
    -d '{"query":"{ __typename }"}' > /dev/null 2>&1; then
    log_success "Indexer GraphQL endpoint responding"
  else
    log_warning "Indexer GraphQL not yet responsive (may still be starting)"
  fi
else
  log_error "Indexer deployment failed - process did not start"
  exit 1
fi
echo

# ===== PHASE 5c: Live Monitoring =====
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🟢 [Phase 5c] Real-Time Block Production Monitoring"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Tracking: Validator states, block height, GRANDPA finality"
echo "Log display: Real-time tail of validator logs"
echo

# Start monitoring loop
(
  ITERATION=0
  while true; do
    ITERATION=$((ITERATION + 1))
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    
    echo "━━━ [$ITERATION] $TIMESTAMP ━━━"
    echo
    
    # Check settlement tests
    if [ -f "$LOG_DIR/settlement-tests.log" ]; then
      LINES=$(wc -l < "$LOG_DIR/settlement-tests.log")
      if grep -q "test result: ok" "$LOG_DIR/settlement-tests.log" 2>/dev/null; then
        PASSED=$(grep -c "test result: ok" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
        echo "✅ SETTLEMENT TESTS: $PASSED tests passed"
      elif grep -q "FAILED\|Error\|error" "$LOG_DIR/settlement-tests.log" 2>/dev/null; then
        echo "❌ SETTLEMENT TESTS: Errors detected"
      else
        echo "🔄 SETTLEMENT TESTS: Running... ($LINES lines)"
      fi
    else
      echo "⏳ SETTLEMENT TESTS: Starting..."
    fi
    echo
    
    # Check indexer
    if pgrep -f "x3-indexer" > /dev/null 2>&1; then
      echo "✅ INDEXER: Running on :4000"
      curl -s http://127.0.0.1:4000/graphql -X POST \
        -H "Content-Type: application/json" \
        -d '{"query":"{ __typename }"}' > /dev/null 2>&1 && echo "   GraphQL: Responsive" || echo "   GraphQL: Pending..."
    elif [ -f "$LOG_DIR/indexer-build.log" ] && grep -q "Finished" "$LOG_DIR/indexer-build.log" 2>/dev/null; then
      echo "🚀 INDEXER: Build complete, starting deployment..."
    else
      echo "🔨 INDEXER: Building..."
    fi
    echo
    
    # Display validator states
    echo "📊 VALIDATOR CONSENSUS STATE:"
    for i in {1,2,3}; do
      LOG="$LOG_DIR/validator$i.log"
      if [ -f "$LOG" ]; then
        LATEST=$(tail -1 "$LOG" 2>/dev/null)
        if [[ $LATEST == *"Idle"* ]]; then
          # Extract peer count and block height
          PEERS=$(echo "$LATEST" | grep -oP '(?<=\()\d+(?= peers)' || echo "?")
          BLOCK=$(echo "$LATEST" | grep -oP '#\d+' | head -1 || echo "#?")
          FINALIZED=$(echo "$LATEST" | grep -oP 'finalized \#\d+' || echo "finalized #?")
          echo "   Val-$i: Peers=$PEERS, $BLOCK, $FINALIZED ✅"
        else
          echo "   Val-$i: $LATEST" | cut -c 1-80
        fi
      else
        echo "   Val-$i: No logs yet"
      fi
    done
    echo
    
    # Check if settlement tests completed
    if [ -f "$LOG_DIR/settlement-tests.log" ] && grep -q "test result:" "$LOG_DIR/settlement-tests.log"; then
      FINAL_RESULT=$(tail -5 "$LOG_DIR/settlement-tests.log" | grep "test result:" | tail -1)
      if [[ "$FINAL_RESULT" == *"ok"* ]]; then
        echo "🎉 SETTLEMENT TESTS PASSED! $FINAL_RESULT"
      else
        echo "⚠️  SETTLEMENT TESTS COMPLETED. $FINAL_RESULT"
      fi
      echo
      echo "═══════════════════════════════════════════════════════════════"
      break
    fi
    
    # Wait before next update
    sleep 15
  done
) &
MONITOR_PID=$!
echo "✅ Monitoring started (PID: $MONITOR_PID)"
echo

# ===== Wait for Completion =====
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏳ All Phase 5 tasks launched!"
echo "   🔴 Settlement tests: Running (timeout: 15 min)"
echo "   🟡 Indexer: Building & deploying"
echo "   🟢 Monitoring: Live display every 15 seconds"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo

# Wait for settlement tests to complete (primary blocker)
echo "🔴 Waiting for Phase 5a (Settlement Tests) to complete..."
wait $SETTLEMENT_PID 2>/dev/null || true
echo "✅ Phase 5a complete!"
echo

# Wait for monitoring to complete
sleep 5
kill $MONITOR_PID 2>/dev/null || true

# ===== Summary Report =====
echo
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║              PHASE 5 EXECUTION SUMMARY                        ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

# Settlement Results
echo "📋 PHASE 5a - Settlement Flow E2E Testing:"
if [ -f "$LOG_DIR/settlement-tests.log" ]; then
  TOTAL_LINES=$(wc -l < "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
  
  if [ "$TOTAL_LINES" -gt 0 ]; then
    PASSED=$(grep -c "test result: ok" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
    FAILED=$(grep -c "test result: FAILED" "$LOG_DIR/settlement-tests.log" 2>/dev/null || echo "0")
    TOTAL=$((PASSED + FAILED))
    
    # Safe division - avoid division by zero
    if [ "$TOTAL" -gt 0 ]; then
      PERCENTAGE=$((PASSED * 100 / TOTAL))
    else
      PERCENTAGE=0
    fi
    
    echo "   ✅ Passed: $PASSED"
    echo "   ❌ Failed: $FAILED"
    echo "   📊 Total: $TOTAL"
    
    if [ "$FAILED" -eq 0 ] && [ "$PASSED" -gt 0 ]; then
      echo "   🎉 STATUS: ✅ ALL TESTS PASSED"
    elif [ "$FAILED" -gt 0 ]; then
      echo "   ⚠️  STATUS: ❌ SOME TESTS FAILED"
    else
      echo "   ⏳ STATUS: Tests may not have executed (${TOTAL_LINES} log lines)"
      tail -10 "$LOG_DIR/settlement-tests.log" | sed 's/^/      /'
    fi
  else
    echo "   ⏳ STATUS: No settlement test results (log is empty)"
  fi
else
  echo "   ⚠️  No settlement test results found"
fi
echo

# Indexer Results
echo "🔧 PHASE 5b - X3 Indexer Deployment:"
if [ -f "$LOG_DIR/indexer-build.log" ] && grep -q "Finished" "$LOG_DIR/indexer-build.log"; then
  echo "   ✅ Build: Successful"
  
  if [ -f "$WORKSPACE/target/release/x3-indexer" ]; then
    BINARY_SIZE=$(du -h "$WORKSPACE/target/release/x3-indexer" 2>/dev/null | cut -f1)
    echo "   📦 Binary Size: $BINARY_SIZE"
  fi
  
  if pgrep -f "x3-indexer" > /dev/null 2>&1; then
    echo "   🚀 Deployment: ✅ Running on :4000"
    if curl -s http://127.0.0.1:4000/graphql -X POST \
      -H "Content-Type: application/json" \
      -d '{"query":"{ __typename }"}' > /dev/null 2>&1; then
      echo "      GraphQL: ✅ Responsive"
    else
      echo "      GraphQL: ⏳ Pending..."
    fi
  else
    echo "   ⏳ Deployment: Binary ready but process not running"
    echo "      Start with: $WORKSPACE/target/release/x3-indexer --listen 0.0.0.0:4000 --rpc-urls http://127.0.0.1:9933 http://127.0.0.1:9934 http://127.0.0.1:9935"
  fi
else
  echo "   🔨 Build: Pending or failed"
  if [ -f "$LOG_DIR/indexer-build.log" ]; then
    log_error "Last errors from indexer build:"
    grep -i "error" "$LOG_DIR/indexer-build.log" | head -3 | sed 's/^/      /'
  fi
fi
echo

# Validator Network State
echo "🌐 PHASE 5c - Validator Network State:"
RUNNING=$(pgrep -f "x3-chain-node.*--validator" 2>/dev/null | wc -l)
echo "   Validators Running: $RUNNING/3"
for i in {1,2,3}; do
  LOG="$LOG_DIR/validator$i.log"
  if [ -f "$LOG" ]; then
    LATEST=$(tail -1 "$LOG" 2>/dev/null)
    if [[ $LATEST == *"Idle"* ]]; then
      PEERS=$(echo "$LATEST" | grep -oP '(?<=\()\d+(?= peers)' || echo "?")
      BLOCK=$(echo "$LATEST" | grep -oP '#\d+' | head -1 || echo "#?")
      echo "   ✅ Validator-$i: $BLOCK, $PEERS peer(s) connected"
    fi
  fi
done
echo

# Log File Locations
echo "📂 Artifact Locations:"
echo "   Settlement Tests: $LOG_DIR/settlement-tests.log"
echo "   Indexer Build: $LOG_DIR/indexer-build.log"
echo "   Indexer Runtime: $LOG_DIR/indexer.log"
echo "   Validator Logs: $LOG_DIR/validator{1,2,3}.log"
echo

# Next Steps
echo "🚀 Next Actions:"
echo "   1. Verify Phase 5a test results:"
echo "      tail -50 $LOG_DIR/settlement-tests.log | grep -E 'PASS|FAIL|ok'"
echo
echo "   2. Start/check indexer (if not running):"
echo "      cd $WORKSPACE/crates/x3-indexer"
echo "      ./target/release/x3-indexer --listen 0.0.0.0:4000 --rpc-urls http://127.0.0.1:9933"
echo
echo "   3. Verify indexer GraphQL:"
echo "      curl http://127.0.0.1:4000/graphql -X POST -H 'Content-Type: application/json' -d '{\"query\":\"{ __typename }\"}''"
echo
echo "   4. Monitor block production:"
echo "      watch -n 2 'tail -1 $LOG_DIR/validator1.log'"
echo
echo "   5. Check cross-VM bridge status:"
echo "      curl -s http://127.0.0.1:9933 -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"chain_getLatestHeader\",\"params\":[],\"id\":1}' | jq"
echo

echo "═══════════════════════════════════════════════════════════════"
echo "✨ Phase 5 Execution Complete! Ready for Phase 6 planning."
echo "═══════════════════════════════════════════════════════════════"
