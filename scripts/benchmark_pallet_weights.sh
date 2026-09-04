#!/usr/bin/env bash
set -euo pipefail

# X3 Pallet Weight Benchmark Runner
#
# Runs `frame-benchmarking-cli` for every X3 pallet that has benchmarking
# support, generating real weights.rs files for production use.
#
# Usage:
#   ./scripts/benchmark_pallet_weights.sh [--all] [--pallet <name>]
#
# Prerequisites:
#   - Build node with runtime-benchmarks feature:
#     cargo build -p node --release --features runtime-benchmarks
#
#   - The chain must be running in benchmark mode (or use --wasm-execution compiled)

BINARY="${X3_BINARY:-./target/release/x3-chain-node}"
OUTPUT_DIR="${X3_WEIGHT_OUTPUT:-./runtime/src/weights}"
TEMPLATE="${X3_WEIGHT_TEMPLATE:-.maintain/frame-weight-template.hbs}"
STEPS="${X3_BENCH_STEPS:-50}"
REPEAT="${X3_BENCH_REPEAT:-20}"
HEAP_PAGES="${X3_BENCH_HEAP_PAGES:-4096}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ── Pallet registry ─────────────────────────────────────────────────────────

# Format: "pallet_name:crate_name:output_weight_file"
# Pallet name = how frame-benchmarking-cli knows it
# Crate name = Cargo workspace member name
PALLETS=(
    "pallet_x3_kernel:x3-kernel:x3_kernel.rs"
    "pallet_atomic_trade_engine:atomic-trade-engine:atomic_trade_engine.rs"
    "pallet_x3_dex:x3-dex:x3_dex.rs"
    "pallet_x3_atomic_kernel:x3-atomic-kernel:x3_atomic_kernel.rs"
    "pallet_x3_custody:x3-custody:x3_custody.rs"
    "pallet_x3_cross_vm_router:x3-cross-vm-router:x3_cross_vm_router.rs"
    "pallet_x3_supply_ledger:x3-supply-ledger:x3_supply_ledger.rs"
    "pallet_x3_asset_registry:x3-asset-registry:x3_asset_registry.rs"
    "pallet_x3_token_factory:x3-token-factory:x3_token_factory.rs"
    "pallet_x3_staking:x3-staking:x3_staking.rs"
    "pallet_x3_oracle:x3-oracle:x3_oracle.rs"
    "pallet_x3_governance:x3-governance:x3_governance.rs"
    "pallet_x3_slash:x3-slash:x3_slash.rs"
    "pallet_x3_launchpad:x3-launchpad:x3_launchpad.rs"
    "pallet_x3_flashloan:x3-flashloan:x3_flashloan.rs"
    "pallet_x3_auction:x3-auction:x3_auction.rs"
    "pallet_x3_compute_market:x3-compute-market:x3_compute_market.rs"
    "pallet_x3_sequencer:x3-sequencer:x3_sequencer.rs"
    "pallet_x3_da:x3-da:x3_da.rs"
    "pallet_x3_invariants:x3-invariants:x3_invariants.rs"
    "pallet_x3_lp_locker:x3-lp-locker:x3_lp_locker.rs"
    "pallet_x3_wrapped:x3-wrapped:x3_wrapped.rs"
    "pallet_x3_reconciliation:x3-reconciliation:x3_reconciliation.rs"
    "pallet_x3_solvency:x3-solvency:x3_solvency.rs"
    "pallet_x3_inventory:x3-inventory:x3_inventory.rs"
    "pallet_x3_reservation:x3-reservation:x3_reservation.rs"
    "pallet_x3_rebalance:x3-rebalance:x3_rebalance.rs"
)

log_info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── Pre-flight checks ───────────────────────────────────────────────────────

check_binary() {
    if [[ ! -x "$BINARY" ]]; then
        log_error "Node binary not found: $BINARY"
        log_error "Build it first: cargo build -p node --release --features runtime-benchmarks"
        exit 1
    fi
}

check_benchmark_support() {
    if ! "$BINARY" benchmark --help &>/dev/null; then
        log_error "Node binary does not have benchmark subcommand. Rebuild with --features runtime-benchmarks"
        exit 1
    fi
}

# ── Run benchmark for a single pallet ────────────────────────────────────────

run_pallet_benchmark() {
    local pallet_name="$1"
    local output_file="$2"

    log_info "Benchmarking pallet: $pallet_name"

    "$BINARY" benchmark pallet \
        --chain=dev \
        --steps="$STEPS" \
        --repeat="$REPEAT" \
        --pallet="$pallet_name" \
        --extrinsic="*" \
        --wasm-execution=compiled \
        --heap-pages="$HEAP_PAGES" \
        --output="$output_file" \
        2>&1 | tail -20

    if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
        log_error "Pallet '$pallet_name' benchmark failed"
        return 1
    fi
    log_info "Benchmarked $pallet_name → $output_file"
    return 0
}

# ── Overhead benchmark ───────────────────────────────────────────────────────

run_overhead_benchmark() {
    log_info "Running overhead benchmark..."
    "$BINARY" benchmark overhead \
        --chain=dev \
        --wasm-execution=compiled \
        --weight-path="${OUTPUT_DIR}/extrinsic_weights.rs" \
        --warmup=10 \
        --repeat=100 \
        2>&1 | tail -10
    log_info "Overhead benchmark complete"
}

# ── Storage benchmark ────────────────────────────────────────────────────────

run_storage_benchmark() {
    log_info "Running storage benchmark..."
    "$BINARY" benchmark storage \
        --chain=dev \
        --state-version=0 \
        --warmups=10 \
        --weight-path="${OUTPUT_DIR}/" \
        2>&1 | tail -10
    log_info "Storage benchmark complete"
}

# ── Main ─────────────────────────────────────────────────────────────────────

main() {
    mkdir -p "$OUTPUT_DIR"

    check_binary
    check_benchmark_support

    SELECTED_PALLET="${1:-}"

    PASSED=0
    FAILED=0
    SKIPPED=0

    for entry in "${PALLETS[@]}"; do
        IFS=':' read -r pallet_name crate_name weight_file <<< "$entry"

        if [[ -n "$SELECTED_PALLET" && "$crate_name" != "$SELECTED_PALLET" ]]; then
            ((SKIPPED++))
            continue
        fi

        output_path="${OUTPUT_DIR}/${weight_file}"
        if run_pallet_benchmark "$pallet_name" "$output_path"; then
            ((PASSED++))
        else
            ((FAILED++))
        fi
        echo ""
    done

    # Run overhead + storage after pallet benchmarks
    run_overhead_benchmark || true
    echo ""
    run_storage_benchmark || true
    echo ""

    echo "═════════════════════════════════════════════"
    echo " Benchmarks: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}, ${YELLOW}$SKIPPED skipped${NC}"
    echo " Output dir: $OUTPUT_DIR"
    echo "═════════════════════════════════════════════"
}

main "$@"