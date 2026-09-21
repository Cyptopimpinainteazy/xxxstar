#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# cross-domain-svm-gate.sh — real X3VM <-> SVM atomic lifecycle gate
#
# Proves, against a real `solana-test-validator` running the real SBF program
# (built here from `programs/svm/x3_atomic_swap`), that one atomic intent
# settles across the X3 domain and the Solana domain:
#
#   1. `real_x3vm_svm_lock_claim_atomic_lifecycle`
#      X3 escrow leg locked through the node's RPC, Solana leg locked through
#      the real `x3-svm-broadcast` client against the deployed program, the
#      preimage revealed on the Solana leg, and the X3 leg claimed with a
#      secret-release permit carrying real evidence for both domains.
#   2. `real_x3vm_svm_timeout_refund_atomic_lifecycle`
#      The same intent forced down the refund path, with the refund observed
#      finalized on both domains.
#
# Both tests are `#[ignore]`d in `node/tests/x3vm_svm_live.rs` because they need
# a validator and an SBF build, not because they are unfinished. This script
# supplies both.
#
# Usage: bash scripts/cross-domain-svm-gate.sh
#        X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-svm-gate.sh
#
# The environment variable does what it does in the EVM gate: it runs the same
# lifecycles against a dev spec whose `allowUnattestedCrossDomainProofs` is the
# value every joinable network uses, and the test reads that policy back from
# the chain before it starts.
# Requires: the Solana toolchain (`solana`, `solana-keygen`,
# `solana-test-validator`, `cargo build-sbf`) on PATH.
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SVM_DIR="$REPO_ROOT/programs/svm/x3_atomic_swap"
RPC_URL="http://127.0.0.1:18999"
WORKDIR="$(mktemp -d /tmp/x3vm-svm-cross.XXXXXX)"
VALIDATOR_PID=""

cleanup() {
  if [ -n "$VALIDATOR_PID" ] && kill -0 "$VALIDATOR_PID" 2>/dev/null; then
    kill "$VALIDATOR_PID" 2>/dev/null || true
    wait "$VALIDATOR_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for tool in solana solana-keygen solana-test-validator; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "Error: $tool not installed (Solana toolchain)"
    exit 1
  }
done

echo "workdir: $WORKDIR"

echo "=== build the SBF program ==="
( cd "$SVM_DIR" && cargo build-sbf ) || { echo "Error: cargo build-sbf failed"; exit 1; }
# `cargo build-sbf` honors `CARGO_TARGET_DIR`, and the local CI redirects it
# (`docs/local-ci.md`), so the artifact is not always under the program's own
# `target/`. Looking in only one place makes this gate fail with "missing after
# build-sbf" on a tree where the build actually succeeded.
PROGRAM_SO="${CARGO_TARGET_DIR:-$SVM_DIR/target}/deploy/x3_atomic_swap.so"
[ -f "$PROGRAM_SO" ] || PROGRAM_SO="$SVM_DIR/target/deploy/x3_atomic_swap.so"
[ -f "$PROGRAM_SO" ] || { echo "Error: $PROGRAM_SO missing after build-sbf"; exit 1; }

echo "=== build x3-svm-broadcast ==="
( cd "$SVM_DIR/client" && cargo build --release --bin x3-svm-broadcast ) \
  || { echo "Error: could not build x3-svm-broadcast"; exit 1; }
BROADCAST_BIN="${CARGO_TARGET_DIR:-$SVM_DIR/client/target}/release/x3-svm-broadcast"
[ -x "$BROADCAST_BIN" ] || BROADCAST_BIN="$SVM_DIR/client/target/release/x3-svm-broadcast"
[ -x "$BROADCAST_BIN" ] || { echo "Error: $BROADCAST_BIN missing after the build"; exit 1; }
[ -x "$BROADCAST_BIN" ] || { echo "Error: $BROADCAST_BIN missing"; exit 1; }

echo "=== start isolated solana-test-validator on $RPC_URL ==="
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/program-keypair.json" >/dev/null
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/payer.json" >/dev/null
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/claimant.json" >/dev/null
PROGRAM_ID="$(solana-keygen pubkey "$WORKDIR/program-keypair.json")"
PAYER_PK="$(solana-keygen pubkey "$WORKDIR/payer.json")"
CLAIMANT_PK="$(solana-keygen pubkey "$WORKDIR/claimant.json")"

solana-test-validator --reset --quiet \
  --ledger "$WORKDIR/ledger" \
  --rpc-port 18999 \
  --faucet-port 18900 \
  --dynamic-port-range 19000-19100 \
  --bpf-program "$PROGRAM_ID" "$PROGRAM_SO" \
  > "$WORKDIR/validator.log" 2>&1 &
VALIDATOR_PID=$!

ready=0
for _ in $(seq 1 60); do
  if solana cluster-version --url "$RPC_URL" >/dev/null 2>&1; then ready=1; break; fi
  sleep 1
done
if [ "$ready" != 1 ]; then
  echo "Error: solana-test-validator did not start"
  cat "$WORKDIR/validator.log"
  exit 1
fi
echo "validator: $(solana cluster-version --url "$RPC_URL") program: $PROGRAM_ID"

solana airdrop 5 "$PAYER_PK" --url "$RPC_URL" --commitment finalized >/dev/null
solana airdrop 2 "$CLAIMANT_PK" --url "$RPC_URL" --commitment finalized >/dev/null

export X3_TEST_SVM_RPC="$RPC_URL"
export X3_TEST_SVM_PROGRAM_ID="$PROGRAM_ID"
export X3_TEST_SVM_PAYER_KEYPAIR="$WORKDIR/payer.json"
export X3_TEST_SVM_CLAIMANT_KEYPAIR="$WORKDIR/claimant.json"
export X3_TEST_SVM_PAYER_PUBKEY="$PAYER_PK"
export X3_TEST_SVM_CLAIMANT_PUBKEY="$CLAIMANT_PK"
export X3_SVM_BROADCAST_BIN="$BROADCAST_BIN"

# Built up front for the same reason as the EVM gate: a compile error must not
# be reported as a validator failure.
echo "=== build the cross-domain test target ==="
cd "$REPO_ROOT"
env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_svm_live --no-run \
  || { echo "Error: could not build node/tests/x3vm_svm_live.rs"; exit 1; }

if [ -n "${X3_TEST_CHAIN_SPEC:-}" ]; then
  POSTURE="caller-supplied spec ($X3_TEST_CHAIN_SPEC)"
else
  POSTURE="dev (allowUnattestedCrossDomainProofs = true)"
fi
if [ "${X3_STRICT_CROSS_DOMAIN_PROOFS:-0}" = "1" ]; then
  echo "=== strict posture: build a dev spec that refuses unattested proof sets ==="
  env -u SKIP_WASM_BUILD cargo build -p x3-chain-node --bin x3-chain-node \
    || { echo "Error: could not build x3-chain-node"; exit 1; }
  NODE_BIN="${CARGO_TARGET_DIR:-$REPO_ROOT/target}/debug/x3-chain-node"
  [ -x "$NODE_BIN" ] || { echo "Error: $NODE_BIN not found"; exit 1; }
  STRICT_DIR="$(mktemp -d)"
  "$NODE_BIN" build-spec --dev > "$STRICT_DIR/dev.json" 2>/dev/null \
    || { echo "Error: build-spec --dev failed"; exit 1; }
  python3 "$REPO_ROOT/scripts/mainnet/strict-cross-domain-spec.py" \
    "$STRICT_DIR/dev.json" "$STRICT_DIR/strict.json" \
    || { echo "Error: could not build the strict spec"; exit 1; }
  export X3_TEST_CHAIN_SPEC="$STRICT_DIR/strict.json"
  POSTURE="strict ($X3_TEST_CHAIN_SPEC)"
  echo "cross-domain-svm-gate: posture $POSTURE"
fi

TESTS=(
  real_x3vm_svm_lock_claim_atomic_lifecycle
  real_x3vm_svm_timeout_refund_atomic_lifecycle
)

failed=0
for t in "${TESTS[@]}"; do
  echo
  echo "=== $t ==="
  if env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_svm_live "$t" \
    -- --ignored --nocapture --test-threads=1; then
    echo "PASS: $t"
  else
    echo "FAIL: $t"
    failed=1
  fi
done

echo
if [ "$failed" -eq 0 ]; then
  echo "cross-domain-svm-gate: both X3VM<->SVM lifecycles passed [$POSTURE]"
else
  echo "cross-domain-svm-gate: FAILED [$POSTURE]"
fi
exit "$failed"
