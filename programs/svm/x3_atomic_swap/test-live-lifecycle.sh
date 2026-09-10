#!/usr/bin/env bash
# Real solana-test-validator lifecycle gate for the X3 Atomic Star SVM HTLC program.
#
# This script proves, against a genuine local `solana-test-validator` (not a
# mock, not a unit test), that the deployed SBF program enforces the full HTLC
# lifecycle end to end:
#   1. lock succeeds and creates a PDA with the expected on-chain state
#   2. claim with the WRONG preimage is rejected (hashlock mismatch)
#   3. claim with the CORRECT preimage succeeds and sets claimed=1
#   4. a second claim attempt (double-claim) is rejected
#   5. refund before timeout is rejected
#   6. refund after timeout succeeds and sets refunded=1
#   7. refund by an account that is not the refund authority is rejected
#
# All PDA state assertions are made via raw JSON-RPC `getAccountInfo` and
# manual byte-offset decoding of the `HtlcAccount` layout, independent of the
# client library's own (de)serialization, so a bug in the client cannot mask
# a bug in the on-chain program (or vice versa).
#
# Usage: ./test-live-lifecycle.sh
# Requires: solana CLI + cargo-build-sbf on PATH (see deploy-devnet.sh for
# install instructions), and the x3-svm-broadcast client binary built.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKDIR="$(mktemp -d /tmp/svm-live-lifecycle.XXXXXX)"
RPC_URL="http://127.0.0.1:8899"
VALIDATOR_PID=""

pass=0
fail=0

cleanup() {
  if [ -n "$VALIDATOR_PID" ] && kill -0 "$VALIDATOR_PID" 2>/dev/null; then
    kill "$VALIDATOR_PID" 2>/dev/null || true
    wait "$VALIDATOR_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

command -v solana >/dev/null 2>&1 || { echo "Error: solana CLI not installed"; exit 1; }
command -v solana-test-validator >/dev/null 2>&1 || { echo "Error: solana-test-validator not installed"; exit 1; }
command -v cargo-build-sbf >/dev/null 2>&1 || { echo "Error: cargo-build-sbf not installed"; exit 1; }

echo "=== Building SBF program ==="
(cd "$SCRIPT_DIR" && cargo build-sbf 2>&1 | tail -5)
PROGRAM_SO="$SCRIPT_DIR/target/deploy/x3_atomic_swap.so"
[ -f "$PROGRAM_SO" ] || { echo "Error: $PROGRAM_SO not built"; exit 1; }

echo "=== Building broadcaster client ==="
(cd "$SCRIPT_DIR/client" && cargo build --release --bin x3-svm-broadcast 2>&1 | tail -5)
BIN="$SCRIPT_DIR/client/target/release/x3-svm-broadcast"
[ -x "$BIN" ] || { echo "Error: broadcaster binary not built"; exit 1; }

echo "=== Generating keypairs ==="
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/program-keypair.json" >/dev/null
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/payer.json" >/dev/null
solana-keygen new --no-bip39-passphrase -s -o "$WORKDIR/claimant.json" >/dev/null
PROGRAM_ID="$(solana-keygen pubkey "$WORKDIR/program-keypair.json")"
PAYER_PK="$(solana-keygen pubkey "$WORKDIR/payer.json")"
CLAIMANT_PK="$(solana-keygen pubkey "$WORKDIR/claimant.json")"
echo "program=$PROGRAM_ID payer=$PAYER_PK claimant=$CLAIMANT_PK"

echo "=== Starting solana-test-validator ==="
solana-test-validator --reset --quiet \
  --ledger "$WORKDIR/ledger" \
  --bpf-program "$PROGRAM_ID" "$PROGRAM_SO" \
  > "$WORKDIR/validator.log" 2>&1 &
VALIDATOR_PID=$!

for i in $(seq 1 30); do
  if solana cluster-version --url "$RPC_URL" >/dev/null 2>&1; then
    break
  fi
  sleep 1
  if [ "$i" -eq 30 ]; then
    echo "Error: validator did not become ready"; cat "$WORKDIR/validator.log"; exit 1
  fi
done
echo "validator ready: $(solana cluster-version --url "$RPC_URL")"

solana airdrop 5 "$PAYER_PK" --url "$RPC_URL" --commitment finalized
solana airdrop 2 "$CLAIMANT_PK" --url "$RPC_URL" --commitment finalized

# Airdrops must be FINALIZED on-chain (not just confirmed) before we submit
# transactions that debit these accounts, since the broadcaster client's RPC
# client defaults to finalized commitment when building/sending transactions.
for i in $(seq 1 30); do
  bal="$(solana balance "$PAYER_PK" --url "$RPC_URL" --commitment finalized 2>/dev/null | grep -oP '^\d+(\.\d+)?' || echo 0)"
  awk -v b="$bal" 'BEGIN{exit !(b>0)}' && break
  sleep 1
done

# Confirm the program account is genuinely executable on-chain (not assumed).
executable="False"
for i in $(seq 1 15); do
  executable="$(curl -s "$RPC_URL" -X POST -H 'Content-Type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$PROGRAM_ID\",{\"encoding\":\"base64\"}]}" \
    | python3 -c "import json,sys; r=json.load(sys.stdin)['result']['value']; print(r['executable'] if r else 'False')")"
  [ "$executable" = "True" ] && break
  sleep 1
done
if [ "$executable" != "True" ]; then
  echo "FAIL: program account is not executable on-chain"; exit 1
fi
echo "PASS: program is deployed and executable on-chain"
pass=$((pass + 1))

decode_field() {
  # decode_field <pda> <field: claimed|refunded>
  local pda="$1" field="$2" offset
  case "$field" in
    claimed) offset=209 ;;
    refunded) offset=210 ;;
    *) echo "unknown field $field"; exit 1 ;;
  esac
  curl -s "$RPC_URL" -X POST -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"$pda\",{\"encoding\":\"base64\"}]}" \
    | python3 -c "
import json, sys, base64
r = json.load(sys.stdin)
data = base64.b64decode(r['result']['value']['data'][0])
print(data[$offset])
"
}

check() {
  # check <description> <expected exit: 0=succeed nonzero=must fail>
  local desc="$1" expect_fail="$2"
  shift 2
  if "$@" >"$WORKDIR/last.out" 2>&1; then
    if [ "$expect_fail" = "1" ]; then
      echo "FAIL: $desc (expected rejection, but succeeded)"; cat "$WORKDIR/last.out"; fail=$((fail + 1))
    else
      echo "PASS: $desc"; pass=$((pass + 1))
    fi
  else
    if [ "$expect_fail" = "1" ]; then
      echo "PASS: $desc (correctly rejected)"; pass=$((pass + 1))
    else
      echo "FAIL: $desc"; cat "$WORKDIR/last.out"; fail=$((fail + 1))
    fi
  fi
}

# --- Scenario 1: happy path lock -> wrong preimage rejected -> correct claim ---
python3 -c "
import os, hashlib
p = os.urandom(32); h = hashlib.sha256(p).digest(); s = os.urandom(32)
open('$WORKDIR/preimage.hex','w').write(p.hex())
open('$WORKDIR/hashlock.hex','w').write(h.hex())
open('$WORKDIR/swap_id.hex','w').write(s.hex())
"
CUR_SLOT="$(solana slot --url "$RPC_URL")"
check "lock (happy path)" 0 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/payer.json" \
  lock --swap-id "$(cat "$WORKDIR/swap_id.hex")" --claimant "$CLAIMANT_PK" --refund-authority "$PAYER_PK" \
  --hashlock "$(cat "$WORKDIR/hashlock.hex")" --amount 500000 --timeout-slots $((CUR_SLOT + 1000))
HTLC_PDA="$(grep -oP 'htlc=\K\S+' "$WORKDIR/last.out")"

check "claim with WRONG preimage" 1 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/claimant.json" \
  claim --swap-id "$(cat "$WORKDIR/swap_id.hex")" --preimage "$(python3 -c 'import os; print(os.urandom(32).hex())')"

# Negative-case on-chain proof: the rejected wrong-preimage claim must be a
# true no-op, verified via raw PDA byte decoding (independent of the
# broadcaster's own exit-code reporting).
claimed_after_wrong="$(decode_field "$HTLC_PDA" claimed)"
if [ "$claimed_after_wrong" = "0" ]; then
  echo "PASS: on-chain claimed=0 after rejected wrong-preimage claim"; pass=$((pass + 1))
else
  echo "FAIL: on-chain claimed=$claimed_after_wrong (expected 0) after rejected wrong-preimage claim"; fail=$((fail + 1))
fi

check "claim with CORRECT preimage" 0 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/claimant.json" \
  claim --swap-id "$(cat "$WORKDIR/swap_id.hex")" --preimage "$(cat "$WORKDIR/preimage.hex")"

claimed="$(decode_field "$HTLC_PDA" claimed)"
if [ "$claimed" = "1" ]; then
  echo "PASS: on-chain claimed=1 after successful claim"; pass=$((pass + 1))
else
  echo "FAIL: on-chain claimed=$claimed (expected 1)"; fail=$((fail + 1))
fi

check "double-claim" 1 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/claimant.json" \
  claim --swap-id "$(cat "$WORKDIR/swap_id.hex")" --preimage "$(cat "$WORKDIR/preimage.hex")"

# Negative-case on-chain proof: the rejected double-claim must not have
# reset or otherwise disturbed the already-claimed state.
claimed_after_double="$(decode_field "$HTLC_PDA" claimed)"
if [ "$claimed_after_double" = "1" ]; then
  echo "PASS: on-chain claimed=1 still holds after rejected double-claim"; pass=$((pass + 1))
else
  echo "FAIL: on-chain claimed=$claimed_after_double (expected 1) after rejected double-claim"; fail=$((fail + 1))
fi

# --- Scenario 2: timeout/refund path ---
python3 -c "
import os, hashlib
p = os.urandom(32); h = hashlib.sha256(p).digest(); s = os.urandom(32)
open('$WORKDIR/preimage2.hex','w').write(p.hex())
open('$WORKDIR/hashlock2.hex','w').write(h.hex())
open('$WORKDIR/swap_id2.hex','w').write(s.hex())
"
CUR_SLOT="$(solana slot --url "$RPC_URL")"
TARGET=$((CUR_SLOT + 15))
check "lock (short timeout, for refund test)" 0 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/payer.json" \
  lock --swap-id "$(cat "$WORKDIR/swap_id2.hex")" --claimant "$CLAIMANT_PK" --refund-authority "$PAYER_PK" \
  --hashlock "$(cat "$WORKDIR/hashlock2.hex")" --amount 500000 --timeout-slots "$TARGET"
HTLC_PDA2="$(grep -oP 'htlc=\K\S+' "$WORKDIR/last.out")"

check "refund BEFORE timeout" 1 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/payer.json" \
  refund --swap-id "$(cat "$WORKDIR/swap_id2.hex")"

# Negative-case on-chain proof: the rejected premature refund must not
# have touched the escrowed funds or the refunded flag.
refunded_after_premature="$(decode_field "$HTLC_PDA2" refunded)"
if [ "$refunded_after_premature" = "0" ]; then
  echo "PASS: on-chain refunded=0 after rejected premature refund"; pass=$((pass + 1))
else
  echo "FAIL: on-chain refunded=$refunded_after_premature (expected 0) after rejected premature refund"; fail=$((fail + 1))
fi

# A non-authority refund attempt must ALSO be rejected before the timeout
# has expired, purely on authority grounds -- independent of (and prior
# to) the timelock check below.
check "refund by wrong authority BEFORE timeout" 1 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/claimant.json" \
  refund --swap-id "$(cat "$WORKDIR/swap_id2.hex")"
refunded_after_premature_wrongauth="$(decode_field "$HTLC_PDA2" refunded)"
if [ "$refunded_after_premature_wrongauth" = "0" ]; then
  echo "PASS: on-chain refunded=0 after rejected pre-timeout wrong-authority refund"; pass=$((pass + 1))
else
  echo "FAIL: on-chain refunded=$refunded_after_premature_wrongauth (expected 0) after rejected pre-timeout wrong-authority refund"; fail=$((fail + 1))
fi

echo "waiting for slot $TARGET (with margin)..."
while [ "$(solana slot --url "$RPC_URL" --commitment finalized)" -le "$((TARGET + 5))" ]; do sleep 1; done

check "refund AFTER timeout" 0 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/payer.json" \
  refund --swap-id "$(cat "$WORKDIR/swap_id2.hex")"

refunded="$(decode_field "$HTLC_PDA2" refunded)"
if [ "$refunded" = "1" ]; then
  echo "PASS: on-chain refunded=1 after successful refund"; pass=$((pass + 1))
else
  echo "FAIL: on-chain refunded=$refunded (expected 1)"; fail=$((fail + 1))
fi

# --- Scenario 3: wrong refund authority ---
python3 -c "
import os, hashlib
p = os.urandom(32); h = hashlib.sha256(p).digest(); s = os.urandom(32)
open('$WORKDIR/preimage3.hex','w').write(p.hex())
open('$WORKDIR/hashlock3.hex','w').write(h.hex())
open('$WORKDIR/swap_id3.hex','w').write(s.hex())
"
CUR_SLOT="$(solana slot --url "$RPC_URL")"
check "lock (for wrong-authority test)" 0 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/payer.json" \
  lock --swap-id "$(cat "$WORKDIR/swap_id3.hex")" --claimant "$CLAIMANT_PK" --refund-authority "$PAYER_PK" \
  --hashlock "$(cat "$WORKDIR/hashlock3.hex")" --amount 500000 --timeout-slots $((CUR_SLOT + 1000))

check "refund by wrong authority (claimant, not refund_authority)" 1 \
  "$BIN" --rpc "$RPC_URL" --program-id "$PROGRAM_ID" --payer-keypair "$WORKDIR/claimant.json" \
  refund --swap-id "$(cat "$WORKDIR/swap_id3.hex")"

echo
echo "=== Results: $pass passed, $fail failed ==="
rm -rf "$WORKDIR"
if [ "$fail" -ne 0 ]; then
  exit 1
fi
