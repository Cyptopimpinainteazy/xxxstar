#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-substrate-tests.sh — CI-focused non-interactive test suite for all 5
# Substrate tooling integrations in X3_ATOMIC_STAR
#
# Validates (without spawning live networks):
#   1. Node binary          — exists, correct features, help commands
#   2. WASM binary          — exists, valid magic bytes, sane size
#   3. try-runtime          — subcommand reachable, snapshot dir writable
#   4. Zombienet            — binary installed, TOML config valid
#   5. Chopsticks           — binary/npx reachable, YAML config valid
#   6. FRAME benchmarks     — binary has benchmark subcommand, weights.rs valid
#   7. srtool               — Docker running, image available
#
# Exits 0 on all-pass, 1 on any failure.
# Safe to run in CI without a live node or network.
#
# Usage:
#   ./scripts/run-substrate-tests.sh
#   ./scripts/run-substrate-tests.sh --verbose    # show extra detail
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERBOSE="${1:-}"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'
BOLD='\033[1m'; NC='\033[0m'

PASS=0
FAIL=0
WARN=0

# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────
_pass() { echo -e "  ${GREEN}✓${NC} $1"; PASS=$((PASS + 1)); }
_fail() { echo -e "  ${RED}✗${NC} $1"; FAIL=$((FAIL + 1)); }
_warn() { echo -e "  ${YELLOW}⚠${NC} $1"; WARN=$((WARN + 1)); }
_section() {
  echo ""
  echo -e "${BOLD}${CYAN}━━━ $1 ━━━${NC}"
}

_check() {
  local label="$1"; shift
  if "$@" &>/dev/null 2>&1; then
    _pass "$label"
  else
    _fail "$label"
  fi
}

_check_grep() {
  local label="$1"; local file="$2"; local pattern="$3"
  if grep -qE "$pattern" "$file" 2>/dev/null; then
    _pass "$label"
  else
    _fail "$label  (pattern: '$pattern' not found in $file)"
  fi
}

_check_warn() {
  local label="$1"; shift
  if "$@" &>/dev/null 2>&1; then
    _pass "$label"
  else
    _warn "$label  (non-blocking)"
  fi
}

# ─────────────────────────────────────────────────────────────────────────────
# Variables
# ─────────────────────────────────────────────────────────────────────────────
NODE_BIN="$REPO_ROOT/target/release/x3-chain-node"
WASM_PATH="$REPO_ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
ZOMBIENET_BIN="${ZOMBIENET_BIN:-$(which zombienet 2>/dev/null || ls ~/.local/bin/zombienet 2>/dev/null || echo '')}"
ZOMBIENET_CONFIG="$REPO_ROOT/zombienet/x3-local-testnet.toml"
DSSL_FILE="$REPO_ROOT/zombienet/x3-assertions.zndsl"
CHOPSTICKS_CONFIG="$REPO_ROOT/chopsticks/x3-dev.yml"
SRTOOL_IMAGE="${SRTOOL_IMAGE:-paritytech/srtool:1.75.0}"
RUNTIME_TOML="$REPO_ROOT/runtime/Cargo.toml"

declare -A PALLET_WEIGHTS=(
  ["pallet-x3-atomic-kernel"]="pallets/x3-atomic-kernel/src/weights.rs"
  ["pallet-x3-settlement-engine"]="pallets/x3-settlement-engine/src/weights.rs"
  ["pallet-cross-chain-validator"]="pallets/cross-chain-validator/src/weights.rs"
  ["pallet-x3-slash"]="pallets/x3-slash/src/weights.rs"
)

declare -A PALLET_BENCHMARKING=(
  ["pallet-x3-atomic-kernel"]="pallets/x3-atomic-kernel/src/benchmarking.rs"
  ["pallet-x3-settlement-engine"]="pallets/x3-settlement-engine/src/benchmarking.rs"
  ["pallet-cross-chain-validator"]="pallets/cross-chain-validator/src/benchmarking.rs"
  ["pallet-x3-slash"]="pallets/x3-slash/src/benchmarking.rs"
)

# ─────────────────────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  X3_ATOMIC_STAR — Substrate Tooling CI Test Suite${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"

# ─────────────────────────────────────────────────────────────────────────────
# 1. Node Binary
# ─────────────────────────────────────────────────────────────────────────────
_section "1. Node Binary"

_check "x3-chain-node binary exists" test -f "$NODE_BIN"

if [[ -f "$NODE_BIN" ]]; then
  _check "Node has 'benchmark' subcommand (runtime-benchmarks feature)" \
    "$NODE_BIN" benchmark --help
  _check "Node has 'try-runtime' subcommand (try-runtime feature)" \
    "$NODE_BIN" try-runtime --help
  echo -n "  Checking: node --version … "
  ver=$("$NODE_BIN" --version 2>&1 | head -1 || echo "(none)")
  echo -e "${GREEN}✓${NC} $ver"
  PASS=$((PASS + 1))
else
  _fail "Node subcommand checks skipped (binary not built)"
  _fail "try-runtime subcommand check skipped"
fi

# ─────────────────────────────────────────────────────────────────────────────
# 2. WASM Binary
# ─────────────────────────────────────────────────────────────────────────────
_section "2. WASM Binary"

_check "Compressed WASM exists" test -f "$WASM_PATH"

if [[ -f "$WASM_PATH" ]]; then
  # Magic bytes: 00 61 73 6d = \0asm
  echo -n "  Checking: WASM magic bytes (\\x00asm) … "
  magic=$(xxd -p -l4 "$WASM_PATH" 2>/dev/null || \
    hexdump -e '1/1 "%02x"' -n4 "$WASM_PATH" 2>/dev/null || echo "")
  if [[ "$magic" == "0061736d" ]]; then
    echo -e "${GREEN}✓${NC} magic=$magic"
    PASS=$((PASS + 1))
  else
    echo -e "${RED}✗${NC} expected 0061736d, got: $magic"
    FAIL=$((FAIL + 1))
  fi

  # Size check: >100KB, <20MB (compressed)
  wasm_size=$(wc -c < "$WASM_PATH")
  echo -n "  Checking: WASM size (100KB–20MB compressed) … "
  if [[ $wasm_size -gt 102400 && $wasm_size -lt 20971520 ]]; then
    echo -e "${GREEN}✓${NC} $(( wasm_size / 1024 )) KB"
    PASS=$((PASS + 1))
  else
    echo -e "${RED}✗${NC} size=${wasm_size} bytes (outside expected range)"
    FAIL=$((FAIL + 1))
  fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# 3. try-runtime
# ─────────────────────────────────────────────────────────────────────────────
_section "3. try-runtime"

if [[ -f "$NODE_BIN" ]]; then
  _check "try-runtime --help exits 0" "$NODE_BIN" try-runtime --help
fi

mkdir -p "$REPO_ROOT/.try-runtime-snapshots"
_check "Snapshot directory writable" test -w "$REPO_ROOT/.try-runtime-snapshots"

# ─────────────────────────────────────────────────────────────────────────────
# 4. Zombienet
# ─────────────────────────────────────────────────────────────────────────────
_section "4. Zombienet"

if [[ -n "$ZOMBIENET_BIN" && -x "$ZOMBIENET_BIN" ]]; then
  _pass "zombienet binary found: $ZOMBIENET_BIN"
  _check "zombienet --version" "$ZOMBIENET_BIN" --version
else
  _fail "zombienet binary not found (install: npm install -g @zombienet/cli)"
fi

_check "Zombienet TOML config exists"    test -f "$ZOMBIENET_CONFIG"
_check_grep "TOML has relaychain section" "$ZOMBIENET_CONFIG" '\[relaychain\]|\[\[relaychain'
_check_grep "TOML has Alice validator"    "$ZOMBIENET_CONFIG" 'alice|Alice'
_check_grep "TOML has Bob validator"      "$ZOMBIENET_CONFIG" 'bob|Bob'
_check_grep "TOML has Charlie validator"  "$ZOMBIENET_CONFIG" 'charlie|Charlie'

# DSSL assertions
if [[ -f "$DSSL_FILE" ]]; then
  _pass "DSSL assertions file exists"
  _check_grep "DSSL has 'alice: is up'"    "$DSSL_FILE" 'alice: is up'
  _check_grep "DSSL has finality check"    "$DSSL_FILE" 'finalized block'
  _check_grep "DSSL has peers check"       "$DSSL_FILE" 'peers count'
else
  _warn "DSSL assertions file not found — will be generated on first run"
fi

# ─────────────────────────────────────────────────────────────────────────────
# 5. Chopsticks
# ─────────────────────────────────────────────────────────────────────────────
_section "5. Chopsticks"

if command -v chopsticks &>/dev/null; then
  _pass "chopsticks binary found: $(which chopsticks)"
elif command -v npx &>/dev/null; then
  _pass "npx available (will use npx @acala-network/chopsticks@latest)"
else
  _fail "Neither chopsticks nor npx found"
fi

_check "Chopsticks YAML config exists"       test -f "$CHOPSTICKS_CONFIG"
_check_grep "Config has endpoint field"      "$CHOPSTICKS_CONFIG" 'endpoint'
_check_grep "Config has port field"          "$CHOPSTICKS_CONFIG" 'port'
_check_grep "Config points to port 9944"     "$CHOPSTICKS_CONFIG" '9944'

_check "curl available (RPC calls)"     command -v curl
_check "python3 available (JSON fmt)"   command -v python3

# ─────────────────────────────────────────────────────────────────────────────
# 6. FRAME Benchmarks
# ─────────────────────────────────────────────────────────────────────────────
_section "6. FRAME Benchmarks"

if [[ -f "$NODE_BIN" ]]; then
  _check "Node has 'benchmark pallet' subcommand" \
    "$NODE_BIN" benchmark pallet --help
else
  _fail "benchmark pallet check skipped (node binary not built)"
fi

# weights.rs files
for pallet in "${!PALLET_WEIGHTS[@]}"; do
  wfile="$REPO_ROOT/${PALLET_WEIGHTS[$pallet]}"
  if [[ ! -f "$wfile" ]]; then
    _fail "weights.rs missing: $pallet"
    continue
  fi
  wsize=$(wc -c < "$wfile")
  if [[ $wsize -lt 100 ]]; then
    _fail "weights.rs too small (empty stub?): $pallet  (${wsize} bytes)"
    continue
  fi
  if ! grep -qE 'WeightInfo|impl.*Weight' "$wfile"; then
    _fail "weights.rs has no WeightInfo impl: $pallet"
    continue
  fi
  fn_count=$(grep -cE 'fn [a-z_]+.*->.*Weight|RefTime|Weight::from_parts' "$wfile" 2>/dev/null || echo 0)
  _pass "weights.rs valid: $pallet  (${fn_count} weight fns, $(( wsize / 1024 ))KB)"
done

# benchmarking.rs files
for pallet in "${!PALLET_BENCHMARKING[@]}"; do
  _check "benchmarking.rs exists: $pallet" \
    test -f "$REPO_ROOT/${PALLET_BENCHMARKING[$pallet]}"
done

# ─────────────────────────────────────────────────────────────────────────────
# 7. srtool
# ─────────────────────────────────────────────────────────────────────────────
_section "7. srtool / Docker"

_check "Docker daemon running" docker info

echo -n "  Checking: srtool Docker image ($SRTOOL_IMAGE) … "
if docker image inspect "$SRTOOL_IMAGE" &>/dev/null; then
  echo -e "${GREEN}✓${NC} cached"
  PASS=$((PASS + 1))
else
  echo -e "${YELLOW}⚠${NC} not cached — will pull on first 'srtool build'"
  WARN=$((WARN + 1))
fi

_check "Runtime Cargo.toml exists" test -f "$RUNTIME_TOML"
_check_grep "Runtime package name = 'x3-chain-runtime'" \
  "$RUNTIME_TOML" '^name.*=.*"x3-chain-runtime"'

# srtool report (optional)
LATEST_REPORT="$REPO_ROOT/.srtool-reports/latest.json"
if [[ -f "$LATEST_REPORT" ]]; then
  _pass "srtool report exists: $LATEST_REPORT"
  if command -v python3 &>/dev/null; then
    b2=$(python3 -c "
import json
try:
    d=json.load(open('$LATEST_REPORT'))
    print(d.get('runtimes',{}).get('compact',{}).get('blake2_256','NOT_FOUND'))
except Exception as e:
    print('ERROR:'+str(e))
" 2>/dev/null || echo "")
    if [[ "$b2" == "NOT_FOUND" || "$b2" == ERROR* || -z "$b2" ]]; then
      _warn "srtool report exists but blake2_256 field missing"
    else
      _pass "srtool report has blake2_256: ${b2:0:32}…"
    fi
  fi
else
  _warn "No srtool report found (run './scripts/run-srtool.sh build' to generate)"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Summary
# ─────────────────────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  SUBSTRATE TOOLING CI RESULTS${NC}"
echo "───────────────────────────────────────────────────────────────"
echo -e "  ${GREEN}✓ PASSED:${NC}  $PASS"
echo -e "  ${RED}✗ FAILED:${NC}  $FAIL"
echo -e "  ${YELLOW}⚠ WARNED:${NC}  $WARN"
echo -e "${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

if [[ $FAIL -eq 0 ]]; then
  echo -e "${GREEN}${BOLD}✅ ALL SUBSTRATE TOOLING CHECKS PASSED${NC}"
  echo ""
  if [[ $WARN -gt 0 ]]; then
    echo -e "${YELLOW}  $WARN non-blocking warning(s) — review output above${NC}"
    echo ""
  fi
  exit 0
else
  echo -e "${RED}${BOLD}❌ $FAIL FAILURE(S) DETECTED — review output above${NC}"
  echo ""
  echo "  Common fixes:"
  echo "    • Binary missing:  cargo build --release -p x3-chain-node"
  echo "    • Benchmarks gap:  ./scripts/run-frame-benchmarks.sh build"
  echo "    • Weights missing: ./scripts/run-frame-benchmarks.sh run"
  echo "    • WASM missing:    cargo build --release -p x3-chain-node"
  echo ""
  exit 1
fi
