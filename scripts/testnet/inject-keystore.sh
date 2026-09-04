#!/usr/bin/env bash
# inject-keystore.sh <base-path> <validator-index> [chain-id]
# Injects aura(sr25519)+grandpa(ed25519) session keys for fresh validator $index
# into <base-path>/chains/<chain-id>/keystore using the stored SURI.
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASE="${1:?base-path}"; IDX="${2:?validator-index}"
SPEC="$ROOT/deployment/chain-specs/fresh/x3-testnet-plain.json"
CHAIN_ID="${3:-$(python3 -c "import json;print(json.load(open('$SPEC'))['id'])")}"
SURI_FILE="$ROOT/deployment/chain-specs/fresh/validator-keys/validator-$IDX.suri"
kdir="$BASE/chains/$CHAIN_ID/keystore"; mkdir -p "$kdir"
AURA="$(grep '^aura=' "$SURI_FILE" | cut -d= -f2)"
GRAN="$(grep '^grandpa=' "$SURI_FILE" | cut -d= -f2)"
apub="$(subkey inspect --scheme sr25519 "$AURA" | awk '/Public key \(hex\):/{print $4}')"
gpub="$(subkey inspect --scheme ed25519 "$GRAN" | awk '/Public key \(hex\):/{print $4}')"
printf '%s' "$AURA" > "$kdir/61757261${apub#0x}"; chmod 600 "$kdir/61757261${apub#0x}"
printf '%s' "$GRAN" > "$kdir/6772616e${gpub#0x}"; chmod 600 "$kdir/6772616e${gpub#0x}"
echo "injected validator-$IDX session keys into $kdir"
echo "  aura:  ${apub:0:16}…  grandpa: ${gpub:0:16}…"
