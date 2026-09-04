#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-srtool.sh — Deterministic reproducible runtime WASM via srtool
#
# srtool compiles x3-chain-runtime inside a pinned Docker image so ANY
# builder on ANY host produces bit-for-bit identical WASM.  The output JSON
# contains SHA256 + BLAKE2 hashes that governance can verify on-chain.
#
# Modes:
#   ./scripts/run-srtool.sh build       — build WASM + produce JSON report
#   ./scripts/run-srtool.sh verify      — verify last build's hash matches
#   ./scripts/run-srtool.sh info        — show srtool version + Docker image
#   ./scripts/run-srtool.sh help        — show this help
#
# Requirements:
#   • Docker daemon running
#   • srtool CLI installed  (cargo install --git https://github.com/chevdor/srtool-cli)
#   • OR: docker pull paritytech/srtool:1.75.0
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE="${SRTOOL_PACKAGE:-x3-chain-runtime}"
RUNTIME_DIR="${SRTOOL_RUNTIME_DIR:-runtime}"
REPORT_DIR="$REPO_ROOT/.srtool-reports"
REPORT_FILE="$REPORT_DIR/srtool-$(date +%Y%m%d-%H%M%S).json"
LATEST_REPORT="$REPORT_DIR/latest.json"

# srtool Docker image — pin to the Rust toolchain you target
SRTOOL_IMAGE="${SRTOOL_IMAGE:-paritytech/srtool:1.75.0}"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${CYAN}[srtool]${NC} $*"; }
success() { echo -e "${GREEN}[srtool]  ✓${NC} $*"; }
warn()    { echo -e "${YELLOW}[srtool] ⚠${NC} $*"; }
die()     { echo -e "${RED}[srtool] ✗ ERROR:${NC} $*" >&2; exit 1; }

print_help() {
  cat <<EOF

  X3 srtool — deterministic reproducible runtime WASM

  USAGE:
    ./scripts/run-srtool.sh [COMMAND]

  COMMANDS:
    build     Compile runtime in pinned Docker image; emit JSON hash report
    verify    Check that current codebase produces the same WASM as latest.json
    info      Print srtool + Docker image version info
    help      Show this message

  ENVIRONMENT VARIABLES:
    SRTOOL_PACKAGE      Runtime crate name       (default: $PACKAGE)
    SRTOOL_RUNTIME_DIR  Relative path to runtime (default: $RUNTIME_DIR)
    SRTOOL_IMAGE        Docker image + tag        (default: $SRTOOL_IMAGE)

  OUTPUT FILES:
    .srtool-reports/latest.json          — most recent hash report (symlink)
    .srtool-reports/srtool-YYYYMMDD.json — timestamped report archive
    target/release/wbuild/$PACKAGE/      — compiled WASM artifacts

  VERIFY A RUNTIME UPGRADE:
    1. Run: ./scripts/run-srtool.sh build
    2. Grab the 'blake2_256' from .srtool-reports/latest.json
    3. Compare to setCode proposal hash in governance

EOF
}

check_docker() {
  command -v docker &>/dev/null || die "Docker not found. Install Docker Engine: https://docs.docker.com/engine/install"
  docker info &>/dev/null       || die "Docker daemon not running. Start Docker and retry."
  info "Docker OK ($(docker --version | cut -d' ' -f3 | tr -d ','))"
}

check_srtool() {
  if command -v srtool &>/dev/null; then
    info "srtool CLI: $(srtool version 2>/dev/null | head -1 || echo 'installed')"
    echo "srtool"
  else
    warn "srtool CLI not found; using Docker directly (equivalent output)"
    echo "docker"
  fi
}

print_report_summary() {
  local report="$1"
  [[ -f "$report" ]] || return

  echo ""
  echo "  ═══════════════════════════════════════════════════════════════"
  echo "   srtool BUILD REPORT"
  echo "  ═══════════════════════════════════════════════════════════════"

  # Parse key fields from JSON
  if command -v python3 &>/dev/null; then
    python3 - "$report" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)

# srtool JSON schema has top-level keys: runtimes, context, etc.
runtimes = d.get("runtimes", {})
compact  = runtimes.get("compact", {})
blake2   = compact.get("blake2_256", "N/A")
sha256   = compact.get("sha256", "N/A")
ipfs     = compact.get("ipfs", "N/A")
wasm     = compact.get("wasm", "N/A")
rustc    = d.get("context", {}).get("rustc", "N/A")
pkg      = d.get("context", {}).get("package", "N/A")

print(f"  Package      : {pkg}")
print(f"  rustc        : {rustc}")
print(f"  WASM path    : {wasm}")
print(f"  BLAKE2-256   : {blake2}")
print(f"  SHA-256      : {sha256}")
print(f"  IPFS         : {ipfs}")
PY
  else
    cat "$report"
  fi

  echo "  ═══════════════════════════════════════════════════════════════"
  echo ""
}

cmd_build() {
  check_docker
  mkdir -p "$REPORT_DIR"

  local mode
  mode=$(check_srtool)

  if [[ "$mode" == "srtool" ]]; then
    # ── srtool CLI path ──────────────────────────────────────────────────
    info "Building $PACKAGE with srtool CLI …"
    info "Image: $SRTOOL_IMAGE"
    info "(first run: Docker pull ~2 GB; subsequent runs use cache)"
    echo ""

    cd "$REPO_ROOT"
    srtool build \
      --app \
      --json \
      -p "$PACKAGE" \
      --runtime-dir "$RUNTIME_DIR" \
      2>&1 | tee "$REPORT_FILE"

  else
    # ── Raw Docker path ──────────────────────────────────────────────────
    info "Building $PACKAGE with Docker srtool directly …"
    info "Image: $SRTOOL_IMAGE"
    echo ""

    cd "$REPO_ROOT"
    docker run \
      --rm \
      -e PACKAGE="$PACKAGE" \
      -e RUNTIME_DIR="$RUNTIME_DIR" \
      -e VERBOSE=1 \
      -v "$(pwd)":/build \
      "$SRTOOL_IMAGE" \
      build \
      2>&1 | tee "$REPORT_FILE"
  fi

  local exit_code=${PIPESTATUS[0]}
  if [[ $exit_code -ne 0 ]]; then
    die "srtool build FAILED. Check output above."
  fi

  # Symlink latest
  ln -sf "$REPORT_FILE" "$LATEST_REPORT"

  success "srtool build COMPLETE"
  print_report_summary "$LATEST_REPORT"

  # Print WASM artifact location
  local wasm_path="$REPO_ROOT/target/release/wbuild/${PACKAGE}/${PACKAGE//-/_}.compact.compressed.wasm"
  if [[ -f "$wasm_path" ]]; then
    echo "  WASM artifact: $wasm_path"
    echo "  WASM size:     $(du -sh "$wasm_path" | cut -f1)"
  fi
}

cmd_verify() {
  [[ -f "$LATEST_REPORT" ]] || die "No previous build found. Run './scripts/run-srtool.sh build' first."

  info "Rebuilding to verify determinism against $LATEST_REPORT …"
  check_docker

  local prev_blake2
  prev_blake2=$(python3 -c "
import json
with open('$LATEST_REPORT') as f:
    d = json.load(f)
print(d.get('runtimes',{}).get('compact',{}).get('blake2_256','MISSING'))
" 2>/dev/null || echo "MISSING")

  [[ "$prev_blake2" == "MISSING" ]] && die "Cannot read blake2_256 from $LATEST_REPORT"
  info "Previous BLAKE2-256: $prev_blake2"

  local tmp_report="$REPORT_DIR/verify-$(date +%Y%m%d-%H%M%S).json"
  local mode
  mode=$(check_srtool)
  cd "$REPO_ROOT"

  if [[ "$mode" == "srtool" ]]; then
    srtool build --app --json -p "$PACKAGE" --runtime-dir "$RUNTIME_DIR" 2>&1 | tee "$tmp_report"
  else
    docker run --rm -e PACKAGE="$PACKAGE" -e RUNTIME_DIR="$RUNTIME_DIR" \
      -v "$(pwd)":/build "$SRTOOL_IMAGE" build 2>&1 | tee "$tmp_report"
  fi

  local new_blake2
  new_blake2=$(python3 -c "
import json
with open('$tmp_report') as f:
    d = json.load(f)
print(d.get('runtimes',{}).get('compact',{}).get('blake2_256','MISSING'))
" 2>/dev/null || echo "MISSING")

  echo ""
  echo "  Previous BLAKE2-256: $prev_blake2"
  echo "  New      BLAKE2-256: $new_blake2"
  echo ""

  if [[ "$prev_blake2" == "$new_blake2" ]]; then
    success "DETERMINISM VERIFIED — builds match!"
  else
    die "BUILDS DIFFER — non-determinism detected! Investigate Cargo.lock or nondeterministic code."
  fi
}

cmd_info() {
  check_docker
  info "srtool version:"
  command -v srtool &>/dev/null && srtool version || echo "  (srtool CLI not installed)"

  info "Docker image info:"
  docker images "$SRTOOL_IMAGE" 2>/dev/null || warn "Image not pulled yet (will pull on first build)"

  info "Runtime package:    $PACKAGE"
  info "Runtime directory:  $RUNTIME_DIR"

  local wasm_path="$REPO_ROOT/target/release/wbuild/${PACKAGE}/${PACKAGE//-/_}.compact.compressed.wasm"
  if [[ -f "$wasm_path" ]]; then
    info "Existing WASM:      $wasm_path ($(du -sh "$wasm_path" | cut -f1))"
  else
    info "Existing WASM:      none (run 'build')"
  fi

  if [[ -f "$LATEST_REPORT" ]]; then
    info "Last build report:  $LATEST_REPORT"
    print_report_summary "$LATEST_REPORT"
  fi
}

# ─────────────────────────────────────────────────────────────────────────────
# test — CI-friendly non-interactive validation suite
#   1. Docker is running
#   2. srtool image is available (or can be pulled)
#   3. WASM binary exists and has correct magic bytes
#   4. WASM size is within expected range
#   5. srtool report exists (if previously built)
#   6. Report has expected JSON fields
#   7. Runtime Cargo.toml has correct package name
# ─────────────────────────────────────────────────────────────────────────────
cmd_test() {
  local PASS=0 FAIL=0

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo "  srtool — CI Test Suite"
  echo "══════════════════════════════════════════════════════"
  echo ""

  _check() {
    local label="$1"; shift
    echo -n "  Checking: $label … "
    if "$@" &>/dev/null 2>&1; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC}"
      FAIL=$((FAIL + 1))
    fi
  }

  # 1. Docker daemon running
  _check "Docker daemon running" docker info

  # 2. srtool image available locally (non-fatal if not pulled yet)
  echo -n "  Checking: srtool Docker image ($SRTOOL_IMAGE) … "
  if docker image inspect "$SRTOOL_IMAGE" &>/dev/null; then
    echo -e "${GREEN}✓ PASS${NC} (cached)"
    PASS=$((PASS + 1))
  else
    echo -e "${YELLOW}⚠ WARN${NC} (not cached — will pull on first build)"
  fi

  # 3. WASM binary exists
  local wasm_path="$REPO_ROOT/target/release/wbuild/${PACKAGE}/${PACKAGE//-/_}.compact.compressed.wasm"
  _check "Compressed WASM exists" test -f "$wasm_path"

  if [[ -f "$wasm_path" ]]; then
    # 4. WASM magic bytes
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

    # 5. WASM size: >100KB <20MB
    local wasm_size
    wasm_size=$(wc -c < "$wasm_path")
    echo -n "  Checking: WASM size (100KB–20MB) … "
    if [[ $wasm_size -gt 102400 && $wasm_size -lt 20971520 ]]; then
      echo -e "${GREEN}✓ PASS${NC} ($(( wasm_size / 1024 )) KB)"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC} (size: ${wasm_size} bytes)"
      FAIL=$((FAIL + 1))
    fi
  fi

  # 6. srtool report exists (if a build was done)
  mkdir -p "$REPORT_DIR"
  if [[ -f "$LATEST_REPORT" ]]; then
    _check "Latest srtool report readable" test -r "$LATEST_REPORT"

    # 7. Report has blake2_256 field
    echo -n "  Checking: report has blake2_256 hash … "
    if command -v python3 &>/dev/null && python3 -c "
import json,sys
d=json.load(open('$LATEST_REPORT'))
assert d.get('runtimes',{}).get('compact',{}).get('blake2_256')
" 2>/dev/null; then
      local b2
      b2=$(python3 -c "import json; d=json.load(open('$LATEST_REPORT')); print(d['runtimes']['compact']['blake2_256'])")
      echo -e "${GREEN}✓ PASS${NC} ($b2)"
      PASS=$((PASS + 1))
    else
      echo -e "${YELLOW}⚠ WARN${NC} (report exists but blake2_256 field not found)"
    fi
  else
    echo -e "  ${YELLOW}⚠ SKIP${NC}: No srtool report found (run './scripts/run-srtool.sh build' first)"
  fi

  # 8. Runtime Cargo.toml has correct package name
  local rt_toml="$REPO_ROOT/$RUNTIME_DIR/Cargo.toml"
  echo -n "  Checking: runtime package name matches $PACKAGE … "
  if grep -q "^name.*=.*\"$PACKAGE\"" "$rt_toml" 2>/dev/null; then
    echo -e "${GREEN}✓ PASS${NC}"
    PASS=$((PASS + 1))
  else
    echo -e "${RED}✗ FAIL${NC} (package name mismatch in $rt_toml)"
    FAIL=$((FAIL + 1))
  fi

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo -e "  Results: ${GREEN}✓ $PASS PASSED${NC}  |  ${RED}✗ $FAIL FAILED${NC}"
  echo "══════════════════════════════════════════════════════"
  echo ""

  [[ $FAIL -eq 0 ]] && success "All srtool CI tests passed." || die "$FAIL test(s) failed."
}

# ─────────── dispatch ──────────────────────────────────────────────────────
COMMAND="${1:-help}"
case "$COMMAND" in
  build)          cmd_build ;;
  verify)         cmd_verify ;;
  info)           cmd_info ;;
  test)           cmd_test ;;
  help|--help|-h) print_help ;;
  *)              warn "Unknown command: $COMMAND"; print_help; exit 1 ;;
esac
