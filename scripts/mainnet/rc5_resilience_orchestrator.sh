#!/bin/bash
################################################################################
# RC5 RESILIENCE TESTING ORCHESTRATOR - Master Chaos & Attack Coordinator
################################################################################
# Purpose: Coordinate comprehensive resilience testing of RC5 validators:
#   - Run chaos harness at varying intensity levels
#   - Execute targeted attack vectors
#   - Monitor validator recovery
#   - Generate comprehensive resilience report
#   - Identify failure modes and edge cases
#
# Usage: bash rc5_resilience_orchestrator.sh [scenario]
#   Scenarios: light, medium, heavy, extreme, full
################################################################################

set -euo pipefail

SCENARIO="${1:-medium}"
WORKSPACE="${PWD}"
CHAOS_SCRIPT="${WORKSPACE}/scripts/mainnet/rc5_chaos_harness.sh"
ATTACK_SCRIPT="${WORKSPACE}/scripts/mainnet/rc5_attack_vectors.sh"

ORCHESTRATOR_LOG_DIR="${WORKSPACE}/logs/rc5-orchestrator"
ORCHESTRATOR_REPORT_DIR="${WORKSPACE}/reports/rc5-orchestrator"

mkdir -p "$ORCHESTRATOR_LOG_DIR" "$ORCHESTRATOR_REPORT_DIR"

ORCHESTRATOR_LOG="${ORCHESTRATOR_LOG_DIR}/orchestrator.log"
START_TIME=$(date +%s)

# ============================================================================
# Logging
# ============================================================================

log_phase() {
  echo ""
  echo "╔═══════════════════════════════════════════════════════════════╗"
  echo "║  $1"
  echo "╚═══════════════════════════════════════════════════════════════╝"
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [PHASE] $1" | tee -a "$ORCHESTRATOR_LOG"
}

log_test() {
  echo "  → $1"
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [TEST] $1" | tee -a "$ORCHESTRATOR_LOG"
}

log_result() {
  echo "    ✓ $1"
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [RESULT] $1" | tee -a "$ORCHESTRATOR_LOG"
}

log_issue() {
  echo "    ✗ $1"
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [ISSUE] $1" | tee -a "$ORCHESTRATOR_LOG"
}

# ============================================================================
# Test Scenarios
# ============================================================================

# LIGHT: Low-intensity, safe tests
run_scenario_light() {
  log_phase "LIGHT SCENARIO - Low-intensity resilience verification"
  
  log_test "Network latency injection (50ms, 5% packet loss)"
  bash "$CHAOS_SCRIPT" network 300 1
  
  log_test "Disk I/O pressure (background writes)"
  bash "$CHAOS_SCRIPT" disk 300 1
  
  log_test "Memory pressure (safe allocation)"
  bash "$CHAOS_SCRIPT" memory 300 1
  
  log_test "Execute targeted attack vectors"
  bash "$ATTACK_SCRIPT"
}

# MEDIUM: Moderate chaos
run_scenario_medium() {
  log_phase "MEDIUM SCENARIO - Moderate chaos injection"
  
  log_test "Network chaos (latency + packet loss)"
  bash "$CHAOS_SCRIPT" network 600 3
  
  log_test "Process interference (crashes + rapid restarts)"
  bash "$CHAOS_SCRIPT" process 600 2
  
  log_test "RPC error injection"
  bash "$CHAOS_SCRIPT" rpc 600 2
  
  log_test "Execute attack vectors"
  bash "$ATTACK_SCRIPT"
  
  log_test "Verify recovery after moderate chaos"
  check_validator_health
}

# HEAVY: Aggressive stress testing
run_scenario_heavy() {
  log_phase "HEAVY SCENARIO - Aggressive adversarial conditions"
  
  log_test "Cascading chaos (network + process + memory + disk)"
  bash "$CHAOS_SCRIPT" cascade 900 6
  
  log_test "Rapid process restart cascade"
  bash "$CHAOS_SCRIPT" process 900 5
  
  log_test "Database lock contention"
  bash "$CHAOS_SCRIPT" db 600 4
  
  log_test "Comprehensive attack vector suite"
  bash "$ATTACK_SCRIPT"
  
  log_test "Verify recovery after heavy chaos"
  check_validator_health
}

# EXTREME: Maximum stress
run_scenario_extreme() {
  log_phase "EXTREME SCENARIO - Maximum adversarial conditions"
  
  echo "⚠️  WARNING: Extreme scenario will apply maximum stress"
  echo "The system may become unresponsive. Continue? (yes/no)"
  read -r response
  
  if [[ "$response" != "yes" ]]; then
    log_result "Extreme scenario cancelled by user"
    return
  fi
  
  log_test "All chaos vectors simultaneously (intensity 10)"
  bash "$CHAOS_SCRIPT" all 1200 10
  
  log_test "Rapid process crash cascade (5 crash cycles)"
  bash "$CHAOS_SCRIPT" process 1200 9
  
  log_test "Extreme disk I/O saturation"
  bash "$CHAOS_SCRIPT" disk 1200 9
  
  log_test "Maximum memory pressure"
  bash "$CHAOS_SCRIPT" memory 1200 9
  
  log_test "Execute all attack vectors"
  bash "$ATTACK_SCRIPT"
  
  log_test "Long recovery period (60 seconds)"
  sleep 60
  check_validator_health
}

# FULL: Comprehensive multi-phase testing
run_scenario_full() {
  log_phase "FULL SCENARIO - Comprehensive multi-phase resilience validation"
  
  log_test "Phase 1: Light chaos baseline"
  run_scenario_light
  check_validator_health
  
  log_test "Phase 2: Medium chaos progression"
  run_scenario_medium
  check_validator_health
  
  log_test "Phase 3: Heavy chaos stress"
  run_scenario_heavy
  check_validator_health
  
  log_test "Final recovery verification"
  sleep 30
  check_validator_health
}

# ============================================================================
# Health Checks
# ============================================================================

check_validator_health() {
  log_test "Checking validator health..."
  
  local alice_ok=0
  local bob_ok=0
  local charlie_ok=0
  
  # Check if validators are running
  if pgrep -f "\\[x\\]3-chain-node.*alice" >/dev/null 2>&1; then
    ((alice_ok++))
    log_result "Alice validator: running"
  else
    log_issue "Alice validator: NOT RUNNING"
  fi
  
  if pgrep -f "\\[x\\]3-chain-node.*bob" >/dev/null 2>&1; then
    ((bob_ok++))
    log_result "Bob validator: running"
  else
    log_issue "Bob validator: NOT RUNNING"
  fi
  
  if pgrep -f "\\[x\\]3-chain-node.*charlie" >/dev/null 2>&1; then
    ((charlie_ok++))
    log_result "Charlie validator: running"
  else
    log_issue "Charlie validator: NOT RUNNING"
  fi
  
  # Check RPC responsiveness
  local alice_rpc=$(curl -s -X POST "http://127.0.0.1:9964" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
    2>/dev/null | jq -r '.result.isSyncing' 2>/dev/null || echo "unknown")
  
  if [[ "$alice_rpc" != "unknown" ]]; then
    log_result "Alice RPC: responsive (syncing: $alice_rpc)"
  else
    log_issue "Alice RPC: unresponsive"
  fi
  
  # Check for panics
  if grep -q "panic\|PANIC" logs/rc5/rc5_72h.nohup.log 2>/dev/null; then
    log_issue "PANIC detected in main harness!"
  else
    log_result "No panics detected"
  fi
  
  # Check for database corruption
  if grep -q "corrupt\|rocksdb.*error" logs/rc5/rc5_72h.nohup.log 2>/dev/null; then
    log_issue "Database corruption detected!"
  else
    log_result "Database integrity maintained"
  fi
}

# ============================================================================
# Report Generation
# ============================================================================

generate_resilience_report() {
  local report_file="${ORCHESTRATOR_REPORT_DIR}/resilience_report_$(date +%s).md"
  local end_time=$(date +%s)
  local duration=$((end_time - START_TIME))
  
  cat > "$report_file" <<'EOF'
# RC5 Resilience Testing Report

## Executive Summary

This report documents comprehensive resilience testing of the X3 Atomic Star RC5 validator system through adversarial chaos engineering and targeted attack vector execution.

## Test Scenario

Scenario: [SCENARIO]
Start Time: [START_TIME]
End Time: [END_TIME]
Total Duration: [DURATION] seconds

## Testing Phases

### Phase 1: Network Chaos
- Latency injection (50-200ms)
- Packet loss (2-20%)
- Network partition simulation
- RPC timeout cascades

**Result**: [Check logs/rc5-chaos for details]

### Phase 2: Resource Starvation
- Disk I/O saturation
- Disk space pressure
- Memory allocation pressure
- CPU contention

**Result**: [Check logs/rc5-chaos for details]

### Phase 3: Process Interference
- Validator crash injection
- Signal flood attacks
- Rapid restart cascades
- Process termination tracking

**Result**: [Check logs/rc5-chaos for details]

### Phase 4: Targeted Attack Vectors
- RPC numeric parsing exploits
- Finality race conditions
- Cross-validator divergence
- Settlement state violations
- Database lock deadlocks
- Memory leak detection
- Block production stalls

**Result**: [Check reports/rc5-attacks for details]

## Validator Recovery

| Validator | Status | Recovery Time | RPC Health |
|-----------|--------|---------------|-----------|
| Alice     | [STATUS] | [TIME]s | [RESPONSIVE/UNRESPONSIVE] |
| Bob       | [STATUS] | [TIME]s | [RESPONSIVE/UNRESPONSIVE] |
| Charlie   | [STATUS] | [TIME]s | [RESPONSIVE/UNRESPONSIVE] |

## System Integrity Checks

- ✓/✗ No panics detected
- ✓/✗ Database corruption absent
- ✓/✗ Finality monotonicity maintained
- ✓/✗ Validator consensus maintained
- ✓/✗ Settlement invariants intact
- ✓/✗ Cross-validator state consistent

## Vulnerabilities Identified

[See reports/rc5-attacks/attack_assessment_*.json for details]

## Recommendations

1. **Immediate Actions**
   - [Based on vulnerabilities found]

2. **Hardening Priority**
   - RPC parsing robustness
   - Cascade failure circuit breakers
   - Memory pressure handling
   - Database lock management

3. **Future Testing**
   - Run extreme scenario with monitoring
   - Add network partition simulation
   - Implement chaos monkey scheduled runs
   - Profile memory under sustained load

## Artifacts

- Chaos execution logs: `logs/rc5-chaos/`
- Attack assessment: `reports/rc5-attacks/attack_assessment_*.json`
- Chaos report: `reports/rc5-chaos/chaos_assessment_*.json`
- Orchestrator log: `logs/rc5-orchestrator/orchestrator.log`

## Conclusion

RC5 validator system completed [SCENARIO] scenario resilience testing. [VULNERABILITY_COUNT] vulnerabilities identified. System demonstrated [RECOVERY_QUALITY] recovery capabilities.

---
Generated: [TIMESTAMP]
Scenario: [SCENARIO]
Duration: [DURATION]s
EOF

  # Substitute variables
  sed -i "s|\[SCENARIO\]|$SCENARIO|g" "$report_file"
  sed -i "s|\[START_TIME\]|$(date -d @$START_TIME)|g" "$report_file"
  sed -i "s|\[END_TIME\]|$(date -d @$end_time)|g" "$report_file"
  sed -i "s|\[DURATION\]|$duration|g" "$report_file"
  sed -i "s|\[TIMESTAMP\]|$(date)|g" "$report_file"
  
  echo "$report_file"
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
  log_phase "RC5 RESILIENCE TESTING ORCHESTRATOR - Starting $SCENARIO scenario"
  
  case "$SCENARIO" in
    light)
      run_scenario_light
      ;;
    medium)
      run_scenario_medium
      ;;
    heavy)
      run_scenario_heavy
      ;;
    extreme)
      run_scenario_extreme
      ;;
    full)
      run_scenario_full
      ;;
    *)
      echo "Unknown scenario: $SCENARIO"
      echo "Available scenarios: light, medium, heavy, extreme, full"
      exit 1
      ;;
  esac
  
  log_phase "Generating resilience report..."
  local report=$(generate_resilience_report)
  
  log_phase "RESILIENCE TESTING COMPLETE"
  echo ""
  echo "📊 Reports generated:"
  echo "   - $report"
  echo "   - logs/rc5-chaos/"
  echo "   - logs/rc5-attacks/"
  echo "   - logs/rc5-orchestrator/"
  echo ""
}

# Trap to ensure cleanup
trap 'echo "Interrupted - running emergency cleanup"; exit 130' INT TERM

# Run
main
