#!/usr/bin/env bash
set -euo pipefail

required_vars=(chain_spec base_path node_name)

for required_var in "${required_vars[@]}"; do
    if [[ -z "${!required_var:-}" ]]; then
        echo "Error: required environment variable '$required_var' is not set" >&2
        exit 1
    fi
done

node_bin="${node_bin:-/usr/local/bin/x3-chain-node}"
validator="${validator:-false}"
enable_rpc="${enable_rpc:-false}"
rpc_bind="${rpc_bind:-private}"
enable_prometheus="${enable_prometheus:-true}"
prometheus_bind="${prometheus_bind:-private}"
p2p_port="${p2p_port:-30333}"
rpc_port="${rpc_port:-9944}"
prometheus_port="${prometheus_port:-9615}"
pruning="${pruning:-archive}"
database_backend="${database_backend:-}"
bootnodes="${bootnodes:-}"
node_key_file="${node_key_file:-}"
node_key="${node_key:-}"
rpc_cors="${rpc_cors:-}"
telemetry_url="${telemetry_url:-}"
extra_args="${extra_args:-}"

case "$validator" in
    true|false) ;;
    *)
        echo "Error: validator must be 'true' or 'false'" >&2
        exit 1
        ;;
esac

case "$enable_rpc" in
    true|false) ;;
    *)
        echo "Error: enable_rpc must be 'true' or 'false'" >&2
        exit 1
        ;;
esac

case "$enable_prometheus" in
    true|false) ;;
    *)
        echo "Error: enable_prometheus must be 'true' or 'false'" >&2
        exit 1
        ;;
esac

case "$rpc_bind" in
    private|public) ;;
    *)
        echo "Error: rpc_bind must be 'private' or 'public'" >&2
        exit 1
        ;;
esac

case "$prometheus_bind" in
    private|public) ;;
    *)
        echo "Error: prometheus_bind must be 'private' or 'public'" >&2
        exit 1
        ;;
esac

if [[ ! -x "$node_bin" ]]; then
    echo "Error: node binary '$node_bin' is not executable" >&2
    exit 1
fi

args=(
    "$node_bin"
    --chain "$chain_spec"
    --base-path "$base_path"
    --name "$node_name"
    --port "$p2p_port"
    --pruning "$pruning"
)

if [[ "$validator" == "true" ]]; then
    args+=(--validator)
fi

if [[ -n "$database_backend" ]]; then
    args+=(--database "$database_backend")
fi

if [[ -n "$node_key_file" ]]; then
    args+=(--node-key-file "$node_key_file")
fi

if [[ -n "$node_key" ]]; then
    args+=(--node-key "$node_key")
fi

if [[ -n "$bootnodes" ]]; then
    args+=(--bootnodes "$bootnodes")
fi

if [[ "$enable_rpc" == "true" ]]; then
    args+=(--rpc-port "$rpc_port")
    if [[ "$rpc_bind" == "public" ]]; then
        args+=(--rpc-external)
    fi
    if [[ -n "$rpc_cors" ]]; then
        args+=(--rpc-cors "$rpc_cors")
    fi
fi

if [[ "$enable_prometheus" == "true" ]]; then
    args+=(--prometheus-port "$prometheus_port")
    if [[ "$prometheus_bind" == "public" ]]; then
        args+=(--prometheus-external)
    fi
fi

if [[ -n "$telemetry_url" ]]; then
    args+=(--telemetry-url "$telemetry_url")
fi

if [[ -n "$extra_args" ]]; then
    read -r -a extra_args_array <<< "$extra_args"
    args+=("${extra_args_array[@]}")
fi

echo "Starting X3 node: $node_name"
exec "${args[@]}"