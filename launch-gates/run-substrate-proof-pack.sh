#!/usr/bin/env bash
# X3 Substrate proof pack runner.
#
# Produces Substrate-native evidence for runtime upgrade safety, FRAME weights,
# runtime metadata/reproducibility tooling, local network smoke tooling, and
# client compatibility prerequisites.
#
# Default mode is quick and non-destructive. Set RUN_HEAVY_SUBSTRATE_PROOFS=1
# to run compile/build checks that may take a while.

set -u -o pipefail

REPO_ROOT="${REPO_ROOT:-/home/lojak/Desktop/X3_ATOMIC_STAR}"
REPORT_DIR="${REPORT_DIR:-${REPO_ROOT}/reports/substrate}"
EVIDENCE_DIR="${EVIDENCE_DIR:-${REPO_ROOT}/launch-gates/evidence/substrate}"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
REPORT="${REPORT_DIR}/substrate-proof-pack-${TIMESTAMP}.md"
LATEST_REPORT="${REPORT_DIR}/SUBSTRATE_PROOF_PACK_LATEST.md"
HEAVY="${RUN_HEAVY_SUBSTRATE_PROOFS:-0}"

mkdir -p "${REPORT_DIR}" "${EVIDENCE_DIR}"
cd "${REPO_ROOT}" || exit 2

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0
SKIP_COUNT=0

write_header() {
  {
    echo "# X3 Substrate Proof Pack"
    echo
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Repository: ${REPO_ROOT}"
    echo "- Commit: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
    echo "- Heavy checks: ${HEAVY}"
    echo
    echo "This report is evidence, not an external audit certificate. PASS means the listed command passed on this machine. SKIP means the prerequisite/tooling was not present or heavy mode was disabled."
    echo
    echo "| Gate | Status | Evidence |"
    echo "| --- | --- | --- |"
  } > "${REPORT}"
}

record() {
  local gate="$1"
  local status="$2"
  local evidence="$3"

  case "${status}" in
    PASS) PASS_COUNT=$((PASS_COUNT + 1)) ;;
    FAIL) FAIL_COUNT=$((FAIL_COUNT + 1)) ;;
    WARN) WARN_COUNT=$((WARN_COUNT + 1)) ;;
    SKIP) SKIP_COUNT=$((SKIP_COUNT + 1)) ;;
  esac

  echo "| ${gate} | ${status} | ${evidence} |" >> "${REPORT}"
  printf '%-48s %s\n' "${gate}" "${status}"
}

run_gate() {
  local gate="$1"
  local command="$2"
  local log="${EVIDENCE_DIR}/${gate//[^A-Za-z0-9._-]/_}-${TIMESTAMP}.log"

  {
    echo "$ ${command}"
    echo
    eval "${command}"
  } > "${log}" 2>&1

  if [[ $? -eq 0 ]]; then
    sha256sum "${log}" > "${log}.sha256"
    record "${gate}" "PASS" "[log](${log}) / sha256 $(cut -d' ' -f1 "${log}.sha256")"
  else
    sha256sum "${log}" > "${log}.sha256"
    record "${gate}" "FAIL" "[log](${log}) / sha256 $(cut -d' ' -f1 "${log}.sha256")"
  fi
}

skip_gate() {
  record "$1" "SKIP" "$2"
}

warn_gate() {
  local gate="$1"
  local command="$2"
  local log="${EVIDENCE_DIR}/${gate//[^A-Za-z0-9._-]/_}-${TIMESTAMP}.log"

  {
    echo "$ ${command}"
    echo
    eval "${command}"
  } > "${log}" 2>&1

  sha256sum "${log}" > "${log}.sha256"
  if [[ -s "${log}" ]]; then
    record "${gate}" "WARN" "[log](${log}) / sha256 $(cut -d' ' -f1 "${log}.sha256")"
  else
    record "${gate}" "PASS" "[log](${log}) / sha256 $(cut -d' ' -f1 "${log}.sha256")"
  fi
}

write_header

echo "Running X3 Substrate proof pack..."
echo "Report: ${REPORT}"
echo

run_gate "substrate-toolchain-inventory" "rustc --version && cargo --version && cargo metadata --no-deps --format-version 1 >/dev/null"

run_gate "try-runtime-command-wired" "rg -n 'Commands::TryRuntime\\(cmd\\)|cmd\\.run::<|substrate_info' node/src/command.rs node/src/cli.rs"

if [[ "${HEAVY}" == "1" ]]; then
  run_gate "try-runtime-feature-compiles" "CARGO_BUILD_JOBS=\${CARGO_BUILD_JOBS:-1} cargo check -p x3-chain-node --features try-runtime"
else
  skip_gate "try-runtime-feature-compiles" "Set RUN_HEAVY_SUBSTRATE_PROOFS=1 to run cargo check -p x3-chain-node --features try-runtime."
fi

if [[ -x "target/release/x3-chain-node" || -x "target/debug/x3-chain-node" ]]; then
  NODE_BIN="target/release/x3-chain-node"
  [[ -x "${NODE_BIN}" ]] || NODE_BIN="target/debug/x3-chain-node"
  run_gate "node-benchmark-cli-present" "${NODE_BIN} benchmark --help"
else
  skip_gate "node-benchmark-cli-present" "No built x3-chain-node binary found under target/release or target/debug."
fi

warn_gate "placeholder-or-manual-weight-scan" "rg -n 'Weight::from_parts\\(|type WeightInfo = \\(\\)' runtime pallets crates --glob '!**/mock.rs' --glob '!**/tests.rs' --glob '!target/**'"

if command -v srtool >/dev/null 2>&1; then
  run_gate "srtool-installed" "srtool --version"
else
  skip_gate "srtool-installed" "srtool not found in PATH. Install before claiming reproducible runtime Wasm proof."
fi

if command -v subwasm >/dev/null 2>&1; then
  run_gate "subwasm-installed" "NO_COLOR=true subwasm --version"
else
  skip_gate "subwasm-installed" "subwasm not found in PATH. Install before publishing runtime metadata diff proof."
fi

if command -v zombienet >/dev/null 2>&1; then
  run_gate "zombienet-installed" "zombienet version"
else
  skip_gate "zombienet-installed" "zombienet not found in PATH. Install before publishing local validator network proof."
fi

if command -v chopsticks >/dev/null 2>&1; then
  run_gate "chopsticks-installed" "chopsticks --version"
else
  skip_gate "chopsticks-installed" "chopsticks not found in PATH. Install before publishing fork/replay proof."
fi

run_gate "client-compatibility-source-inventory" "rg -n '@polkadot/api|subxt|substrate-api-sidecar|papi|polkadot-api|@dedot|python-substrate-interface' packages apps web scripts docs --glob '!**/node_modules/**' --glob '!target/**'"

run_gate "chain-spec-source-present" "test -f node/src/chain_spec.rs && rg -n 'ChainSpec|GenesisConfig|x3|testnet|mainnet|bootnodes|telemetry' node/src/chain_spec.rs"

{
  echo
  echo "## Summary"
  echo
  echo "- PASS: ${PASS_COUNT}"
  echo "- WARN: ${WARN_COUNT}"
  echo "- FAIL: ${FAIL_COUNT}"
  echo "- SKIP: ${SKIP_COUNT}"
  echo
  echo "## Certificate-Like Labels Allowed After Evidence"
  echo
  echo "- Substrate Runtime Upgrade Check: only claim PASS after try-runtime on-runtime-upgrade runs against a live/snapshot state."
  echo "- FRAME Weights: only claim generated/committed after benchmark output replaces manual placeholder weights."
  echo "- Runtime Wasm Reproducibility: only claim after srtool hash/proposal hash is published."
  echo "- Runtime Metadata Diff: only claim after subwasm diff is published."
  echo "- Local Network Smoke: only claim after Zombienet topology/test logs are published."
  echo "- Fork/Replay Suite: only claim after Chopsticks replay logs are published."
  echo
  echo "## Exit Policy"
  echo
  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    echo "Result: FAIL. At least one required quick proof failed."
  elif [[ "${WARN_COUNT}" -gt 0 || "${SKIP_COUNT}" -gt 0 ]]; then
    echo "Result: PARTIAL. Do not present this as complete Substrate security proof."
  else
    echo "Result: PASS. All configured quick proofs passed."
  fi
} >> "${REPORT}"

cp "${REPORT}" "${LATEST_REPORT}"

echo
echo "Summary: PASS=${PASS_COUNT} WARN=${WARN_COUNT} FAIL=${FAIL_COUNT} SKIP=${SKIP_COUNT}"
echo "Latest report: ${LATEST_REPORT}"

if [[ "${FAIL_COUNT}" -gt 0 ]]; then
  exit 1
fi
