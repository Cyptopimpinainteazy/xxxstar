#!/bin/bash
set -euo pipefail

# X3 Chain public RPC runner.
#
# Recommended: bind node RPC to localhost and expose it publicly via a
# reverse proxy (nginx/caddy) with TLS + rate limiting.
#
# Environment variables:
# - node_name:          Human-readable node name (default: x3-rpc)
# - base_path:          Data dir (default: /var/lib/x3-chain/rpc)
# - chain:              dev|local|staging|testnet|<path>|<builtin id> (default: testnet)
# - chain_spec_dir:     Directory with deployment specs (default: deployment/chain-specs)
# - rpc_port:           RPC port (default: 9944)
# - p2p_port:           P2P port (default: 30333)
# - prometheus_port:    Prometheus port (default: 9615)
# - rpc_bind:           localhost|public (default: localhost)
# - rpc_cors:           Comma-separated origins or "all" (default: "")
# - rpc_methods:        Safe|Unsafe|Auto (default: Safe)
# - rpc_max_connections:                 Max concurrent RPC connections (default: 300)
# - rpc_max_request_size_mb:             Max request size in MB (default: 10)
# - rpc_max_response_size_mb:            Max response size in MB (default: 50)
# - rpc_max_subscriptions_per_connection:Max WS subscriptions per connection (default: 20)
# - require_confirm_public_rpc:          true|false (default: false)
# - confirm_public_rpc:                  Set to "yes" when require_confirm_public_rpc=true
# - require_safe_rpc_methods_on_public:  true|false (default: true)
# - node_bin:           Path to x3-chain-node binary
# - bootnodes:          Comma-separated bootnode multiaddrs to pass via --bootnodes

node_name="${node_name:-x3-rpc}"
base_path="${base_path:-/var/lib/x3-chain/rpc}"
chain="${chain:-testnet}"
chain_spec_dir="${chain_spec_dir:-deployment/chain-specs}"
bootnodes="${bootnodes:-}"

rpc_port="${rpc_port:-9944}"
p2p_port="${p2p_port:-30333}"
prometheus_port="${prometheus_port:-9615}"

rpc_bind="${rpc_bind:-localhost}"
rpc_cors="${rpc_cors:-}"
rpc_methods="${rpc_methods:-Safe}"

rpc_max_connections="${rpc_max_connections:-300}"
rpc_max_request_size_mb="${rpc_max_request_size_mb:-10}"
rpc_max_response_size_mb="${rpc_max_response_size_mb:-50}"
rpc_max_subscriptions_per_connection="${rpc_max_subscriptions_per_connection:-20}"

require_confirm_public_rpc="${require_confirm_public_rpc:-false}"
confirm_public_rpc="${confirm_public_rpc:-}"

require_safe_rpc_methods_on_public="${require_safe_rpc_methods_on_public:-true}"

node_bin="${node_bin:-./target/release/x3-chain-node}"

if [ ! -x "${node_bin}" ]; then
  echo "❌ node_bin not found or not executable: ${node_bin}"
  echo "   Build with: cargo build --release"
  exit 1
fi

chain_arg=()
case "${chain}" in
  dev)
    chain_arg=(--dev)
    ;;
  local)
    chain_arg=(--chain local)
    ;;
  staging)
    if [ -f "${chain_spec_dir}/x3-staging-plain.json" ]; then
      chain_arg=(--chain "${chain_spec_dir}/x3-staging-plain.json")
    else
      chain_arg=(--chain staging)
    fi
    ;;
  testnet)
    if [ -f "${chain_spec_dir}/x3-testnet-raw.json" ]; then
      chain_arg=(--chain "${chain_spec_dir}/x3-testnet-raw.json")
    elif [ -f "${chain_spec_dir}/x3-testnet-plain.json" ]; then
      chain_arg=(--chain "${chain_spec_dir}/x3-testnet-plain.json")
    else
      echo "❌ No testnet chainspec found in ${chain_spec_dir}."
      exit 1
    fi
    ;;
  *)
    if [ -f "${chain}" ]; then
      chain_arg=(--chain "${chain}")
    else
      chain_arg=(--chain "${chain}")
    fi
    ;;
esac

rpc_bind_args=()
case "${rpc_bind}" in
  localhost)
    # Substrate defaults to localhost; keep it that way.
    ;;
  public)
    # WARNING: This exposes RPC directly to the Internet.
    # Prefer nginx + TLS + allowlists.
    # Substrate requires the explicit unsafe flag for non-local access.
    if [ "${require_confirm_public_rpc}" = "true" ] && [ "${confirm_public_rpc}" != "yes" ]; then
      echo "❌ Refusing to expose public RPC without explicit confirmation."
      echo "   Set: confirm_public_rpc=yes (or set require_confirm_public_rpc=false)"
      exit 1
    fi

    if [ "${require_safe_rpc_methods_on_public}" = "true" ] && [ "${rpc_methods}" != "Safe" ]; then
      echo "❌ Refusing to expose public RPC with rpc_methods=${rpc_methods}."
      echo "   Set rpc_methods=Safe (recommended), or set require_safe_rpc_methods_on_public=false."
      exit 1
    fi

    rpc_bind_args=(--rpc-external --unsafe-rpc-external)
    ;;
  *)
    echo "❌ rpc_bind must be 'localhost' or 'public' (got: ${rpc_bind})"
    exit 1
    ;;
esac

rpc_cors_args=()
if [ -n "${rpc_cors}" ]; then
  rpc_cors_args=(--rpc-cors "${rpc_cors}")
fi

bootnode_args=()
if [ -n "${bootnodes}" ]; then
  bootnode_args=(--bootnodes "${bootnodes}")
fi

echo "🌐 X3 Chain RPC Node"
echo "  node_name=${node_name}"
echo "  base_path=${base_path}"
echo "  chain=${chain}"
echo "  bootnodes=${bootnodes:-<from chainspec>}"
echo "  rpc_bind=${rpc_bind} rpc_port=${rpc_port}"
echo "  p2p_port=${p2p_port} prometheus_port=${prometheus_port}"
echo "  rpc_methods=${rpc_methods} rpc_cors=${rpc_cors:-<none>}"
echo "  rpc_max_connections=${rpc_max_connections} rpc_max_request_size_mb=${rpc_max_request_size_mb} rpc_max_response_size_mb=${rpc_max_response_size_mb} rpc_max_subscriptions_per_connection=${rpc_max_subscriptions_per_connection}"

exec "${node_bin}" \
  "${chain_arg[@]}" \
  --name "${node_name}" \
  --base-path "${base_path}" \
  --port "${p2p_port}" \
  --rpc-port "${rpc_port}" \
  --rpc-methods "${rpc_methods}" \
  --rpc-max-connections "${rpc_max_connections}" \
  --rpc-max-request-size "${rpc_max_request_size_mb}" \
  --rpc-max-response-size "${rpc_max_response_size_mb}" \
  --rpc-max-subscriptions-per-connection "${rpc_max_subscriptions_per_connection}" \
  --prometheus-port "${prometheus_port}" \
  --prometheus-external=false \
  --no-hardware-benchmarks \
  "${bootnode_args[@]}" \
  "${rpc_bind_args[@]}" \
  "${rpc_cors_args[@]}" \
  "$@"
