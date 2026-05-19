#!/usr/bin/env bash
# Configure firewall on X3 Chain nodes with explicit network policy.

set -euo pipefail

node_type="${1:-validator}"
admin_cidr="${ADMIN_CIDR:-}"
p2p_cidr="${P2P_CIDR:-0.0.0.0/0}"
metrics_cidr="${METRICS_CIDR:-}"
rpc_cidr="${RPC_CIDR:-}"
public_rpc="${PUBLIC_RPC:-0}"
prometheus_cidr="${PROMETHEUS_CIDR:-}"
grafana_cidr="${GRAFANA_CIDR:-}"

case "$node_type" in
    validator|rpc|bootnode|monitoring) ;;
    *)
        echo "Usage: $0 [validator|rpc|bootnode|monitoring]" >&2
        exit 1
        ;;
esac

if [[ -z "$admin_cidr" ]]; then
    echo "Error: ADMIN_CIDR must be set. Refusing to open SSH to the world by default." >&2
    exit 1
fi

echo "Configuring firewall for $node_type node"

sudo apt-get update
sudo apt-get install -y ufw

sudo ufw default deny incoming
sudo ufw default allow outgoing

sudo ufw allow from "$admin_cidr" to any port 22 proto tcp comment 'X3 SSH admin'

if [[ "$node_type" != "monitoring" ]]; then
    sudo ufw allow from "$p2p_cidr" to any port 30333 proto tcp comment 'X3 P2P'
fi

case "$node_type" in
    validator)
        echo "Validator role: RPC remains local-only; no public 9944 rule added."
        ;;
    bootnode)
        echo "Bootnode role: only SSH and P2P rules applied by default."
        ;;
    rpc)
        if [[ -n "$rpc_cidr" ]]; then
            sudo ufw allow from "$rpc_cidr" to any port 9944 proto tcp comment 'X3 RPC restricted'
        elif [[ "$public_rpc" == "1" ]]; then
            sudo ufw allow 9944/tcp comment 'X3 RPC public'
        else
            echo "RPC role: no 9944 rule added because RPC_CIDR is unset and PUBLIC_RPC is not 1."
        fi
        ;;
    monitoring)
        if [[ -n "$prometheus_cidr" ]]; then
            sudo ufw allow from "$prometheus_cidr" to any port 9090 proto tcp comment 'Prometheus restricted'
        fi
        if [[ -n "$grafana_cidr" ]]; then
            sudo ufw allow from "$grafana_cidr" to any port 3000 proto tcp comment 'Grafana restricted'
        fi
        ;;
esac

if [[ -n "$metrics_cidr" && "$node_type" != "monitoring" ]]; then
    sudo ufw allow from "$metrics_cidr" to any port 9615 proto tcp comment 'X3 metrics restricted'
fi

sudo ufw --force enable
sudo ufw status verbose

echo "Firewall configured for $node_type node"
