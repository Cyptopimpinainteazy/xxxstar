#!/usr/bin/env bash
# public-node-id.sh — a bootnode identity you can publish before the node exists.
#
# A public testnet's first artifact is a bootnode address, and that address contains a
# peer id — which is derived from the node's libp2p identity key. So the key has to
# exist (and be backed up) *before* anything is launched, and the address has to be
# published in the chain spec, which is what every validator then dials.
#
#   ./scripts/testnet/public-node-id.sh --host bootnode.testnet.example --key-file deployment/keys/bootnode.nodekey
#
# Prints the peer id and the multiaddrs to publish. The key file is created 0600 if it
# does not exist; back it up, and never commit it (deployment/keys/.gitignore covers
# this path).
#
# Derivation is the same one the spec builder uses
# (`scripts/mainnet/peer-id-from-ed25519-pubkey.py`), so the peer id printed here is the
# peer id the node reports when started with `--node-key <this file>`.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOST=""
KEY_FILE="$ROOT_DIR/deployment/keys/bootnode.nodekey"
P2P_PORT="${P2P_PORT:-30333}"
IP=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) HOST="${2:-}"; shift 2 ;;
    --ip) IP="${2:-}"; shift 2 ;;
    --key-file) KEY_FILE="${2:-}"; shift 2 ;;
    --p2p-port) P2P_PORT="${2:-}"; shift 2 ;;
    -h|--help) sed -n '2,18p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$HOST" || -n "$IP" ]] || { echo "--host (DNS name) or --ip is required: the address has to be one validators can dial" >&2; exit 2; }

NODE_BIN="${NODE_BIN:-}"
if [[ -z "$NODE_BIN" ]]; then
  for candidate in \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/release/x3-chain-node" \
    "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/debug/x3-chain-node" \
    "$ROOT_DIR/target/release/x3-chain-node" \
    "$ROOT_DIR/target/debug/x3-chain-node"; do
    [[ -x "$candidate" ]] && NODE_BIN="$candidate" && break
  done
fi
[[ -n "$NODE_BIN" && -x "$NODE_BIN" ]] || { echo "node binary not found; build it with cargo build -p x3-chain-node" >&2; exit 1; }

mkdir -p "$(dirname "$KEY_FILE")"
created=0
if [[ ! -s "$KEY_FILE" ]]; then
  python3 -c "import secrets; print('0x' + secrets.token_hex(32))" > "$KEY_FILE"
  chmod 600 "$KEY_FILE"
  created=1
fi
secret="$(tr -d '[:space:]' < "$KEY_FILE")"
[[ "$secret" =~ ^0x[0-9a-f]{64}$ ]] || { echo "$KEY_FILE does not hold a 32-byte hex secret" >&2; exit 1; }

pub="$("$NODE_BIN" keys generate --key-type grandpa --seed "$secret" --output hex 2>/dev/null | tail -1)"
[[ "$pub" =~ ^0x[0-9a-f]{64}$ ]] || { echo "could not derive the identity public key (is $NODE_BIN the X3 node?)" >&2; exit 1; }
peer="$(python3 "$ROOT_DIR/scripts/mainnet/peer-id-from-ed25519-pubkey.py" "$pub")"

echo "[bootnode] key file: $KEY_FILE$([[ "$created" == "1" ]] && echo '  (generated just now — back it up, do not commit it)')"
echo "[bootnode] peer id:  $peer"
echo "[bootnode] publish one of:"
[[ -n "$HOST" ]] && echo "  /dns4/${HOST}/tcp/${P2P_PORT}/p2p/${peer}"
[[ -n "$IP" ]] && echo "  /ip4/${IP}/tcp/${P2P_PORT}/p2p/${peer}"
echo
echo "Start the bootnode from a spec whose bootNodes carry that address, with:"
echo "  x3-chain-node --chain <spec> --base-path <dir> --node-key \"\$(cat $KEY_FILE)\" --listen-addr <address> --validator"
echo "The node reports this peer id via system_localPeerId; if it does not match, the spec is dialling the wrong identity."
