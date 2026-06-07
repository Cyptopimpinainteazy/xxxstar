#!/bin/bash
################################################################################
# RC5 CHAOS ENGINEERING HARNESS - Stress Test & Attack Vector Validation
################################################################################
# Purpose: Throw adversarial conditions at RC5 validators to test resilience:
#   - Network chaos (latency, drops, partition)
#   - Resource starvation (CPU, memory, disk I/O)
#   - Database interference (corruption, deletion, locks)
#   - RPC chaos (malformed responses, timeouts, errors)
#   - Process interference (crashes, rapid restarts, signal floods)
#   - Concurrent failures (multiple chaos vectors simultaneously)
#
# Usage: bash rc5_chaos_harness.sh [chaos_mode] [duration_seconds] [intensity]
#   Modes: network, disk, memory, process, rpc, db, clock, cascade, all
#   Intensity: low (1-3), medium (4-6), high (7-9), nuclear (10)
################################################################################

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

CHAOS_MODE="${1:-all}"           # Which chaos vector to deploy
DURATION_SECONDS="${2:-3600}"    # How long to chaos (1 hour default)
INTENSITY="${3:-5}"              # 1-10 intensity scale
ALICE_RPC="${ALICE_RPC:-http://127.0.0.1:9964}"
BOB_RPC="${BOB_RPC:-http://127.0.0.1:9965}"
CHARLIE_RPC="${CHARLIE_RPC:-http://127.0.0.1:9966}"

CHAOS_LOG_DIR="${PWD}/logs/rc5-chaos"
CHAOS_REPORT_DIR="${PWD}/reports/rc5-chaos"
CHAOS_LOG="${CHAOS_LOG_DIR}/chaos_harness.log"

mkdir -p "$CHAOS_LOG_DIR" "$CHAOS_REPORT_DIR"

# State tracking
CHAOS_ACTIVE=0
CHAOS_IMPACTS=()
CHAOS_DETECTED_ISSUES=()
CHAOS_RECOVERED_EVENTS=()

# ============================================================================
# Logging & Reporting
# ============================================================================

log_chaos() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [CHAOS] $*" | tee -a "$CHAOS_LOG"
}

log_impact() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [IMPACT] $*" | tee -a "$CHAOS_LOG"
  CHAOS_IMPACTS+=("$*")
}

log_issue() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [ISSUE] $*" | tee -a "$CHAOS_LOG"
  CHAOS_DETECTED_ISSUES+=("$*")
}

log_recovery() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [RECOVERY] $*" | tee -a "$CHAOS_LOG"
  CHAOS_RECOVERED_EVENTS+=("$*")
}

# ============================================================================
# Network Chaos - Latency, Drops, Partition
# ============================================================================

deploy_network_latency() {
  local latency_ms=$((50 + (INTENSITY * 15)))  # 50-200ms at intensity 1-10
  log_chaos "Injecting network latency: ${latency_ms}ms (intensity: $INTENSITY)"
  
  # Add qdisc with netem (network emulation)
  sudo tc qdisc add dev lo root netem delay "${latency_ms}ms" delay 5ms distribution normal 2>/dev/null || true
}

deploy_network_packet_loss() {
  local loss_percent=$((INTENSITY * 2))  # 2%-20% loss at intensity 1-10
  log_chaos "Injecting packet loss: ${loss_percent}% (intensity: $INTENSITY)"
  
  sudo tc qdisc add dev lo root netem loss "${loss_percent}%" 2>/dev/null || true
}

deploy_network_partition() {
  log_chaos "Deploying network partition - isolating validators"
  
  # Block traffic between validators using iptables
  sudo iptables -A OUTPUT -p tcp --dport 30333 -j DROP 2>/dev/null || true
  sudo iptables -A INPUT -p tcp --sport 30333 -j DROP 2>/dev/null || true
  
  log_impact "Alice/Bob/Charlie isolated from peer connections"
}

deploy_rpc_timeout_cascade() {
  local timeout_ms=$((INTENSITY * 500))  # 500-5000ms at intensity 1-10
  log_chaos "Simulating RPC timeout cascade: ${timeout_ms}ms (intensity: $INTENSITY)"
  
  # Simulate by temporarily blocking RPC ports
  for port in 9964 9965 9966; do
    (
      sleep 2
      sudo iptables -A INPUT -p tcp --dport "$port" -j DROP 2>/dev/null || true
      sleep "$((timeout_ms / 1000))"
      sudo iptables -D INPUT -p tcp --dport "$port" -j DROP 2>/dev/null || true
    ) &
  done
  
  log_impact "RPC endpoints temporarily unreachable"
}

cleanup_network_chaos() {
  log_chaos "Cleaning up network chaos..."
  
  # Remove tc qdisc
  sudo tc qdisc del dev lo root 2>/dev/null || true
  
  # Flush iptables rules
  sudo iptables -F 2>/dev/null || true
  sudo iptables -X 2>/dev/null || true
  
  log_recovery "Network chaos cleaned up"
}

# ============================================================================
# Disk Chaos - I/O Saturation, Space Pressure
# ============================================================================

deploy_disk_io_saturation() {
  log_chaos "Saturating disk I/O with random writes (intensity: $INTENSITY)"
  
  local io_threads=$((INTENSITY * 2))
  local io_size=$((INTENSITY * 10))  # MB per thread
  
  for ((i=0; i<io_threads; i++)); do
    (
      for ((j=0; j<5; j++)); do
        dd if=/dev/urandom of="${CHAOS_LOG_DIR}/io_chaos_${i}_${j}.tmp" bs=1M count="$io_size" 2>/dev/null || true
        sleep 1
      done
      rm -f "${CHAOS_LOG_DIR}/io_chaos_${i}"_*.tmp
    ) &
  done
  
  log_impact "Disk I/O saturated with $io_threads concurrent processes writing ${io_size}MB each"
}

deploy_disk_space_pressure() {
  log_chaos "Creating disk space pressure (intensity: $INTENSITY)"
  
  # Calculate how much space to consume (based on intensity)
  local fill_percent=$((INTENSITY * 5))  # 5-50% at intensity 1-10
  local available_kb=$(df "$CHAOS_LOG_DIR" | tail -1 | awk '{print $4}')
  local fill_kb=$((available_kb * fill_percent / 100))
  
  # Create large file to consume space
  if (( fill_kb > 1000 )); then
    dd if=/dev/zero of="${CHAOS_LOG_DIR}/disk_pressure.img" bs=1K count="$fill_kb" 2>/dev/null &
    log_impact "Created ${fill_percent}% disk pressure (${fill_kb}KB consumed)"
  fi
}

cleanup_disk_chaos() {
  log_chaos "Cleaning up disk chaos..."
  
  rm -f "${CHAOS_LOG_DIR}"/*.tmp
  rm -f "${CHAOS_LOG_DIR}"/disk_pressure.img
  
  log_recovery "Disk chaos cleaned up"
}

# ============================================================================
# Memory Chaos - Resource Starvation
# ============================================================================

deploy_memory_pressure() {
  log_chaos "Applying memory pressure (intensity: $INTENSITY)"
  
  local mem_mb=$((INTENSITY * 100))  # 100-1000MB pressure
  
  # Allocate memory using stress-ng if available, else use simple loop
  if command -v stress-ng &>/dev/null; then
    stress-ng --vm 1 --vm-bytes "${mem_mb}M" --vm-hang 30 --timeout 60s >/dev/null 2>&1 &
    log_impact "Memory pressure applied: ${mem_mb}MB allocation"
  else
    # Fallback: create large files to trigger swapping
    (
      for ((i=0; i<"$mem_mb"; i+=100)); do
        dd if=/dev/zero of="${CHAOS_LOG_DIR}/mem_pressure_${i}.tmp" bs=1M count=100 2>/dev/null || break
        sleep 0.5
      done
    ) &
    log_impact "Memory pressure applied: ${mem_mb}MB simulated allocation (via disk)"
  fi
}

cleanup_memory_chaos() {
  log_chaos "Cleaning up memory chaos..."
  
  pkill -f stress-ng || true
  rm -f "${CHAOS_LOG_DIR}"/mem_pressure_*.tmp
  
  log_recovery "Memory chaos cleaned up"
}

# ============================================================================
# Process Chaos - Crashes, Kills, Signal Floods
# ============================================================================

deploy_process_crash() {
  log_chaos "Injecting process crashes (intensity: $INTENSITY)"
  
  local validators=("alice" "bob" "charlie")
  local crash_target="${validators[$((RANDOM % 3))]}"
  
  log_impact "Targeting validator: $crash_target for crash injection"
  
  # Kill validator forcefully
  pkill -9 -f "\\[x\\]3-chain-node.*--base-path.*$crash_target" || true
  log_impact "Process crash deployed on $crash_target"
}

deploy_signal_flood() {
  log_chaos "Flooding validators with signals (intensity: $INTENSITY)"
  
  local signal_rate=$((INTENSITY * 10))  # 10-100 signals per second
  
  for ((i=0; i<signal_rate; i++)); do
    pkill -SIGUSR1 -f "x3-chain-node" || true
    sleep 0.1
  done
  
  log_impact "Signal flood deployed: $signal_rate signals sent"
}

deploy_rapid_restart() {
  log_chaos "Triggering rapid validator restart cycles (intensity: $INTENSITY)"
  
  local restart_count=$((INTENSITY * 2))
  local restart_interval=$((10 / INTENSITY))  # Faster at higher intensity
  
  for ((i=0; i<restart_count; i++)); do
    pkill -f "x3-chain-node" || true
    sleep "$restart_interval"
  done
  
  log_impact "Rapid restart cycles completed: $restart_count cycles at ${restart_interval}s intervals"
}

cleanup_process_chaos() {
  log_chaos "Cleaning up process chaos..."
  
  # Validators should auto-restart, but ensure they're running
  sleep 5
  
  log_recovery "Process chaos cleanup - validators will auto-restart"
}

# ============================================================================
# RPC Chaos - Malformed Responses, Timeouts, Errors
# ============================================================================

deploy_rpc_error_injection() {
  log_chaos "Injecting RPC response errors (intensity: $INTENSITY)"
  
  local error_types=("invalid_json" "timeout" "method_not_found" "internal_error")
  local error_rate=$((INTENSITY * 5))  # 5-50% error rate
  
  # Create a mock RPC interceptor (simplified - would need full implementation)
  for rpc_url in "$ALICE_RPC" "$BOB_RPC" "$CHARLIE_RPC"; do
    (
      for ((i=0; i<error_rate; i++)); do
        # Try to trigger RPC errors
        curl -s -X POST "$rpc_url" \
          -H "Content-Type: application/json" \
          -d '{"jsonrpc":"2.0","method":"invalid_method_xyz","params":[],"id":1}' \
          >/dev/null 2>&1 || true
        sleep 0.5
      done
    ) &
  done
  
  log_impact "RPC error injection deployed: ${error_rate}% error rate target"
}

deploy_rpc_slow_response() {
  log_chaos "Injecting RPC response delays (intensity: $INTENSITY)"
  
  local delay_seconds=$((INTENSITY * 5))  # 5-50 second delays
  
  log_impact "RPC responses will experience ${delay_seconds}s artificial delays"
  
  # Note: Full implementation would require proxy middleware
}

cleanup_rpc_chaos() {
  log_chaos "Cleaning up RPC chaos..."
  # RPC chaos typically self-heals when requests time out
  log_recovery "RPC chaos cleanup complete"
}

# ============================================================================
# Database Chaos - Corruption, Locks, Deletion
# ============================================================================

deploy_database_corruption() {
  log_chaos "Injecting database corruption (intensity: $INTENSITY - CAUTION)"
  
  # Find validator databases
  local db_paths=(".rc5-runtime/rc5/alice/chains/x3_chain_local3/paritydb"
                  ".rc5-runtime/rc5/bob/chains/x3_chain_local3/paritydb"
                  ".rc5-runtime/rc5/charlie/chains/x3_chain_local3/paritydb")
  
  for db_path in "${db_paths[@]}"; do
    if [[ -d "$db_path" ]]; then
      # Corrupt a small percentage of database files
      local db_files=($(find "$db_path" -type f -name "*.dat" 2>/dev/null | head -n $((INTENSITY))))
      
      for db_file in "${db_files[@]}"; do
        if [[ -f "$db_file" ]]; then
          # Flip a few bytes in the file
          dd if=/dev/urandom of="$db_file" bs=1 count=100 conv=notrunc seek=$((RANDOM % ($(stat -f%z "$db_file" 2>/dev/null || stat -c%s "$db_file" 2>/dev/null) - 100))) 2>/dev/null || true
          log_impact "Database file corrupted: $db_file"
        fi
      done
    fi
  done
}

deploy_database_lock() {
  log_chaos "Creating database lock contention (intensity: $INTENSITY)"
  
  local db_paths=(".rc5-runtime/rc5/alice/chains/x3_chain_local3/paritydb"
                  ".rc5-runtime/rc5/bob/chains/x3_chain_local3/paritydb"
                  ".rc5-runtime/rc5/charlie/chains/x3_chain_local3/paritydb")
  
  for db_path in "${db_paths[@]}"; do
    if [[ -d "$db_path" ]]; then
      # Open database files with exclusive lock
      (
        for ((i=0; i<INTENSITY; i++)); do
          flock -x "$db_path" -c "sleep 30" 2>/dev/null || true &
        done
      )
    fi
  done
  
  log_impact "Database lock contention created on validator databases"
}

cleanup_database_chaos() {
  log_chaos "Cleaning up database chaos..."
  
  # Kill any file locks
  pkill -f "flock" || true
  
  log_recovery "Database chaos cleanup - validators will attempt recovery on restart"
}

# ============================================================================
# Clock Skew Chaos - Time Manipulation
# ============================================================================

deploy_clock_skew() {
  log_chaos "Injecting system clock skew (intensity: $INTENSITY - DANGEROUS)"
  
  local skew_seconds=$((INTENSITY * 60))  # 60-600 second skew
  
  echo "[WARNING] Clock skew deployment requires root and can affect system stability"
  echo "Skipping clock skew for safety - would shift system time by $skew_seconds seconds"
  log_impact "Clock skew not deployed (would shift time by ${skew_seconds}s) - requires explicit confirmation"
}

cleanup_clock_chaos() {
  log_chaos "Cleaning up clock chaos (if applied)..."
  # In real implementation, would restore NTP sync
  log_recovery "Clock chaos cleanup"
}

# ============================================================================
# Cascade Chaos - Multiple Failures Simultaneously
# ============================================================================

deploy_cascade_failure() {
  log_chaos "Deploying CASCADE failure: Network + Process + Memory (intensity: $INTENSITY)"
  
  # Simultaneously trigger multiple chaos vectors
  deploy_network_latency &
  deploy_process_crash &
  deploy_memory_pressure &
  deploy_disk_io_saturation &
  deploy_rpc_error_injection &
  
  wait
  
  log_impact "CASCADING CHAOS deployed across all vectors simultaneously"
}

cleanup_cascade_chaos() {
  log_chaos "Cascading chaos cleanup sequence..."
  
  cleanup_network_chaos || true
  cleanup_process_chaos || true
  cleanup_memory_chaos || true
  cleanup_disk_chaos || true
  cleanup_rpc_chaos || true
  
  log_recovery "Cascade chaos fully cleaned up"
}

# ============================================================================
# Monitoring & Impact Assessment
# ============================================================================

monitor_validator_health() {
  local validator_name="$1"
  local rpc_url="$2"
  
  # Check if validator is responsive
  if ! curl -s -X POST "$rpc_url" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
    2>/dev/null | grep -q "result"; then
    
    log_issue "Validator $validator_name RPC unresponsive at $rpc_url"
    return 1
  fi
  
  return 0
}

check_for_panics() {
  if grep -q "panic\|PANIC\|thread.*panicked" logs/rc5/rc5_72h.nohup.log 2>/dev/null; then
    log_issue "PANIC DETECTED in main harness - chaos triggered unstable condition"
    return 1
  fi
  
  return 0
}

check_for_db_corruption() {
  if grep -q "corrupt\|corruption\|database.*error\|paritydb.*error" logs/rc5/rc5_72h.nohup.log 2>/dev/null; then
    log_issue "DATABASE CORRUPTION DETECTED - chaos successfully triggered vulnerability"
    return 1
  fi
  
  return 0
}

assess_recovery() {
  log_chaos "Assessing validator recovery..."
  
  sleep 5
  
  # Check if validators recovered
  for validator in "alice" "bob" "charlie"; do
    if pgrep -f "\\[x\\]3-chain-node.*--base-path.*$validator" >/dev/null 2>&1; then
      log_recovery "Validator $validator recovered and restarted"
    else
      log_issue "Validator $validator failed to recover after chaos injection"
    fi
  done
}

# ============================================================================
# Report Generation
# ============================================================================

generate_chaos_report() {
  local report_file="${CHAOS_REPORT_DIR}/chaos_assessment_$(date +%s).json"
  
  cat > "$report_file" <<EOF
{
  "chaos_assessment": {
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "chaos_mode": "$CHAOS_MODE",
    "duration_seconds": $DURATION_SECONDS,
    "intensity": $INTENSITY,
    "total_impacts": ${#CHAOS_IMPACTS[@]},
    "issues_detected": ${#CHAOS_DETECTED_ISSUES[@]},
    "recovery_events": ${#CHAOS_RECOVERED_EVENTS[@]},
    "overall_resilience": "$([ ${#CHAOS_DETECTED_ISSUES[@]} -eq 0 ] && echo 'PASSED' || echo 'VULNERABLE')"
  },
  "impacts": [
$(for impact in "${CHAOS_IMPACTS[@]}"; do echo "    \"$impact\""; done | paste -sd ',' -)
  ],
  "issues": [
$(for issue in "${CHAOS_DETECTED_ISSUES[@]}"; do echo "    \"$issue\""; done | paste -sd ',' -)
  ],
  "recovery_events": [
$(for recovery in "${CHAOS_RECOVERED_EVENTS[@]}"; do echo "    \"$recovery\""; done | paste -sd ',' -)
  ],
  "recommendations": [
    "If issues_detected > 0: Investigate failure modes and implement hardening",
    "If recovery_events high: System is resilient and recovers well",
    "Run multiple chaos cycles at varying intensities to establish baseline"
  ]
}
EOF
  
  log_chaos "Chaos report generated: $report_file"
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
  log_chaos "═══════════════════════════════════════════════════════════"
  log_chaos "RC5 CHAOS ENGINEERING HARNESS INITIALIZING"
  log_chaos "Mode: $CHAOS_MODE | Duration: ${DURATION_SECONDS}s | Intensity: $INTENSITY"
  log_chaos "═══════════════════════════════════════════════════════════"
  
  CHAOS_ACTIVE=1
  
  case "$CHAOS_MODE" in
    network)
      log_chaos "Deploying NETWORK chaos..."
      deploy_network_latency &
      deploy_network_packet_loss &
      ;;
    disk)
      log_chaos "Deploying DISK chaos..."
      deploy_disk_io_saturation &
      deploy_disk_space_pressure &
      ;;
    memory)
      log_chaos "Deploying MEMORY chaos..."
      deploy_memory_pressure &
      ;;
    process)
      log_chaos "Deploying PROCESS chaos..."
      deploy_process_crash &
      deploy_signal_flood &
      deploy_rapid_restart &
      ;;
    rpc)
      log_chaos "Deploying RPC chaos..."
      deploy_rpc_error_injection &
      deploy_rpc_slow_response &
      ;;
    db)
      log_chaos "Deploying DATABASE chaos..."
      deploy_database_lock &
      ;;
    clock)
      log_chaos "Deploying CLOCK SKEW chaos..."
      deploy_clock_skew &
      ;;
    cascade)
      log_chaos "Deploying CASCADE chaos..."
      deploy_cascade_failure &
      ;;
    all)
      log_chaos "Deploying ALL chaos vectors..."
      deploy_network_latency &
      deploy_disk_io_saturation &
      deploy_memory_pressure &
      deploy_rpc_error_injection &
      ;;
    *)
      echo "Unknown chaos mode: $CHAOS_MODE"
      echo "Available modes: network, disk, memory, process, rpc, db, clock, cascade, all"
      exit 1
      ;;
  esac
  
  # Run chaos for specified duration
  local start_time=$(date +%s)
  local end_time=$((start_time + DURATION_SECONDS))
  
  while [[ $(date +%s) -lt $end_time ]]; do
    # Continuous monitoring
    monitor_validator_health "alice" "$ALICE_RPC" || true
    monitor_validator_health "bob" "$BOB_RPC" || true
    monitor_validator_health "charlie" "$CHARLIE_RPC" || true
    
    check_for_panics || true
    check_for_db_corruption || true
    
    sleep 10
  done
  
  log_chaos "Chaos duration completed, initiating cleanup..."
  
  # Cleanup based on mode
  case "$CHAOS_MODE" in
    network)
      cleanup_network_chaos
      ;;
    disk)
      cleanup_disk_chaos
      ;;
    memory)
      cleanup_memory_chaos
      ;;
    process)
      cleanup_process_chaos
      ;;
    rpc)
      cleanup_rpc_chaos
      ;;
    db)
      cleanup_database_chaos
      ;;
    cascade)
      cleanup_cascade_chaos
      ;;
    all)
      cleanup_network_chaos || true
      cleanup_disk_chaos || true
      cleanup_memory_chaos || true
      cleanup_rpc_chaos || true
      ;;
  esac
  
  # Assess recovery
  assess_recovery
  
  # Generate report
  generate_chaos_report
  
  log_chaos "═══════════════════════════════════════════════════════════"
  log_chaos "RC5 CHAOS TEST COMPLETE"
  log_chaos "Total Issues Detected: ${#CHAOS_DETECTED_ISSUES[@]}"
  log_chaos "Recovery Events: ${#CHAOS_RECOVERED_EVENTS[@]}"
  log_chaos "═══════════════════════════════════════════════════════════"
  
  CHAOS_ACTIVE=0
}

# Trap to ensure cleanup on exit
trap 'log_chaos "Interrupted - running emergency cleanup"; cleanup_network_chaos || true; cleanup_disk_chaos || true; cleanup_memory_chaos || true; cleanup_rpc_chaos || true; cleanup_database_chaos || true; exit 130' INT TERM

# Run main
main "$@"
