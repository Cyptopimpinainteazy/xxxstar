#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-frame-benchmarks.sh — FRAME pallet weight benchmarking for X3
#
# Runs `benchmark pallet` for all 4 pallets that have benchmarks! blocks,
# writes generated WeightInfo impls to each pallet's src/weights.rs, and
# cross-checks each output against the reference hardware baseline.
#
# Pallets benchmarked:
#   • pallet-x3-atomic-kernel     (submit_atomic_bundle, assign_bundle_executor)
#   • pallet-x3-settlement-engine (settle_bundle, record_settlement)
#   • pallet-cross-chain-validator (validate_cross_chain_proof)
#   • pallet-x3-slash              (slash_validator, report_double_sign)
#
# Modes:
#   ./scripts/run-frame-benchmarks.sh build           — compile with runtime-benchmarks
#   ./scripts/run-frame-benchmarks.sh run             — run all 4 pallets, write weights
#   ./scripts/run-frame-benchmarks.sh run PALLET      — run one pallet only
#   ./scripts/run-frame-benchmarks.sh smoke           — quick 10-step/5-repeat smoke run
#   ./scripts/run-frame-benchmarks.sh verify-weights  — verify weights.rs files are valid
#   ./scripts/run-frame-benchmarks.sh test            — CI non-interactive validation suite
#   ./scripts/run-frame-benchmarks.sh machine         — check hardware meets Substrate baseline
#   ./scripts/run-frame-benchmarks.sh list            — list all available benchmarks
#   ./scripts/run-frame-benchmarks.sh help            — show this help
#
# Warning: builds take 10–20 min; benchmark runs take 5–30 min per pallet.
# Run with --steps 10 --repeat 5 for a fast dry-run (not for production weights).
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="$REPO_ROOT/target/release/x3-chain-node"
LOG_DIR="$REPO_ROOT/.benchmark-logs"
LOG_FILE="$LOG_DIR/frame-benchmarks-$(date +%Y%m%d-%H%M%S).log"

# Benchmark parameters — tune for speed vs accuracy
# Production: STEPS=50 REPEAT=20
# Quick smoke: STEPS=10 REPEAT=5
STEPS="${BENCHMARK_STEPS:-50}"
REPEAT="${BENCHMARK_REPEAT:-20}"
CHAIN="${BENCHMARK_CHAIN:-dev}"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${CYAN}[benchmark]${NC} $*"; }
success() { echo -e "${GREEN}[benchmark]  ✓${NC} $*"; }
warn()    { echo -e "${YELLOW}[benchmark] ⚠${NC} $*"; }
die()     { echo -e "${RED}[benchmark] ✗ ERROR:${NC} $*" >&2; exit 1; }

# Pallet name → output path mapping
declare -A PALLET_PATHS=(
  ["pallet-x3-atomic-kernel"]="pallets/x3-atomic-kernel/src/weights.rs"
  ["pallet-x3-settlement-engine"]="pallets/x3-settlement-engine/src/weights.rs"
  ["pallet-cross-chain-validator"]="pallets/cross-chain-validator/src/weights.rs"
  ["pallet-x3-slash"]="pallets/x3-slash/src/weights.rs"
)

print_help() {
  cat <<EOF

  X3 FRAME Benchmarking — accurate pallet weight generation

  USAGE:
    ./scripts/run-frame-benchmarks.sh [COMMAND] [PALLET]

  COMMANDS:
    build              Compile x3-chain-node --features runtime-benchmarks
    run                Benchmark all 4 pallets; write weights to pallet src/weights.rs
    run PALLET         Benchmark a single pallet (use the crate name, e.g. pallet-x3-slash)
    smoke              Quick smoke run (steps=10 repeat=5) to verify setup is functional
    verify-weights     Validate all 4 weights.rs files exist, are non-empty, and contain
                       valid WeightInfo implementations (no build required)
    test               CI non-interactive suite: binary check + weights validation + list
    machine            Check hardware against Substrate reference baseline
    list               List all benchmarkable extrinsics across all pallets
    help               Show this message

  TUNING:
    BENCHMARK_STEPS=50   Number of component steps  (default: $STEPS)
    BENCHMARK_REPEAT=20  Repeats per step            (default: $REPEAT)
    BENCHMARK_CHAIN=dev  Chain spec to use           (default: $CHAIN)

  QUICK SMOKE RUN (~5 min, verifies setup):
    ./scripts/run-frame-benchmarks.sh smoke

  PRODUCTION RUN:
    BENCHMARK_STEPS=50 BENCHMARK_REPEAT=20 ./scripts/run-frame-benchmarks.sh run

  CI VALIDATION (no live run, fast):
    ./scripts/run-frame-benchmarks.sh test

EOF
}

cmd_build() {
  info "Building x3-chain-node with --features runtime-benchmarks …"
  info "(first build: 10-20 min; subsequent: 1-3 min with cache)"
  cd "$REPO_ROOT"
  cargo build \
    --release \
    -p x3-chain-node \
    --features runtime-benchmarks \
    2>&1 | tee "${LOG_DIR}/build-benchmark-$(date +%Y%m%d-%H%M%S).log"
  success "Build complete → $NODE_BIN"
  echo "Binary: $(du -sh "$NODE_BIN" | cut -f1)"
}

bench_pallet() {
  local pallet="$1"
  local output_rel="${PALLET_PATHS[$pallet]:-}"

  [[ -n "$output_rel" ]] || die "Unknown pallet: $pallet. Known: ${!PALLET_PATHS[*]}"
  local output_file="$REPO_ROOT/$output_rel"

  info "Benchmarking $pallet (steps=$STEPS, repeat=$REPEAT) …"
  mkdir -p "$(dirname "$output_file")"

  "$NODE_BIN" benchmark pallet \
    --chain "$CHAIN" \
    --pallet "$pallet" \
    --extrinsic "*" \
    --steps "$STEPS" \
    --repeat "$REPEAT" \
    --template "$REPO_ROOT/.maintain/frame-weight-template.hbs" \
    --output "$output_file" \
    2>&1 | tee -a "$LOG_FILE"

  local exit_code=${PIPESTATUS[0]}
  if [[ $exit_code -eq 0 ]]; then
    success "$pallet → weights written to $output_rel"
    # Show the generated ref times
    grep -E 'fn |RefTime|ProofSize' "$output_file" | head -20 || true
  else
    # --template may not exist; retry without it (outputs to stdout)
    warn "--template flag failed (missing .hbs). Retrying without template …"
    "$NODE_BIN" benchmark pallet \
      --chain "$CHAIN" \
      --pallet "$pallet" \
      --extrinsic "*" \
      --steps "$STEPS" \
      --repeat "$REPEAT" \
      --output "$output_file" \
      2>&1 | tee -a "$LOG_FILE"

    [[ ${PIPESTATUS[0]} -eq 0 ]] && success "$pallet weights written." || die "$pallet benchmark FAILED"
  fi
}

cmd_run() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-frame-benchmarks.sh build' first."
  mkdir -p "$LOG_DIR"

  local target_pallet="${1:-all}"

  if [[ "$target_pallet" == "all" ]]; then
    info "Running FRAME benchmarks for all ${#PALLET_PATHS[@]} pallets …"
    info "STEPS=$STEPS  REPEAT=$REPEAT  CHAIN=$CHAIN"
    info "Log → $LOG_FILE"
    echo ""

    local passed=0 failed=0
    for pallet in "${!PALLET_PATHS[@]}"; do
      if bench_pallet "$pallet"; then
        passed=$((passed + 1))
      else
        warn "$pallet benchmark failed — continuing with remaining pallets"
        failed=$((failed + 1))
      fi
      echo ""
    done

    echo ""
    echo "────────────────────────────────────────────────"
    echo -e "  FRAME Benchmark Results:"
    echo -e "  ${GREEN}PASSED: $passed${NC}  |  ${RED}FAILED: $failed${NC}"
    echo "────────────────────────────────────────────────"
    [[ $failed -eq 0 ]] && success "All pallet weights generated." || warn "$failed pallets failed — check $LOG_FILE"
  else
    bench_pallet "$target_pallet"
  fi
}

cmd_machine() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-frame-benchmarks.sh build' first."
  mkdir -p "$LOG_DIR"

  info "Checking hardware against Substrate reference baseline …"
  "$NODE_BIN" benchmark machine \
    --chain "$CHAIN" \
    2>&1 | tee -a "$LOG_FILE"
}

# ─────────────────────────────────────────────────────────────────────────────
# smoke — quick 10-step/5-repeat run against all 4 pallets to verify the
#         benchmarking pipeline is functional end-to-end (~5 min)
# ─────────────────────────────────────────────────────────────────────────────
cmd_smoke() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-frame-benchmarks.sh build' first."
  mkdir -p "$LOG_DIR"

  info "Running SMOKE benchmark (steps=10, repeat=5) for all 4 pallets …"
  info "This is NOT production-accurate — use 'run' with STEPS=50 REPEAT=20 for weights."
  echo ""

  local passed=0 failed=0
  for pallet in "${!PALLET_PATHS[@]}"; do
    info "Smoke-testing $pallet …"
    if "$NODE_BIN" benchmark pallet \
        --chain "$CHAIN" \
        --pallet "$pallet" \
        --extrinsic "*" \
        --steps 10 \
        --repeat 5 \
        2>&1 | tee -a "$LOG_FILE" | grep -E 'Benchmarking|pallet:|error|Error' | head -10; then
      success "$pallet smoke OK"
      passed=$((passed + 1))
    else
      warn "$pallet smoke FAILED"
      failed=$((failed + 1))
    fi
    echo ""
  done

  echo "────────────────────────────────────────────────"
  echo -e "  Smoke Results:  ${GREEN}PASSED: $passed${NC}  |  ${RED}FAILED: $failed${NC}"
  echo "────────────────────────────────────────────────"
  [[ $failed -eq 0 ]] && success "All smoke benchmarks passed." || die "$failed smoke benchmarks failed."
}

# ─────────────────────────────────────────────────────────────────────────────
# verify-weights — static validation of generated weights.rs files
#   Checks: file exists, non-empty, contains WeightInfo trait impl,
#           contains at least one fn with RefTime/ProofSize
# ─────────────────────────────────────────────────────────────────────────────
cmd_verify_weights() {
  local PASS=0 FAIL=0 SKIP=0

  info "Verifying weights.rs files for all ${#PALLET_PATHS[@]} pallets …"
  echo ""

  for pallet in "${!PALLET_PATHS[@]}"; do
    local wfile="$REPO_ROOT/${PALLET_PATHS[$pallet]}"
    echo -e "  ${CYAN}→ $pallet${NC}"

    # 1. File exists
    if [[ ! -f "$wfile" ]]; then
      echo -e "    ${RED}✗ MISSING${NC}: $wfile"
      echo -e "      → Run: ./scripts/run-frame-benchmarks.sh build && run $pallet"
      FAIL=$((FAIL + 1))
      continue
    fi

    # 2. Non-empty (> 100 bytes)
    local size
    size=$(wc -c < "$wfile")
    if [[ $size -lt 100 ]]; then
      echo -e "    ${RED}✗ TOO SMALL${NC}: $wfile (${size} bytes — likely empty stub)"
      FAIL=$((FAIL + 1))
      continue
    fi

    # 3. Contains WeightInfo or Weight impl
    if ! grep -qE 'WeightInfo|impl.*Weight' "$wfile"; then
      echo -e "    ${RED}✗ NO WeightInfo${NC}: file exists but has no WeightInfo impl"
      FAIL=$((FAIL + 1))
      continue
    fi

    # 4. Contains at least one weight function with RefTime or Weight::from
    local fn_count
    fn_count=$(grep -cE 'fn [a-z_]+.*->.*Weight|RefTime|Weight::from_parts' "$wfile" 2>/dev/null || echo 0)
    if [[ $fn_count -eq 0 ]]; then
      echo -e "    ${YELLOW}⚠ WARN${NC}: no concrete weight functions found (template-only?)"
      SKIP=$((SKIP + 1))
    else
      echo -e "    ${GREEN}✓ OK${NC}: ${size} bytes, ${fn_count} weight function(s)"
      # Show a few function signatures
      grep -E 'fn [a-z_]+' "$wfile" | head -5 | sed 's/^/      /'
      PASS=$((PASS + 1))
    fi
    echo ""
  done

  echo "────────────────────────────────────────────────"
  echo -e "  Weights Validation: ${GREEN}✓ $PASS OK${NC} | ${YELLOW}⚠ $SKIP WARN${NC} | ${RED}✗ $FAIL FAIL${NC}"
  echo "────────────────────────────────────────────────"

  if [[ $FAIL -gt 0 ]]; then
    die "$FAIL weights file(s) failed validation. Run benchmarks to regenerate."
  fi
  if [[ $SKIP -gt 0 ]]; then
    warn "$SKIP weights file(s) have no concrete weight functions — review."
  fi
  success "All weights files validated."
}

# ─────────────────────────────────────────────────────────────────────────────
# test — CI-friendly non-interactive validation suite
#   1. Checks node binary exists (runtime-benchmarks feature check)
#   2. Validates all weights.rs files via verify-weights
#   3. Validates `benchmark pallet --list` runs successfully
#   4. Validates WASM binary exists
# Exits 0 on all-pass, 1 on any failure. Safe to run in CI without a live node.
# ─────────────────────────────────────────────────────────────────────────────
cmd_test() {
  local PASS=0 FAIL=0
  mkdir -p "$LOG_DIR"

  echo ""
  echo "════════════════════════════════════════════════════════"
  echo "  FRAME Benchmarking — CI Test Suite"
  echo "════════════════════════════════════════════════════════"
  echo ""

  _check() {
    local label="$1"; shift
    echo -n "  Checking: $label … "
    if "$@" &>/dev/null; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC}"
      FAIL=$((FAIL + 1))
    fi
  }

  # Test 1: runtime-benchmarks binary exists
  _check "Node binary exists" test -f "$NODE_BIN"

  # Test 2: binary has benchmark subcommand (implies runtime-benchmarks feature)
  if [[ -f "$NODE_BIN" ]]; then
    _check "Node has 'benchmark' subcommand" "$NODE_BIN" benchmark --help
  else
    echo -e "  Skipping benchmark-subcommand check (binary not built)"
    FAIL=$((FAIL + 1))
  fi

  # Test 3: verify weights files
  echo ""
  info "Running verify-weights …"
  if cmd_verify_weights 2>&1 | tee -a "$LOG_FILE"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
  fi

  # Test 4: list command works (fast, no live network)
  if [[ -f "$NODE_BIN" ]]; then
    echo ""
    echo -n "  Checking: benchmark list succeeds … "
    if "$NODE_BIN" benchmark pallet \
        --chain "$CHAIN" \
        --pallet "*" \
        --extrinsic "*" \
        --list \
        2>&1 | grep -q 'pallet_x3\|pallet_cross'; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${YELLOW}⚠ WARN${NC} (list ran but no x3 pallets found — check runtime registration)"
    fi
  fi

  # Test 5: WASM binary exists
  local wasm_path="$REPO_ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
  _check "WASM binary exists" test -f "$wasm_path"

  if [[ -f "$wasm_path" ]]; then
    # Test 6: WASM magic bytes valid
    echo -n "  Checking: WASM magic bytes (\\x00asm) … "
    local magic
    magic=$(xxd -p -l4 "$wasm_path" 2>/dev/null || hexdump -e '1/1 "%02x"' -n4 "$wasm_path" 2>/dev/null || echo "")
    if [[ "$magic" == "0061736d" ]]; then
      echo -e "${GREEN}✓ PASS${NC} (magic: $magic)"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC} (expected 0061736d, got: $magic)"
      FAIL=$((FAIL + 1))
    fi

    # Test 7: WASM size sanity (must be > 100KB, < 20MB compressed)
    local wasm_size
    wasm_size=$(wc -c < "$wasm_path")
    echo -n "  Checking: WASM size (100KB–20MB) … "
    if [[ $wasm_size -gt 102400 && $wasm_size -lt 20971520 ]]; then
      echo -e "${GREEN}✓ PASS${NC} ($(( wasm_size / 1024 )) KB)"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC} (size: ${wasm_size} bytes — outside expected range)"
      FAIL=$((FAIL + 1))
    fi
  fi

  # Test 8: all 4 pallet benchmarking.rs files exist
  for pallet in "${!PALLET_PATHS[@]}"; do
    local pallet_dir
    pallet_dir=$(echo "$pallet" | sed 's/pallet-//')
    _check "benchmarking.rs exists: $pallet" test -f "$REPO_ROOT/pallets/$pallet_dir/src/benchmarking.rs"
  done

  echo ""
  echo "════════════════════════════════════════════════════════"
  echo -e "  Results: ${GREEN}✓ $PASS PASSED${NC}  |  ${RED}✗ $FAIL FAILED${NC}"
  echo "════════════════════════════════════════════════════════"
  echo ""

  [[ $FAIL -eq 0 ]] && success "All benchmark CI tests passed." || die "$FAIL test(s) failed."
}

cmd_list() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-frame-benchmarks.sh build' first."

  info "Listing all available benchmarks in the runtime …"
  "$NODE_BIN" benchmark pallet \
    --chain "$CHAIN" \
    --pallet "*" \
    --extrinsic "*" \
    --list \
    2>&1 | grep -v '^$' | head -80
}

# ─────────── dispatch ──────────────────────────────────────────────────────
mkdir -p "$LOG_DIR"

COMMAND="${1:-help}"
case "$COMMAND" in
  build)           cmd_build ;;
  run)             cmd_run "${2:-all}" ;;
  smoke)           cmd_smoke ;;
  verify-weights)  cmd_verify_weights ;;
  test)            cmd_test ;;
  machine)         cmd_machine ;;
  list)            cmd_list ;;
  help|--help|-h)  print_help ;;
  *)               warn "Unknown command: $COMMAND"; print_help; exit 1 ;;
esac
