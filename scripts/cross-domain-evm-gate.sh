#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# cross-domain-evm-gate.sh — real X3VM <-> EVM atomic lifecycle gate
#
# Proves, against a real local `anvil` chain with a real deployed AtlasHTLC
# contract and a real local X3 node, that one atomic intent settles across both
# domains:
#
#   1. `real_x3vm_evm_lock_claim_atomic_lifecycle`
#      X3 escrow leg locked through the node's own RPC, EVM leg locked through
#      the real broadcaster, the preimage revealed on the EVM leg, and the X3
#      leg claimed with a secret-release permit built from real evidence for
#      both declared domains.
#   2. `real_x3vm_evm_timeout_refund_atomic_lifecycle`
#      The same intent forced down the refund path: canonical refund proof set
#      submitted, refund observed finalized on both domains, and a later claim
#      on either leg rejected.
#
# Both tests are `#[ignore]`d in `node/tests/x3vm_evm_live.rs` because they need
# a chain, not because they are unfinished. This script supplies the chain, so
# the ignore attribute no longer means "never runs".
#
# Usage: bash scripts/cross-domain-evm-gate.sh
#        X3_STRICT_CROSS_DOMAIN_PROOFS=1 bash scripts/cross-domain-evm-gate.sh
#
# Without the environment variable the tests boot the dev chain, whose genesis
# allows unattested cross-domain proof sets (`allowUnattestedCrossDomainProofs:
# true`) — a dev-only posture, because there is no external chain to prove
# against locally. With it, the same two lifecycles run against a dev spec with
# that one policy flipped to the value every joinable network uses, and the test
# reads the policy back from the chain before it starts, so a run that silently
# ignored the spec fails rather than passing as "strict".
# Requires: foundry (anvil, forge, cast) and openssl on PATH.
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVM_DIR="$REPO_ROOT/X3-contracts/evm"
RPC_URL="http://127.0.0.1:18545"
CHAIN_ID=1337
ANVIL_PID=""

cleanup() {
  if [ -n "$ANVIL_PID" ] && kill -0 "$ANVIL_PID" 2>/dev/null; then
    kill "$ANVIL_PID" 2>/dev/null || true
    wait "$ANVIL_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for tool in anvil forge cast openssl; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "Error: $tool not installed (foundry: https://getfoundry.sh)"
    exit 1
  }
done

cd "$REPO_ROOT"

# The forge dependencies are not vendored: they are pinned in foundry.lock and
# cloned on demand, the same way .github/workflows/x3vm-evm-live-lifecycle.yml
# does it. `forge install` is deliberately not used — it refuses to run when
# `.gitmodules` or the target path is already dirty, which is how the EVM job
# failed on the runner's reused workspace (PR #203).
ensure_pinned() {
  local path="$1" url="$2" tag="$3"
  if [ ! -d "$path/.git" ]; then
    git clone --quiet --depth 1 --branch "$tag" "$url" "$path" || return 1
  elif [ "$(git -C "$path" describe --tags --exact-match 2>/dev/null || echo none)" != "$tag" ]; then
    git -C "$path" fetch --quiet --depth 1 origin "refs/tags/${tag}:refs/tags/${tag}" || return 1
    git -C "$path" checkout --quiet "$tag" || return 1
  fi
}

echo "=== forge dependencies (pinned via foundry.lock) ==="
FORGE_STD_TAG="$(python3 -c "import json; print(json.load(open('$EVM_DIR/foundry.lock'))['lib/forge-std']['tag']['name'])")"
OZ_TAG="$(python3 -c "import json; print(json.load(open('$EVM_DIR/foundry.lock'))['lib/openzeppelin-contracts']['tag']['name'])")"
ensure_pinned "$EVM_DIR/lib/forge-std" https://github.com/foundry-rs/forge-std "$FORGE_STD_TAG" \
  || { echo "Error: could not install lib/forge-std at $FORGE_STD_TAG (offline?)"; exit 1; }
ensure_pinned "$EVM_DIR/lib/openzeppelin-contracts" https://github.com/OpenZeppelin/openzeppelin-contracts "$OZ_TAG" \
  || { echo "Error: could not install lib/openzeppelin-contracts at $OZ_TAG (offline?)"; exit 1; }

# Built up front so a compile error is reported as a compile error, not as an
# anvil failure ten seconds later. `SKIP_WASM_BUILD` must be unset: these tests
# boot a node whose chain spec is decoded by the embedded runtime.
echo "=== build the cross-domain test target ==="
env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_evm_live --no-run \
  || { echo "Error: could not build node/tests/x3vm_evm_live.rs"; exit 1; }

# Report the posture from whichever spec the run will actually boot with: the
# caller may set `X3_TEST_CHAIN_SPEC` itself (the test documents it), and a
# summary that said "dev" while a strict spec was in force would be the same
# kind of lie this whole path exists to avoid.
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
  echo "cross-domain-evm-gate: posture $POSTURE"
fi

echo "=== start isolated anvil on $RPC_URL (chain id $CHAIN_ID) ==="
anvil --port 18545 --chain-id "$CHAIN_ID" --silent > /tmp/x3-cross-domain-evm-anvil.log 2>&1 &
ANVIL_PID=$!
ready=0
for _ in $(seq 1 40); do
  if cast chain-id --rpc-url "$RPC_URL" >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.5
done
if [ "$ready" != 1 ]; then
  echo "Error: anvil did not start"
  cat /tmp/x3-cross-domain-evm-anvil.log
  exit 1
fi

echo "=== deploy AtlasHTLC with ephemeral actors ==="
cd "$EVM_DIR"
# Ephemeral, single-run keys against an ephemeral local chain.
LOCKER_KEY="$(openssl rand -hex 32)"
CLAIMANT_KEY="$(openssl rand -hex 32)"
LOCKER_ADDR="$(cast wallet address --private-key "$LOCKER_KEY")"
CLAIMANT_ADDR="$(cast wallet address --private-key "$CLAIMANT_KEY")"
cast rpc anvil_setBalance "$LOCKER_ADDR" 0x3635C9ADC5DEA00000 --rpc-url "$RPC_URL" >/dev/null
cast rpc anvil_setBalance "$CLAIMANT_ADDR" 0x3635C9ADC5DEA00000 --rpc-url "$RPC_URL" >/dev/null
DEPLOY_OUT="$(forge create contracts/AtlasHTLC.sol:AtlasHTLC \
  --rpc-url "$RPC_URL" --private-key "$LOCKER_KEY" --broadcast 2>&1)"
CONTRACT="$(printf '%s' "$DEPLOY_OUT" | grep -oE 'Deployed to: 0x[0-9a-fA-F]{40}' | awk '{print $3}')"
if [ -z "$CONTRACT" ]; then
  echo "Error: AtlasHTLC deploy failed"
  printf '%s\n' "$DEPLOY_OUT" | tail -30
  exit 1
fi
echo "AtlasHTLC: $CONTRACT"

cd "$REPO_ROOT"
export X3_TEST_EVM_HTLC="$CONTRACT"
export X3_TEST_EVM_LOCKER_KEY="$LOCKER_KEY"
export X3_TEST_EVM_CLAIMANT_KEY="$CLAIMANT_KEY"

TESTS=(
  real_x3vm_evm_lock_claim_atomic_lifecycle
  real_x3vm_evm_timeout_refund_atomic_lifecycle
  # Not a lifecycle: it proves the *anchor* the EVM receipt verifier reads can be
  # populated on a live chain — the council motion that enrolls the first header
  # submitter (this genesis configures no sudo key), and an attestation of a block
  # anvil actually produced. Without it "the verifier is anchored" was a claim
  # about code paths no chain had run.
  real_evm_header_attestation_populates_the_verifiers_anchor
)

failed=0
for t in "${TESTS[@]}"; do
  echo
  echo "=== $t ==="
  if env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_evm_live "$t" \
    -- --ignored --nocapture --test-threads=1; then
    echo "PASS: $t"
  else
    echo "FAIL: $t"
    failed=1
  fi
done

echo
if [ "$failed" -eq 0 ]; then
  echo "cross-domain-evm-gate: both X3VM<->EVM lifecycles passed [$POSTURE]"
else
  echo "cross-domain-evm-gate: FAILED [$POSTURE]"
fi
exit "$failed"
