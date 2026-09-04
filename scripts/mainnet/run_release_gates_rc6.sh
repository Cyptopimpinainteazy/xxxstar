#!/usr/bin/env bash
set -u

ROOT="/home/lojak/Desktop/X3_ATOMIC_STAR"

# Ensure Rust toolchain and local Node 20 are available for gate scripts.
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi
if [ -d "$ROOT/.tools/node20/bin" ]; then
  export PATH="$ROOT/.tools/node20/bin:$PATH"
fi

mkdir -p reports/rc6
TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT="reports/rc6/release_gate_sequence_${TS}.md"

run_step() {
  local name="$1"
  local cmd="$2"
  local log="$3"
  echo "## ${name}" >> "$OUT"
  bash -lc "$cmd" > "$log" 2>&1
  local code=$?
  echo "exit_code=${code}" >> "$OUT"
  echo >> "$OUT"
}

echo "# Release Gate Sequence Run (${TS})" > "$OUT"
echo >> "$OUT"

run_step "1) Build (cargo build -p x3-chain-node --release)" "cd ${ROOT} && cargo build -p x3-chain-node --release" "reports/rc6/build_${TS}.log"
run_step "2) Cross-chain smoke (scripts/mainnet/rc2_internal_settlement_smoke.sh)" "cd ${ROOT} && bash scripts/mainnet/rc2_internal_settlement_smoke.sh" "reports/rc6/rc2_smoke_${TS}.log"
run_step "3) Mock+Live E2E gate (scripts/mainnet/rc2_mock_and_live_gate.sh)" "cd ${ROOT} && bash scripts/mainnet/rc2_mock_and_live_gate.sh" "reports/rc6/mock_live_gate_${TS}.log"
run_step "4) Invariant/Security suite (scripts/run-security-gates.sh all)" "cd ${ROOT} && bash scripts/run-security-gates.sh all" "reports/rc6/security_gates_${TS}.log"
run_step "5) RC6 readiness (scripts/mainnet/rc6_public_testnet_readiness.sh)" "cd ${ROOT} && bash scripts/mainnet/rc6_public_testnet_readiness.sh" "reports/rc6/rc6_readiness_${TS}.log"

echo "## Log Files" >> "$OUT"
echo "- reports/rc6/build_${TS}.log" >> "$OUT"
echo "- reports/rc6/rc2_smoke_${TS}.log" >> "$OUT"
echo "- reports/rc6/mock_live_gate_${TS}.log" >> "$OUT"
echo "- reports/rc6/security_gates_${TS}.log" >> "$OUT"
echo "- reports/rc6/rc6_readiness_${TS}.log" >> "$OUT"

echo "$OUT"
