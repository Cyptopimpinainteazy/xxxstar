#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/mainnet/runtime_upgrade_rehearsal.sh
#
# A runtime upgrade rehearsal that runs on this box, on the path this chain
# actually has.
#
# The bullet this serves is "Runtime upgrade: performed successfully on the live
# 7-node testnet", and the seven physical validators do not exist yet. What can be
# proven here is the upgrade itself, on the built-in three-validator `local3`
# chain, driven the only way this chain permits.
#
# There is no sudo path. `pallet_sudo` is compiled into the dev runtime but
# `development_config` leaves its key unset, and no live chain carries the pallet
# at all. Governance is what remains: `pallet_governance::enact_proposal` is the
# single dispatch in this runtime that reaches `RawOrigin::Root`, which is what
# `system.set_code` requires. So the rehearsal is:
#
#   council motion    -> authorize Alice/Bob/Charlie as governance voters
#   council motion    -> enactment period = 1 block
#   Alice             -> governance.submit_proposal(system.set_code(next runtime))
#   Alice/Bob/Charlie -> governance.vote(Aye)
#   council motion    -> fast_track(proposal, voting_period = 0)
#   Alice             -> governance.finalize_proposal(proposal)
#   on_initialize     -> enactment dispatches with Root -> the code really changes
#
# What the gate then requires of the live chain, all read back over RPC:
#
#   1. every validator reports the *next* spec version after the swap;
#   2. `:code` in storage is a different blob than before;
#   3. blocks keep being authored and GRANDPA keeps finalizing after the swap;
#   4. a value transfer signed and submitted after the swap still moves balance,
#      because "the chain is alive" and "the chain is correct" are different
#      claims and only the second one matters.
#
# The "next runtime" is built by `build_runtime_upgrade_artifact.sh` from this
# tree with `VERSION.spec_version` incremented by one — the smallest real next
# version. It is an input to the rehearsal, like the node binary, and its
# provenance (revision, both spec versions, sha256) is recorded in the report.
#
# Negative control (this is the load-bearing part). Point the rehearsal at the
# runtime the chain is *already* running and it must FAIL the version check:
#
#   X3_UPGRADE_WASM="$ROOT_DIR/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm" \
#     scripts/mainnet/runtime_upgrade_rehearsal.sh
#
# The governance path still enacts a `set_code` with Root, and the spec version
# does not move — so the check that claims "the runtime version changed" is proven
# to be able to say no. Exit 1, no report claiming PASS.
#
# Environment:
#   X3_NODE_BIN               node binary (default target/release/x3-chain-node)
#   X3_UPGRADE_WASM           next-runtime artifact; built if missing
#   X3_REHEARSAL_REPORT       report path (default reports/runtime_upgrade_rehearsal.md)
#   X3_REHEARSAL_EVIDENCE_DIR where the driver's JSON evidence lands
#   X3_REHEARSAL_TIMEOUT      seconds for each RPC wait (default 300)
#
# Exit 0 → rehearsal PASS, safe to read the report as evidence.
# Exit 1 → rehearsal FAIL; the report names the step that failed.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT="${X3_REHEARSAL_REPORT:-$ROOT_DIR/reports/runtime_upgrade_rehearsal.md}"
EVIDENCE_DIR="${X3_REHEARSAL_EVIDENCE_DIR:-$ROOT_DIR/reports/runtime_upgrade_rehearsal}"
TIMEOUT="${X3_REHEARSAL_TIMEOUT:-300}"
BUMP="${X3_REHEARSAL_EXPECTED_BUMP:-1}"

NODE_BIN="${X3_NODE_BIN:-$ROOT_DIR/target/release/x3-chain-node}"
ARTIFACT="${X3_UPGRADE_WASM:-$ROOT_DIR/target/upgrade-artifact/x3_chain_runtime.next.compact.compressed.wasm}"
ARTIFACT_PROVENANCE="$ARTIFACT.provenance.json"

NODE_MODULES="${X3_REHEARSAL_NODE_MODULES:-$ROOT_DIR/packages/blockchain-connector/node_modules}"
DRIVER="$ROOT_DIR/scripts/mainnet/runtime_upgrade_governance_driver.cjs"

declare -A RESULTS
OVERALL="PASS"
declare -a NOTES=()

pass() { RESULTS["$1"]="PASS"; printf '[PASS] %s\n' "$1"; }
fail() { RESULTS["$1"]="FAIL"; OVERALL="FAIL"; printf '[FAIL] %s %s\n' "$1" "${2:-}"; }
skip() { RESULTS["$1"]="SKIP"; printf '[SKIP] %s — %s\n' "$1" "${2:-}"; }
note() { NOTES+=("$1"); printf '[note] %s\n' "$1"; }
info() { printf '[rehearsal] %s\n' "$*"; }

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-rehearsal.XXXXXX")"
NODE_PIDS=()

cleanup() {
    for pid in "${NODE_PIDS[@]:-}"; do
        kill "$pid" >/dev/null 2>&1 || true
    done
    sleep 1
    for pid in "${NODE_PIDS[@]:-}"; do
        kill -9 "$pid" >/dev/null 2>&1 || true
    done
    rm -rf "$WORK_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# ── RPC helpers ──────────────────────────────────────────────────────────────
rpc() {  # rpc <port> <method> [params]
    curl -s -m 10 -H 'Content-Type: application/json' \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]},\"id\":1}" \
        "http://127.0.0.1:$1"
}

jstr() { sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p" | head -1; }

spec_version() { rpc "$1" state_getRuntimeVersion | jq -r '.result.specVersion // empty'; }
code_hash() { rpc "$1" state_getStorageHash '["0x3a636f6465"]' | jq -r '.result // empty'; }
best_number() { rpc "$1" chain_getHeader | jq -r '.result.number // empty' | xargs -r printf '%d\n'; }

finalized_number() {  # finalized_number <port>
    local hash
    hash="$(rpc "$1" chain_getFinalizedHead | jq -r '.result // empty')"
    [ -n "$hash" ] || return 1
    rpc "$1" chain_getHeader "[\"$hash\"]" | jq -r '.result.number // empty' | xargs -r printf '%d\n'
}

peer_count() { rpc "$1" system_health | jq -r '.result.peers // 0'; }

wait_for_rpc() {  # wait_for_rpc <name> <port>
    local deadline=$(( $(date +%s) + TIMEOUT ))
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        if rpc "$2" chain_getHeader | grep -q '"number"'; then
            info "$1 answered RPC on :$2"
            return 0
        fi
        sleep 1
    done
    return 1
}

write_minimal_report() {  # write_minimal_report <reason>
    mkdir -p "$(dirname "$REPORT")"
    {
        echo "# Runtime Upgrade Rehearsal Report"
        echo
        echo "- **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "- **Overall**: FAIL"
        echo
        echo "$1"
    } > "$REPORT"
}

# ── artifact ─────────────────────────────────────────────────────────────────
if [[ ! -f "$ARTIFACT" ]]; then
    info "no next-runtime artifact at $ARTIFACT — building one (slow, once)"
    if ! bash "$ROOT_DIR/scripts/mainnet/build_runtime_upgrade_artifact.sh" --out "$ARTIFACT"; then
        write_minimal_report "The next-runtime artifact could not be built; nothing about the upgrade was proven."
        echo "runtime_upgrade_rehearsal: FAIL"
        exit 1
    fi
fi

ARTIFACT_SHA=""
ARTIFACT_SPEC=""
ARTIFACT_REV=""
if [[ -f "$ARTIFACT" ]] && [[ "$(head -c 8 "$ARTIFACT" | xxd -p)" == "52bc537646db8e05" ]]; then
    pass "upgrade_artifact_present"
    ARTIFACT_SHA="$(sha256sum "$ARTIFACT" | awk '{print $1}')"
    if [[ -f "$ARTIFACT_PROVENANCE" ]]; then
        ARTIFACT_SPEC="$(jq -r '.spec_version_after // empty' "$ARTIFACT_PROVENANCE")"
        ARTIFACT_REV="$(jq -r '.revision // empty' "$ARTIFACT_PROVENANCE")"
    fi
    note "artifact: revision ${ARTIFACT_REV:-unknown}, spec_version ${ARTIFACT_SPEC:-unknown}, sha256 $ARTIFACT_SHA"
else
    fail "upgrade_artifact_present" "($ARTIFACT missing or not a compact+compressed blob)"
fi

# ── node binary ──────────────────────────────────────────────────────────────
FROZEN_NODE=""
NODE_SHA=""
if [[ -x "$NODE_BIN" ]]; then
    # The path is shared with every other build on this box; a relink between two
    # nodes of one set gives them different genesis state, and the set then never
    # peers. Freeze one copy and give every validator that exact file.
    FROZEN_NODE="$WORK_DIR/x3-chain-node"
    cp "$NODE_BIN" "$FROZEN_NODE"
    chmod +x "$FROZEN_NODE"
    NODE_SHA="$(sha256sum "$FROZEN_NODE" | awk '{print $1}')"
    pass "node_binary_frozen"
    note "node binary: $NODE_SHA"
else
    fail "node_binary_frozen" "(no executable at $NODE_BIN)"
fi

if [[ "$OVERALL" == "FAIL" ]]; then
    write_minimal_report "Prerequisites were missing (see the FAIL lines above); the live rehearsal did not run."
    echo "runtime_upgrade_rehearsal: FAIL"
    exit 1
fi

# ── boot local3 ──────────────────────────────────────────────────────────────
# Ports are probed rather than guessed: a second validator set on the same ports
# answers the first set's RPC and wedges instead of failing.
read -r A_RPC A_P2P A_PROM B_RPC B_P2P B_PROM C_RPC C_P2P C_PROM < <(
    python3 - <<'PY'
import socket

def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port

ports = set()
while len(ports) < 9:
    ports.add(free_port())
print(" ".join(str(port) for port in sorted(ports)))
PY
)

BASE_DIR="$WORK_DIR/chain"
LOG_DIR="$WORK_DIR/logs"
mkdir -p "$BASE_DIR" "$LOG_DIR"

start_validator() {  # start_validator <name> <key-seed> <rpc> <p2p> <prom> [extra...]
    local name="$1" seed="$2" rpc_port="$3" p2p_port="$4" prom_port="$5"; shift 5
    "$FROZEN_NODE" --chain local3 --base-path "$BASE_DIR/$name" \
        --rpc-port "$rpc_port" --port "$p2p_port" --prometheus-port "$prom_port" \
        --rpc-methods unsafe --no-mdns \
        --node-key "$(printf '%064x' "$seed")" "$@" >"$LOG_DIR/$name.log" 2>&1 &
    local pid=$!
    NODE_PIDS+=("$pid")
    # Take the validator out of the shell's job table: killing it in `cleanup`
    # otherwise makes bash print "<pid> Killed" after the verdict, which reads like
    # part of the result when it is only teardown noise.
    disown "$pid" 2>/dev/null || true
}

info "starting three validators (rpc $A_RPC/$B_RPC/$C_RPC)"
start_validator alice 1 "$A_RPC" "$A_P2P" "$A_PROM" --alice
if wait_for_rpc alice "$A_RPC"; then pass "alice_rpc"; else fail "alice_rpc" "(see $LOG_DIR/alice.log)"; fi

ALICE_PEER_ID="$(rpc "$A_RPC" system_localPeerId | jstr result)"
ALICE_BOOTNODE="/ip4/127.0.0.1/tcp/$A_P2P/p2p/$ALICE_PEER_ID"
start_validator bob 2 "$B_RPC" "$B_P2P" "$B_PROM" --bob --bootnodes "$ALICE_BOOTNODE"
if wait_for_rpc bob "$B_RPC"; then pass "bob_rpc"; else fail "bob_rpc" "(see $LOG_DIR/bob.log)"; fi

BOB_PEER_ID="$(rpc "$B_RPC" system_localPeerId | jstr result)"
BOB_BOOTNODE="/ip4/127.0.0.1/tcp/$B_P2P/p2p/$BOB_PEER_ID"
start_validator charlie 3 "$C_RPC" "$C_P2P" "$C_PROM" --charlie \
    --bootnodes "$ALICE_BOOTNODE" "$BOB_BOOTNODE"
if wait_for_rpc charlie "$C_RPC"; then pass "charlie_rpc"; else fail "charlie_rpc" "(see $LOG_DIR/charlie.log)"; fi

if [[ "$OVERALL" == "FAIL" ]]; then
    write_minimal_report "The local3 chain did not come up; nothing about the upgrade was proven."
    echo "runtime_upgrade_rehearsal: FAIL"
    exit 1
fi

connected=0
for _ in $(seq 1 90); do
    pa="$(peer_count "$A_RPC")"; pb="$(peer_count "$B_RPC")"; pc="$(peer_count "$C_RPC")"
    if [[ "${pa:-0}" -ge 2 && "${pb:-0}" -ge 2 && "${pc:-0}" -ge 2 ]]; then connected=1; break; fi
    sleep 1
done
if [[ "$connected" == 1 ]]; then
    pass "validators_connected"
else
    fail "validators_connected" "(peers alice=${pa:-?} bob=${pb:-?} charlie=${pc:-?})"
fi

PRE_FINALIZED=0
fa=0; fb=0; fc=0
for _ in $(seq 1 "$TIMEOUT"); do
    fa="$(finalized_number "$A_RPC" || echo 0)"
    fb="$(finalized_number "$B_RPC" || echo 0)"
    fc="$(finalized_number "$C_RPC" || echo 0)"
    PRE_FINALIZED=$(( fa < fb ? (fa < fc ? fa : fc) : (fb < fc ? fb : fc) ))
    if [[ "$PRE_FINALIZED" -ge 3 ]]; then break; fi
    sleep 1
done
if [[ "$PRE_FINALIZED" -ge 3 ]]; then
    pass "pre_upgrade_finality"
else
    fail "pre_upgrade_finality" "(alice=$fa bob=$fb charlie=$fc)"
fi

PRE_SPEC="$(spec_version "$A_RPC")"
PRE_CODE="$(code_hash "$A_RPC")"
PRE_BEST="$(best_number "$A_RPC")"
info "pre-upgrade: spec=$PRE_SPEC code=$PRE_CODE best=$PRE_BEST finalized=$PRE_FINALIZED"

if [[ -n "$ARTIFACT_SPEC" && -n "$PRE_SPEC" && "$ARTIFACT_SPEC" == "$PRE_SPEC" ]]; then
    note "the artifact names the running spec version ($PRE_SPEC) — the version check below must fail (negative control)"
fi

# ── run the governance upgrade ───────────────────────────────────────────────
DRIVER_JSON="$EVIDENCE_DIR/governance_upgrade.json"
mkdir -p "$EVIDENCE_DIR"
if X3_WS_URL="ws://127.0.0.1:$A_RPC" \
   X3_WASM_FILE="$ARTIFACT" \
   X3_EXPECT_OLD_SPEC_VERSION="$PRE_SPEC" \
   X3_OUT_JSON="$DRIVER_JSON" \
   NODE_PATH="$NODE_MODULES" \
   node "$DRIVER" > "$EVIDENCE_DIR/governance_upgrade.log" 2>&1; then
    pass "governance_upgrade_enacted"
else
    fail "governance_upgrade_enacted" "(see $EVIDENCE_DIR/governance_upgrade.log)"
fi

ENACTMENT_BLOCK="$(jq -r '.enactment_block // empty' "$DRIVER_JSON" 2>/dev/null || true)"
DRIVER_ERROR="$(jq -r '.error // empty' "$DRIVER_JSON" 2>/dev/null || true)"
POST_SPEC="$(spec_version "$A_RPC")"
POST_CODE="$(code_hash "$A_RPC")"
EXPECTED_SPEC=$(( PRE_SPEC + BUMP ))

# The negative control fails here, and its failure has a specific shape worth
# naming: pointed at the runtime the chain already runs, `system.set_code` is
# reached with Root and *refused* — the runtime will not accept code whose spec
# version is not greater than the running one. That refusal is the reason the
# version check below can be trusted to say no.
if [[ "$DRIVER_ERROR" == *SpecVersionNeedsToIncrease* ]]; then
    note "the chain refused the code swap with System::SpecVersionNeedsToIncrease — the artifact's spec_version is not greater than the running one"
fi

if [[ -n "$PRE_SPEC" && -n "$POST_SPEC" && "$POST_SPEC" -eq "$EXPECTED_SPEC" ]]; then
    pass "spec_version_incremented"
else
    fail "spec_version_incremented" "(alice reported $PRE_SPEC before and $POST_SPEC after; expected $EXPECTED_SPEC)"
fi

if [[ -n "$PRE_CODE" && -n "$POST_CODE" && "$POST_CODE" != "$PRE_CODE" ]]; then
    pass "code_hash_changed"
else
    fail "code_hash_changed" "(the :code hash is $POST_CODE on both sides)"
fi

# Every validator has to have applied the swap, not just the node that was asked.
upgraded=0
sa=""; sb=""; sc=""
for _ in $(seq 1 60); do
    sa="$(spec_version "$A_RPC")"; sb="$(spec_version "$B_RPC")"; sc="$(spec_version "$C_RPC")"
    if [[ -n "$PRE_SPEC" && "$sa" == "$EXPECTED_SPEC" && "$sb" == "$EXPECTED_SPEC" && "$sc" == "$EXPECTED_SPEC" ]]; then
        upgraded=1; break
    fi
    sleep 2
done
if [[ "$upgraded" == 1 ]]; then
    pass "all_validators_upgraded"
else
    fail "all_validators_upgraded" "(spec versions: alice=$sa bob=$sb charlie=$sc; expected $EXPECTED_SPEC)"
fi

# ── the chain has to keep working ────────────────────────────────────────────
POST_BEST="$(best_number "$A_RPC")"
POST_FINALIZED="$(finalized_number "$A_RPC" || echo 0)"
if [[ -z "$ENACTMENT_BLOCK" ]]; then
    skip "blocks_and_finality_after_upgrade" "no enactment block (the code swap never landed)"
elif [[ -n "$POST_BEST" && "$POST_BEST" -gt "$(( ENACTMENT_BLOCK + 2 ))" \
      && "$POST_FINALIZED" -gt "$ENACTMENT_BLOCK" ]]; then
    pass "blocks_and_finality_after_upgrade"
else
    fail "blocks_and_finality_after_upgrade" \
        "(enactment=$ENACTMENT_BLOCK best=$POST_BEST finalized=$POST_FINALIZED)"
fi

TRANSFER_DELTA="$(jq -r '.post_upgrade_transfer.observed_delta // empty' "$DRIVER_JSON" 2>/dev/null || true)"
TRANSFER_BLOCK="$(jq -r '.post_upgrade_transfer.block_number // empty' "$DRIVER_JSON" 2>/dev/null || true)"
TRANSFER_AFTER_ENACTMENT="$(jq -r 'if (.post_upgrade_transfer.block_number // 0) > (.enactment_block // 0) then "yes" else "no" end' "$DRIVER_JSON" 2>/dev/null || echo no)"
if [[ -z "$ENACTMENT_BLOCK" || -z "$TRANSFER_DELTA" ]]; then
    skip "post_upgrade_state_operation" "the upgrade did not land, so there is no post-upgrade state to exercise"
elif [[ "$TRANSFER_DELTA" -gt 0 && "$TRANSFER_AFTER_ENACTMENT" == "yes" ]]; then
    pass "post_upgrade_state_operation"
else
    fail "post_upgrade_state_operation" \
        "(transfer delta=$TRANSFER_DELTA block=$TRANSFER_BLOCK enactment=$ENACTMENT_BLOCK)"
fi

# ── report ───────────────────────────────────────────────────────────────────
mkdir -p "$(dirname "$REPORT")"
{
    echo "# Runtime Upgrade Rehearsal Report"
    echo
    echo "- **Generated**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- **Chain**: built-in \`local3\` (Alice/Bob/Charlie), three live validators"
    echo "- **Overall**: $OVERALL"
    echo "- **Method**: council governance -> \`governance.submit_proposal(system.set_code)\` -> Root enactment"
    echo "- **Sudo used**: no — this chain has no reachable sudo; governance is the only path to Root"
    if [[ -n "${DRIVER_ERROR:-}" ]]; then
        echo "- **Driver error**: \`$DRIVER_ERROR\`"
    fi
    echo
    echo "## Inputs"
    echo
    echo "| Input | Value |"
    echo "|---|---|"
    echo "| Node binary | \`$NODE_BIN\` |"
    echo "| Node binary sha256 | \`${NODE_SHA:-unknown}\` |"
    echo "| Next-runtime artifact | \`$ARTIFACT\` |"
    echo "| Artifact revision | \`${ARTIFACT_REV:-unknown}\` |"
    echo "| Artifact sha256 | \`${ARTIFACT_SHA:-unknown}\` |"
    echo "| Artifact spec_version | \`${ARTIFACT_SPEC:-unknown}\` |"
    echo "| Driver evidence | \`$DRIVER_JSON\` |"
    echo
    echo "## Pre-Upgrade State"
    echo
    echo "| Field | Value |"
    echo "|---|---|"
    echo "| spec_version | \`$PRE_SPEC\` |"
    echo "| \`:code\` hash | \`$PRE_CODE\` |"
    echo "| best block | $PRE_BEST |"
    echo "| lowest finalized block | $PRE_FINALIZED |"
    echo
    echo "## Post-Upgrade State"
    echo
    echo "| Field | Value |"
    echo "|---|---|"
    echo "| spec_version | \`$POST_SPEC\` |"
    echo "| \`:code\` hash | \`$POST_CODE\` |"
    echo "| enactment block | ${ENACTMENT_BLOCK:-unknown} |"
    echo "| best block | $POST_BEST |"
    echo "| finalized block | $POST_FINALIZED |"
    echo "| post-upgrade transfer | block ${TRANSFER_BLOCK:-unknown}, +${TRANSFER_DELTA:-0} planck |"
    echo
    echo "## Checks"
    echo
    for key in \
        upgrade_artifact_present \
        node_binary_frozen \
        alice_rpc \
        bob_rpc \
        charlie_rpc \
        validators_connected \
        pre_upgrade_finality \
        governance_upgrade_enacted \
        spec_version_incremented \
        code_hash_changed \
        all_validators_upgraded \
        blocks_and_finality_after_upgrade \
        post_upgrade_state_operation; do
        echo "- $key: ${RESULTS[$key]:-NOT_RUN}"
    done
    echo
    if [[ "${#NOTES[@]}" -gt 0 ]]; then
        echo "## Notes"
        echo
        for line in "${NOTES[@]}"; do
            echo "- $line"
        done
        echo
    fi
    echo "## Gate"
    echo
    if [[ "$OVERALL" == "PASS" ]]; then
        echo 'runtime_upgrade_rehearsal: PASS'
    else
        echo 'runtime_upgrade_rehearsal: FAIL'
    fi
} > "$REPORT"

SELF_HASH="$(sha256sum "$REPORT" | awk '{print $1}')"
echo "" >> "$REPORT"
echo "_Report hash: \`$SELF_HASH\`_" >> "$REPORT"

echo
echo "══════════════════════════════════════════"
echo "  runtime_upgrade_rehearsal: $OVERALL"
echo "  Report: $REPORT"
echo "══════════════════════════════════════════"

if [[ "$OVERALL" == "FAIL" ]]; then
    exit 1
fi
exit 0
