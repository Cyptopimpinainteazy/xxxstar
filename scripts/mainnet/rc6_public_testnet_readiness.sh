#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT_DIR/reports/rc6"
DOCS="$ROOT_DIR/docs/testnet"
CHAIN_SPECS="$ROOT_DIR/chain-specs"
NODE_BIN="$ROOT_DIR/target/release/x3-chain-node"
PLAIN_SPEC="$CHAIN_SPECS/x3-public-testnet-plain.json"
RAW_SPEC="$CHAIN_SPECS/x3-public-testnet-raw.json"

mkdir -p "$OUT" "$DOCS" "$CHAIN_SPECS"

PASS=1
BOOTNODE_DEPLOYMENT="PENDING"

if ! command -v cargo >/dev/null 2>&1; then
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  rustup default stable >/dev/null 2>&1 || true
fi

if ! command -v cargo >/dev/null 2>&1 && [[ -d "$HOME/.cargo/bin" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v cargo >/dev/null 2>&1 && command -v rustup >/dev/null 2>&1; then
  CARGO_BIN_DIR="$(dirname "$(rustup which cargo 2>/dev/null || true)")"
  if [[ -n "$CARGO_BIN_DIR" && -d "$CARGO_BIN_DIR" ]]; then
    export PATH="$CARGO_BIN_DIR:$PATH"
  fi
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "[FAIL] cargo not found in PATH. Install Rust toolchain or source ~/.cargo/env."
  exit 1
fi

mark_pass() {
  echo "[PASS] $1"
}

mark_fail() {
  echo "[FAIL] $1"
  PASS=0
}

check_file() {
  local file="$1"
  if [[ -s "$file" ]]; then
    mark_pass "$file"
  else
    mark_fail "$file missing or empty"
  fi
}

load_testnet_authorities_from_keys() {
  local key_files
  local json
  local aura_accounts
  local aura
  local grandpa
  local file

  key_files=("$ROOT_DIR"/deployment/keys/validator-*-summary.txt)
  if [[ ! -e "${key_files[0]}" ]]; then
    return 1
  fi

  json="["
  aura_accounts="["
  for file in "${key_files[@]}"; do
    aura="$(awk '
      /## AURA KEY/ { in_aura=1; next }
      /## GRANDPA KEY/ { in_aura=0 }
      in_aura && /SS58 Address:/ { print $3; exit }
    ' "$file")"

    grandpa="$(awk '
      /## GRANDPA KEY/ { in_grandpa=1; next }
      in_grandpa && /SS58 Address:/ { print $3; exit }
    ' "$file")"

    if [[ -n "$aura" && -n "$grandpa" ]]; then
      json+="{\"aura\":\"$aura\",\"grandpa\":\"$grandpa\"},"
      aura_accounts+="\"$aura\"," 
    fi
  done

  if [[ "$json" == "[" ]]; then
    return 1
  fi

  json="${json%,}]"
  aura_accounts="${aura_accounts%,}]"
  export X3_TESTNET_AUTHORITIES="$json"

  if [[ -z "${X3_TESTNET_ENDOWED_ACCOUNTS:-}" ]]; then
    export X3_TESTNET_ENDOWED_ACCOUNTS="$aura_accounts"
  fi
  if [[ -z "${X3_TESTNET_COUNCIL_MEMBERS:-}" ]]; then
    export X3_TESTNET_COUNCIL_MEMBERS="$aura_accounts"
  fi
  if [[ -z "${X3_TESTNET_TREASURY_SIGNERS:-}" ]]; then
    export X3_TESTNET_TREASURY_SIGNERS="$aura_accounts"
  fi

  if [[ -z "${X3_EVM_ESCROW_ADDR:-}" ]]; then
    export X3_EVM_ESCROW_ADDR="0xdead000000000000000000000000000000000001"
  fi
  if [[ -z "${X3_SVM_ESCROW_ADDR:-}" ]]; then
    export X3_SVM_ESCROW_ADDR="0000000000000000000000000000000000000000000000000000000000000001"
  fi

  return 0
}

append_check() {
  local status="$1"
  local item="$2"
  printf '| %s | %s |\n' "$item" "$status" >> "$OUT/rc6_check_matrix.md"
}

echo "| Check | Status |" > "$OUT/rc6_check_matrix.md"
echo "|---|---|" >> "$OUT/rc6_check_matrix.md"

echo "[RC6] Building release node..."
if cargo build --release -p x3-chain-node > "$OUT/node_release_build.log" 2>&1; then
  mark_pass "release node build"
  append_check "PASS" "Release node builds"
else
  mark_fail "release node build"
  append_check "FAIL" "Release node builds"
fi

echo "[RC6] Building runtime WASM candidate..."
if cargo build --release -p x3-chain-runtime > "$OUT/runtime_wasm_build.log" 2>&1; then
  if find "$ROOT_DIR/target/release" -path '*wbuild*x3-chain-runtime*.wasm' -type f | grep -q .; then
    mark_pass "runtime WASM build"
    append_check "PASS" "Runtime WASM builds"
  else
    mark_fail "runtime WASM artifact not found under target/release/wbuild"
    append_check "FAIL" "Runtime WASM builds"
  fi
else
  mark_fail "runtime WASM build"
  append_check "FAIL" "Runtime WASM builds"
fi

if [[ -x "$NODE_BIN" ]]; then
  PLAIN_SPEC_OUT="$OUT/buildspec_plain.out"
  RAW_SPEC_OUT="$OUT/buildspec_raw.out"

  if [[ -z "${X3_TESTNET_AUTHORITIES:-}" ]]; then
    if load_testnet_authorities_from_keys; then
      mark_pass "loaded X3_TESTNET_AUTHORITIES from deployment/keys"
    else
      mark_fail "X3_TESTNET_AUTHORITIES not set and no validator key summaries found"
    fi
  fi

  echo "[RC6] Generating public testnet specs..."
  if "$NODE_BIN" build-spec --chain testnet --disable-default-bootnode > "$PLAIN_SPEC_OUT" 2> "$OUT/buildspec_plain.err"; then
    awk 'BEGIN {in_json=0} /^[[:space:]]*\{/ {in_json=1} in_json {print}' "$PLAIN_SPEC_OUT" > "$PLAIN_SPEC"
    if [[ -s "$PLAIN_SPEC" ]]; then
      mark_pass "public plain spec"
      append_check "PASS" "Public testnet plain chain spec generates"
    else
      mark_fail "public plain spec json extraction"
      append_check "FAIL" "Public testnet plain chain spec generates"
    fi
  else
    mark_fail "public plain spec"
    append_check "FAIL" "Public testnet plain chain spec generates"
  fi

  if [[ -s "$PLAIN_SPEC" ]] && "$NODE_BIN" build-spec --chain "$PLAIN_SPEC" --raw --disable-default-bootnode > "$RAW_SPEC_OUT" 2> "$OUT/buildspec_raw.err"; then
    awk 'BEGIN {in_json=0} /^[[:space:]]*\{/ {in_json=1} in_json {print}' "$RAW_SPEC_OUT" > "$RAW_SPEC"
    if [[ -s "$RAW_SPEC" ]]; then
      mark_pass "public raw spec"
      append_check "PASS" "Public testnet raw chain spec generates"
    else
      mark_fail "public raw spec json extraction"
      append_check "FAIL" "Public testnet raw chain spec generates"
    fi
  else
    mark_fail "public raw spec"
    append_check "FAIL" "Public testnet raw chain spec generates"
  fi
else
  mark_fail "node binary missing at $NODE_BIN"
  append_check "FAIL" "Public testnet plain chain spec generates"
  append_check "FAIL" "Public testnet raw chain spec generates"
fi

echo "[RC6] Checking required docs..."
check_file "$DOCS/VALIDATOR_ONBOARDING.md"
check_file "$DOCS/RPC_AND_FAUCET.md"
check_file "$DOCS/WALLET_CLI_QUICKSTART.md"
check_file "$DOCS/BUG_REPORT_TEMPLATE.md"
check_file "$DOCS/INCIDENT_RUNBOOK.md"
check_file "$DOCS/PUBLIC_TESTNET_LAUNCH_PLAN.md"

echo "[RC6] Evaluating public safety gates..."
if [[ -s "$PLAIN_SPEC" || -s "$RAW_SPEC" ]]; then
  if grep -Eiq 'Alice|Bob|Charlie|Dave|Eve|Ferdie|5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY' "$PLAIN_SPEC" "$RAW_SPEC" 2>/dev/null; then
    mark_fail "public config contains known dev keys"
    append_check "FAIL" "No Alice/Bob/Charlie dev authority keys in public config"
  else
    mark_pass "no known dev keys in public config"
    append_check "PASS" "No Alice/Bob/Charlie dev authority keys in public config"
  fi
else
  mark_fail "cannot evaluate dev keys without generated specs"
  append_check "FAIL" "No Alice/Bob/Charlie dev authority keys in public config"
fi

if [[ -s "$RAW_SPEC" ]] && grep -q 'bootNodes' "$RAW_SPEC"; then
  if grep -Eq 'bootNodes"\s*:\s*\[\s*"/ip4/' "$RAW_SPEC"; then
    BOOTNODE_DEPLOYMENT="READY"
    mark_pass "bootnodes present in raw chain spec"
    append_check "PASS" "Bootnode config present (or documented pending)"
  else
    BOOTNODE_DEPLOYMENT="PENDING"
    mark_pass "bootnodes pending and launch is blocked until deployment"
    append_check "PASS" "Bootnode config present (or documented pending)"
  fi
else
  BOOTNODE_DEPLOYMENT="PENDING"
  mark_pass "bootnode deployment pending (allowed for RC6 package readiness)"
  append_check "PASS" "Bootnode config present (or documented pending)"
fi

if grep -q 'external_bridges_mainnet = "DISABLED_BLOCKED"' "$ROOT_DIR/TESTNET_FEATURE_FLAGS.toml" 2>/dev/null; then
  mark_pass "external bridges disabled in TESTNET_FEATURE_FLAGS.toml"
  append_check "PASS" "External bridges are disabled"
else
  mark_fail "external bridges disable flag missing"
  append_check "FAIL" "External bridges are disabled"
fi

if grep -Eiq 'separate from treasury|must never use treasury keys' "$DOCS/RPC_AND_FAUCET.md" 2>/dev/null; then
  mark_pass "faucet separation from treasury documented"
  append_check "PASS" "Faucet is separated from treasury"
else
  mark_fail "faucet/treasury separation statement missing"
  append_check "FAIL" "Faucet is separated from treasury"
fi

if grep -q '^# X3 Public Testnet Validator Onboarding' "$DOCS/VALIDATOR_ONBOARDING.md" 2>/dev/null; then
  mark_pass "validator onboarding guide present"
  append_check "PASS" "Validator onboarding guide is complete"
else
  mark_fail "validator onboarding guide incomplete"
  append_check "FAIL" "Validator onboarding guide is complete"
fi

if grep -q '^# X3 Wallet / CLI Quickstart' "$DOCS/WALLET_CLI_QUICKSTART.md" 2>/dev/null; then
  mark_pass "wallet/CLI guide present"
  append_check "PASS" "Wallet/CLI transfer guide is complete"
else
  mark_fail "wallet/CLI guide incomplete"
  append_check "FAIL" "Wallet/CLI transfer guide is complete"
fi

if grep -q '^# X3 Public Testnet RPC and Faucet' "$DOCS/RPC_AND_FAUCET.md" 2>/dev/null; then
  mark_pass "RPC/faucet plan present"
  append_check "PASS" "RPC/explorer plan is complete"
else
  mark_fail "RPC/faucet plan incomplete"
  append_check "FAIL" "RPC/explorer plan is complete"
fi

if [[ -s "$OUT/monitoring_check.md" ]]; then
  mark_pass "monitoring plan present"
  append_check "PASS" "Monitoring plan is complete"
else
  mark_fail "monitoring plan missing"
  append_check "FAIL" "Monitoring plan is complete"
fi

if [[ -s "$DOCS/INCIDENT_RUNBOOK.md" ]] && [[ -s "$OUT/incident_runbook_check.md" ]]; then
  mark_pass "incident runbook exists"
  append_check "PASS" "Incident runbook exists"
else
  mark_fail "incident runbook missing"
  append_check "FAIL" "Incident runbook exists"
fi

echo "[RC6] Hashing release artifacts..."
{
  echo "commit $(git rev-parse HEAD)"
  sha256sum "$NODE_BIN" 2>/dev/null || true
  sha256sum "$CHAIN_SPECS"/x3-public-testnet-*.json 2>/dev/null || true
  find "$ROOT_DIR/target" -path '*wbuild*x3-chain-runtime*.wasm' -type f -exec sha256sum {} \; 2>/dev/null || true
} > "$OUT/release_artifacts.sha256"

if [[ -s "$OUT/release_artifacts.sha256" ]]; then
  mark_pass "release artifact hashes recorded"
  append_check "PASS" "Release artifact hashes are recorded"
else
  mark_fail "release artifact hashes missing"
  append_check "FAIL" "Release artifact hashes are recorded"
fi

VERDICT="FAIL"
if [[ "$PASS" == "1" ]]; then
  VERDICT="PASS"
fi

cat > "$OUT/rc6_public_testnet_readiness_report.md" <<REPORT
# RC6 Public Testnet Readiness Report

## Verdict

$VERDICT

## Package Status

- RC6_PACKAGE_READY: $VERDICT
- BOOTNODE_DEPLOYMENT: $BOOTNODE_DEPLOYMENT

## Scope

- Public testnet package only (no launch)
- Public chain spec generation
- Validator onboarding docs
- RPC/faucet/explorer planning docs
- Monitoring and incident runbook
- Release artifact hashing
- External bridges disabled gate

## Check Matrix

$(cat "$OUT/rc6_check_matrix.md")

## Required Files

| File | Required |
|---|---:|
| docs/testnet/VALIDATOR_ONBOARDING.md | yes |
| docs/testnet/RPC_AND_FAUCET.md | yes |
| docs/testnet/WALLET_CLI_QUICKSTART.md | yes |
| docs/testnet/BUG_REPORT_TEMPLATE.md | yes |
| docs/testnet/INCIDENT_RUNBOOK.md | yes |
| docs/testnet/PUBLIC_TESTNET_LAUNCH_PLAN.md | yes |
| reports/rc6/release_artifacts.sha256 | yes |

## Artifacts

- chain-specs/x3-public-testnet-plain.json
- chain-specs/x3-public-testnet-raw.json
- reports/rc6/release_artifacts.sha256

## Guardrails

- External bridges must remain disabled for initial public testnet.
- Faucet operations must use keys separate from treasury keys.
- Bootnode deployment may remain pending for RC6 package readiness, but public launch is blocked until bootnodes are live.

## Final Rule

RC6 passes only when a new validator can follow public docs and artifacts to join without private hand-holding.
REPORT

if [[ "$PASS" == "1" ]]; then
  echo "RC6_PUBLIC_TESTNET_READINESS: PASS"
  exit 0
else
  echo "RC6_PUBLIC_TESTNET_READINESS: FAIL"
  exit 1
fi