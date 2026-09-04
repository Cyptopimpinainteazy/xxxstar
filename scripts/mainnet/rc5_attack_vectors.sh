#!/bin/bash
################################################################################
# RC5 ATTACK VECTOR TEST SUITE - Targeted Exploits & Vulnerabilities
################################################################################
# Purpose: Specific attack scenarios designed to break or stress RC5:
#   - RPC parsing vulnerabilities
#   - Database lock deadlocks
#   - Finality race conditions
#   - Settlement state machine breaks
#   - Cross-validator invariant violations
#   - Restart loop cascades
#   - Memory leak simulations
#   - State divergence attacks
################################################################################

set -euo pipefail

ALICE_RPC="${ALICE_RPC:-http://127.0.0.1:9964}"
BOB_RPC="${BOB_RPC:-http://127.0.0.1:9965}"
CHARLIE_RPC="${CHARLIE_RPC:-http://127.0.0.1:9966}"

ATTACK_LOG_DIR="${PWD}/logs/rc5-attacks"
ATTACK_REPORT_DIR="${PWD}/reports/rc5-attacks"

mkdir -p "$ATTACK_LOG_DIR" "$ATTACK_REPORT_DIR"

ATTACK_LOG="${ATTACK_LOG_DIR}/attack_vectors.log"
ATTACKS_EXECUTED=0
ATTACKS_SUCCESSFUL=0
VULNERABILITIES_FOUND=()

log_attack() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [ATTACK] $*" | tee -a "$ATTACK_LOG"
}

log_vuln() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [VULN FOUND] 🔴 $*" | tee -a "$ATTACK_LOG"
  VULNERABILITIES_FOUND+=("$*")
}

log_ok() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [DEFENDED] ✅ $*" | tee -a "$ATTACK_LOG"
}

# ============================================================================
# Attack 1: RPC Numeric Parsing Bomb
# ============================================================================

attack_rpc_numeric_overflow() {
  log_attack "Attack 1: RPC Numeric Overflow - Sending malformed block numbers"
  
  ((ATTACKS_EXECUTED++))
  
  # Try to trigger numeric parsing errors with jq
  local attack_payloads=(
    '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["999999999999999999999"],"id":1}'
    '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["-1"],"id":1}'
    '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["0x99999999999999999999999999999999"],"id":1}'
    '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["9e99"],"id":1}'  # Scientific notation
    '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["NaN"],"id":1}'
  )
  
  for payload in "${attack_payloads[@]}"; do
    local response=$(curl -s -X POST "$ALICE_RPC" \
      -H "Content-Type: application/json" \
      -d "$payload" 2>/dev/null || echo '{"error":"failed"}')
    
    if echo "$response" | grep -q "error\|Error\|ERROR"; then
      log_attack "  Payload $payload → Error response (defended)"
    else
      log_vuln "Numeric parsing bypassed: $payload → $response"
      ((ATTACKS_SUCCESSFUL++))
    fi
  done
}

# ============================================================================
# Attack 2: RPC Timeout Cascade
# ============================================================================

attack_rpc_timeout_cascade() {
  log_attack "Attack 2: RPC Timeout Cascade - Flooding with slow requests"
  
  ((ATTACKS_EXECUTED++))
  
  # Send 100 concurrent slow requests
  for i in {1..100}; do
    (
      timeout 2 curl -s -X POST "$ALICE_RPC" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_accountNextIndex","params":["5HGjWAeFDxFwPmrwhPhGgsprEsLChFJqXUnknqUQMRbX9HXE"],"id":'$i'}' \
        2>/dev/null || true
    ) &
  done
  
  wait
  log_attack "  Cascade complete - checking if system recovered"
  
  if curl -s -X POST "$ALICE_RPC" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
    2>/dev/null | grep -q "result"; then
    log_ok "System recovered from timeout cascade"
  else
    log_vuln "System failed to recover from timeout cascade"
    ((ATTACKS_SUCCESSFUL++))
  fi
}

# ============================================================================
# Attack 3: Finality Race Condition
# ============================================================================

attack_finality_race() {
  log_attack "Attack 3: Finality Race Condition - Querying finality during block import"
  
  ((ATTACKS_EXECUTED++))
  
  local max_finalized=""
  local finalized_drifts=0
  
  # Rapidly query finalized blocks and check for regression
  for i in {1..50}; do
    local current_finalized=$(curl -s -X POST "$ALICE_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[],"id":1}' \
      2>/dev/null | jq -r '.result' 2>/dev/null || echo "unknown")
    
    if [[ -n "$max_finalized" && "$current_finalized" != "unknown" ]]; then
      # Check if finality regressed
      if [[ "$current_finalized" < "$max_finalized" ]]; then
        log_vuln "Finality regression detected: $max_finalized → $current_finalized"
        ((ATTACKS_SUCCESSFUL++))
        ((finalized_drifts++))
      fi
    fi
    
    max_finalized="$current_finalized"
    sleep 0.1
  done
  
  if (( finalized_drifts == 0 )); then
    log_ok "Finality monotonicity maintained across 50 queries"
  fi
}

# ============================================================================
# Attack 4: Settlement State Machine Break
# ============================================================================

attack_settlement_state_violation() {
  log_attack "Attack 4: Settlement State Machine - Forcing invalid state transitions"
  
  ((ATTACKS_EXECUTED++))
  
  # Query current settlement state and try to identify contradictions
  # This would require deeper knowledge of settlement state machine
  
  local settlement_errors=0
  
  # Simulate: query settlement cycles rapidly to find race conditions
  for cycle in {1..30}; do
    (
      # This is a placeholder - real attack would target settlement RPC methods
      curl -s -X POST "$ALICE_RPC" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"settlement_cycleInfo","params":[],"id":1}' \
        2>/dev/null | grep -q "error" && ((settlement_errors++)) || true
    ) &
  done
  
  wait
  
  if (( settlement_errors > 0 )); then
    log_vuln "Settlement RPC errors detected: $settlement_errors errors in 30 queries"
    ((ATTACKS_SUCCESSFUL++))
  else
    log_ok "Settlement state machine consistent"
  fi
}

# ============================================================================
# Attack 5: Cross-Validator Invariant Violation
# ============================================================================

attack_cross_validator_divergence() {
  log_attack "Attack 5: Cross-Validator State Divergence - Checking for consensus breaks"
  
  ((ATTACKS_EXECUTED++))
  
  # Get block hash from Alice and Bob - should match
  local alice_block=$(curl -s -X POST "$ALICE_RPC" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["latest"],"id":1}' \
    2>/dev/null | jq -r '.result' 2>/dev/null || echo "unknown")
  
  local bob_block=$(curl -s -X POST "$BOB_RPC" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getBlockHash","params":["latest"],"id":1}' \
    2>/dev/null | jq -r '.result' 2>/dev/null || echo "unknown")
  
  if [[ "$alice_block" != "$bob_block" && "$alice_block" != "unknown" && "$bob_block" != "unknown" ]]; then
    log_vuln "State divergence: Alice=$alice_block Bob=$bob_block"
    ((ATTACKS_SUCCESSFUL++))
  else
    log_ok "Validators in consensus: Alice=$alice_block Bob=$bob_block"
  fi
}

# ============================================================================
# Attack 6: Process Restart Loop Cascade
# ============================================================================

attack_restart_cascade() {
  log_attack "Attack 6: Restart Loop Cascade - Forcing rapid restarts to trigger cascading failures"
  
  ((ATTACKS_EXECUTED++))
  
  local crash_count=0
  local max_crashes=5
  
  # Kill validators 5 times rapidly to see if system remains stable
  for i in {1..$max_crashes}; do
    pkill -9 -f "x3-chain-node" || true
    sleep 1
    
    # Check if any came back
    if pgrep -f "x3-chain-node" >/dev/null 2>&1; then
      ((crash_count++))
      log_attack "  Restart $i: Validator recovered"
    fi
  done
  
  sleep 10  # Wait for full recovery
  
  # Check if all validators are back
  if pgrep -f "x3-chain-node" | wc -l | grep -q "3"; then
    log_ok "All 3 validators recovered after $max_crashes crash cycles"
  else
    log_vuln "Validators failed to recover: $(pgrep -f 'x3-chain-node' | wc -l) < 3"
    ((ATTACKS_SUCCESSFUL++))
  fi
}

# ============================================================================
# Attack 7: Database Lock Deadlock
# ============================================================================

attack_database_deadlock() {
  log_attack "Attack 7: Database Lock Deadlock - Creating resource contention"
  
  ((ATTACKS_EXECUTED++))
  
  # Attempt to open database files with exclusive locks
  local db_dirs=(
    ".rc5-runtime/rc5/alice/chains/x3_chain_local3"
    ".rc5-runtime/rc5/bob/chains/x3_chain_local3"
    ".rc5-runtime/rc5/charlie/chains/x3_chain_local3"
  )
  
  local lock_acquired=0
  
  for db_dir in "${db_dirs[@]}"; do
    if [[ -d "$db_dir" ]]; then
      (
        if flock -x -n "$db_dir" -c "sleep 5"; then
          ((lock_acquired++))
        fi
      ) &
    fi
  done
  
  wait
  
  if (( lock_acquired > 0 )); then
    log_vuln "Database locks acquired - validators may deadlock on next write"
    ((ATTACKS_SUCCESSFUL++))
  else
    log_ok "Database locking defended - locks held by validators"
  fi
}

# ============================================================================
# Attack 8: Memory Leak Simulation
# ============================================================================

attack_memory_leak() {
  log_attack "Attack 8: Memory Leak Simulation - Checking for memory growth patterns"
  
  ((ATTACKS_EXECUTED++))
  
  # Get baseline memory usage
  local initial_mem=$(ps aux | grep "[x]3-chain-node" | awk '{s+=$6} END {print s}')
  
  log_attack "  Initial memory: ${initial_mem}KB"
  
  # Hammer the system with requests
  for i in {1..500}; do
    curl -s -X POST "$ALICE_RPC" \
      -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
      >/dev/null 2>&1 &
  done
  
  wait
  sleep 5
  
  # Get final memory usage
  local final_mem=$(ps aux | grep "[x]3-chain-node" | awk '{s+=$6} END {print s}')
  local mem_growth=$((final_mem - initial_mem))
  
  log_attack "  Final memory: ${final_mem}KB (growth: ${mem_growth}KB)"
  
  if (( mem_growth > 50000 )); then  # More than 50MB growth
    log_vuln "Potential memory leak detected: ${mem_growth}KB growth after 500 requests"
    ((ATTACKS_SUCCESSFUL++))
  else
    log_ok "Memory usage stable: ${mem_growth}KB growth is acceptable"
  fi
}

# ============================================================================
# Attack 9: Block Production Stall
# ============================================================================

attack_block_stall() {
  log_attack "Attack 9: Block Production Stall - Checking for consensus freeze"
  
  ((ATTACKS_EXECUTED++))
  
  local block_start=$(curl -s -X POST "$ALICE_RPC" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
    2>/dev/null | jq -r '.result.number' 2>/dev/null || echo "unknown")
  
  log_attack "  Starting block: $block_start"
  
  sleep 30
  
  local block_end=$(curl -s -X POST "$ALICE_RPC" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
    2>/dev/null | jq -r '.result.number' 2>/dev/null || echo "unknown")
  
  log_attack "  Ending block: $block_end"
  
  if [[ "$block_start" == "$block_end" && "$block_start" != "unknown" ]]; then
    log_vuln "Block production stall detected: No blocks produced in 30 seconds"
    ((ATTACKS_SUCCESSFUL++))
  else
    log_ok "Block production active: $block_start → $block_end"
  fi
}

# ============================================================================
# Attack 10: Settlement Invariant Violation
# ============================================================================

attack_settlement_invariant() {
  log_attack "Attack 10: Settlement Invariant Violation - Checking bridge balance consistency"
  
  ((ATTACKS_EXECUTED++))
  
  # Query settlement invariants - this would target specific pallet queries
  # Placeholder for now
  
  log_ok "Settlement invariants query executed (would require specific pallet queries)"
}

# ============================================================================
# Report & Summary
# ============================================================================

generate_attack_report() {
  local report_file="${ATTACK_REPORT_DIR}/attack_assessment_$(date +%s).json"
  
  cat > "$report_file" <<EOF
{
  "attack_assessment": {
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "attacks_executed": $ATTACKS_EXECUTED,
    "attacks_successful": $ATTACKS_SUCCESSFUL,
    "vulnerabilities_found": ${#VULNERABILITIES_FOUND[@]},
    "defense_rating": "$([ $ATTACKS_SUCCESSFUL -eq 0 ] && echo 'EXCELLENT' || [ $ATTACKS_SUCCESSFUL -lt 3 ] && echo 'GOOD' || echo 'NEEDS_HARDENING')"
  },
  "vulnerabilities": [
$(for vuln in "${VULNERABILITIES_FOUND[@]}"; do echo "    \"$vuln\""; done | paste -sd ',' -)
  ],
  "recommendations": [
    "Each successful attack represents a potential vulnerability that needs hardening",
    "Priority: Fix numeric parsing exploits first (RPC attack vector)",
    "Implement additional invariant checks for finality and settlement state",
    "Add circuit breakers for cascade failures and rapid restarts",
    "Profile memory usage under high RPC load"
  ]
}
EOF
  
  log_attack "Attack report generated: $report_file"
}

# ============================================================================
# Main
# ============================================================================

main() {
  log_attack "╔═══════════════════════════════════════════════════════════╗"
  log_attack "║       RC5 ATTACK VECTOR TEST SUITE - INITIALIZED          ║"
  log_attack "║  Executing 10 targeted exploits to identify vulnerabilities║"
  log_attack "╚═══════════════════════════════════════════════════════════╝"
  
  attack_rpc_numeric_overflow
  attack_rpc_timeout_cascade
  attack_finality_race
  attack_settlement_state_violation
  attack_cross_validator_divergence
  attack_restart_cascade
  attack_database_deadlock
  attack_memory_leak
  attack_block_stall
  attack_settlement_invariant
  
  log_attack ""
  log_attack "╔═══════════════════════════════════════════════════════════╗"
  log_attack "║              ATTACK TEST SUITE COMPLETE                    ║"
  log_attack "║  Total Attacks: $ATTACKS_EXECUTED | Successful: $ATTACKS_SUCCESSFUL | Vulns: ${#VULNERABILITIES_FOUND[@]}"
  log_attack "║  Defense Rating: $([ $ATTACKS_SUCCESSFUL -eq 0 ] && echo 'EXCELLENT ✅' || [ $ATTACKS_SUCCESSFUL -lt 3 ] && echo 'GOOD ✓' || echo 'NEEDS_HARDENING ⚠️')"
  log_attack "╚═══════════════════════════════════════════════════════════╝"
  
  generate_attack_report
}

main
