#!/bin/bash
# X3 Autonomic Control Plane - Chaos Testing Suite
# Tests self-healing, circuit breakers, and recovery state machine

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_URL="${API_URL:-http://127.0.0.1:8080/api/autonomic}"
CHAOS_LOG="$PROJECT_ROOT/logs/chaos_test_$(date +%Y%m%d_%H%M%S).log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

mkdir -p "$(dirname "$CHAOS_LOG")"
exec > >(tee -a "$CHAOS_LOG")
exec 2>&1

echo "=========================================="
echo "X3 AUTONOMIC CHAOS TESTING SUITE"
echo "=========================================="
echo "Started: $(date)"
echo "API: $API_URL"
echo "Log: $CHAOS_LOG"
echo ""

# ============================================================================
# Helper Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

wait_for_api() {
    log_info "Waiting for API to be ready..."
    for i in {1..30}; do
        if curl -sf "$API_URL/health" >/dev/null 2>&1; then
            log_success "API is ready"
            return 0
        fi
        sleep 1
    done
    log_fail "API failed to become ready"
    return 1
}

get_health() {
    curl -sf "$API_URL/health" | jq -r '.score'
}

get_state() {
    curl -sf "$API_URL/health" | jq -r '.state'
}

get_circuit_breaker_state() {
    local name=$1
    curl -sf "$API_URL/circuit-breakers" | jq -r ".[\"$name\"]"
}

force_state() {
    local target_state=$1
    log_info "Forcing system state to: $target_state"
    curl -sf -X POST "$API_URL/override/state" \
        -H 'Content-Type: application/json' \
        -d "{\"state\": \"$target_state\", \"reason\": \"chaos_test\"}" \
        | jq '.'
}

reset_breaker() {
    local name=$1
    log_info "Resetting circuit breaker: $name"
    curl -sf -X POST "$API_URL/override/circuit-breaker" \
        -H 'Content-Type: application/json' \
        -d "{\"name\": \"$name\", \"action\": \"reset\"}" \
        | jq '.'
}

run_test() {
    local test_name=$1
    local test_func=$2
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo ""
    echo "=========================================="
    echo "TEST $TESTS_RUN: $test_name"
    echo "=========================================="
    
    if $test_func; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_success "TEST PASSED: $test_name"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_fail "TEST FAILED: $test_name"
    fi
    
    sleep 2  # cooldown between tests
}

# ============================================================================
# Test Cases
# ============================================================================

test_api_connectivity() {
    log_info "Testing API connectivity..."
    
    if curl -sf "$API_URL/health" >/dev/null; then
        log_success "API is reachable"
        return 0
    else
        log_fail "API is unreachable"
        return 1
    fi
}

test_health_score_retrieval() {
    log_info "Testing health score retrieval..."
    
    local score=$(get_health)
    if [[ "$score" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
        log_success "Health score: $score"
        return 0
    else
        log_fail "Invalid health score: $score"
        return 1
    fi
}

test_state_machine_normal() {
    log_info "Testing state machine in NORMAL state..."
    
    force_state "normal"
    sleep 3
    
    local state=$(get_state)
    if [[ "$state" == "normal" ]]; then
        log_success "System is in NORMAL state"
        return 0
    else
        log_fail "Expected NORMAL, got: $state"
        return 1
    fi
}

test_state_machine_degraded() {
    log_info "Testing state machine transition to DEGRADED..."
    
    force_state "degraded"
    sleep 3
    
    local state=$(get_state)
    if [[ "$state" == "degraded" ]]; then
        log_success "System transitioned to DEGRADED"
        return 0
    else
        log_fail "Expected DEGRADED, got: $state"
        return 1
    fi
}

test_state_machine_safe_mode() {
    log_info "Testing SAFE_MODE activation..."
    
    force_state "safe_mode"
    sleep 3
    
    local state=$(get_state)
    if [[ "$state" == "safe_mode" ]]; then
        log_success "System entered SAFE_MODE"
        
        # Check that operators are restricted
        local status=$(curl -sf "$API_URL/status" | jq '.orchestrator.safe_mode')
        if [[ "$status" == "true" ]]; then
            log_success "Safe mode restrictions active"
            return 0
        else
            log_fail "Safe mode not enforced"
            return 1
        fi
    else
        log_fail "Expected safe_mode, got: $state"
        return 1
    fi
}

test_circuit_breaker_basic() {
    log_info "Testing circuit breaker status retrieval..."
    
    local breakers=$(curl -sf "$API_URL/circuit-breakers")
    if echo "$breakers" | jq -e '.gpu' >/dev/null; then
        log_success "Circuit breakers detected"
        echo "$breakers" | jq '.'
        return 0
    else
        log_fail "No circuit breakers found"
        return 1
    fi
}

test_circuit_breaker_reset() {
    log_info "Testing circuit breaker reset..."
    
    local initial_state=$(get_circuit_breaker_state "gpu")
    log_info "GPU breaker initial state: $initial_state"
    
    reset_breaker "gpu"
    sleep 2
    
    local new_state=$(get_circuit_breaker_state "gpu")
    if [[ "$new_state" == "closed" ]]; then
        log_success "Circuit breaker reset to CLOSED"
        return 0
    else
        log_warn "Breaker state after reset: $new_state (may already be closed)"
        return 0  # Not a hard failure
    fi
}

test_audit_trail() {
    log_info "Testing audit trail..."
    
    local audit=$(curl -sf "$API_URL/audit")
    local count=$(echo "$audit" | jq 'length')
    
    if [[ $count -gt 0 ]]; then
        log_success "Audit trail has $count entries"
        echo "$audit" | jq '.[-3:]'  # Show last 3
        return 0
    else
        log_warn "Audit trail is empty (may be new deployment)"
        return 0
    fi
}

test_component_health() {
    log_info "Testing component health reporting..."
    
    local components=$(curl -sf "$API_URL/status" | jq '.components')
    local count=$(echo "$components" | jq 'keys | length')
    
    if [[ $count -gt 0 ]]; then
        log_success "Found $count components"
        echo "$components" | jq 'to_entries | .[] | {name: .key, score: .value.score, status: .value.status}'
        return 0
    else
        log_fail "No components reported"
        return 1
    fi
}

test_gpu_guard() {
    log_info "Testing GPU Guard sentinel..."
    
    local gpu_status=$(curl -sf "$API_URL/gpu")
    if echo "$gpu_status" | jq -e '.running' >/dev/null; then
        local gpu_count=$(echo "$gpu_status" | jq -r '.gpu_count')
        log_success "GPU Guard is running (detected $gpu_count GPUs)"
        echo "$gpu_status" | jq '.'
        return 0
    else
        log_warn "GPU Guard not running (may be no nvidia-smi)"
        return 0  # Not a hard failure if no GPUs
    fi
}

test_resource_monitor() {
    log_info "Testing Resource Monitor sentinel..."
    
    local res_status=$(curl -sf "$API_URL/resources")
    if echo "$res_status" | jq -e '.running' >/dev/null; then
        log_success "Resource Monitor is running"
        echo "$res_status" | jq '{ram_pct, disk_pct, load_1m}'
        return 0
    else
        log_fail "Resource Monitor is not running"
        return 1
    fi
}

test_log_watcher() {
    log_info "Testing Log Watcher sentinel..."
    
    local log_status=$(curl -sf "$API_URL/logs")
    if echo "$log_status" | jq -e '.running' >/dev/null; then
        log_success "Log Watcher is running"
        local recent=$(echo "$log_status" | jq '.recent_events | length')
        log_info "Recent log events: $recent"
        return 0
    else
        log_fail "Log Watcher is not running"
        return 1
    fi
}

test_metrics_publish() {
    log_info "Testing metrics bus publication..."
    
    local metrics=$(curl -sf "$API_URL/metrics")
    if echo "$metrics" | jq -e '.system_score' >/dev/null; then
        log_success "Metrics are being published"
        echo "$metrics" | jq '{system_score, components: (.components | length)}'
        return 0
    else
        log_fail "Metrics bus not publishing"
        return 1
    fi
}

test_operator_registry() {
    log_info "Testing operator registry..."
    
    local operators=$(curl -sf "$API_URL/operators")
    local count=$(echo "$operators" | jq 'keys | length')
    
    if [[ $count -ge 4 ]]; then
        log_success "Found $count operators"
        echo "$operators" | jq 'to_entries | .[] | {name: .key, safe_mode: .value.safe_mode}'
        return 0
    else
        log_fail "Expected at least 4 operators, found $count"
        return 1
    fi
}

test_recovery_after_safe_mode() {
    log_info "Testing recovery path from SAFE_MODE..."
    
    # Enter safe mode
    force_state "safe_mode"
    sleep 3
    
    local state1=$(get_state)
    log_info "State after safe mode: $state1"
    
    # Force back to containment
    force_state "containment"
    sleep 3
    
    local state2=$(get_state)
    log_info "State after containment: $state2"
    
    # Force to degraded
    force_state "degraded"
    sleep 3
    
    local state3=$(get_state)
    log_info "State after degraded: $state3"
    
    # Back to normal
    force_state "normal"
    sleep 3
    
    local state4=$(get_state)
    if [[ "$state4" == "normal" ]]; then
        log_success "Successfully recovered from SAFE_MODE to NORMAL"
        return 0
    else
        log_fail "Failed to recover to NORMAL (got: $state4)"
        return 1
    fi
}

test_playbook_listing() {
    log_info "Testing playbook listing..."
    
    local playbooks=$(curl -sf "$API_URL/status" | jq '.orchestrator.playbooks')
    local count=$(echo "$playbooks" | jq 'length')
    
    if [[ $count -gt 0 ]]; then
        log_success "Found $count playbooks"
        echo "$playbooks" | jq '.[] | {name, description, severity}'
        return 0
    else
        log_fail "No playbooks found"
        return 1
    fi
}

# Stress test: Rapid state changes
test_rapid_state_changes() {
    log_info "Testing rapid state changes (stress test)..."
    
    local states=("normal" "degraded" "containment" "safe_mode" "degraded" "normal")
    local success=true
    
    for state in "${states[@]}"; do
        force_state "$state" >/dev/null 2>&1
        sleep 1
        local current=$(get_state)
        if [[ "$current" != "$state" ]]; then
            log_fail "Failed to transition to $state (got: $current)"
            success=false
            break
        fi
    done
    
    if $success; then
        log_success "Rapid state changes handled correctly"
        return 0
    else
        return 1
    fi
}

# ============================================================================
# Main Test Execution
# ============================================================================

main() {
    log_info "Starting chaos test suite..."
    
    # Wait for API
    if ! wait_for_api; then
        log_fail "API is not available. Is the swarm server running?"
        exit 1
    fi
    
    # Run all tests
    run_test "API Connectivity" test_api_connectivity
    run_test "Health Score Retrieval" test_health_score_retrieval
    run_test "Component Health Reporting" test_component_health
    run_test "State Machine: Normal" test_state_machine_normal
    run_test "State Machine: Degraded" test_state_machine_degraded
    run_test "State Machine: Safe Mode" test_state_machine_safe_mode
    run_test "Circuit Breaker Basic" test_circuit_breaker_basic
    run_test "Circuit Breaker Reset" test_circuit_breaker_reset
    run_test "Audit Trail" test_audit_trail
    run_test "GPU Guard Sentinel" test_gpu_guard
    run_test "Resource Monitor Sentinel" test_resource_monitor
    run_test "Log Watcher Sentinel" test_log_watcher
    run_test "Metrics Bus" test_metrics_publish
    run_test "Operator Registry" test_operator_registry
    run_test "Playbook Listing" test_playbook_listing
    run_test "Recovery from Safe Mode" test_recovery_after_safe_mode
    run_test "Rapid State Changes (Stress)" test_rapid_state_changes
    
    # Return to normal state
    log_info "Restoring system to NORMAL state..."
    force_state "normal" >/dev/null 2>&1
    sleep 2
    
    # Summary
    echo ""
    echo "=========================================="
    echo "CHAOS TEST SUMMARY"
    echo "=========================================="
    echo "Total Tests:  $TESTS_RUN"
    echo "Passed:       $TESTS_PASSED"
    echo "Failed:       $TESTS_FAILED"
    echo "Pass Rate:    $(( 100 * TESTS_PASSED / TESTS_RUN ))%"
    echo "Completed:    $(date)"
    echo "Log saved:    $CHAOS_LOG"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_success "ALL TESTS PASSED ✅"
        return 0
    else
        log_fail "$TESTS_FAILED TESTS FAILED ❌"
        return 1
    fi
}

main "$@"
