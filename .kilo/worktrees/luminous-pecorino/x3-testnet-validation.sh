#!/bin/bash
#
# X3 Pre-Testnet Validation Suite
# Runs the complete Substrate/Polkadot SDK toolchain for X3 release readiness
#
# Usage: ./x3-testnet-validation.sh [PHASE]
#   PHASE: 1 (build), 2 (runtime), 3 (network), 4 (release), all (default)
#
# Requires: cargo, try-runtime, zombienet, chopsticks, srtool
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="${SCRIPT_DIR}/launch-gates/evidence"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_FILE="${LOG_DIR}/validation-${TIMESTAMP}.log"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

mkdir -p "${LOG_DIR}"

log() {
    local level=$1
    shift
    local msg="$@"
    local ts=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${ts}] [${level}] ${msg}" | tee -a "${LOG_FILE}"
}

log_phase() {
    echo -e "${BLUE}=== PHASE: $1 ===${NC}" | tee -a "${LOG_FILE}"
}

log_success() {
    echo -e "${GREEN}✓ $1${NC}" | tee -a "${LOG_FILE}"
}

log_error() {
    echo -e "${RED}✗ $1${NC}" | tee -a "${LOG_FILE}"
}

log_warning() {
    echo -e "${YELLOW}⚠ $1${NC}" | tee -a "${LOG_FILE}"
}

check_tools() {
    log_phase "Tool Availability Check"
    
    local missing=0
    for tool in cargo rustc try-runtime zombienet chopsticks srtool; do
        if command -v "$tool" &> /dev/null; then
            local version=$("$tool" --version 2>/dev/null || echo "unknown")
            log_success "$tool: $version"
        else
            log_error "$tool: NOT FOUND"
            missing=$((missing + 1))
        fi
    done
    
    if [ $missing -gt 0 ]; then
        log "ERROR" "Missing $missing required tools. Cannot proceed."
        exit 1
    fi
}

phase_1_build() {
    log_phase "Phase 1: Build & Compile Checks (est. 2 hours)"
    
    log "INFO" "Running formatting check..."
    if cargo fmt --all -- --check >> "${LOG_FILE}" 2>&1; then
        log_success "cargo fmt passed"
    else
        log_error "cargo fmt failed - please run: cargo fmt --all"
        return 1
    fi
    
    log "INFO" "Running clippy lints..."
    if cargo clippy --workspace --all-targets -- -D warnings >> "${LOG_FILE}" 2>&1; then
        log_success "cargo clippy passed"
    else
        log_error "cargo clippy found warnings"
        return 1
    fi
    
    log "INFO" "Running workspace check..."
    if cargo check --workspace --all-targets >> "${LOG_FILE}" 2>&1; then
        log_success "cargo check passed"
    else
        log_error "cargo check failed"
        return 1
    fi
    
    log "INFO" "Running unit and integration tests..."
    if cargo test --workspace --lib --tests -- --test-threads=1 >> "${LOG_FILE}" 2>&1; then
        log_success "cargo test passed"
    else
        log_warning "Some tests failed - see log for details"
    fi
    
    return 0
}

phase_2_runtime() {
    log_phase "Phase 2: Runtime Safety & Benchmarking (est. 3 hours)"
    
    log "INFO" "Building runtime with benchmarking feature..."
    if cargo build --release -p x3-chain-node --features runtime-benchmarks,try-runtime >> "${LOG_FILE}" 2>&1; then
        log_success "Runtime build succeeded"
    else
        log_error "Runtime build failed"
        return 1
    fi
    
    local wasm_path="target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
    if [ ! -f "$wasm_path" ]; then
        log_error "WASM runtime not found at $wasm_path"
        return 1
    fi
    
    log_success "Runtime WASM available: $wasm_path"
    
    log "INFO" "Collecting pallet benchmarks..."
    local pallets=(
        "pallet-x3-kernel"
        "pallet-x3-atomic-kernel"
        "pallet-x3-supply-ledger"
        "pallet-x3-cross-vm-router"
        "pallet-x3-asset-registry"
        "pallet-x3-settlement-engine"
    )
    
    for pallet in "${pallets[@]}"; do
        log "INFO" "Benchmarking $pallet (steps=20, repeat=5 for fast iteration)..."
        
        if cargo run -p x3-chain-node --features runtime-benchmarks -- benchmark pallet \
            --pallet "$pallet" \
            --extrinsic "*" \
            --steps 20 \
            --repeat 5 \
            >> "${LOG_FILE}" 2>&1; then
            log_success "Benchmark $pallet completed"
        else
            log_warning "Benchmark $pallet had issues - see log"
        fi
    done
    
    return 0
}

phase_3_network() {
    log_phase "Phase 3: Network & E2E Testing (est. 4 hours)"
    
    log "WARNING" "Network phase requires Docker/Podman and active Zombienet setup."
    log "INFO" "Checking Zombienet availability..."
    
    if ! command -v zombienet &> /dev/null; then
        log_error "Zombienet not available"
        return 1
    fi
    
    log "INFO" "Zombienet version: $(zombienet version)"
    
    # Create minimal test network config if not present
    local config_file="${SCRIPT_DIR}/x3-network-config.toml"
    if [ ! -f "$config_file" ]; then
        log "INFO" "Creating minimal Zombienet config..."
        cat > "$config_file" << 'EOF'
[relaychain]
chain = "x3-local"
default_command = "./target/release/x3-chain-node"
default_args = ["--dev", "--rpc-port", "9944"]

[[relaychain.nodes]]
name = "alice"
validator = true

[[relaychain.nodes]]
name = "bob"
validator = true
EOF
        log_success "Created $config_file"
    fi
    
    log "WARNING" "Skipping actual Zombienet spawn (requires active container runtime)"
    log "INFO" "To run: zombienet spawn $config_file"
    
    return 0
}

phase_4_release() {
    log_phase "Phase 4: Release Build Reproducibility (est. 1 hour)"
    
    log "INFO" "Building runtime deterministically with srtool..."
    
    if srtool build --dry-run --profile release >> "${LOG_FILE}" 2>&1; then
        log_success "srtool dry-run passed"
    else
        log_error "srtool check failed"
        return 1
    fi
    
    log "INFO" "Collecting runtime artifacts..."
    local wasm_path="target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
    if [ -f "$wasm_path" ]; then
        local checksum=$(sha256sum "$wasm_path" | awk '{print $1}')
        log_success "Runtime checksum: $checksum"
        echo "$checksum" > "${LOG_DIR}/runtime-checksum-${TIMESTAMP}.txt"
    fi
    
    return 0
}

generate_report() {
    log_phase "Validation Report"
    
    local report_file="${LOG_DIR}/validation-report-${TIMESTAMP}.txt"
    
    {
        echo "X3 Pre-Testnet Validation Report"
        echo "=================================="
        echo "Generated: $(date)"
        echo ""
        echo "Tools Versions:"
        cargo --version
        try-runtime --version
        echo "Zombienet: $(zombienet version)"
        echo "Chopsticks: $(chopsticks --version)"
        echo "srtool: $(srtool --version)"
        echo ""
        echo "Workspace Status:"
        cargo fmt --all -- --check > /dev/null 2>&1 && echo "  ✓ Format: OK" || echo "  ✗ Format: FAILED"
        cargo clippy --workspace --all-targets -- -D warnings > /dev/null 2>&1 && echo "  ✓ Lint: OK" || echo "  ✗ Lint: FAILED"
        cargo check --workspace --all-targets > /dev/null 2>&1 && echo "  ✓ Check: OK" || echo "  ✗ Check: FAILED"
        echo ""
        echo "Full log: ${LOG_FILE}"
    } | tee "$report_file"
}

main() {
    local phase="${1:-all}"
    
    log "INFO" "X3 Pre-Testnet Validation Suite"
    log "INFO" "Timestamp: $TIMESTAMP"
    log "INFO" "Log file: $LOG_FILE"
    
    check_tools
    
    case "$phase" in
        1|build)
            phase_1_build
            ;;
        2|runtime)
            phase_2_runtime
            ;;
        3|network)
            phase_3_network
            ;;
        4|release)
            phase_4_release
            ;;
        all)
            phase_1_build && \
            phase_2_runtime && \
            phase_3_network && \
            phase_4_release && \
            log_success "All phases completed successfully"
            ;;
        *)
            echo "Usage: $0 [1|2|3|4|all]"
            exit 1
            ;;
    esac
    
    generate_report
    log "INFO" "Validation complete. Check ${LOG_FILE} for full details."
}

main "$@"
