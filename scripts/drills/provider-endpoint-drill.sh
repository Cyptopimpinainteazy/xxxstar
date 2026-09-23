#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# provider-endpoint-drill.sh — check that the operator's paid RPC endpoints are
# live, are on the chain they claim, and agree with an independent provider.
#
# The provider keys became environment variables on 2026-09-23: an Alchemy key,
# a paid DRPC key, an Ankr key and a wallet private key had been committed to
# this repository, and removing them from `HEAD` does not un-expose them
# (TICKET-102). What replaces them is `ProviderCredentials::from_env`, which
# turns `DRPC_API_KEY` into `https://lb.drpc.org/<network>/<key>`.
#
# Nothing in the repository can tell a rotated-in key from a revoked one — that
# needs egress and the key — so this drill asks each endpoint a question only a
# real node on the right chain can answer, and writes down what it said.
#
# Per network it requires:
#   1. the paid endpoint's `eth_chainId` to equal the chain this repository
#      believes that network is — chain identity, not a hostname;
#   2. the paid endpoint and an independent keyless public endpoint to return
#      the same block hash at the same height, 64 blocks behind the head: an
#      endpoint that invents a block cannot agree with another, and that
#      agreement is what the RPC-quorum path assumes;
#   3. a receipt the paid endpoint reports successful (`status == 0x1`) and that
#      names the agreed block, with the public endpoint reporting the same — the
#      receipt is the evidence, submission is not, the rule the EVM adapters
#      follow.
#
# The key is never printed and never written: endpoints are shown with the key
# redacted, and the evidence file contains no key.
#
# Usage:
#   ./scripts/drills/provider-endpoint-drill.sh
#   ./scripts/drills/provider-endpoint-drill.sh --key-file ~/.x3-provider-keys
#   ./scripts/drills/provider-endpoint-drill.sh --networks arbitrum,base
#
# Options:
#   --key-file <path>          read <KEY_VAR> from this file (default
#                              ~/.x3-provider-keys, tried only if the variable
#                              is not already set)
#   --key-var <NAME>           environment variable holding the key (default
#                              DRPC_API_KEY)
#   --paid-url-template <t>    endpoint built per network; `{network}` and `{key}`
#                              are substituted. Default
#                              `https://lb.drpc.org/{network}/{key}`. A template
#                              with no `{key}` needs no key — that is how the
#                              drill itself is tested against two public
#                              providers, where nothing can be leaked.
#   --networks <a,b>           restrict to these slugs
#   --out <path>               evidence file (default
#                              .ai/reports/provider-endpoint-<utc>.json)
#
# Exit: 0 every check passed · 1 a check failed · 2 no key to test with.
# Requires: curl, jq.
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# slug, expected chain id, and a keyless public endpoint on the same chain. The
# public endpoint is the second opinion — it is never allowed to be the only one.
NETWORKS=(
  "ethereum 1 https://eth.llamarpc.com"
  "arbitrum 42161 https://arb1.arbitrum.io/rpc"
  "base 8453 https://mainnet.base.org"
  "optimism 10 https://mainnet.optimism.io"
  "polygon 137 https://polygon-rpc.com"
  "avalanche 43114 https://api.avax.network/ext/bc/C/rpc"
  "bsc 56 https://bsc-dataseed.binance.org"
)

# How far behind the head the agreed block is taken: deep enough that both
# providers have it final, so neither can still be reorganising it.
CONFIRMATIONS=64
# How many consecutive blocks to look back for one that carries a transaction.
MAX_LOOKBACK=16
# How many transactions in that block to try for one the paid endpoint reports
# successful — a block can legitimately contain a reverted transaction.
MAX_TX_TRIES=8

KEY_VAR="DRPC_API_KEY"
PAID_TEMPLATE="https://lb.drpc.org/{network}/{key}"
KEY=""
KEY_SOURCE=""
KEY_FILE=""
OUT=""
WANTED=""

usage() { awk 'NR>1 && /^#/ { sub(/^# ?/, ""); print; next } NR>1 { exit }' "${BASH_SOURCE[0]}"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --key-file) KEY_FILE="${2:-}"; shift 2 ;;
    --key-var) KEY_VAR="${2:-}"; shift 2 ;;
    --paid-url-template) PAID_TEMPLATE="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    --networks) WANTED="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

# ── the key ──────────────────────────────────────────────────────────────────
# Environment first, then the file the operator keeps it in. The file exists so
# the key never has to be pasted into a shell whose history records it.
need_key=0
case "$PAID_TEMPLATE" in *'{key}'*) need_key=1 ;; esac

if [ "$need_key" -eq 1 ]; then
  if [ -n "${!KEY_VAR:-}" ]; then
    KEY="${!KEY_VAR}"
    KEY_SOURCE="environment (\$${KEY_VAR})"
  else
    [ -n "$KEY_FILE" ] || KEY_FILE="$HOME/.x3-provider-keys"
    if [ -f "$KEY_FILE" ]; then
      KEY="$(sed -n "s/^[[:space:]]*\\(export[[:space:]]\\+\\)\\?${KEY_VAR}[[:space:]]*=[[:space:]]*//p" "$KEY_FILE" \
        | tail -n 1 \
        | sed -e 's/^"//' -e 's/"$//' -e "s/^'//" -e "s/'$//")"
      [ -n "$KEY" ] && KEY_SOURCE="$KEY_FILE"
    fi
  fi

  if [ -z "$KEY" ]; then
    cat >&2 <<EOF
[provider-endpoint] no ${KEY_VAR} configured — nothing to test.

Set it for one command:
    ${KEY_VAR}=... ./scripts/drills/provider-endpoint-drill.sh

or keep it in a file only the operator can read, and this drill finds it:
    umask 077 && printf '${KEY_VAR}=%s\\n' '<key>' > ~/.x3-provider-keys

The key is read, never printed, and never written to the evidence file.
EOF
    exit 2
  fi
else
  KEY_SOURCE="not needed (the endpoint template has no {key})"
fi

redact() { printf '%s' "${1//"$KEY"/<key>}"; }

paid_url_for() { # paid_url_for <slug> — the paid endpoint for one network
  local url="${PAID_TEMPLATE//\{network\}/$1}"
  printf '%s' "${url//\{key\}/$KEY}"
}

rpc() { # rpc <url> <method> [params-json]
  curl -sS --max-time 25 -H 'Content-Type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$2\",\"params\":${3:-[]}}" \
    "$1" 2>/dev/null || true
}

# Empty output means "the endpoint said nothing usable": a missing field, a
# JSON-RPC error, no response at all, or the literal null are all the same
# answer here, and all of them fail the check that asked rather than passing it.
field() { # field <json> <jq filter>
  printf '%s' "${1:-}" | jq -er "$2" 2>/dev/null | grep -vx 'null' || true
}

dec() { # dec <hex-or-decimal> — decimal, or empty when it is neither
  case "${1:-}" in
    ''|null) printf '' ;;
    0x[0-9a-fA-F]*) printf '%d' "$((16#${1#0x}))" ;;
    [0-9]*) printf '%d' "$1" ;;
    *) printf '' ;;
  esac
}

RESULTS="[]"
FAILURES=0

record() { # record <json-object>
  RESULTS="$(printf '%s' "$RESULTS" | jq -c --argjson o "$1" '. + [$o]')"
}

check_network() { # check_network <slug> <expected-chain-id> <public-url>
  local slug="$1" expected="$2" public_url="$3"
  local paid_url; paid_url="$(paid_url_for "$slug")"
  local errors=() height="" block="" tx="" receipt="" status="" blockhash=""
  local paid_hash="" public_hash="" paid_head="" head_dec=""
  local paid_id="" public_id="" paid_reason="" public_reason="" raw=""

  printf '── %s (chain %s)\n' "$slug" "$expected"
  printf '   paid   : %s\n' "$(redact "$paid_url")"
  printf '   public : %s\n' "$public_url"

  # 1. Chain identity, from both endpoints. A key a provider revoked, a host
  #    that is not the chain it claims, and an endpoint that answers with an
  #    error all land here. The provider's own words are carried into the report
  #    because "no usable chain id" and "personal token required" are different
  #    problems with different fixes.
  raw="$(rpc "$paid_url" eth_chainId)"
  paid_id="$(field "$raw" '.result')"
  paid_reason="$(field "$raw" '.error.message')"
  raw="$(rpc "$public_url" eth_chainId)"
  public_id="$(field "$raw" '.result')"
  public_reason="$(field "$raw" '.error.message')"
  if [ -z "$(dec "$paid_id")" ]; then
    errors+=("the paid endpoint answered no usable chain id${paid_reason:+ — ${paid_reason}}")
  elif [ "$(dec "$paid_id")" != "$expected" ]; then
    errors+=("the paid endpoint is chain $(dec "$paid_id"), expected ${expected}")
  fi
  if [ -z "$(dec "$public_id")" ]; then
    errors+=("the public endpoint answered no usable chain id${public_reason:+ — ${public_reason}}")
  elif [ "$(dec "$public_id")" != "$expected" ]; then
    errors+=("the public endpoint is chain $(dec "$public_id"), expected ${expected}")
  fi

  # 2. The same block, from two providers.
  paid_head="$(field "$(rpc "$paid_url" eth_blockNumber)" '.result')"
  head_dec="$(dec "$paid_head")"
  if [ -z "$head_dec" ]; then
    errors+=("the paid endpoint answered no usable block number (${paid_head:-no response})")
  else
    height=$((head_dec - CONFIRMATIONS))
    local tries=0 hhex
    while [ "$tries" -lt "$MAX_LOOKBACK" ]; do
      hhex="$(printf '0x%x' "$height")"
      block="$(field "$(rpc "$paid_url" eth_getBlockByNumber "[\"$hhex\",false]")" '.result')"
      tx="$(field "$block" '.transactions[0]')"
      [ -n "$tx" ] && break
      tries=$((tries + 1))
      height=$((height - 1))
    done

    if [ -z "$tx" ]; then
      errors+=("no block carrying a transaction within ${MAX_LOOKBACK} of the head")
      height=""
    else
      hhex="$(printf '0x%x' "$height")"
      paid_hash="$(field "$block" '.hash')"
      public_hash="$(field "$(rpc "$public_url" eth_getBlockByNumber "[\"$hhex\",false]")" '.result.hash')"
      if [ -z "$public_hash" ]; then
        errors+=("the public endpoint has no block at height ${height}")
      elif [ "$paid_hash" != "$public_hash" ]; then
        errors+=("block ${height} differs between providers: paid ${paid_hash} vs public ${public_hash}")
      fi

      # 3. A receipt the paid endpoint calls successful, confirmed by the public
      #    one. Any transaction in the block will do — what is being checked is
      #    that `status` and the block a receipt names come from a chain two
      #    providers describe the same way, not one transaction's outcome.
      local txs n i matched last_reason=""
      txs="$(field "$block" '.transactions')"
      n="$(field "$txs" 'length')"
      [ -n "$n" ] || n=0
      i=0; matched=0
      while [ "$i" -lt "$n" ] && [ "$i" -lt "$MAX_TX_TRIES" ]; do
        local candidate; candidate="$(field "$txs" ".[$i]")"
        raw="$(rpc "$paid_url" eth_getTransactionReceipt "[\"$candidate\"]")"
        receipt="$(field "$raw" '.result')"
        status="$(field "$receipt" '.status')"
        blockhash="$(field "$receipt" '.blockHash')"
        last_reason="$(field "$raw" '.error.message')"
        if [ "$status" = "0x1" ] && [ "$blockhash" = "$paid_hash" ]; then
          tx="$candidate"; matched=1; break
        fi
        i=$((i + 1))
      done

      if [ "$matched" -eq 0 ]; then
        errors+=("the paid endpoint reports no successful receipt in block ${height}${last_reason:+ — ${last_reason}}")
      else
        local public_receipt public_status public_block
        public_receipt="$(field "$(rpc "$public_url" eth_getTransactionReceipt "[\"$tx\"]")" '.result')"
        public_status="$(field "$public_receipt" '.status')"
        public_block="$(field "$public_receipt" '.blockHash')"
        if [ -z "$public_receipt" ]; then
          errors+=("the public endpoint has no receipt for ${tx} — nothing confirms the block holds it")
        elif [ "$public_status" != "0x1" ]; then
          errors+=("the public endpoint reports ${tx} status ${public_status:-none}, not 0x1")
        elif [ "$public_block" != "$paid_hash" ]; then
          errors+=("the public endpoint places ${tx} in block ${public_block}, not the agreed one")
        fi
      fi
    fi
  fi

  if [ "${#errors[@]}" -eq 0 ]; then
    printf '   [PASS] chain %s, block %s %s, receipt %s on both providers\n' \
      "$expected" "$height" "$paid_hash" "$tx"
  else
    FAILURES=$((FAILURES + 1))
    local e; for e in "${errors[@]}"; do printf '   [FAIL] %s\n' "$e"; done
  fi

  record "$(jq -nc \
    --arg slug "$slug" \
    --arg expected "${expected}" \
    --arg paid_endpoint "$(redact "$paid_url")" \
    --arg public_endpoint "$public_url" \
    --arg paid_chain_id "${paid_id:-}" \
    --arg public_chain_id "${public_id:-}" \
    --arg height "${height:-}" \
    --arg block_hash "${paid_hash:-}" \
    --arg public_block_hash "${public_hash:-}" \
    --arg transaction "${tx:-}" \
    --arg receipt_status "${status:-}" \
    --argjson errors "$(printf '%s\n' "${errors[@]:-}" | jq -R -s 'split("\n") | map(select(length > 0))')" \
    '{network:$slug, expected_chain_id:$expected, paid_endpoint:$paid_endpoint,
      public_endpoint:$public_endpoint, paid_chain_id:$paid_chain_id,
      public_chain_id:$public_chain_id, height:$height, block_hash:$block_hash,
      public_block_hash:$public_block_hash, transaction:$transaction,
      receipt_status:$receipt_status, errors:$errors,
      passed:($errors | length == 0)}')"
}

if [ -n "$WANTED" ]; then
  IFS=',' read -r -a SELECTED <<<"$WANTED"
else
  SELECTED=()
fi

echo "[provider-endpoint] key: ${KEY_SOURCE}"
echo "[provider-endpoint] endpoint template: $(redact "$PAID_TEMPLATE")"
echo

for entry in "${NETWORKS[@]}"; do
  read -r slug expected public_url <<<"$entry"
  if [ "${#SELECTED[@]}" -gt 0 ]; then
    keep=0
    for want in "${SELECTED[@]}"; do [ "$want" = "$slug" ] && keep=1; done
    [ "$keep" -eq 1 ] || continue
  fi
  check_network "$slug" "$expected" "$public_url"
  echo
done

[ -n "$OUT" ] || OUT="$ROOT/.ai/reports/provider-endpoint-$(date -u +%Y%m%d-%H%M%S).json"
mkdir -p "$(dirname "$OUT")"
jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg key_source "$KEY_SOURCE" \
  --arg endpoint_template "$(redact "$PAID_TEMPLATE")" \
  --argjson confirmations "$CONFIRMATIONS" \
  --argjson results "$RESULTS" \
  '{generated_at:$generated_at, key_source:$key_source, provider:"drpc",
    endpoint_template:$endpoint_template, confirmations:$confirmations,
    passed:($results | all(.passed)), networks:$results}' \
  >"$OUT"

echo "[provider-endpoint] evidence: ${OUT#"$ROOT"/}"
if [ "$FAILURES" -eq 0 ]; then
  echo "[provider-endpoint] PASS — every endpoint served the chain it claims and agreed with a second provider"
  exit 0
fi
echo "[provider-endpoint] FAIL — ${FAILURES} network(s) did not agree"
exit 1
