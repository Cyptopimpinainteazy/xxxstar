#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="${REPO_ROOT}/launch-gates/evidence/ci/startup-gate-${TIMESTAMP}"
# Use an isolated startup-gate target cache by default so reruns reuse prior
# artifacts without interference from unrelated shared target users.
TARGET_MODE="${STARTUP_GATE_TARGET_MODE:-isolated}"
SHARED_TARGET_DIR="${REPO_ROOT}/launch-gates/evidence/ci/startup-gate-target"
ISOLATED_TARGET_DIR="${STARTUP_GATE_ISOLATED_TARGET_DIR:-${REPO_ROOT}/launch-gates/evidence/ci/startup-gate-target-isolated}"
if [[ -n "${STARTUP_GATE_TARGET_DIR:-}" ]]; then
  TARGET_DIR="${STARTUP_GATE_TARGET_DIR}"
elif [[ "${TARGET_MODE}" == "shared" ]]; then
  TARGET_DIR="${SHARED_TARGET_DIR}"
else
  TARGET_DIR="${ISOLATED_TARGET_DIR}"
fi

if [[ "${TARGET_MODE}" == "shared" ]] && pgrep -f -- "--target-dir ${TARGET_DIR}" >/dev/null 2>&1; then
  echo "startup-gate-run: shared target busy; falling back to isolated target for this run"
  TARGET_MODE="isolated-fallback"
  TARGET_DIR="${ISOLATED_TARGET_DIR}"
fi
LOCK_FILE="${STARTUP_GATE_LOCK_FILE:-${REPO_ROOT}/launch-gates/evidence/ci/startup-gate.lock}"
TMP_DIR="${STARTUP_GATE_TMPDIR:-${REPO_ROOT}/launch-gates/evidence/ci/startup-gate-tmp}"
RUSTFLAGS_VALUE="${STARTUP_GATE_RUSTFLAGS:--C debuginfo=0 -C codegen-units=1 -C link-arg=-fuse-ld=lld}"
ALLOW_CONCURRENT="${STARTUP_GATE_ALLOW_CONCURRENT:-0}"
SKIP_WASM_BUILD_VALUE="${STARTUP_GATE_SKIP_WASM_BUILD:-1}"
CXXFLAGS_VALUE="${STARTUP_GATE_CXXFLAGS:--pipe -g0}"
MALLOC_ARENA_MAX_VALUE="${STARTUP_GATE_MALLOC_ARENA_MAX:-2}"
NUM_JOBS_VALUE="${STARTUP_GATE_NUM_JOBS:-1}"
CMAKE_BUILD_PARALLEL_LEVEL_VALUE="${STARTUP_GATE_CMAKE_BUILD_PARALLEL_LEVEL:-1}"
BINARYEN_CORES_VALUE="${STARTUP_GATE_BINARYEN_CORES:-1}"

if [[ -n "${STARTUP_GATE_TARGET_DIR:-}" ]]; then
  RUNTIME_TARGET_DIR="${TARGET_DIR}"
  NODE_TARGET_DIR="${TARGET_DIR}"
else
  RUNTIME_TARGET_DIR="${TARGET_DIR}/runtime"
  NODE_TARGET_DIR="${TARGET_DIR}/node"
fi

if [[ "${ALLOW_CONCURRENT}" != "1" ]]; then
  EXISTING_STARTUP_GATE_PIDS="$(pgrep -f 'cargo test -p x3-chain-runtime .*fraud_proofs::startup_gate::tests::|cargo test -p x3-chain-node startup_gate_passes_for_reference_authority_build' || true)"
  if [[ -n "${EXISTING_STARTUP_GATE_PIDS}" ]]; then
    echo "startup-gate-run: detected existing startup-gate cargo process(es): ${EXISTING_STARTUP_GATE_PIDS}"
    echo "startup-gate-run: set STARTUP_GATE_ALLOW_CONCURRENT=1 to override"
    exit 76
  fi
fi

mkdir -p "$(dirname "${LOCK_FILE}")"
exec 9>"${LOCK_FILE}"
if ! flock -n 9; then
  echo "Another startup-gate run is already active (lock: ${LOCK_FILE})."
  exit 75
fi

mkdir -p "${OUT_DIR}"
mkdir -p "${TMP_DIR}"
mkdir -p "${TARGET_DIR}"

on_interrupt() {
  local sig="$1"
  {
    echo "finished_at=$(date -Is)"
    echo "interrupted_signal=${sig}"
  } >> "${OUT_DIR}/summary.env"
  echo "startup-gate-run: interrupted by ${sig}"
  exit 128
}

trap 'on_interrupt SIGTERM' TERM
trap 'on_interrupt SIGINT' INT

echo "startup-gate-run: out_dir=${OUT_DIR}"
echo "startup-gate-run: target_dir=${TARGET_DIR}"
echo "startup-gate-run: target_mode=${TARGET_MODE}"
echo "startup-gate-run: rustflags=${RUSTFLAGS_VALUE}"
echo "startup-gate-run: skip_wasm_build=${SKIP_WASM_BUILD_VALUE}"
echo "startup-gate-run: cxxflags=${CXXFLAGS_VALUE}"
echo "startup-gate-run: malloc_arena_max=${MALLOC_ARENA_MAX_VALUE}"
echo "startup-gate-run: num_jobs=${NUM_JOBS_VALUE}"
echo "startup-gate-run: cmake_build_parallel_level=${CMAKE_BUILD_PARALLEL_LEVEL_VALUE}"
echo "startup-gate-run: binaryen_cores=${BINARYEN_CORES_VALUE}"
echo "startup-gate-run: tmp_dir=${TMP_DIR}"

{
  echo "matrix_id=${TIMESTAMP}"
  echo "started_at=$(date -Is)"
  echo "workspace=${REPO_ROOT}"
  echo "target_mode=${TARGET_MODE}"
  echo "target_dir=${TARGET_DIR}"
  echo "rustflags=${RUSTFLAGS_VALUE}"
  echo "allow_concurrent=${ALLOW_CONCURRENT}"
  echo "skip_wasm_build=${SKIP_WASM_BUILD_VALUE}"
  echo "cxxflags=${CXXFLAGS_VALUE}"
  echo "malloc_arena_max=${MALLOC_ARENA_MAX_VALUE}"
  echo "num_jobs=${NUM_JOBS_VALUE}"
  echo "cmake_build_parallel_level=${CMAKE_BUILD_PARALLEL_LEVEL_VALUE}"
  echo "binaryen_cores=${BINARYEN_CORES_VALUE}"
} > "${OUT_DIR}/summary.env"

echo "job,result,exit_code,duration_sec,log_file" > "${OUT_DIR}/matrix.csv"

run_job() {
  local name="$1"
  shift
  local log_file="${OUT_DIR}/${name}.log"
  local status_file="${OUT_DIR}/${name}.status"
  local heartbeat_sec="${STARTUP_GATE_HEARTBEAT_SECONDS:-30}"
  local start_ts
  start_ts=$(date +%s)

  {
    echo "=== JOB: ${name} ==="
    echo "pwd=${REPO_ROOT}"
    echo "command: $*"
  } > "${log_file}"

  set +e
  (
    cd "${REPO_ROOT}"
    "$@"
  ) >> "${log_file}" 2>&1 &
  local cmd_pid=$!

  while kill -0 "${cmd_pid}" >/dev/null 2>&1; do
    echo "${name} => RUNNING (pid=${cmd_pid}, elapsed=$(( $(date +%s) - start_ts ))s)"
    sleep "${heartbeat_sec}"
  done

  wait "${cmd_pid}"
  local ec=$?
  set -e
  local end_ts
  end_ts=$(date +%s)
  local dur=$((end_ts - start_ts))
  local result="FAIL"
  if [[ ${ec} -eq 0 ]]; then
    result="PASS"
  fi

  {
    echo "result=${result}"
    echo "exit_code=${ec}"
    echo "duration_sec=${dur}"
    echo "log_file=$(basename "${log_file}")"
  } > "${status_file}"

  echo "${name},${result},${ec},${dur},$(basename "${log_file}")" >> "${OUT_DIR}/matrix.csv"
  echo "${name} => ${result} (exit=${ec}, ${dur}s)"

  return ${ec}
}

COMMON_ENV=(env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 NUM_JOBS="${NUM_JOBS_VALUE}" CMAKE_BUILD_PARALLEL_LEVEL="${CMAKE_BUILD_PARALLEL_LEVEL_VALUE}" BINARYEN_CORES="${BINARYEN_CORES_VALUE}" MALLOC_ARENA_MAX="${MALLOC_ARENA_MAX_VALUE}" SKIP_WASM_BUILD="${SKIP_WASM_BUILD_VALUE}" RUSTFLAGS="${RUSTFLAGS_VALUE}" TMPDIR="${TMP_DIR}" CXXFLAGS="${CXXFLAGS_VALUE}")

run_job runtime_gate_reference "${COMMON_ENV[@]}" cargo test -p x3-chain-runtime fraud_proofs::startup_gate::tests::gate_passes_with_reference_scheduler --lib --target-dir "${RUNTIME_TARGET_DIR}" -- --nocapture || true
run_job runtime_gate_deterministic "${COMMON_ENV[@]}" cargo test -p x3-chain-runtime fraud_proofs::startup_gate::tests::gate_is_deterministic --lib --target-dir "${RUNTIME_TARGET_DIR}" -- --nocapture || true
run_job node_gate_reference "${COMMON_ENV[@]}" cargo test -p x3-chain-node startup_gate_passes_for_reference_authority_build --lib --target-dir "${NODE_TARGET_DIR}" -- --nocapture || true

FAILS=$(awk -F, 'NR>1 && $2=="FAIL" {c++} END {print c+0}' "${OUT_DIR}/matrix.csv")
PASSES=$(awk -F, 'NR>1 && $2=="PASS" {c++} END {print c+0}' "${OUT_DIR}/matrix.csv")

{
  echo "finished_at=$(date -Is)"
  echo "passes=${PASSES}"
  echo "fails=${FAILS}"
} >> "${OUT_DIR}/summary.env"

echo "OUT_DIR=${OUT_DIR}"
cat "${OUT_DIR}/matrix.csv"

if [[ "${FAILS}" -ne 0 ]]; then
  exit 1
fi