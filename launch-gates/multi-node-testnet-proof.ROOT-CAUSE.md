# Multi-Node Testnet Proof — Root Cause Analysis (0 Blocks)

**Script**: `launch-gates/multi-node-testnet-proof.sh`
**Symptom**: PASS_COUNT for "consecutive blocks" stays at 0; chain never produces blocks.
**Date**: 2026-04-28

## Root causes (in order of severity)

### 1. Validators are killed after 10 seconds (BLOCKER)
**Lines 117–124**:
```bash
timeout 10 ./target/release/x3-chain-node \
    --base-path "$TEST_DIR/validator-$i" \
    --chain "$TEST_DIR/chain-spec.json" \
    --validator \
    ...
    > "$TEST_DIR/validator-$i.log" 2>&1 &
```
The `timeout 10 ... &` background job dies after 10s. The script then sleeps 5s, then loops up to 12 × 5s = 60s checking for blocks. By the time the first RPC poll runs, all four validators have already exited.

**Fix**: drop `timeout 10`. Use the `trap cleanup EXIT` (already present, line 49) to kill children at script end.

### 2. Bootnode uses fabricated peer ID (BLOCKER)
**Line 122**: `--bootnodes "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWSJ5YhzNFU2EqCPzpvfWpZGMf6Yjs6XGxHqEXnVjRNLSQ"`

That peer ID is hardcoded and will never match Alice's actual ed25519 node identity (which is randomly generated unless `--node-key`/`--node-key-file` is set). All non-Alice validators reject the bootnode handshake → no peer → no consensus.

**Fix**:
- Generate a deterministic node-key for validator 0 with `subkey generate-node-key --file alice.key`
- Pass `--node-key-file alice.key` to validator 0 so its peer ID is stable
- Compute its peer ID via `subkey inspect-node-key --file alice.key`
- Use that real peer ID in `--bootnodes` for validators 1–3
- Validator 0 itself doesn't need `--bootnodes`

### 3. `--chain dev` only has Alice as session-key authority (BLOCKER)
**Line 75**: `./target/release/x3-chain-node build-spec --chain dev`

The `dev` chain spec (per substrate convention) seeds session keys for **Alice only**. Bob/Charlie/Dave have addresses in the genesis but no aura/grandpa session keys → they can't propose or finalize blocks.

**Fix**: use `--chain local` (which seeds Alice + Bob keys by default) or generate a custom 4-authority spec with `build-spec --chain local --raw > spec.json` and ensure all 4 validator keys are inserted via `key insert --suri //Alice` (and //Bob, //Charlie, //Dave) into each validator's keystore.

### 4. P2P port collision (LIKELY)
**Line 121**: `--port $((PORT + i))` where `PORT=${VALIDATOR_PORTS[$i]}` and `VALIDATOR_PORTS=(9944 9954 9964 9974)`.
This evaluates to: 9944+0=9944, 9954+1=9955, 9964+2=9966, 9974+3=9977.
The first validator binds P2P on 9944 — but `--rpc-port "$PORT"` (line 122) ALSO uses 9944 for validator 0 → port conflict on the same process.

**Fix**: use disjoint port ranges, e.g. P2P=30333..30336, RPC=9944..9947.

### 5. Each validator has fresh keystore but no key insertion (BLOCKER)
Even with `--chain local`, the per-validator `--base-path "$TEST_DIR/validator-$i"` creates a fresh empty keystore. Nothing in the script runs `x3-chain-node key insert ...` to populate it with the dev keys.

**Fix**: between starting and waiting, insert the dev session keys:
```bash
./target/release/x3-chain-node key insert --base-path "$TEST_DIR/validator-$i" \
    --chain "$TEST_DIR/chain-spec.json" \
    --scheme Sr25519 --suri "//$VALIDATOR_NAME" --key-type aura
./target/release/x3-chain-node key insert --base-path "$TEST_DIR/validator-$i" \
    --chain "$TEST_DIR/chain-spec.json" \
    --scheme Ed25519 --suri "//$VALIDATOR_NAME" --key-type gran
```

### 6. `--unsafe-rpc-external` may not include `--rpc-methods unsafe` for `system_addReservedPeer`
**Line 173**: tests `system_addReservedPeer` which is an Unsafe-class RPC. Modern substrate disallows unsafe methods unless `--rpc-methods unsafe` is set explicitly.

**Fix**: add `--rpc-methods unsafe` (only for testnet proof script).

## Estimated rewrite scope
~80 lines changed in `multi-node-testnet-proof.sh`:
- Generate stable node-key for alice
- Switch to `--chain local`
- Insert session keys per validator
- Remove `timeout 10`, fix port allocation
- Plumb the real peer ID into `--bootnodes`

Out of scope for this session — flagged for next focused work.
