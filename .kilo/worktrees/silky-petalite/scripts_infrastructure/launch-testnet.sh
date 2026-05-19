#!/usr/bin/env bash
# =============================================================================
# X3 Chain Testnet Launch Script
# =============================================================================
# Launches a 4-validator testnet on localhost with staggered ports.
#
# Usage:
#   ./launch-testnet.sh            # Launch all 4 validators
#   ./launch-testnet.sh validator1 # Launch a single validator
#   ./launch-testnet.sh stop       # Stop all validators
#
# Each validator gets unique ports:
#   TestnetAlpha:  P2P=30333, RPC=9944, WS=9944
#   TestnetBeta:   P2P=30334, RPC=9945, WS=9945
#   TestnetGamma:  P2P=30335, RPC=9946, WS=9946
#   TestnetDelta:  P2P=30336, RPC=9947, WS=9947
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY="${ROOT_DIR}/target/release/x3-chain-node"
BASE_PATH="${ROOT_DIR}/testnet-data"
CHAIN="testnet"
CHAIN_ID="x3_chain_testnet"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check binary exists
check_binary() {
    if [ ! -f "$BINARY" ]; then
        log_error "Binary not found at $BINARY"
        log_info "Build with: cargo build --release -p x3-chain-node"
        exit 1
    fi
}

# Validator configuration
declare -A VALIDATORS=(
    [validator1]="TestnetAlpha:30333:9944"
    [validator2]="TestnetBeta:30334:9945"
    [validator3]="TestnetGamma:30335:9946"
    [validator4]="TestnetDelta:30336:9947"
)

# Insert aura (sr25519) and grandpa (ed25519) keys into a validator's keystore.
# Keys are seeded from the derivation paths used in testnet_config() in chain_spec.rs.
insert_keys() {
    local name="$1"
    local seed="$2"
    local keystore_path="${BASE_PATH}/${name}/chains/${CHAIN_ID}/keystore"
    local aura_pub_hex=""
    local gran_pub_hex=""

    case "$seed" in
        TestnetAlpha)
            aura_pub_hex="787046590a67ef11eef127784f72d74e8e2693f330e481dd45db2f15ec05a83a"
            gran_pub_hex="2b981ee8cffd21b55441e715b993e2c45c0156c1c0e6b6aaf32cf0449089e749"
            ;;
        TestnetBeta)
            aura_pub_hex="94038d1ccc0a360e8e7d106203dc588fdaa068cb590a625b7a3fa93f05fe7e50"
            gran_pub_hex="b2eefabe8a9021eb68c5b9eb73c70e54696ddb4a000ebc78ce7f09cf0f6f1d4c"
            ;;
        TestnetGamma)
            aura_pub_hex="28e766946f5f500671ebb04f8d5dd2c14010deffe11e3450acff5c1f4f4be962"
            gran_pub_hex="464d82bf9dd86503a9e9494f087e65babaac9a6259765e4a1f7196bbc5a9dbae"
            ;;
        TestnetDelta)
            aura_pub_hex="58cd0afc7cab708ba8802c3197f26f5957f1b2c99483284d0619d9874855ee06"
            gran_pub_hex="cb7c3b745e2bc95bd89cadc30bafdb374527681fe9f99eca64d7a12c28ec9c2c"
            ;;
        *)
            log_error "Unknown validator seed '${seed}' for ${name}"
            return 1
            ;;
    esac

    mkdir -p "$keystore_path"

    # Substrate file keystore format: <keytype_hex><pubkey_hex> => "//Seed"
    # keytype "aura" => 61757261, "gran" => 6772616e
    printf '"//%s"' "$seed" > "${keystore_path}/61757261${aura_pub_hex}"
    printf '"//%s"' "$seed" > "${keystore_path}/6772616e${gran_pub_hex}"

    log_info "Keys seeded for ${name} (///${seed}) → ${keystore_path}"
}

launch_validator() {
    local name="$1"
    local bootnode="${2:-}"
    local config="${VALIDATORS[$name]}"
    IFS=':' read -r seed p2p_port rpc_port <<< "$config"
    local prometheus_port=$((9615 + rpc_port - 9944))

    local data_dir="${BASE_PATH}/${name}"
    mkdir -p "$data_dir"

    log_info "Launching $seed (P2P: $p2p_port, RPC: $rpc_port)..."

    local boot_args=()
    if [ -n "$bootnode" ]; then
        boot_args=(--bootnodes "$bootnode")
    fi

    "$BINARY" \
        --chain="$CHAIN" \
        --base-path="$data_dir" \
        --name="$seed" \
        --validator \
        --port="$p2p_port" \
        --rpc-port="$rpc_port" \
        --rpc-cors=all \
        --rpc-methods=Unsafe \
        --unsafe-rpc-external \
        --prometheus-port="$prometheus_port" \
        --allow-private-ip \
        --force-authoring \
        --log="info" \
        --telemetry-url="wss://telemetry.polkadot.io/submit/ 0" \
        "${boot_args[@]}" \
        > "${data_dir}/${name}.log" 2>&1 &

    echo $! > "${data_dir}/${name}.pid"
    log_info "$seed started (PID: $(cat "${data_dir}/${name}.pid"))"
}

wait_for_rpc() {
    local rpc_port="$1"
    for _ in $(seq 1 40); do
        if curl -s -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
            "http://127.0.0.1:${rpc_port}" | grep -q '"result"'; then
            return 0
        fi
        sleep 1
    done
    return 1
}

fetch_local_peer_id() {
    local rpc_port="$1"
    curl -s -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
        "http://127.0.0.1:${rpc_port}" | python -c 'import json,sys; print(json.load(sys.stdin).get("result", ""))' 2>/dev/null || true
}

stop_all() {
    log_info "Stopping all testnet validators..."
    for name in "${!VALIDATORS[@]}"; do
        local pid_file="${BASE_PATH}/${name}/${name}.pid"
        if [ -f "$pid_file" ]; then
            local pid
            pid=$(cat "$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid"
                log_info "Stopped $name (PID: $pid)"
            fi
            rm -f "$pid_file"
        fi
    done
}

status() {
    log_info "Testnet validator status:"
    for name in "${!VALIDATORS[@]}"; do
        local config="${VALIDATORS[$name]}"
        IFS=':' read -r seed p2p_port rpc_port <<< "$config"
        local pid_file="${BASE_PATH}/${name}/${name}.pid"

        if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
            echo -e "  ${GREEN}●${NC} $seed ($name) - Running (PID: $(cat "$pid_file"), RPC: $rpc_port)"
        else
            echo -e "  ${RED}●${NC} $seed ($name) - Stopped"
        fi
    done
}

# Main
case "${1:-all}" in
    validator1|validator2|validator3|validator4)
        check_binary
        launch_validator "$1"
        ;;
    all)
        check_binary
        log_info "Launching X3 Chain testnet with 4 validators..."

        # Seed keystores so each validator can author/finalize blocks.
        # Keys must match the genesis authorities in testnet_config() (chain_spec.rs).
        log_info "Seeding validator keystores..."
        insert_keys "validator1" "TestnetAlpha"
        insert_keys "validator2" "TestnetBeta"
        insert_keys "validator3" "TestnetGamma"
        insert_keys "validator4" "TestnetDelta"
        log_info "All validator keystores seeded."

        launch_validator "validator1"

        if ! wait_for_rpc 9944; then
            log_error "validator1 RPC did not become ready on port 9944"
            exit 1
        fi

        peer_id="$(fetch_local_peer_id 9944)"
        if [ -z "$peer_id" ]; then
            log_error "Failed to obtain validator1 peer ID from RPC"
            exit 1
        fi

        bootnode="/ip4/127.0.0.1/tcp/30333/p2p/${peer_id}"
        log_info "Using bootnode: $bootnode"

        for name in validator2 validator3 validator4; do
            launch_validator "$name" "$bootnode"
            sleep 1 # Stagger launches
        done

        log_info "All validators launched!"
        log_info "RPC endpoints: ws://localhost:9944 - ws://localhost:9947"
        ;;
    stop)
        stop_all
        ;;
    status)
        status
        ;;
    *)
        echo "Usage: $0 {all|validator1|validator2|validator3|validator4|stop|status}"
        exit 1
        ;;
esac
