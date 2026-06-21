#!/usr/bin/env bash
set -euo pipefail
# X3 All-Benchmarks Orchestrator
#
# Runs all benchmark suites in order, aggregates results, generates report.
#
# Usage:
#   ./scripts/run_all_benchmarks.sh              # run all benchmarks
#   ./scripts/run_all_benchmarks.sh --quick      # reduced sample sizes
#   ./scripts/run_all_benchmarks.sh --skip-k6    # skip k6 (no node running)
#   ./scripts/run_all_benchmarks.sh --skip-pallets # skip pallet weight bench

QUICK="${QUICK:-false}"
SKIP_K6="${SKIP_K6:-false}"
SKIP_PALLETS="${SKIP_PALLETS:-false}"
BENCH_OUT_DIR="${BENCH_OUT_DIR:-reports/benchmarks}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
REPORT_FILE="${BENCH_OUT_DIR}/x3-benchmark-report-$(date -u +%Y-%m-%d).json"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

say()   { echo -e "${CYAN}[bench]${NC} $*"; }
ok()    { echo -e "${GREEN}[  OK]${NC} $*"; }
warn()  { echo -e "${YELLOW}[ WARN]${NC} $*"; }
fail()  { echo -e "${RED}[ FAIL]${NC} $*"; }

PASSED=0
FAILED=0
TOTAL=0

pass() { PASSED=$((PASSED+1)); TOTAL=$((TOTAL+1)); ok "$1"; }
fail() { FAILED=$((FAILED+1)); TOTAL=$((TOTAL+1)); fail "$1"; }

# ─── Suite A: Criterion Microbenchmarks ─────────────────────────────────────

bench_criterion() {
    say "Suite A: Criterion Rust microbenchmarks"

    local quiet_flag=""
    local quick_flag=""
    if "$QUICK"; then
        quick_flag="-- --sample-size 10 --warm-up-time 1 --measurement-time 2"
    fi

    local benches=(
        atomic_swap_bench
        dex_route_bench
        bridge_proof_bench
        vm_dispatch_bench
        rpc_encoding_bench
        signature_verify_bench
    )

    for bench in "${benches[@]}"; do
        say "  Running $bench..."
        if cargo bench --bench "$bench" $quick_flag 2>&1 | tail -5; then
            pass "criterion/$bench"
        else
            fail "criterion/$bench"
        fi
        echo ""
    done
}

# ─── Suite B: FRAME Pallet Weight Benchmarks ─────────────────────────────────

bench_pallets() {
    if "$SKIP_PALLETS"; then
        warn "Skipping pallet weight benchmarks (--skip-pallets)"
        return
    fi

    say "Suite B: FRAME pallet weight benchmarks"

    if [[ ! -x "./target/release/x3-chain-node" ]]; then
        warn "Node binary not found. Attempting to build..."
        if cargo build -p node --release --features runtime-benchmarks 2>&1 | tail -10; then
            ok "built node with runtime-benchmarks"
        else
            fail "build node with runtime-benchmarks"
            return
        fi
    fi

    # Run the pallet benchmark script
    if bash scripts/benchmark_pallet_weights.sh 2>&1 | tail -30; then
        pass "pallet_weights"
    else
        fail "pallet_weights"
    fi
}

# ─── Suite C: k6 RPC Load Test ──────────────────────────────────────────────

bench_k6() {
    if "$SKIP_K6"; then
        warn "Skipping k6 load tests (--skip-k6)"
        return
    fi

    if ! command -v k6 &>/dev/null; then
        warn "k6 not installed. Install with: brew install k6 (mac) or apt install k6"
        return
    fi

    say "Suite C: k6 RPC/WebSocket load test"

    mkdir -p "$BENCH_OUT_DIR"

    if k6 run \
        --vus 20 \
        --duration 30s \
        --summary-export "$BENCH_OUT_DIR/x3-k6-summary.json" \
        benchmarks/k6/x3_rpc_load.js 2>&1 | tail -20; then
        pass "k6_rpc_load"
    else
        fail "k6_rpc_load"
    fi
}

# ─── Suite D: Storage / I/O Benchmarks ──────────────────────────────────────

bench_storage() {
    say "Suite D: Storage I/O benchmarks"

    if command -v fio &>/dev/null; then
        say "  fio random read (4k, 64 depth)..."
        if fio --name=randread --rw=randread --bs=4k --size=256M \
               --iodepth=64 --runtime=10 --time_based --end_fsync=1 \
               --output-format=json 2>/dev/null; then
            pass "storage/fio_randread"
        else
            fail "storage/fio_randread"
        fi
    else
        warn "fio not installed. Skipping disk benchmarks."
    fi
}

# ─── Suite E: Network Benchmarks ────────────────────────────────────────────

bench_network() {
    say "Suite E: Network benchmarks"

    if command -v iperf3 &>/dev/null; then
        say "  iperf3 (requires a server: iperf3 -s on target)"
        # Self-test: localhost loopback bandwidth
        if iperf3 -c 127.0.0.1 -t 5 -J 2>/dev/null; then
            pass "network/iperf3_localhost"
        else
            warn "network/iperf3_localhost (no server running)"
        fi
    else
        warn "iperf3 not installed. Skipping network benchmarks."
    fi
}

# ─── Report Generation ──────────────────────────────────────────────────────

generate_report() {
    say "Generating benchmark report..."

    if python scripts/benchmark_report.py \
        --criterion-dir target/criterion \
        --output "$REPORT_FILE" 2>&1; then
        ok "Report: $REPORT_FILE"
    else
        warn "Report generation had issues. Check output."
    fi
}

# ─── Main ────────────────────────────────────────────────────────────────────

main() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --quick) QUICK=true; shift ;;
            --skip-k6) SKIP_K6=true; shift ;;
            --skip-pallets) SKIP_PALLETS=true; shift ;;
            *) shift ;;
        esac
    done

    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  X3 Chain — Full Benchmark Suite"
    echo "  Started: $TIMESTAMP"
    echo "  Mode:    $( $QUICK && echo 'QUICK' || echo 'FULL' )"
    echo "═══════════════════════════════════════════════════════════"
    echo ""

    mkdir -p "$BENCH_OUT_DIR"

    # Run all suites
    bench_criterion
    bench_pallets
    bench_k6
    bench_storage
    bench_network

    # Generate aggregate report
    generate_report

    # Final scoreboard
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "  Benchmark Run Complete"
    echo "  Passed: ${GREEN}$PASSED${NC}  Failed: ${RED}$FAILED${NC}  Total: $TOTAL"
    echo "  Report: $REPORT_FILE"
    echo "═══════════════════════════════════════════════════════════"
}

main "$@"