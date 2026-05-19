#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
examples_dir="$project_root/deployment/systemd/examples"
output_dir="${OUTPUT_DIR:-$project_root/deployment/systemd/rendered}"
servers_env_out="${SERVERS_ENV_OUT:-$project_root/deployment/servers.env}"

bootnode_multiaddr="${BOOTNODE_MULTIADDR:-}"
bootnode_host="${BOOTNODE_HOST:-}"
validator_01_host="${VALIDATOR_01_HOST:-}"
validator_02_host="${VALIDATOR_02_HOST:-}"
validator_03_host="${VALIDATOR_03_HOST:-}"
rpc_01_host="${RPC_01_HOST:-}"
monitoring_01_host="${MONITORING_01_HOST:-}"

usage() {
    cat <<EOF
Usage:
  BOOTNODE_MULTIADDR='/ip4/<ip>/tcp/30333/p2p/<peer-id>' \
  ./deployment/scripts/render-rack-config.sh

Optional host exports:
  BOOTNODE_HOST=user@host
  VALIDATOR_01_HOST=user@host
  VALIDATOR_02_HOST=user@host
  VALIDATOR_03_HOST=user@host
  RPC_01_HOST=user@host
  MONITORING_01_HOST=user@host

Optional output overrides:
  OUTPUT_DIR=/path/to/rendered-envs
  SERVERS_ENV_OUT=/path/to/servers.env
EOF
}

if [[ "${1:-}" == "--help" ]]; then
    usage
    exit 0
fi

if [[ -z "$bootnode_multiaddr" ]]; then
    echo "Error: BOOTNODE_MULTIADDR is required." >&2
    usage >&2
    exit 1
fi

mkdir -p "$output_dir"

render_env() {
    local source_file="$1"
    local destination_file="$2"
    cp "$source_file" "$destination_file"
}

set_bootnodes_value() {
    local destination_file="$1"
    local value="$2"
    python3 - "$destination_file" "$value" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
value = sys.argv[2]
lines = path.read_text().splitlines()
updated = []
replaced = False
for line in lines:
    if line.startswith("bootnodes="):
        updated.append(f"bootnodes={value}")
        replaced = True
    else:
        updated.append(line)
if not replaced:
    updated.append(f"bootnodes={value}")
path.write_text("\n".join(updated) + "\n")
PY
}

render_env "$examples_dir/x3-lab-val-01.env.example" "$output_dir/x3-lab-val-01.env"
render_env "$examples_dir/x3-lab-val-02.env.example" "$output_dir/x3-lab-val-02.env"
render_env "$examples_dir/x3-lab-val-03.env.example" "$output_dir/x3-lab-val-03.env"
render_env "$examples_dir/x3-lab-rpc-01.env.example" "$output_dir/x3-lab-rpc-01.env"

set_bootnodes_value "$output_dir/x3-lab-val-01.env" ""
set_bootnodes_value "$output_dir/x3-lab-val-02.env" "$bootnode_multiaddr"
set_bootnodes_value "$output_dir/x3-lab-val-03.env" "$bootnode_multiaddr"
set_bootnodes_value "$output_dir/x3-lab-rpc-01.env" "$bootnode_multiaddr"

cat > "$servers_env_out" <<EOF
BOOTNODE_HOST=${bootnode_host}
VALIDATOR_01_HOST=${validator_01_host}
VALIDATOR_02_HOST=${validator_02_host}
VALIDATOR_03_HOST=${validator_03_host}

# Optional non-authority hosts for later expansion.
RPC_01_HOST=${rpc_01_host}
MONITORING_01_HOST=${monitoring_01_host}
EOF

echo "Rendered rack env files to: $output_dir"
echo "Wrote host target file to: $servers_env_out"
echo "Next steps:"
echo "  1. Review the rendered env files before copying them to /etc/x3-chain/nodes/."
echo "  2. Fill any still-empty *_HOST values in $servers_env_out if you did not export them yet."
echo "  3. Update deployment/inventory.yaml with the real management and public addresses."