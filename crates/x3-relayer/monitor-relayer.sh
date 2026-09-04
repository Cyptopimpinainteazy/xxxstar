#!/bin/bash
# X3 Bridge Relayer - Testnet Monitoring Script
# Monitors relay loop health and key metrics in real-time
# 
# Usage: ./monitor-relayer.sh [log_file]
# Example: ./monitor-relayer.sh relayer.log

set -euo pipefail

LOG_FILE="${1:-relayer.log}"
UPDATE_INTERVAL=5  # Seconds between updates
CHECK_COUNT=0
STARTUP_GRACE=30   # Seconds to wait before alerting on zero metrics

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Metrics tracking
PREV_BLOCKS_POLLED=0
PREV_BLOCKS_FINALIZED=0
PREV_PROOFS_SUBMITTED=0
PREV_PROOFS_FAILED=0

alert() { echo -e "${RED}⚠ ALERT:${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
info() { echo -e "${BLUE}ℹ${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }

extract_metric() {
    local metric_name="$1"
    local log_data="$2"
    
    # Extract metric value from log (format: metric_name=value)
    echo "$log_data" | grep -oP "${metric_name}=\K[0-9]+" | tail -1 || echo "0"
}

extract_status() {
    local log_data="$1"
    
    # Check for shutdown signal
    if echo "$log_data" | grep -q "Shutdown signal received"; then
        echo "SHUTDOWN"
        return
    fi
    
    # Check for critical errors
    if echo "$log_data" | grep -q "ERROR"; then
        echo "ERROR"
        return
    fi
    
    # Check for pause status
    if echo "$log_data" | grep -q "Paused"; then
        echo "PAUSED"
        return
    fi
    
    # Check if actively polling
    if echo "$log_data" | grep -q "Polling"; then
        echo "ACTIVE"
        return
    fi
    
    echo "UNKNOWN"
}

check_log_exists() {
    if [[ ! -f "${LOG_FILE}" ]]; then
        alert "Log file not found: ${LOG_FILE}"
        echo "Start the relayer with: ./deploy-testnet.sh <infura_key>"
        exit 1
    fi
}

get_uptime() {
    if [[ ! -f "${LOG_FILE}" ]]; then
        echo "0"
        return
    fi
    
    local first_line_time=$(head -1 "${LOG_FILE}" | grep -oP '\d{2}:\d{2}:\d{2}' | head -1)
    if [[ -z "${first_line_time}" ]]; then
        echo "0"
        return
    fi
    
    local first_seconds=$(date -d "$first_line_time" +%s 2>/dev/null || echo 0)
    local now_seconds=$(date +%s)
    
    if [[ $first_seconds -gt 0 ]]; then
        echo $((now_seconds - first_seconds))
    else
        echo "0"
    fi
}

format_duration() {
    local seconds=$1
    local hours=$((seconds / 3600))
    local minutes=$(((seconds % 3600) / 60))
    local secs=$((seconds % 60))
    
    printf "%02d:%02d:%02d" $hours $minutes $secs
}

display_header() {
    clear
    echo -e "${CYAN}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║  X3 Bridge Relayer - Testnet Monitoring                       ║"
    echo "║  Real-time Health Check                                       ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

display_metrics() {
    local log_data="$1"
    local uptime="$2"
    
    # Extract current metrics
    local blocks_polled=$(extract_metric "blocks_polled" "$log_data")
    local blocks_finalized=$(extract_metric "blocks_finalized" "$log_data")
    local proofs_submitted=$(extract_metric "proofs_submitted" "$log_data")
    local proofs_failed=$(extract_metric "proofs_failed" "$log_data")
    local poll_failures=$(extract_metric "poll_failures" "$log_data")
    local pause_events=$(extract_metric "pause_events" "$log_data")
    
    # Calculate deltas
    local blocks_polled_delta=$((blocks_polled - PREV_BLOCKS_POLLED))
    local blocks_finalized_delta=$((blocks_finalized - PREV_BLOCKS_FINALIZED))
    local proofs_submitted_delta=$((proofs_submitted - PREV_PROOFS_SUBMITTED))
    local proofs_failed_delta=$((proofs_failed - PREV_PROOFS_FAILED))
    
    # Update previous values
    PREV_BLOCKS_POLLED=$blocks_polled
    PREV_BLOCKS_FINALIZED=$blocks_finalized
    PREV_PROOFS_SUBMITTED=$proofs_submitted
    PREV_PROOFS_FAILED=$proofs_failed
    
    # Get status
    local status=$(extract_status "$log_data")
    
    # Display header
    local uptime_formatted=$(format_duration "$uptime")
    local status_color="${GREEN}"
    [[ "$status" == "PAUSED" ]] && status_color="${YELLOW}"
    [[ "$status" == "ERROR" ]] && status_color="${RED}"
    [[ "$status" == "SHUTDOWN" ]] && status_color="${YELLOW}"
    
    echo -e "Status: ${status_color}${status}${NC} | Uptime: ${uptime_formatted} | Last Update: $(date '+%H:%M:%S')"
    echo ""
    
    # Core metrics
    echo -e "${CYAN}── Core Metrics ──${NC}"
    printf "  Blocks Polled:          %5d (Δ %+d)\n" "$blocks_polled" "$blocks_polled_delta"
    printf "  Blocks Finalized:       %5d (Δ %+d)\n" "$blocks_finalized" "$blocks_finalized_delta"
    printf "  Proofs Submitted:       %5d (Δ %+d)\n" "$proofs_submitted" "$proofs_submitted_delta"
    printf "  Proofs Failed:          %5d (Δ %+d)\n" "$proofs_failed" "$proofs_failed_delta"
    echo ""
    
    # Error metrics
    echo -e "${CYAN}── Error Metrics ──${NC}"
    printf "  Poll Failures:          %5d\n" "$poll_failures"
    printf "  Pause Events:           %5d\n" "$pause_events"
    echo ""
    
    # Health checks
    echo -e "${CYAN}── Health Checks ──${NC}"
    
    # Check 1: Is relayer polling?
    if [[ $blocks_polled -gt 0 ]] && [[ $blocks_polled_delta -ge 0 ]]; then
        success "Relayer is polling headers"
    else
        alert "Relayer not polling or metrics not updating"
    fi
    
    # Check 2: Are blocks being finalized?
    if [[ $uptime -gt $STARTUP_GRACE ]]; then
        if [[ $blocks_finalized -gt 0 ]]; then
            success "Blocks are reaching finality"
        else
            warn "No finalized blocks yet (normal in first 10 min)"
        fi
    fi
    
    # Check 3: Are proofs being submitted?
    if [[ $uptime -gt $((STARTUP_GRACE * 2)) ]]; then
        if [[ $proofs_submitted -gt 0 ]]; then
            success "Proofs are being submitted"
        else
            warn "No proofs submitted yet (check finality status)"
        fi
    fi
    
    # Check 4: Error rate
    local error_rate=0
    if [[ $blocks_polled -gt 0 ]]; then
        error_rate=$((poll_failures * 100 / (blocks_polled + 1)))
    fi
    
    if [[ $error_rate -lt 5 ]]; then
        success "Error rate is healthy (< 5%)"
    elif [[ $error_rate -lt 20 ]]; then
        warn "Error rate elevated ($error_rate%)"
    else
        alert "High error rate ($error_rate%)"
    fi
    
    # Check 5: Is relayer paused?
    if [[ "$status" == "PAUSED" ]]; then
        warn "Relayer is paused by governance"
    elif [[ "$status" == "ERROR" ]]; then
        alert "Relayer encountered an error"
    elif [[ "$status" == "SHUTDOWN" ]]; then
        warn "Relayer is shutting down"
    fi
}

display_latest_logs() {
    echo -e "${CYAN}── Recent Logs (Last 8 lines) ──${NC}"
    tail -8 "${LOG_FILE}" | sed 's/^/  /'
    echo ""
}

display_commands() {
    echo -e "${CYAN}── Quick Commands ──${NC}"
    echo "  View full logs:     tail -f ${LOG_FILE}"
    echo "  Filter errors:      grep -i error ${LOG_FILE} | tail -20"
    echo "  Filter submissions: grep -i submitted ${LOG_FILE} | tail -20"
    echo "  Stop relayer:       Ctrl+C in relayer terminal"
    echo ""
}

main() {
    check_log_exists
    
    local iteration=0
    while true; do
        display_header
        
        # Get current metrics
        local log_data=$(tail -100 "${LOG_FILE}" 2>/dev/null || echo "")
        local uptime=$(get_uptime)
        
        # Display metrics
        display_metrics "$log_data" "$uptime"
        display_latest_logs
        display_commands
        
        # Wait before next update
        echo -e "${BLUE}Refreshing in ${UPDATE_INTERVAL}s... (Ctrl+C to stop)${NC}"
        sleep "$UPDATE_INTERVAL"
        
        ((iteration++))
    done
}

# Handle Ctrl+C gracefully
trap 'echo ""; info "Monitor stopped"; exit 0' INT

# Run monitoring
main
