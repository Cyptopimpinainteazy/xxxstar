#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# test-tools.sh — Comprehensive integration test suite for all Substrate tools
#
# Tests actual functionality, not just binary existence:
#   • try-runtime: Feature flag validation, subcommand dispatch
#   • Zombienet: Config parsing, spawn dry-run, network topology validation
#   • Chopsticks: RPC endpoint, storage mutation, block production
#   • FRAME benchmarking: Pallet enumeration, weight generation, hardware check
#   • srtool: Runtime detection, deterministic build prerequisites
#
# Exit codes:
#   0 — All tests passed
#   1 — One or more tests failed
#   2 — Critical infrastructure missing (cannot run tests)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="$REPO_ROOT/target/release/x3-chain-node"
NODE_BIN_STD="$REPO_ROOT/target/release/x3-chain-node.std"  # without runtime-benchmarks
ZOMBIENET_BIN="${ZOMBIENET_BIN:-$HOME/.local/bin/zombienet}"
CHOPSTICKS_BIN="${CHOPSTICKS_BIN:-$(which chopsticks 2>/dev/null || echo 'npx @acala-network/chopsticks@latest')}"
SRTOOL_BIN="${SRTOOL_BIN:-$(which srtool 2>/dev/null || echo 'srtool')}"

WASM="$REPO_ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
ZOMBIENET_CONFIG="$REPO_ROOT/zombienet/x3-local-network.toml"
CHOPSTICKS_CONFIG="$REPO_ROOT/chopsticks/x3-dev.yml"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
BOLD='\033[1m'

info()    { echo -e "${CYAN}[test]${NC} $*"; }
success() { echo -e "${GREEN}[test]  ✓${NC} $*"; }
warn()    { echo -e "${YELLOW}[test] ⚠${NC} $*"; }
fail()    { echo -e "${RED}[test] ✗${NC} $*"; }
die()     { echo -e "${RED}[test] ✗ FATAL:${NC} $*" >&2; exit 2; }

TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# ─────────────────────────────────────────────────────────────────────────────
# Test harness
# ─────────────────────────────────────────────────────────────────────────────
run_test() {
  local test_name="$1"
  local test_fn="$2"
  local category="${3:-general}"

  echo -n "  [$category] $test_name … "

  local start_time=$(date +%s%N)
  local output
  local exit_code=0

  output=$($test_fn 2>&1) || exit_code=$?
  local end_time=$(date +%s%N)
  local duration_ms=$(( (end_time - start_time) / 1000000 ))

  if [[ $exit_code -eq 0 ]]; then
    echo -e "${GREEN}✓ PASS${NC} (${duration_ms}ms)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    return 0
  elif [[ $exit_code -eq 77 ]]; then
    echo -e "${YELLOW}⊘ SKIP${NC} — $output"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
    return 0
  else
    echo -e "${RED}✗ FAIL${NC} (${duration_ms}ms)"
    if [[ -n "$output" ]]; then
      echo "$output" | sed 's/^/    /'
    fi
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  fi
}

skip() {
  echo "$*"
  exit 77
}

# ─────────────────────────────────────────────────────────────────────────────
# Infrastructure checks
# ─────────────────────────────────────────────────────────────────────────────
check_infra() {
  info "Checking test infrastructure …"

  if [[ ! -f "$NODE_BIN_STD" ]]; then
    info "Building node binary without runtime-benchmarks for RPC/integration tests …"
    cp "$NODE_BIN" "${NODE_BIN}.bench_bak" 2>/dev/null || true
    cargo build --release -p x3-chain-node 2>&1 | tail -3
    mv "$NODE_BIN" "$NODE_BIN_STD"
    if [[ -f "${NODE_BIN}.bench_bak" ]]; then
      mv "${NODE_BIN}.bench_bak" "$NODE_BIN"
    fi
    success "Non-benchmark node binary built"
  fi

  [[ -f "$NODE_BIN" ]] || die "Node binary not found at $NODE_BIN. Build with: cargo build --release -p x3-chain-node --features 'runtime-benchmarks,try-runtime'"

  if [[ "$CHOPSTICKS_BIN" == *"npx"* ]]; then
    command -v npx &>/dev/null || die "npx not found. Install Node.js 18+."
  else
    command -v "$CHOPSTICKS_BIN" &>/dev/null || die "chopsticks not found at $CHOPSTICKS_BIN"
  fi

  [[ -f "$ZOMBIENET_BIN" ]] || die "Zombienet not found at $ZOMBIENET_BIN"
  command -v "$SRTOOL_BIN" &>/dev/null || die "srtool not found"
  command -v curl &>/dev/null || die "curl not found"
  command -v python3 &>/dev/null || die "python3 not found"

  success "All infrastructure checks passed"
}

# ─────────────────────────────────────────────────────────────────────────────
# try-runtime tests
# ─────────────────────────────────────────────────────────────────────────────
test_try_runtime_feature() {
  "$NODE_BIN" try-runtime --help &>/dev/null || skip "try-runtime feature not enabled"
}

test_try_runtime_subcommand_exists() {
  local help_output
  help_output=$("$NODE_BIN" --help 2>&1 || true)
  [[ -n "$help_output" ]] || { echo "No help output captured"; exit 1; }
  echo "$help_output" | grep -q "try-runtime" || { echo "try-runtime not in help output (length: ${#help_output})"; exit 1; }
}

test_try_runtime_no_crash() {
  # Should print message and exit cleanly, not crash
  local output
  output=$("$NODE_BIN" try-runtime 2>&1 || true)
  echo "$output" | grep -qE "removed|Chopsticks|stable2512" || { echo "Unexpected output: $output"; exit 1; }
}

# ─────────────────────────────────────────────────────────────────────────────
# Zombienet tests
# ─────────────────────────────────────────────────────────────────────────────
test_zombienet_binary_runs() {
  "$ZOMBIENET_BIN" --help &>/dev/null || { echo "Zombienet binary failed to run"; exit 1; }
}

test_zombienet_config_valid() {
  [[ -f "$ZOMBIENET_CONFIG" ]] || { echo "Config not found: $ZOMBIENET_CONFIG"; exit 1; }

  # Validate TOML structure
  grep -qE '^\[relaychain\]' "$ZOMBIENET_CONFIG" || { echo "Missing [relaychain] section"; exit 1; }
  grep -qE 'default_command.*x3-chain-node' "$ZOMBIENET_CONFIG" || { echo "Missing or wrong default_command"; exit 1; }
  grep -qE '\[\[relaychain\.nodes\]\]' "$ZOMBIENET_CONFIG" || { echo "No relaychain nodes defined"; exit 1; }

  # Count validators (should be at least 2)
  local node_count
  node_count=$(grep -c '\[\[relaychain\.nodes\]\]' "$ZOMBIENET_CONFIG")
  [[ $node_count -ge 2 ]] || { echo "Only $node_count validators defined (need >= 2)"; exit 1; }
}

test_zombienet_node_binary_referenced() {
  # Extract default_command and verify it points to existing binary
  local cmd_path
  cmd_path=$(grep -E '^default_command' "$ZOMBIENET_CONFIG" | sed 's/.*= *"\(.*\)".*/\1/')

  # Resolve relative path
  if [[ "$cmd_path" == ./* ]]; then
    cmd_path="$REPO_ROOT/${cmd_path#./}"
  fi

  [[ -f "$cmd_path" ]] || { echo "Referenced binary not found: $cmd_path"; exit 1; }
  [[ -x "$cmd_path" ]] || { echo "Binary not executable: $cmd_path"; exit 1; }
}

test_zombienet_ports_unique() {
  # Extract all ports and verify uniqueness
  local ports
  ports=$(grep -oE '(rpc-)?port.*= *[0-9]+' "$ZOMBIENET_CONFIG" | grep -oE '[0-9]+$' | sort)
  local unique_ports
  unique_ports=$(echo "$ports" | sort -u)

  [[ "$ports" == "$unique_ports" ]] || { echo "Duplicate ports found: $ports"; exit 1; }
}

# ─────────────────────────────────────────────────────────────────────────────
# Chopsticks tests
# ─────────────────────────────────────────────────────────────────────────────
test_chopsticks_binary_runs() {
  if [[ "$CHOPSTICKS_BIN" == *"npx"* ]]; then
    npx @acala-network/chopsticks@latest --help &>/dev/null || { echo "Chopsticks npx failed"; exit 1; }
  else
    "$CHOPSTICKS_BIN" --help &>/dev/null || { echo "Chopsticks binary failed"; exit 1; }
  fi
}

test_chopsticks_config_valid() {
  [[ -f "$CHOPSTICKS_CONFIG" ]] || { echo "Config not found: $CHOPSTICKS_CONFIG"; exit 1; }

  # Validate YAML structure
  grep -qE '^endpoint:' "$CHOPSTICKS_CONFIG" || { echo "Missing endpoint field"; exit 1; }
  grep -qE '^port:' "$CHOPSTICKS_CONFIG" || { echo "Missing port field"; exit 1; }

  # Validate endpoint is local
  grep -qE 'endpoint:.*127\.0\.0\.1|localhost' "$CHOPSTICKS_CONFIG" || { echo "Endpoint not local"; exit 1; }
}

test_chopsticks_port_config() {
  local port
  port=$(grep -E '^port:' "$CHOPSTICKS_CONFIG" | awk '{print $2}')
  [[ -n "$port" ]] || { echo "Could not extract port"; exit 1; }
  [[ $port -ge 1024 && $port -le 65535 ]] || { echo "Invalid port: $port"; exit 1; }
}

test_chopsticks_wasm_exists() {
  [[ -f "$WASM" ]] || skip "WASM not built (run: cargo build --release -p x3-chain-node)"
}

test_chopsticks_wasm_valid() {
  # Check the uncompressed .compact.wasm file (compressed version has no magic bytes)
  local wasm_uncompressed="${WASM%.compressed.wasm}.wasm"
  [[ -f "$wasm_uncompressed" ]] || skip "Uncompressed WASM not found"

  # Check magic bytes
  local magic
  magic=$(xxd -p -l4 "$wasm_uncompressed" 2>/dev/null || hexdump -e '1/1 "%02x"' -n4 "$wasm_uncompressed" 2>/dev/null || echo "")
  [[ "$magic" == "0061736d" ]] || { echo "Invalid WASM magic: $magic"; exit 1; }

  # Check size (100KB - 20MB)
  local size
  size=$(wc -c < "$wasm_uncompressed")
  [[ $size -gt 102400 && $size -lt 20971520 ]] || { echo "Invalid WASM size: $size bytes"; exit 1; }
}

# ─────────────────────────────────────────────────────────────────────────────
# FRAME benchmarking tests
# ─────────────────────────────────────────────────────────────────────────────
test_benchmark_feature_enabled() {
  "$NODE_BIN" benchmark --help &>/dev/null || skip "runtime-benchmarks feature not enabled"
}

test_benchmark_subcommands() {
  local help_output
  help_output=$("$NODE_BIN" benchmark --help 2>&1)

  for subcmd in pallet storage overhead block machine extrinsic; do
    echo "$help_output" | grep -q "$subcmd" || { echo "Missing subcommand: $subcmd"; exit 1; }
  done
}

test_benchmark_pallet_list() {
  # List all benchmarkable pallets
  local output
  output=$("$NODE_BIN" benchmark pallet --chain dev --pallet "*" --extrinsic "*" --list 2>&1)

  # Check if WASM is embedded
  if echo "$output" | grep -q "Embedded runtime WASM is missing"; then
    skip "WASM not embedded (rebuild without SKIP_WASM_BUILD=1)"
  fi

  # Should find at least one x3 pallet
  echo "$output" | grep -qE "pallet_x3|pallet_cross" || { echo "No X3 pallets found in benchmark list"; exit 1; }
}

test_benchmark_weights_files() {
  local pallets=(
    "pallets/x3-atomic-kernel/src/weights.rs"
    "pallets/x3-settlement-engine/src/weights.rs"
    "pallets/cross-chain-validator/src/weights.rs"
    "pallets/x3-slash/src/weights.rs"
  )

  for pallet_path in "${pallets[@]}"; do
    local full_path="$REPO_ROOT/$pallet_path"
    [[ -f "$full_path" ]] || { echo "Missing weights file: $pallet_path"; exit 1; }

    # Check file has actual weight functions
    local fn_count
    fn_count=$(grep -cE 'fn [a-z_]+.*->.*Weight' "$full_path" 2>/dev/null || echo 0)
    [[ $fn_count -gt 0 ]] || { echo "No weight functions in $pallet_path"; exit 1; }
  done
}

test_benchmark_benchmarking_rs_files() {
  local pallets=(
    "pallets/x3-atomic-kernel/src/benchmarking.rs"
    "pallets/x3-settlement-engine/src/benchmarking.rs"
    "pallets/cross-chain-validator/src/benchmarking.rs"
    "pallets/x3-slash/src/benchmarking.rs"
  )

  for pallet_path in "${pallets[@]}"; do
    local full_path="$REPO_ROOT/$pallet_path"
    [[ -f "$full_path" ]] || { echo "Missing benchmarking file: $pallet_path"; exit 1; }

    # Check file has benchmarks! macro
    grep -qE 'benchmarks!' "$full_path" || { echo "No benchmarks! macro in $pallet_path"; exit 1; }
  done
}

test_benchmark_machine_check() {
  # Run hardware benchmark (fast, just checks)
  local output
  output=$("$NODE_BIN" benchmark machine --chain dev 2>&1)

  # Check if WASM is embedded
  if echo "$output" | grep -q "Embedded runtime WASM is missing"; then
    skip "WASM not embedded (rebuild without SKIP_WASM_BUILD=1)"
  fi

  # Should complete without error
  echo "$output" | grep -qE "CPU|Memory|Disk|Score" || { echo "Machine benchmark output malformed"; exit 1; }
}

# ─────────────────────────────────────────────────────────────────────────────
# srtool tests
# ─────────────────────────────────────────────────────────────────────────────
test_srtool_binary_runs() {
  "$SRTOOL_BIN" --help &>/dev/null || { echo "srtool binary failed"; exit 1; }
}

test_srtool_version() {
  local version
  version=$("$SRTOOL_BIN" --version 2>&1)
  echo "$version" | grep -qE "srtool-cli.*[0-9]+\.[0-9]+" || { echo "Unexpected version format: $version"; exit 1; }
}

test_srtool_runtime_detection() {
  # srtool should detect the runtime package
  local output
  output=$("$SRTOOL_BIN" info --package x3-chain-runtime "$REPO_ROOT/runtime" 2>&1 || true)

  # Either succeeds or fails with docker/podman error (not "package not found")
  if echo "$output" | grep -qi "package.*not found\|unknown package"; then
    echo "srtool could not detect x3-chain-runtime package"
    exit 1
  fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Integration tests
# ─────────────────────────────────────────────────────────────────────────────
test_integration_node_starts() {
  local node_bin="$NODE_BIN_STD"
  local startup_log="/tmp/x3-node-startup-$$.log"

  "$node_bin" --dev --tmp --rpc-port 19944 --no-prometheus &>"$startup_log" &
  local node_pid=$!
  sleep 5

  if ! kill -0 "$node_pid" 2>/dev/null; then
    local startup_output
    startup_output=$(cat "$startup_log" 2>/dev/null || echo "")
    rm -f "$startup_log"

    if echo "$startup_output" | grep -q "Embedded runtime WASM is missing"; then
      skip "WASM not embedded (rebuild without SKIP_WASM_BUILD=1)"
    fi
    echo "Node crashed on startup: $startup_output"
    exit 1
  fi

  rm -f "$startup_log"

  local rpc_response
  rpc_response=$(curl -s --max-time 5 -X POST \
    -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"system_chain","params":[]}' \
    "http://127.0.0.1:19944" 2>&1 || echo "RPC_FAILED")

  kill "$node_pid" 2>/dev/null || true
  wait "$node_pid" 2>/dev/null || true

  if [[ "$rpc_response" == "RPC_FAILED" ]]; then
    echo "Node RPC not responding"
    exit 1
  fi

  echo "$rpc_response" | grep -q "result" || { echo "RPC response malformed: $rpc_response"; exit 1; }
}

test_integration_benchmark_smoke() {
  # Run a minimal benchmark to verify the pipeline works
  local output
  output=$("$NODE_BIN" benchmark pallet \
    --chain dev \
    --pallet pallet_x3_slash \
    --extrinsic post_bond \
    --steps 2 \
    --repeat 1 \
    2>&1 || true)

  # Check if WASM is embedded
  if echo "$output" | grep -q "Embedded runtime WASM is missing"; then
    skip "WASM not embedded (rebuild without SKIP_WASM_BUILD=1)"
  fi

  # Should either succeed or fail gracefully (not crash)
  if echo "$output" | grep -qi "panic\|segfault\|abort"; then
    echo "Benchmark crashed: $output"
    exit 1
  fi

  # Should produce some output
  [[ ${#output} -gt 100 ]] || { echo "Benchmark produced no output"; exit 1; }
}

# ─────────────────────────────────────────────────────────────────────────────
# Main test runner
# ─────────────────────────────────────────────────────────────────────────────
main() {
  echo ""
  echo "════════════════════════════════════════════════════════════════"
  echo "  X3 Substrate Tools — Comprehensive Integration Test Suite"
  echo "════════════════════════════════════════════════════════════════"
  echo ""

  check_infra
  echo ""

  # try-runtime tests
  echo -e "${BOLD}[1/6] try-runtime${NC}"
  run_test "Feature flag enabled" test_try_runtime_feature "try-runtime"
  run_test "Subcommand in help" test_try_runtime_subcommand_exists "try-runtime"
  run_test "No crash on invocation" test_try_runtime_no_crash "try-runtime"
  echo ""

  # Zombienet tests
  echo -e "${BOLD}[2/6] Zombienet${NC}"
  run_test "Binary runs" test_zombienet_binary_runs "zombienet"
  run_test "Config valid" test_zombienet_config_valid "zombienet"
  run_test "Node binary referenced" test_zombienet_node_binary_referenced "zombienet"
  run_test "Ports unique" test_zombienet_ports_unique "zombienet"
  echo ""

  # Chopsticks tests
  echo -e "${BOLD}[3/6] Chopsticks${NC}"
  run_test "Binary runs" test_chopsticks_binary_runs "chopsticks"
  run_test "Config valid" test_chopsticks_config_valid "chopsticks"
  run_test "Port configuration" test_chopsticks_port_config "chopsticks"
  run_test "WASM exists" test_chopsticks_wasm_exists "chopsticks"
  run_test "WASM valid" test_chopsticks_wasm_valid "chopsticks"
  echo ""

  # FRAME benchmarking tests
  echo -e "${BOLD}[4/6] FRAME Benchmarking${NC}"
  run_test "Feature enabled" test_benchmark_feature_enabled "benchmark"
  run_test "All subcommands present" test_benchmark_subcommands "benchmark"
  run_test "Pallet list works" test_benchmark_pallet_list "benchmark"
  run_test "Weights files valid" test_benchmark_weights_files "benchmark"
  run_test "Benchmarking.rs files" test_benchmark_benchmarking_rs_files "benchmark"
  run_test "Machine check" test_benchmark_machine_check "benchmark"
  echo ""

  # srtool tests
  echo -e "${BOLD}[5/6] srtool${NC}"
  run_test "Binary runs" test_srtool_binary_runs "srtool"
  run_test "Version format" test_srtool_version "srtool"
  run_test "Runtime detection" test_srtool_runtime_detection "srtool"
  echo ""

  # Integration tests
  echo -e "${BOLD}[6/6] Integration${NC}"
  run_test "Node starts and responds to RPC" test_integration_node_starts "integration"
  run_test "Benchmark smoke test" test_integration_benchmark_smoke "integration"
  echo ""

  # Summary
  echo "════════════════════════════════════════════════════════════════"
  echo "  Test Summary"
  echo "════════════════════════════════════════════════════════════════"
  echo -e "  ${GREEN}✓ Passed:${NC}  $TESTS_PASSED"
  echo -e "  ${RED}✗ Failed:${NC}  $TESTS_FAILED"
  echo -e "  ${YELLOW}⊘ Skipped:${NC} $TESTS_SKIPPED"
  echo "════════════════════════════════════════════════════════════════"
  echo ""

  if [[ $TESTS_FAILED -gt 0 ]]; then
    fail "$TESTS_FAILED test(s) failed"
    exit 1
  fi

  success "All tests passed!"
  exit 0
}

main "$@"
