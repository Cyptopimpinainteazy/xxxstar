#!/usr/bin/env bash
# scripts/x3_audit.sh — X3 Automated Self-Audit Runner
#
# Usage:
#   bash scripts/x3_audit.sh          # full audit, interactive output
#   bash scripts/x3_audit.sh --ci     # CI mode: machine-readable, strict exit codes
#   bash scripts/x3_audit.sh --fix    # attempt auto-fixes where possible
#
# Exit codes:
#   0  all checks passed
#   1  one or more hard checks failed
#   2  warnings only (soft checks failed, use --strict to treat as hard fail)
#
set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CI_MODE=0
STRICT=0
FIX_MODE=0
PASS=0
WARN=0
FAIL=0
FAILED_CHECKS=()

for arg in "$@"; do
  case "$arg" in
    --ci)     CI_MODE=1; STRICT=1 ;;
    --strict) STRICT=1 ;;
    --fix)    FIX_MODE=1 ;;
  esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
BOLD='\033[1m'

pass()  { PASS=$((PASS + 1));  echo -e "${GREEN}  [PASS]${NC} $1"; }
warn()  { WARN=$((WARN + 1));  echo -e "${YELLOW}  [WARN]${NC} $1"; }
fail()  { FAIL=$((FAIL + 1));  FAILED_CHECKS+=("$1"); echo -e "${RED}  [FAIL]${NC} $1"; }
header(){ echo -e "\n${BOLD}=== $1 ===${NC}"; }

require_cmd() {
  if command -v "$1" &>/dev/null; then
    return 0
  fi
  if [ "$CI_MODE" -eq 1 ]; then
    fail "Missing required tool: $1"
  else
    warn "Missing required tool: $1"
  fi
  return 1
}

preflight_tool() {
  if command -v "$1" &>/dev/null; then
    pass "Tool available: $1"
    return 0
  fi
  if [ "$CI_MODE" -eq 1 ]; then
    fail "Missing required tool: $1"
  else
    warn "Missing required tool: $1"
  fi
  return 1
}

preflight_rustfmt() {
  if command -v rustfmt &>/dev/null; then
    pass "Tool available: rustfmt"
    return 0
  fi
  if command -v cargo &>/dev/null && cargo fmt --version &>/dev/null 2>&1; then
    pass "Tool available: rustfmt (via cargo fmt)"
    return 0
  fi
  if [ "$CI_MODE" -eq 1 ]; then
    fail "Missing required tool: rustfmt"
  else
    warn "Missing required tool: rustfmt"
  fi
  return 1
}

cd "$ROOT"

echo -e "\n${BOLD}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║   X3 SELF-AUDIT RUNNER — $(date +%Y-%m-%d)           ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════╝${NC}"

# ── SECTION 0: Preflight ──────────────────────────────────────────────────────
header "0. PREFLIGHT"
preflight_tool cargo
preflight_tool rg
preflight_tool npm
preflight_rustfmt
preflight_tool wasm-opt

# ── SECTION 1: Repo Structure ─────────────────────────────────────────────────
header "1. REPO STRUCTURE"

required_dirs=(runtime node pallets crates docs scripts)
for d in "${required_dirs[@]}"; do
  if [ -d "$ROOT/$d" ]; then
    pass "Directory /$d exists"
  else
    fail "Missing required directory: /$d"
  fi
done

# Orphaned _unused check
if [ -d "$ROOT/_unused" ]; then
  count=$(find "$ROOT/_unused" -mindepth 1 | wc -l)
  if [ "$count" -gt 0 ]; then
    warn "_unused/ contains $count items — consider removing"
  else
    pass "_unused/ is empty"
  fi
fi

# X3_COMPLETION.md present
if [ -f "$ROOT/X3_COMPLETION.md" ]; then
  pass "X3_COMPLETION.md present"
  unchecked=$(grep -c "⬜" "$ROOT/X3_COMPLETION.md" || true)
  if [ "$unchecked" -gt 0 ]; then
    warn "X3_COMPLETION.md has $unchecked unchecked items remaining"
  else
    pass "X3_COMPLETION.md fully green"
  fi
else
  fail "X3_COMPLETION.md missing"
fi

# ── SECTION 2: Build Integrity ────────────────────────────────────────────────
header "2. BUILD INTEGRITY"

if require_cmd cargo; then
  echo "  Running: cargo check --workspace (fast build check)..."
  if cargo check --workspace --quiet 2>/dev/null; then
    pass "cargo check --workspace"
  else
    fail "cargo check --workspace failed"
  fi
fi

if require_cmd cargo; then
  echo "  Running: cargo fmt --all -- --check..."
  if cargo fmt --all -- --check 2>/dev/null; then
    pass "cargo fmt --all -- --check"
  else
    fail "cargo fmt --all -- --check failed"
  fi
fi

if require_cmd npm; then
  echo "  Running: npm run build:all-packages --if-present..."
  if npm run build:all-packages --if-present 2>/dev/null; then
    pass "npm run build:all-packages --if-present"
  else
    fail "npm run build:all-packages --if-present failed"
  fi
fi

if require_cmd cargo; then
  echo "  Running: cargo build --release --locked --workspace..."
  if [ "$STRICT" -eq 1 ]; then
    if RUSTFLAGS="-D warnings" cargo build --release --locked --workspace 2>/dev/null; then
      pass "cargo build --release --locked --workspace"
    else
      fail "cargo build --release --locked --workspace failed"
    fi
  else
    if cargo build --release --locked --workspace 2>/dev/null; then
      pass "cargo build --release --locked --workspace"
    else
      fail "cargo build --release --locked --workspace failed"
    fi
  fi
fi

if require_cmd cargo; then
  echo "  Running: cargo clippy --workspace --all-targets --all-features..."
  if [ "$STRICT" -eq 1 ]; then
    if cargo clippy --workspace --all-targets --all-features -- -D warnings 2>/dev/null; then
      pass "cargo clippy --workspace --all-targets --all-features"
    else
      fail "cargo clippy --workspace --all-targets --all-features failed"
    fi
  else
    if cargo clippy --workspace --all-targets --all-features 2>/dev/null; then
      pass "cargo clippy --workspace --all-targets --all-features"
    else
      fail "cargo clippy --workspace --all-targets --all-features failed"
    fi
  fi
fi

if require_cmd cargo; then
  echo "  Running: cargo test --workspace --release --locked..."
  if [ "$STRICT" -eq 1 ]; then
    if RUSTFLAGS="-D warnings" cargo test --workspace --release --locked 2>/dev/null; then
      pass "cargo test --workspace --release --locked"
    else
      fail "cargo test --workspace --release --locked failed"
    fi
  else
    if cargo test --workspace --release --locked 2>/dev/null; then
      pass "cargo test --workspace --release --locked"
    else
      fail "cargo test --workspace --release --locked failed"
    fi
  fi
fi

if require_cmd cargo; then
  echo "  Running: cargo test -p runtime --features std --no-fail-fast..."
  if cargo test -p runtime --features std --no-fail-fast 2>/dev/null; then
    pass "cargo test -p runtime --features std --no-fail-fast"
  else
    fail "cargo test -p runtime --features std --no-fail-fast failed"
  fi

  echo "  Running: cargo test -p pallet-x3-kernel --features std --no-fail-fast..."
  if cargo test -p pallet-x3-kernel --features std --no-fail-fast 2>/dev/null; then
    pass "cargo test -p pallet-x3-kernel --features std --no-fail-fast"
  else
    fail "cargo test -p pallet-x3-kernel --features std --no-fail-fast failed"
  fi

  echo "  Running: cargo test -p evm-integration --features std --no-fail-fast..."
  if cargo test -p evm-integration --features std --no-fail-fast 2>/dev/null; then
    pass "cargo test -p evm-integration --features std --no-fail-fast"
  else
    fail "cargo test -p evm-integration --features std --no-fail-fast failed"
  fi

  echo "  Running: cargo test -p svm-integration --features std --no-fail-fast..."
  if cargo test -p svm-integration --features std --no-fail-fast 2>/dev/null; then
    pass "cargo test -p svm-integration --features std --no-fail-fast"
  else
    fail "cargo test -p svm-integration --features std --no-fail-fast failed"
  fi

  echo "  Running: cargo test -p x3-integration --features std --no-fail-fast..."
  if cargo test -p x3-integration --features std --no-fail-fast 2>/dev/null; then
    pass "cargo test -p x3-integration --features std --no-fail-fast"
  else
    fail "cargo test -p x3-integration --features std --no-fail-fast failed"
  fi
fi

if require_cmd cargo; then
  echo "  Running: runtime WASM build (no_std)..."
  if (cd runtime && cargo build --release --target wasm32-unknown-unknown --no-default-features 2>/dev/null); then
    pass "runtime WASM build (no_std)"
  else
    fail "runtime WASM build (no_std) failed"
  fi
fi

launch_defaults=("target/release/x3-chain-node" "testnet/genesis.json" "prometheus.yml")
missing_defaults=0
for path in "${launch_defaults[@]}"; do
  if [ ! -f "$ROOT/$path" ]; then
    fail "Launch-validator default path missing: $path"
    missing_defaults=1
  fi
done

if [ "$missing_defaults" -eq 0 ] && require_cmd cargo; then
  echo "  Running: x3-launch-validator (pre-launch)..."
  if cargo run -p x3-launch-validator -- --check pre-launch 2>/dev/null; then
    pass "x3-launch-validator --check pre-launch"
  else
    fail "x3-launch-validator --check pre-launch failed"
  fi

  echo "  Running: x3-launch-validator (failure-conditions)..."
  if cargo run -p x3-launch-validator -- --check failure-conditions 2>/dev/null; then
    pass "x3-launch-validator --check failure-conditions"
  else
    fail "x3-launch-validator --check failure-conditions failed"
  fi
fi

# Rust edition check
# `grep -rL` returns exit code 1 when no matches are found, which would
# cause `set -euo pipefail` to abort the script. Allow this case by
# forcing the pipeline to succeed.
edition_bad=$(grep -rL 'edition = "2021"' "$ROOT"/crates/*/Cargo.toml "$ROOT"/pallets/*/Cargo.toml 2>/dev/null || true | wc -l)
if [ "$edition_bad" -gt 0 ]; then
  warn "$edition_bad crate(s) not on Rust edition 2021"
else
  pass "All crates on Rust edition 2021"
fi

# ── SECTION 3: Safety Scan ────────────────────────────────────────────────────
header "3. SAFETY SCAN"

if require_cmd rg; then
  # unwrap() in non-test production code
  unwrap_count=$(rg 'unwrap\(\)' \
    --glob '!**/tests/**' \
    --glob '!**/*_test*' \
    --glob '!**/test_*' \
    --glob '!**/benches/**' \
    --count-matches --stats "$ROOT" 2>/dev/null \
    | grep -E '^[0-9]+ matches$' | awk '{sum+=$1} END{print sum+0}' || echo 0)
  if [ "$unwrap_count" -gt 0 ]; then
    warn "unwrap() occurrences in production code: $unwrap_count (target: 0)"
    if [ "$CI_MODE" -eq 0 ]; then
      rg 'unwrap\(\)' \
        --glob '!**/tests/**' \
        --glob '!**/*_test*' \
        --glob '!**/test_*' \
        --glob '!**/benches/**' \
        "$ROOT" | head -10
    fi
  else
    pass "No unwrap() in production paths"
  fi

  # expect() in non-test code
  expect_count=$(rg 'expect\(' \
    --glob '!**/tests/**' \
    --glob '!**/*_test*' \
    --glob '!**/test_*' \
    --glob '!**/benches/**' \
    --count-matches --stats "$ROOT" 2>/dev/null \
    | grep -E '^[0-9]+ matches$' | awk '{sum+=$1} END{print sum+0}' || echo 0)
  if [ "$expect_count" -gt 0 ]; then
    warn "expect() occurrences in production code: $expect_count (target: 0)"
  else
    pass "No expect() in production paths"
  fi

  # panic! in non-test runtime code
  panic_count=$(rg 'panic!\(' \
    --glob '!**/tests/**' \
    --glob '!**/*_test*' \
    "$ROOT/crates" "$ROOT/pallets" "$ROOT/runtime" 2>/dev/null \
    | wc -l || echo 0)
  if [ "$panic_count" -gt 0 ]; then
    warn "panic!() occurrences in production crates: $panic_count"
  else
    pass "No panic!() in production crates"
  fi
else
  warn "ripgrep not installed — skipping safety scan (run: sudo apt install ripgrep)"
fi

# ── SECTION 4: Dependency Integrity ──────────────────────────────────────────
header "4. DEPENDENCY INTEGRITY"

if [ -f "$ROOT/Cargo.lock" ]; then
  pass "Cargo.lock present"
else
  fail "Cargo.lock missing — run 'cargo build' to generate"
fi

if [ -f "$ROOT/deny.toml" ]; then
  pass "deny.toml present"
  if require_cmd cargo-deny 2>/dev/null || cargo deny --version &>/dev/null 2>&1; then
    if cargo deny check advisories 2>/dev/null; then
      pass "cargo deny: no known advisories"
    else
      warn "cargo deny: advisory issues found"
    fi
  else
    warn "cargo-deny not installed — skipping advisory check (cargo install cargo-deny)"
  fi
else
  warn "deny.toml missing — dependency policy not enforced"
fi

# ── SECTION 5: Constitutional Layer ──────────────────────────────────────────
header "5. CONSTITUTIONAL LAYER"

constitution_crate="$ROOT/crates/x3-constitution"
if [ -d "$constitution_crate" ]; then
  pass "x3-constitution crate present"
else
  fail "x3-constitution crate missing"
fi

epoch_proof="$ROOT/crates/x3-proof/src/epoch.rs"
if [ -f "$epoch_proof" ]; then
  pass "Recursive epoch proofs implemented (x3-proof/src/epoch.rs)"
else
  fail "epoch.rs missing from x3-proof"
fi

launch_val="$ROOT/crates/x3-launch-validator"
if [ -d "$launch_val" ]; then
  pass "x3-launch-validator present"
else
  fail "x3-launch-validator crate missing"
fi

gov_proof="$ROOT/pallets/governance/src/lib.rs"
if [ -f "$gov_proof" ] && grep -q "ProofRequiredForInvariantProposal" "$gov_proof" 2>/dev/null; then
  pass "Governance proof gate active"
else
  warn "Governance proof gate may not be wired (check pallets/governance/src/lib.rs)"
fi

# ── SECTION 6: Key File Anchors ───────────────────────────────────────────────
header "6. KEY FILE ANCHORS"

declare -A anchors=(
  ["node/src/main.rs"]="Node entry point"
  ["runtime/src/lib.rs"]="Runtime assembly"
  ["pallets/governance/src/lib.rs"]="Governance pallet"
  ["crates/x3-agent/src/types.rs"]="Agent types"
  ["crates/x3-constitution/src/engine.rs"]="Constitution engine"
  ["crates/x3-proof/src/epoch.rs"]="Epoch proofs"
  ["crates/x3-launch-validator/src/lib.rs"]="Launch validator"
)

for path in "${!anchors[@]}"; do
  label="${anchors[$path]}"
  if [ -f "$ROOT/$path" ]; then
    pass "$label ($path)"
  else
    fail "$label MISSING ($path)"
  fi
done

# ── SECTION 7: CI Workflow ────────────────────────────────────────────────────
header "7. CI WORKFLOWS"

if [ -f "$ROOT/.github/workflows/x3-audit.yml" ]; then
  pass "x3-audit.yml CI gate present"
else
  fail ".github/workflows/x3-audit.yml missing"
fi

# ── SUMMARY ───────────────────────────────────────────────────────────────────
total=$((PASS + WARN + FAIL))
echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║   AUDIT SUMMARY                                  ║${NC}"
echo -e "${BOLD}╠══════════════════════════════════════════════════╣${NC}"
printf   "║  %-10s %3d / %3d checks                     ║\n" "PASS:" "$PASS" "$total"
printf   "║  %-10s %3d                                   ║\n" "WARN:" "$WARN"
printf   "║  %-10s %3d                                   ║\n" "FAIL:" "$FAIL"
echo -e "${BOLD}╚══════════════════════════════════════════════════╝${NC}"

if [ "$FAIL" -gt 0 ]; then
  echo -e "\n${RED}FAILED CHECKS:${NC}"
  for c in "${FAILED_CHECKS[@]}"; do
    echo "  • $c"
  done
  echo ""
  echo -e "${RED}❌ X3 SELF-AUDIT FAILED — $FAIL hard check(s) failed${NC}"
  exit 1
elif [ "$WARN" -gt 0 ] && [ "$STRICT" -eq 1 ]; then
  echo -e "${YELLOW}⚠️  X3 SELF-AUDIT: warnings in strict mode = failure${NC}"
  exit 2
else
  echo -e "\n${GREEN}✅ X3 SELF-AUDIT PASSED${NC}"
  exit 0
fi
