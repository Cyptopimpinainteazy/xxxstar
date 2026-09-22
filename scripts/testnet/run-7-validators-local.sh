#!/usr/bin/env bash
set -euo pipefail

# Testnet-only local 7-validator launcher.
# Seeds are exposed here by design for local testing. Do NOT use these for mainnet.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN_DEFAULT="$ROOT_DIR/target/release/x3-chain-node"
CHAIN_SPEC_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-raw.json"
CHAIN_SPEC_PLAIN_DEFAULT="$ROOT_DIR/deployment/chain-specs/x3-testnet-plain.json"
BASE_DIR_DEFAULT="$HOME/.local/share/x3/testnet-local"
LOG_DIR_DEFAULT="$ROOT_DIR/logs/testnet"

NODE_BIN="${NODE_BIN:-$NODE_BIN_DEFAULT}"
CHAIN_SPEC="${CHAIN_SPEC:-$CHAIN_SPEC_DEFAULT}"
CHAIN_SPEC_PLAIN="${CHAIN_SPEC_PLAIN:-$CHAIN_SPEC_PLAIN_DEFAULT}"
BASE_DIR="${BASE_DIR:-$BASE_DIR_DEFAULT}"
LOG_DIR="${LOG_DIR:-$LOG_DIR_DEFAULT}"
PID_DIR="${PID_DIR:-}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-}"
KEYSTORE_PASSWORD_FILE="${KEYSTORE_PASSWORD_FILE:-}"
# `--only <n>` starts exactly one validator from an existing base dir and exits.
# That is how a node is brought back after a failure without touching its peers:
# the base path, keystore and node key are already there, and a Live spec carries
# the bootnodes, so it rejoins on its own.
ONLY_INDEX="${ONLY_INDEX:-0}"
# Per-validator seeds written by `scripts/testnet/build-x3-testnet-spec.py`. A spec
# built from fresh keys and nodes started from the built-in dev seeds is a network
# whose authorities hold none of its keys: it starts, and authors nothing. Prefer
# the seed files whenever they are there.
KEYS_DIR="${KEYS_DIR:-$ROOT_DIR/deployment/chain-specs/fresh/generated/validator-keys}"
# A node needs a libp2p identity. Without `--node-key` (or a pre-existing
# `network/secret_ed25519` under the base path) this node build exits with
# `NetworkKeyNotFound`, which is why the launcher could not start anything. One key
# per validator, stable across runs so a Live spec's bootnode entries stay valid
# (`build-x3-testnet-spec.py` derives the same file into the spec).
NODE_KEYS_DIR="${NODE_KEYS_DIR:-$BASE_DIR/node-keys}"
COUNT="${COUNT:-7}"
LISTEN_IP="${LISTEN_IP:-127.0.0.1}"
PROMETHEUS="${PROMETHEUS:-0}"
NO_MDNS="${NO_MDNS:-1}"
NO_TELEMETRY="${NO_TELEMETRY:-1}"
DISABLE_LOG_COLOR="${DISABLE_LOG_COLOR:-1}"
NODE_NICE="${NODE_NICE:-}"
NODE_DB_CACHE_MIB="${NODE_DB_CACHE_MIB:-}"

WIPE_BASE_DIR=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--wipe] [--base-dir PATH] [--chain-spec PATH] [--node-bin PATH] [--log-dir PATH] [--keys-dir PATH]

Testnet-only local 7-validator launcher.

Key material: if ${KEYS_DIR} holds `validator-<n>.suri` files (written by
`scripts/testnet/build-x3-testnet-spec.py`), those seeds are used and each is
checked against the spec's authority sets. Otherwise the built-in dev seeds are
used, which only matches a spec built from them.

Options:
  --wipe              Stop existing nodes (via PID files) and wipe base dir before starting.
  --base-dir PATH     Override BASE_DIR (default: ${BASE_DIR_DEFAULT})
  --chain-spec PATH   Override CHAIN_SPEC (default: ${CHAIN_SPEC_DEFAULT})
  --node-bin PATH     Override NODE_BIN (default: ${NODE_BIN_DEFAULT})
  --log-dir PATH      Override LOG_DIR (default: ${LOG_DIR_DEFAULT})
  --keys-dir PATH     Override KEYS_DIR (default: ${KEYS_DIR})
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe)
      WIPE_BASE_DIR=1
      shift
      ;;
    --base-dir)
      BASE_DIR="${2:-}"
      shift 2
      ;;
    --chain-spec)
      CHAIN_SPEC="${2:-}"
      shift 2
      ;;
    --node-bin)
      NODE_BIN="${2:-}"
      shift 2
      ;;
    --log-dir)
      LOG_DIR="${2:-}"
      shift 2
      ;;
    --keys-dir)
      KEYS_DIR="${2:-}"
      shift 2
      ;;
    --only)
      ONLY_INDEX="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 2
      ;;
  esac
done

PID_DIR="${PID_DIR:-$BASE_DIR/pids}"
CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN:-$BASE_DIR/chain-spec.json}"

stop_nodes() {
  if [[ ! -d "$PID_DIR" ]]; then
    return 0
  fi

  shopt -s nullglob
  local pids=("$PID_DIR"/node-*.pid)
  shopt -u nullglob
  if [[ ${#pids[@]} -eq 0 ]]; then
    return 0
  fi

  for pid_file in "${pids[@]}"; do
    local pid
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
    fi
  done

  # A pid file can be stale (a `--only` restart rewrites one), so the pid-file kill
  # above can miss the process actually holding the ports. Sweep by base path too.
  pkill -f -- "--base-path ${BASE_DIR}/node-" 2>/dev/null || true

  sleep 1
}

wipe_base_dir() {
  local dir="$1"
  if [[ -z "$dir" || "$dir" == "/" ]]; then
    echo "Refusing to wipe BASE_DIR='$dir'"
    exit 1
  fi
  rm -rf "$dir"
}

if [[ "$WIPE_BASE_DIR" -eq 1 ]]; then
  stop_nodes
  wipe_base_dir "$BASE_DIR"
fi

mkdir -p "$BASE_DIR" "$LOG_DIR" "$PID_DIR"

if [[ ! -x "$NODE_BIN" ]]; then
  echo "Node binary not found: $NODE_BIN"
  exit 1
fi

# Key insertion uses the node binary itself (`keys insert` / `keys list`, which
# open a `LocalKeystore` and write the same `<key-type-hex><public-key-hex>` files
# this launcher used to hand-write after asking `subkey` for the public key). That
# removes a hard dependency on a tool that is not installed on the build boxes and
# is not part of this repository — the launcher could not run at all without it.

if [[ ! -f "$CHAIN_SPEC" ]]; then
  echo "Chain spec not found: $CHAIN_SPEC"
  exit 1
fi

ensure_raw_spec() {
  if [[ -s "$CHAIN_SPEC" ]]; then
    return 0
  fi

  if [[ ! -x "$NODE_BIN" ]]; then
    echo "Node binary not found for build-spec: $NODE_BIN"
    exit 1
  fi

  if [[ ! -f "$CHAIN_SPEC_PLAIN" ]]; then
    echo "Plain chain spec not found: $CHAIN_SPEC_PLAIN"
    exit 1
  fi

  echo "Raw chain spec is empty. Regenerating from $CHAIN_SPEC_PLAIN..."
  tmp_spec="${CHAIN_SPEC}.tmp"
  "$NODE_BIN" build-spec --chain "$CHAIN_SPEC_PLAIN" --raw --disable-log-color > "$tmp_spec" 2>/dev/null
  TMP_SPEC="$tmp_spec" CHAIN_SPEC="$CHAIN_SPEC" python3 - <<'PY'
import json
import os
from pathlib import Path

src = Path(os.environ["TMP_SPEC"])
dst = Path(os.environ["CHAIN_SPEC"])
text = src.read_text()
start = text.find("{")
end = text.rfind("}")
if start == -1 or end == -1 or end <= start:
    raise SystemExit("Failed to locate JSON object in build-spec output")
json_text = text[start:end+1]
json.loads(json_text)
dst.write_text(json_text)
print(f"Regenerated raw spec: {dst}")
PY
  rm -f "$tmp_spec"
}

ensure_raw_spec

# Sanitize chain spec to avoid conflicting bootnodes embedded in *raw* specs.
#
# A raw spec carries whatever bootnode list it was generated with, which is why
# this used to empty `bootNodes` unconditionally and let the launcher pass
# `--bootnodes` on the command line instead. That cannot work for a plain Live spec:
# the node refuses to start a Live chain whose spec has no bootnode
# (`Error: Input("Live chain spec requires at least one bootnode")`), and a spec
# built by `build-x3-testnet-spec.py` lists exactly the peer ids this launcher is
# about to start (the preflight below requires it). So strip only the raw form.
CHAIN_SPEC="${CHAIN_SPEC}" CHAIN_SPEC_RUN="${CHAIN_SPEC_RUN}" python3 - <<'PY'
import os
import json
from pathlib import Path

src = Path(os.environ["CHAIN_SPEC"])
dst = Path(os.environ["CHAIN_SPEC_RUN"])

text = src.read_text()
start = text.find("{")
if start == -1:
    raise SystemExit(f"Invalid chain spec (no JSON object found): {src}")
spec = json.loads(text[start:])

if "bootNodes" not in spec:
    raise SystemExit("bootNodes key not found in chain spec")

is_raw = isinstance(spec.get("genesis", {}).get("raw"), dict)
if is_raw:
    stripped = len(spec.get("bootNodes") or [])
    spec["bootNodes"] = []
    print(f"Using sanitized chain spec: {dst} (raw form; dropped {stripped} embedded bootnode(s))")
else:
    print(f"Using chain spec: {dst} (plain form; keeping {len(spec.get('bootNodes') or [])} bootnode(s))")

dst.write_text(json.dumps(spec, indent=2))
PY

DEV_SEEDS=(
  "//Alice"
  "//Bob"
  "//Charlie"
  "//Dave"
  "//Eve"
  "//Ferdie"
  "//One"
)

# Which keys the nodes are started with. `SEEDS` is what `start_node` uses; the
# built-in dev seeds are only correct for a spec whose authorities were built from
# them. `build-x3-testnet-spec.py` writes one `validator-<n>.suri` per validator
# and a spec whose authorities come from those, so read them when present.
SEEDS=("${DEV_SEEDS[@]}")
SEEDS_SOURCE="built-in dev seeds (${DEV_SEEDS[0]} …)"
if [[ -f "${KEYS_DIR}/validator-1.suri" ]]; then
  loaded_seeds=()
  for i in $(seq 1 "$COUNT"); do
    seed_file="${KEYS_DIR}/validator-${i}.suri"
    if [[ ! -s "$seed_file" ]]; then
      echo "Missing or empty ${seed_file}; run scripts/testnet/build-x3-testnet-spec.py ${COUNT} first, or pass --keys-dir"
      exit 1
    fi
    # `build-x3-testnet-spec.py` writes `seed=`/`aura=`/`grandpa=` lines (hex key
    # material, not a SURI). Take the `seed=` value; a file that is a single SURI
    # line is still accepted, so either format works.
    seed_value="$(grep -m1 '^seed=' "$seed_file" | cut -d= -f2- || true)"
    if [[ -z "$seed_value" ]]; then
      seed_value="$(head -1 "$seed_file")"
    fi
    loaded_seeds+=("$seed_value")
  done
  SEEDS=("${loaded_seeds[@]}")
  SEEDS_SOURCE="${KEYS_DIR}/validator-*.suri"
fi

CHAIN_ID="$(CHAIN_SPEC_RUN="$CHAIN_SPEC_RUN" python3 - <<'PY'
import json
import os
from pathlib import Path

spec = json.loads(Path(os.environ["CHAIN_SPEC_RUN"]).read_text())
print(spec.get("id", ""))
PY
)"

if [[ -z "$CHAIN_ID" ]]; then
  echo "Failed to read chain id from ${CHAIN_SPEC_RUN}"
  exit 1
fi

# bootability + authority-consistency preflight (rollback-safe; override with
# ALLOW_RAW_LIVE=1 and/or SKIP_SPEC_AUTHORITY_CHECK=1).
if [[ "${ALLOW_RAW_LIVE:-0}" != "1" ]]; then
  # Derive each seed's public keys with the node itself, so the preflight can check
  # the keys the launch will use against the authorities the spec actually names.
  # A spec built from fresh keys plus nodes started from dev seeds launches and
  # authors nothing; that has to fail here, not look like a slow network.
  LAUNCHER_AURA=""
  LAUNCHER_GRANDPA=""
  LAUNCHER_PEERS=""
  for seed in "${SEEDS[@]}"; do
    derived_aura="$("$NODE_BIN" keys generate --key-type aura --seed "$seed" --output ss58 2>/dev/null | tail -1)"
    derived_grandpa="$("$NODE_BIN" keys generate --key-type grandpa --seed "$seed" --output ss58 2>/dev/null | tail -1)"
    if [[ -z "$derived_aura" || -z "$derived_grandpa" ]]; then
      echo "Could not derive Aura/GRANDPA keys from a validator seed with ${NODE_BIN}"
      exit 1
    fi
    LAUNCHER_AURA+="${derived_aura}"$'\n'
    LAUNCHER_GRANDPA+="${derived_grandpa}"$'\n'
  done

  # Peer ids for the node keys this run will start with, derived the same way the
  # spec builder derived the bootNodes entries.
  for i in $(seq 1 "$COUNT"); do
    node_key_file="${KEYS_DIR}/validator-${i}.nodekey"
    if [[ ! -s "$node_key_file" ]]; then
      node_key_file="${NODE_KEYS_DIR}/node-${i}.key"
    fi
    if [[ -s "$node_key_file" ]]; then
      node_key_hex="$(tr -d '[:space:]' < "$node_key_file")"
      node_pub="$("$NODE_BIN" keys generate --key-type grandpa --seed "$node_key_hex" --output hex 2>/dev/null | tail -1)"
      if [[ -n "$node_pub" ]]; then
        LAUNCHER_PEERS+="$(python3 "$ROOT_DIR/scripts/mainnet/peer-id-from-ed25519-pubkey.py" "$node_pub")"$'\n'
      fi
    fi
  done

  CHECK="$CHAIN_SPEC_RUN" EXPECTED_AUTHORITIES="${#SEEDS[@]}" \
    LAUNCHER_AURA="$LAUNCHER_AURA" LAUNCHER_GRANDPA="$LAUNCHER_GRANDPA" \
    LAUNCHER_PEERS="$LAUNCHER_PEERS" \
    SEEDS_SOURCE="$SEEDS_SOURCE" python3 - <<'PY'
import json, os, sys
from pathlib import Path
p = Path(os.environ["CHECK"])
try:
    spec = json.loads(p.read_text())
except Exception as e:
    sys.exit(f"[validate] cannot parse chain spec {p}: {e}")
if str(spec.get("chainType", "")).lower() == "live":
    g = spec.get("genesis", {})
    cfg = g.get("runtimeGenesis", {}).get("config", {}) if "runtimeGenesis" in g else None
    if cfg is None:
        # storage-raw Live spec: this node's load_json_spec() rejects storage-raw
        # Live specs (no aura/grandpa/council/treasury config arrays), so it cannot
        # boot on a raw-Live file no matter the authority count.
        print("[validate] spec is a storage-*raw* Live spec -> node boot will be rejected.")
        print("  Boot a bootable *plain* Live spec whose Aura+Grandpa authorities == the "
              "launcher's 7 dev seeds (see start_node comment). To force the raw path use "
              "ALLOW_RAW_LIVE=1 (node error then surfaces directly).")
        sys.exit(2)
    dev = int(os.environ.get("EXPECTED_AUTHORITIES", "7"))
    na = len(cfg.get("aura", {}).get("authorities", []))
    ng = len(cfg.get("grandpa", {}).get("authorities", []))
    if (na != dev or ng != dev) and os.environ.get("SKIP_SPEC_AUTHORITY_CHECK") != "1":
        print(f"[validate] FAIL: spec Aura authorities={na}, Grandpa authorities={ng} "
              f"but the launcher will start {dev} validator(s). They must match to "
              f"author+finalize.")
        sys.exit(3)

    launcher_aura = [x for x in os.environ.get("LAUNCHER_AURA", "").splitlines() if x]
    launcher_grandpa = [x for x in os.environ.get("LAUNCHER_GRANDPA", "").splitlines() if x]
    spec_aura = set(cfg.get("aura", {}).get("authorities", []))
    spec_grandpa = {
        (e[0] if isinstance(e, list) else e)
        for e in cfg.get("grandpa", {}).get("authorities", [])
    }
    missing_aura = [a for a in launcher_aura if a not in spec_aura]
    missing_grandpa = [g for g in launcher_grandpa if g not in spec_grandpa]
    if (missing_aura or missing_grandpa) and os.environ.get("SKIP_SPEC_AUTHORITY_CHECK") != "1":
        print(f"[validate] FAIL: {len(missing_aura)} launcher Aura key(s) and "
              f"{len(missing_grandpa)} GRANDPA key(s) are not authorities in this spec, "
              f"e.g. {missing_aura[:1] + missing_grandpa[:1]}")
        print("  The nodes would start and author nothing. Start from the seeds the "
              "spec was built from (--keys-dir) or rebuild the spec from these seeds.")
        sys.exit(4)
    print(f"[validate] ok: plain Live spec Aura={na} Grandpa={ng}; every launcher key "
          f"(Aura {len(launcher_aura)}, GRANDPA {len(launcher_grandpa)}) is in the "
          f"authority sets (seeds from {os.environ.get('SEEDS_SOURCE', '?')}).")

    # A Live spec also has to name the peer ids the nodes will actually have, or the
    # network starts and never connects. `--node-key` for each validator is derived
    # from `validator-<n>.nodekey` by the launcher; the spec's bootNodes were built
    # from the same files.
    node_keys = [x for x in os.environ.get("LAUNCHER_PEERS", "").splitlines() if x]
    spec_boot = spec.get("bootNodes") or []
    listed = {b.rsplit("/p2p/", 1)[-1] for b in spec_boot}
    unknown = [p for p in node_keys if p not in listed]
    if (not listed or unknown) and os.environ.get("SKIP_SPEC_AUTHORITY_CHECK") != "1":
        print(f"[validate] FAIL: this spec lists {len(listed)} bootnode peer id(s) and "
              f"{len(unknown)} of the {len(node_keys)} nodes this launcher will start "
              f"are not among them, e.g. {unknown[:1]}")
        print("  The nodes would start and never find each other. Rebuild the spec "
              "with scripts/testnet/build-x3-testnet-spec.py, which writes the node "
              "keys beside the seeds and derives bootNodes from them.")
        sys.exit(5)
    print(f"[validate] ok: all {len(node_keys)} launcher peer id(s) are in the "
          f"spec's {len(listed)} bootnode entry(ies).")
sys.exit(0)
PY
  rc=$?
  if [[ "$rc" -ne 0 ]]; then
    echo "run-7-validators-local.sh preflight failed (rc=$rc). Pass --chain-spec <equal-authority plain Live spec> or set ALLOW_RAW_LIVE=1 to force."
    exit 1
  fi
fi

if [[ "$COUNT" -lt 1 || "$COUNT" -gt 7 ]]; then
  echo "COUNT must be between 1 and 7 (got: ${COUNT})"
  exit 1
fi

if [[ "$COUNT" -lt 5 ]]; then
  echo "WARNING: COUNT<5 means GRANDPA finality will stall (7 authorities in genesis)."
  echo "         Anything waiting for finalized blocks (e.g. submit-remark.js) may hang."
fi

insert_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"

  mkdir -p "$keystore_dir"

  local key_type
  for key_type in aura grandpa; do
    if ! "$NODE_BIN" keys insert \
      --key-type "$key_type" \
      --seed "$suri" \
      --keystore-path "$keystore_dir" >/dev/null; then
      echo "Failed to insert the ${key_type} key for ${suri} into ${keystore_dir}"
      exit 1
    fi
  done
}

validate_keys() {
  local base_path="$1"
  local suri="$2"
  local keystore_dir="${base_path}/chains/${CHAIN_ID}/keystore"

  local listed
  listed="$("$NODE_BIN" keys list --keystore-path "$keystore_dir" 2>/dev/null || true)"

  local key_type
  for key_type in aura grandpa; do
    if ! grep -q "^${key_type}: " <<<"$listed"; then
      echo "Keystore ${keystore_dir} has no ${key_type} key for ${suri} after insert"
      exit 1
    fi
  done
}

wait_for_rpc() {
  local rpc_port="$1"
  # 180s, not 60: a cold debug-build node reads a 17 MB spec and a fresh keystore
  # before it answers RPC, and back-to-back launches (a drill followed by another
  # drill, say) have taken longer than a minute. The wait is still bounded, and the
  # message below says which port never answered.
  for _ in $(seq 1 180); do
    if curl -s -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
      "http://127.0.0.1:${rpc_port}" | grep -q '"isSyncing"'; then
      return 0
    fi
    sleep 1
  done
  echo "RPC not ready on port ${rpc_port}"
  return 1
}

start_node() {
  local i="$1"
  local bootnode="${2:-}"

  local p2p_port=$((30333 + i - 1))
  local rpc_port=$((9944 + i - 1))
  local prom_port=$((9615 + i - 1))
  local base_path="${BASE_DIR}/node-${i}"
  local name="x3-testnet-node-$(printf '%02d' "$i")"
  local dev_seed="${SEEDS[$((i-1))]}"
  local log_file="${LOG_DIR}/node-${i}.log"

  mkdir -p "$base_path"

  local boot_args=()
  if [[ -n "$bootnode" ]]; then
    boot_args=(--bootnodes "$bootnode")
  fi

  insert_keys "$base_path" "$dev_seed"
  validate_keys "$base_path" "$dev_seed"

  # AUTHORING-DRIVER NOTE (verified 2026-09-04 on this fork):
  #   run-7's authority set IS consistent with the genesis it boots -- the checked-in
  #   raw spec installs the SAME 7 dev seeds (Alice..One) as Aura (sr25519) AND Grandpa
  #   (ed25519) authorities (see deployment/chain-specs/x3-testnet-raw.json, decoded
  #   storage: Aura Authorities=7, `:grandpa_authorities`=7). So DEV_SEEDS/COUNT=7 need
  #   no change (there is NO 5-vs-7 mismatch as previously assumed).
  #   The actual blockers for a REAL finalized run are:
  #   (1) node/src/service.rs only drives Aura authoring + GRANDPA finality by inserting
  #       keys from X3_DEV_SEED (maybe_insert_dev_keys), NOT from keystore files alone.
  #   (2) the node's Live key loader rejects storage-*raw* Live specs (no aura/grandpa
  #       config arrays), so launch each node from a bootable *plain* Live spec whose
  #       authority set matches these same dev seeds, and export X3_DEV_SEED below.
  local env_args=()
  if [[ -n "$dev_seed" ]]; then
    env_args+=(env "X3_DEV_SEED=$dev_seed")
  fi

  local password_args=()
  if [[ -n "$KEYSTORE_PASSWORD_FILE" ]]; then
    password_args=(--password-filename "$KEYSTORE_PASSWORD_FILE")
  fi

  local log_args=()
  if [[ "$DISABLE_LOG_COLOR" == "1" ]]; then
    log_args+=(--disable-log-color)
  fi

  local net_args=(
    --listen-addr "/ip4/${LISTEN_IP}/tcp/${p2p_port}"
  )
  if [[ "$NO_MDNS" == "1" ]]; then
    net_args+=(--no-mdns)
  fi
  if [[ "$NO_TELEMETRY" == "1" ]]; then
    net_args+=(--no-telemetry)
  fi

  local prom_args=()
  if [[ "$PROMETHEUS" == "1" ]]; then
    prom_args+=(--prometheus-port "$prom_port")
  else
    prom_args+=(--no-prometheus)
  fi

  local db_args=()
  if [[ -n "$NODE_DB_CACHE_MIB" ]]; then
    db_args+=(--db-cache "$NODE_DB_CACHE_MIB")
  fi

  local nice_args=()
  if [[ -n "$NODE_NICE" ]]; then
    nice_args=(nice -n "$NODE_NICE")
  fi

  # libp2p identity for this node: `--node-key` takes 32 bytes of hex. The key is
  # generated once per base dir and reused, so the peer id a Live spec lists for
  # this validator keeps pointing at it across restarts.
  # Prefer the key the spec's bootNodes entry was derived from
  # (`build-x3-testnet-spec.py` writes it beside the seed); otherwise use the
  # per-base-dir key.
  local node_key_file="${KEYS_DIR}/validator-${i}.nodekey"
  if [[ ! -s "$node_key_file" ]]; then
    node_key_file="${NODE_KEYS_DIR}/node-${i}.key"
    mkdir -p "$NODE_KEYS_DIR"
    if [[ ! -s "$node_key_file" ]]; then
      python3 -c "import secrets; print('0x' + secrets.token_hex(32))" > "$node_key_file"
      chmod 600 "$node_key_file"
    fi
  fi
  local node_key
  node_key="$(tr -d '[:space:]' < "$node_key_file")"

  # export X3_DEV_SEED so service.maybe_insert_dev_keys() inserts Aura(sr25519) +
  # GRANDPA(ed25519) from <<dev_seed>> (the fork's authoring driver, see comment above).
  nohup "${env_args[@]}" "${nice_args[@]}" "$NODE_BIN" \
    --chain "$CHAIN_SPEC_RUN" \
    --base-path "$base_path" \
    --name "$name" \
    --rpc-port "$rpc_port" \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    "${log_args[@]}" \
    "${net_args[@]}" \
    "${prom_args[@]}" \
    "${db_args[@]}" \
    "${password_args[@]}" \
    --validator \
    --force-authoring \
    --allow-private-ip \
    --node-key "$node_key" \
    "${boot_args[@]}" \
    > "$log_file" 2>&1 &

  echo $! > "${PID_DIR}/node-${i}.pid"
  echo "Started ${name} (p2p=${p2p_port}, rpc=${rpc_port}, prom=${prom_port})"

  wait_for_rpc "$rpc_port"
  echo "Node ${name} ready"
}

echo "Starting node 1 (bootnode)..."
start_node 1

peer_id=""
for _ in $(seq 1 60); do
  peer_id="$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' \
    "http://127.0.0.1:9944" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  if [[ -n "$peer_id" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "$peer_id" ]]; then
  echo "Failed to detect node-1 peer ID via RPC (system_localPeerId)"
  exit 1
fi

BOOTNODE="/ip4/${LISTEN_IP}/tcp/30333/p2p/${peer_id}"
echo "Bootnode: ${BOOTNODE}"

if [[ "$ONLY_INDEX" != "0" ]]; then
  if ! [[ "$ONLY_INDEX" =~ ^[0-9]+$ ]] || [[ "$ONLY_INDEX" -lt 1 ]] || [[ "$ONLY_INDEX" -gt "$COUNT" ]]; then
    echo "--only takes a validator index between 1 and ${COUNT} (got: ${ONLY_INDEX})"
    exit 2
  fi
  echo "Starting only node ${ONLY_INDEX} (restart path; peers keep running)"
  start_node "$ONLY_INDEX" "$BOOTNODE"
  echo "Node ${ONLY_INDEX} is back."
  exit 0
fi

for i in $(seq 2 "$COUNT"); do
  echo "Starting node ${i}..."
  start_node "$i" "$BOOTNODE"
done

echo "All ${COUNT} validators started."
echo "Logs: ${LOG_DIR}/node-*.log"
echo "PIDs: ${PID_DIR}/node-*.pid"
