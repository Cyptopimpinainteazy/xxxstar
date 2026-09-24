#!/usr/bin/env bash
# Snapshot murder test.
#
# The storage audit's state-sync checklist is a list of things a snapshot source
# must refuse: a malicious mirror, a corrupt chunk, a wrong state root, a wrong
# block hash, a wrong runtime version, a stale snapshot, a snapshot from another
# chain, an incomplete snapshot. The unit tests cover those cases in memory; this
# script covers them *on disk*, through the shipped binary and the same file
# layout an operator would download, so the refusal is demonstrated end to end
# rather than asserted about a function.
#
# Every case below is applied to a real snapshot built from this repository's own
# raw chain spec. The honest copy must verify; each mutation must be refused.
#
# Usage: bash launch-gates/snapshot-murder-test.sh
# Exit 0 = every case behaved correctly.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SPEC="${X3_MURDER_SPEC:-$REPO_ROOT/deployment/chain-specs/x3-testnet-raw.json}"
CHAIN_ID="x3_testnet_v1"
BLOCK_NUMBER=1000
BLOCK_HASH="0x$(printf 'ab%.0s' $(seq 32))"
RUNTIME_VERSION=23
FINALITY_PROOF="0xdeadbeef"
# Small chunks on purpose: several chunks are what make the "chunk missing" and
# "chunk truncated" cases mean anything.
CHUNK_SIZE=262144

pass=0
fail=0

say() { printf '%s\n' "$*"; }

note_pass() {
  say "PASS  $1"
  pass=$((pass + 1))
}

note_fail() {
  say "FAIL  $1"
  fail=$((fail + 1))
}

# expect_refused <case> <manifest> <chunks-dir> [verifier args...]
expect_refused() {
  local case_name="$1"
  local manifest="$2"
  local chunks="$3"
  shift 3

  local output
  if output="$("$VERIFIER" verify --manifest "$manifest" --chunks "$chunks" "$@" 2>&1)"; then
    note_fail "$case_name (verifier accepted it; it must refuse)"
    say "      output: $output"
  else
    note_pass "$case_name"
  fi
}

# expect_ok <case> <manifest> <chunks-dir> [verifier args...]
expect_ok() {
  local case_name="$1"
  local manifest="$2"
  local chunks="$3"
  shift 3

  local output
  if output="$("$VERIFIER" verify --manifest "$manifest" --chunks "$chunks" "$@" 2>&1)"; then
    note_pass "$case_name"
  else
    note_fail "$case_name (verifier refused the honest snapshot)"
    say "      output: $output"
  fi
}

# ── Build the verifier and the honest snapshot ──────────────────────────────
VERIFIER="${X3_SNAPSHOT_VERIFIER:-}"
if [[ -z "$VERIFIER" ]]; then
  for candidate in target/release/x3-state-snapshot target/debug/x3-state-snapshot; do
    if [[ -x "$REPO_ROOT/$candidate" ]]; then
      VERIFIER="$REPO_ROOT/$candidate"
      break
    fi
  done
fi
if [[ -z "$VERIFIER" || ! -x "$VERIFIER" ]]; then
  say "building target/debug/x3-state-snapshot ..."
  if ! (cd "$REPO_ROOT" && SKIP_WASM_BUILD=1 cargo build -q -p x3-state-snapshot); then
    say "FAIL  could not build the verifier"
    exit 1
  fi
  VERIFIER="$REPO_ROOT/target/debug/x3-state-snapshot"
fi

if [[ ! -f "$SPEC" ]]; then
  say "FAIL  chain spec not found: $SPEC"
  exit 1
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/x3-murder-XXXXXX")"
HONEST="$WORK/honest"
mkdir -p "$HONEST"

BUILD_OUTPUT="$("$VERIFIER" build \
  --from-raw-spec "$SPEC" \
  --out "$HONEST" \
  --chain-id "$CHAIN_ID" \
  --block-number "$BLOCK_NUMBER" \
  --block-hash "$BLOCK_HASH" \
  --runtime-version "$RUNTIME_VERSION" \
  --finality-proof "$FINALITY_PROOF" \
  --chunk-size "$CHUNK_SIZE" 2>&1)"
if [[ $? -ne 0 ]]; then
  say "FAIL  could not build a snapshot from $SPEC"
  say "      $BUILD_OUTPUT"
  exit 1
fi

STATE_ROOT="$(python3 -c "
import json,sys
print(json.load(open('$HONEST/manifest.json'))['state_root'])
")"
CHUNK_COUNT="$(python3 -c "
import json,sys
print(json.load(open('$HONEST/manifest.json'))['chunk_count'])
")"

say "snapshot: $CHUNK_COUNT chunk(s), state root $STATE_ROOT"
say ""

ANCHOR=(--chain-id "$CHAIN_ID" --block-hash "$BLOCK_HASH" --state-root "$STATE_ROOT" --runtime-version "$RUNTIME_VERSION")

# ── Case 0: the honest snapshot must verify ─────────────────────────────────
expect_ok "honest snapshot verifies" "$HONEST/manifest.json" "$HONEST" "${ANCHOR[@]}"

# ── Helper to make a fresh copy of the snapshot for a mutation ──────────────
fresh_copy() {
  local dir="$WORK/$1"
  mkdir -p "$dir"
  cp "$HONEST"/manifest.json "$dir/"
  cp "$HONEST"/*.chunk "$dir/"
  printf '%s' "$dir"
}

# ── Case 1: a single flipped byte in a chunk ────────────────────────────────
C1="$(fresh_copy corrupt-chunk)"
printf '\x7f' | dd of="$C1/0.chunk" bs=1 seek=100 count=1 conv=notrunc status=none
expect_refused "corrupt chunk (one flipped byte)" "$C1/manifest.json" "$C1" "${ANCHOR[@]}"

# ── Case 2: a truncated chunk ───────────────────────────────────────────────
C2="$(fresh_copy truncated-chunk)"
head -c $((CHUNK_SIZE / 2)) "$HONEST/0.chunk" > "$C2/0.chunk"
expect_refused "truncated chunk" "$C2/manifest.json" "$C2" "${ANCHOR[@]}"

# ── Case 3: an incomplete snapshot (a chunk is missing from the mirror) ─────
C3="$(fresh_copy missing-chunk)"
if [[ "$CHUNK_COUNT" -lt 2 ]]; then
  say "SKIP  incomplete snapshot (the fixture produced a single chunk)"
else
  rm -f "$C3/1.chunk"
  expect_refused "incomplete snapshot (chunk 1 deleted)" "$C3/manifest.json" "$C3" "${ANCHOR[@]}"
fi

# ── Case 4: the state root the mirror claims is not the state it serves ─────
C4="$(fresh_copy wrong-state-root)"
python3 - "$C4/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
doc["state_root"] = "0x" + "ee" * 32
json.dump(doc, open(path, "w"), indent=2)
PY
expect_refused "substituted state (manifest points at another block's root)" "$C4/manifest.json" "$C4" \
  --chain-id "$CHAIN_ID" --block-hash "$BLOCK_HASH" --state-root "0x$(printf 'ee%.0s' $(seq 32))" --runtime-version "$RUNTIME_VERSION"
expect_refused "substituted state (anchor left at the real root)" "$C4/manifest.json" "$C4" "${ANCHOR[@]}"

# ── Case 5: a snapshot anchored to a different block ────────────────────────
C5="$(fresh_copy wrong-block)"
python3 - "$C5/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
doc["block_hash"] = "0x" + "11" * 32
json.dump(doc, open(path, "w"), indent=2)
PY
expect_refused "wrong block hash" "$C5/manifest.json" "$C5" "${ANCHOR[@]}"

# ── Case 6: a snapshot from another chain ───────────────────────────────────
C6="$(fresh_copy wrong-chain)"
python3 - "$C6/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
doc["chain_id"] = "x3_mainnet"
json.dump(doc, open(path, "w"), indent=2)
PY
expect_refused "snapshot from another chain" "$C6/manifest.json" "$C6" "${ANCHOR[@]}"

# ── Case 7: a snapshot produced by a different runtime ──────────────────────
C7="$(fresh_copy wrong-runtime)"
python3 - "$C7/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
doc["runtime_spec_version"] = doc["runtime_spec_version"] + 1
json.dump(doc, open(path, "w"), indent=2)
PY
expect_refused "snapshot from a different runtime version" "$C7/manifest.json" "$C7" "${ANCHOR[@]}"

# ── Case 8: a stale snapshot, judged by the caller's freshness floor ────────
expect_refused "stale snapshot (caller's minimum block is higher)" "$HONEST/manifest.json" "$HONEST" \
  "${ANCHOR[@]}" --min-block $((BLOCK_NUMBER + 1))

# ── Case 9: the manifest is not the published one ───────────────────────────
expect_refused "manifest hash does not match the published one" "$HONEST/manifest.json" "$HONEST" \
  "${ANCHOR[@]}" --manifest-hash "0x$(printf '00%.0s' $(seq 32))"

# ── Case 10: the manifest lies about how many chunks there are ──────────────
C10="$(fresh_copy wrong-chunk-count)"
python3 - "$C10/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path))
doc["chunk_count"] = doc["chunk_count"] + 1
json.dump(doc, open(path, "w"), indent=2)
PY
expect_refused "manifest declares more chunks than it has hashes" "$C10/manifest.json" "$C10" "${ANCHOR[@]}"

# ── Case 11: substituted state that passes the chunk layer ──────────────────
# The careful attack. Instead of corrupting bytes (which the chunk hash catches),
# the mirror serves a *different but well-formed* state and rewrites the manifest
# to declare that state's chunk hashes, so the chunk layer is satisfied. What it
# cannot rewrite is the state root, because the caller's anchor does not come
# from the mirror. The entries must therefore recompute to something other than
# the declared root, and that is the check that has to catch it.
SUBSTITUTED="$WORK/substituted"
python3 - "$SPEC" "$WORK/substituted-spec.json" <<'PY'
import json, sys
spec = json.load(open(sys.argv[1]))
top = spec["genesis"]["raw"]["top"]
key = sorted(top)[0]
value = bytearray.fromhex(top[key][2:])
value[-1] ^= 0x01
top[key] = "0x" + value.hex()
json.dump(spec, open(sys.argv[2], "w"))
PY
mkdir -p "$SUBSTITUTED"
"$VERIFIER" build \
  --from-raw-spec "$WORK/substituted-spec.json" \
  --out "$SUBSTITUTED" \
  --chain-id "$CHAIN_ID" \
  --block-number "$BLOCK_NUMBER" \
  --block-hash "$BLOCK_HASH" \
  --runtime-version "$RUNTIME_VERSION" \
  --finality-proof "$FINALITY_PROOF" \
  --chunk-size "$CHUNK_SIZE" > /dev/null 2>&1

# Forge a manifest in its own directory: keep the honest anchor fields, adopt the
# substituted state's chunk list and chunks, so every chunk hash resolves.
C11="$WORK/forged"
mkdir -p "$C11"
cp "$SUBSTITUTED"/*.chunk "$C11/"
python3 - "$HONEST/manifest.json" "$SUBSTITUTED/manifest.json" "$C11/manifest.json" <<'PY'
import json, sys
forged = json.load(open(sys.argv[1]))
substituted = json.load(open(sys.argv[2]))
forged["chunk_size"] = substituted["chunk_size"]
forged["chunk_count"] = substituted["chunk_count"]
forged["chunk_hashes"] = substituted["chunk_hashes"]
json.dump(forged, open(sys.argv[3], "w"), indent=2)
PY

expect_refused "substituted state with a forged, self-consistent manifest" "$C11/manifest.json" "$C11" "${ANCHOR[@]}"

say ""
say "murder test: $pass passed, $fail failed (workdir $WORK)"

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi
