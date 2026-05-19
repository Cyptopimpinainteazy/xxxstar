#!/bin/bash
#
# Inferstructor 300× Solana Test - Master Test Harness
#
# This script orchestrates the complete test:
# 1. Starts all services (lanes, orchestrator, bridge, dashboard)
# 2. Runs test phases (baseline, acceleration, failover, etc.)
# 3. Triggers controlled failures
# 4. Collects metrics and generates proof
#
# Usage:
#   ./run_300x_test.sh --duration 8h --export-proof --enable-all-phases

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TPS_TESTING_DIR="$PROJECT_ROOT/TPS TESTING"
RESULTS_DIR="$SCRIPT_DIR/results/$(date +%Y%m%d_%H%M%S)"

# Default settings
DURATION_SECONDS=28800  # 8 hours
TARGET_TPS=19500000     # 300× Solana
EXPORT_PROOF=false
ENABLE_ALL_PHASES=false
PHASE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --duration)
            DURATION_SECONDS=$(echo "$2" | sed 's/h/*3600/;s/m/*60/;s/s//' | bc)
            shift 2
            ;;
        --target-tps)
            TARGET_TPS=$2
            shift 2
            ;;
        --export-proof)
            EXPORT_PROOF=true
            shift
            ;;
        --enable-all-phases)
            ENABLE_ALL_PHASES=true
            shift
            ;;
        --phase)
            PHASE=$2
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

section_header() {
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}  $1${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

wait_for_service() {
    local name=$1
    local url=$2
    local max_attempts=30
    
    log_info "Waiting for $name to be ready..."
    
    for i in $(seq 1 $max_attempts); do
        if curl -sf "$url" > /dev/null 2>&1; then
            log_success "$name is ready!"
            return 0
        fi
        sleep 2
    done
    
    log_error "$name failed to start"
    return 1
}

cleanup() {
    log_info "Cleaning up services..."
    pkill -f "lane_orchestrator.py" || true
    pkill -f "tps_bridge.py" || true
    pkill -f "metrics_dashboard.py" || true
    pkill -f "tps_inferstructor_adapter" || true
}

trap cleanup EXIT

# Create results directory
mkdir -p "$RESULTS_DIR"

section_header "Inferstructor 300× Solana Test"

log_info "Configuration:"
log_info "  Duration: ${DURATION_SECONDS}s ($(echo "$DURATION_SECONDS / 3600" | bc -l | xargs printf "%.1f")h)"
log_info "  Target TPS: $TARGET_TPS"
log_info "  Results: $RESULTS_DIR"

# Check prerequisites
section_header "Checking Prerequisites"

log_info "Checking Python environment..."
if ! python3 -c "import aiohttp, yaml, prometheus_client" 2>/dev/null; then
    log_error "Missing Python dependencies. Run: pip install aiohttp pyyaml prometheus-client"
    exit 1
fi
log_success "Python environment OK"

log_info "Checking Go environment..."
if ! command -v go &> /dev/null; then
    log_error "Go not found. Please install Go."
    exit 1
fi
log_success "Go environment OK"

log_info "Checking GPU availability..."
if ! nvidia-smi &> /dev/null; then
    log_warning "nvidia-smi not found. GPU tests may fail."
else
    log_success "GPU detected"
fi

# Phase 1: Start Services
if [[ -z "$PHASE" || "$PHASE" == "start-services" ]]; then
    section_header "Phase 1: Starting Services"
    
    log_info "Starting Lane Orchestrator..."
    python3 "$SCRIPT_DIR/lane_orchestrator.py" \
        > "$RESULTS_DIR/orchestrator.log" 2>&1 &
    ORCHESTRATOR_PID=$!
    wait_for_service "Orchestrator" "http://localhost:8000/metrics"
    
    log_info "Starting TPS Bridge..."
    python3 "$SCRIPT_DIR/tps_bridge.py" \
        > "$RESULTS_DIR/tps_bridge.log" 2>&1 &
    BRIDGE_PID=$!
    wait_for_service "TPS Bridge" "http://localhost:9999/health"
    
    log_info "Starting Metrics Dashboard..."
    python3 "$SCRIPT_DIR/metrics_dashboard.py" \
        > "$RESULTS_DIR/dashboard.log" 2>&1 &
    DASHBOARD_PID=$!
    wait_for_service "Dashboard" "http://localhost:8080"
    
    log_success "All services started!"
    log_info "Dashboard: http://localhost:8080"
fi

# Phase 2: Baseline Measurement
if [[ "$ENABLE_ALL_PHASES" == "true" || "$PHASE" == "baseline" ]]; then
    section_header "Phase 2: Baseline Measurement"
    
    log_info "This would measure native Solana/Ethereum baseline..."
    log_info "Skipping in test mode - see docs/INFERSTRUCTOR_300X_TEST_PLAN.md"
    
    # Placeholder for baseline measurement
    echo "Solana baseline: 65,000 TPS" > "$RESULTS_DIR/baseline.txt"
fi

# Phase 3: Acceleration Test
if [[ "$ENABLE_ALL_PHASES" == "true" || "$PHASE" == "acceleration" || -z "$PHASE" ]]; then
    section_header "Phase 3: GPU Acceleration Test"
    
    log_info "Building Go adapter..."
    cd "$TPS_TESTING_DIR/inferstructor"
    go build -o tps_inferstructor_adapter tps_inferstructor_adapter.go
    log_success "Go adapter built"
    
    log_info "Starting TPS load generator..."
    log_info "  Target: $TARGET_TPS TPS"
    log_info "  Duration: ${DURATION_SECONDS}s"
    
    ./tps_inferstructor_adapter \
        --target-tps "$TARGET_TPS" \
        --duration "$DURATION_SECONDS" \
        --batch-size 1000 \
        --workers 1000 \
        --bridge http://localhost:9999 \
        | tee "$RESULTS_DIR/tps_load.log" &
    
    TPS_PID=$!
    
    log_info "Load generation started (PID: $TPS_PID)"
    log_info "Monitor at: http://localhost:8080"
    
    # Wait a bit for load to stabilize
    sleep 30
    
    log_success "Acceleration test running..."
fi

# Phase 4: Failover Testing
if [[ "$ENABLE_ALL_PHASES" == "true" || "$PHASE" == "failover" ]]; then
    section_header "Phase 4: Failover Testing"
    
    # Calculate test times
    QUARTER_TIME=$((DURATION_SECONDS / 4))
    
    log_info "Scheduling failover triggers..."
    
    # Trigger 1: Kill primary GPU at 25% mark
    (
        sleep "$QUARTER_TIME"
        log_warning "Triggering: Kill primary GPU"
        python3 "$SCRIPT_DIR/failover_triggers.py" \
            --trigger kill_primary_gpu \
            --lane primary \
            >> "$RESULTS_DIR/failover_events.log" 2>&1
    ) &
    
    # Trigger 2: Network latency spike at 50% mark
    (
        sleep "$((QUARTER_TIME * 2))"
        log_warning "Triggering: Network latency spike"
        python3 "$SCRIPT_DIR/failover_triggers.py" \
            --trigger inject_latency_spike \
            --lane primary \
            --duration 60 \
            --intensity 0.8 \
            >> "$RESULTS_DIR/failover_events.log" 2>&1
    ) &
    
    # Trigger 3: Cascade failure at 75% mark
    (
        sleep "$((QUARTER_TIME * 3))"
        log_warning "Triggering: Cascade failure (primary + shadow)"
        python3 "$SCRIPT_DIR/failover_triggers.py" \
            --trigger cascade_failure \
            >> "$RESULTS_DIR/failover_events.log" 2>&1
    ) &
    
    log_success "Failover triggers scheduled"
fi

# Wait for test completion
if [[ -n "$TPS_PID" ]]; then
    section_header "Test Running"
    
    log_info "Test in progress... (PID: $TPS_PID)"
    log_info "Duration: ${DURATION_SECONDS}s"
    log_info "Dashboard: http://localhost:8080"
    log_info "Press Ctrl+C to stop early"
    
    # Show progress
    START_TIME=$(date +%s)
    while kill -0 "$TPS_PID" 2>/dev/null; do
        ELAPSED=$(($(date +%s) - START_TIME))
        REMAINING=$((DURATION_SECONDS - ELAPSED))
        PERCENT=$((ELAPSED * 100 / DURATION_SECONDS))
        
        echo -ne "\r${BLUE}Progress: ${PERCENT}% | Elapsed: ${ELAPSED}s | Remaining: ${REMAINING}s${NC}"
        
        sleep 10
    done
    
    echo ""
    wait "$TPS_PID" || true
    log_success "Test completed!"
fi

# Phase 5: Results Collection
section_header "Phase 5: Collecting Results"

log_info "Fetching final metrics..."

# Get final stats from bridge
curl -s http://localhost:9999/stats > "$RESULTS_DIR/final_stats.json" || true

# Get dashboard metrics
curl -s http://localhost:8080/api/current > "$RESULTS_DIR/final_metrics.json" || true
curl -s "http://localhost:8080/api/history?count=1000" > "$RESULTS_DIR/metrics_history.json" || true
curl -s http://localhost:8080/api/failovers > "$RESULTS_DIR/failover_events.json" || true

log_success "Metrics collected"

# Analyze results
section_header "Phase 6: Analyzing Results"

python3 - <<EOF
import json
import sys

try:
    with open("$RESULTS_DIR/final_stats.json") as f:
        stats = json.load(f)
    
    tps = stats.get("current_tps", 0)
    total_tx = stats.get("total_forwarded", 0)
    failed = stats.get("total_failed", 0)
    received = stats.get("total_received", 1)
    
    success_rate = ((received - failed) / received * 100) if received > 0 else 0
    
    solana_baseline = 65000
    speedup = tps / solana_baseline
    
    print(f"\n{'='*60}")
    print("  INFERSTRUCTOR 300× TEST RESULTS")
    print(f"{'='*60}\n")
    print(f"Total Transactions: {total_tx:,}")
    print(f"Success Rate:       {success_rate:.2f}%")
    print(f"Achieved TPS:       {tps:,.2f}")
    print(f"Target TPS:         $TARGET_TPS")
    print(f"Solana Baseline:    {solana_baseline:,} TPS")
    print(f"\nSpeedup:            {speedup:.2f}× Solana")
    print(f"Target Speedup:     300× Solana")
    print(f"Progress:           {speedup/300*100:.2f}%")
    
    if speedup >= 300:
        print(f"\n✅ SUCCESS: 300× Solana target ACHIEVED!")
        sys.exit(0)
    elif speedup >= 200:
        print(f"\n⚠️  PARTIAL: {speedup:.0f}× achieved (target: 300×)")
        sys.exit(1)
    else:
        print(f"\n❌ INCOMPLETE: Only {speedup:.0f}× achieved")
        sys.exit(2)
        
except Exception as e:
    print(f"Error analyzing results: {e}")
    sys.exit(3)
EOF

ANALYSIS_RESULT=$?

# Export proof document
if [[ "$EXPORT_PROOF" == "true" ]]; then
    section_header "Phase 7: Generating Proof Document"
    
    log_info "Generating proof document..."
    
    cat > "$RESULTS_DIR/PROOF_OF_300X_SOLANA.md" <<EOF
# Inferstructor 300× Solana Speed - Proof of Concept

**Test Date:** $(date)  
**Duration:** ${DURATION_SECONDS}s  
**Target TPS:** $TARGET_TPS  

## Results

See attached files:
- \`final_stats.json\` - Complete statistics
- \`metrics_history.json\` - Time-series data
- \`failover_events.json\` - Failover event log
- \`tps_load.log\` - Load generator output
- \`orchestrator.log\` - Orchestrator log
- \`tps_bridge.log\` - Bridge log

## Reproducibility

To reproduce these results:
\`\`\`bash
cd cross-chain-gpu-validator/tests/inferstructor
./run_300x_test.sh --duration 8h --export-proof --enable-all-phases
\`\`\`

## Validation

All metrics can be independently verified via Prometheus endpoints:
- Orchestrator: http://localhost:8000/metrics
- Bridge: http://localhost:8002/metrics
- Dashboard: http://localhost:8080

---
Generated by Inferstructor Test Harness
EOF
    
    log_success "Proof document generated: $RESULTS_DIR/PROOF_OF_300X_SOLANA.md"
fi

# Final summary
section_header "Test Complete"

log_info "Results directory: $RESULTS_DIR"
log_info "Dashboard (still running): http://localhost:8080"

if [[ $ANALYSIS_RESULT -eq 0 ]]; then
    log_success "✅ 300× Solana target ACHIEVED!"
    exit 0
else
    log_warning "⚠️ 300× target not fully reached. See results for details."
    exit $ANALYSIS_RESULT
fi
