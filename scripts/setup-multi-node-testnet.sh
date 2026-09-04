#!/bin/bash
# X3 Chain Multi-Node Testnet Setup Script
# Sets up a multi-node testnet with proper configuration

set -e

echo "=== X3 Chain Multi-Node Testnet Setup ==="
echo ""

# Configuration
CHAIN_SPEC="${CHAIN_SPEC:-x3-testnet-raw.json}"
NODES="${NODES:-4}"
RPC_PORT="${RPC_PORT:-9933}"
WS_PORT="${WS_PORT:-9944}"
P2P_PORT="${P2P_PORT:-30333}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if chain spec exists
if [ ! -f "chain-specs/${CHAIN_SPEC}" ]; then
    echo -e "${RED}ERROR: Chain spec not found: chain-specs/${CHAIN_SPEC}${NC}"
    echo "Available chain specs:"
    ls -la chain-specs/
    exit 1
fi

echo -e "${GREEN}Using chain spec: ${CHAIN_SPEC}${NC}"
echo -e "${GREEN}Number of nodes: ${NODES}${NC}"
echo ""

# Function to generate node keys
generate_node_keys() {
    local node_id=$1
    local node_dir="deployment/keys/node-${node_id}"
    
    echo -e "${YELLOW}Generating keys for node ${node_id}...${NC}"
    
    mkdir -p "${node_dir}"
    
    # Generate node key (Ed25519)
    if [ ! -f "${node_dir}/node-key" ]; then
        # Generate a random Ed25519 key
        local node_key=$(xxd -p -l 64 /dev/urandom | tr -d '\n')
        echo "${node_key}" > "${node_dir}/node-key"
        echo -e "${GREEN}Node key generated${NC}"
    fi
    
    # Generate authority keys (Aura and Grandpa)
    if [ ! -f "${node_dir}/authority-keys.json" ]; then
        # Generate keys using subkey (if available)
        if command -v subkey &> /dev/null; then
            local aura_seed="//Node${node_id}Aura"
            local grandpa_seed="//Node${node_id}Grandpa"
            
            local aura_pub=$(subkey inspect "${aura_seed}" 2>/dev/null | grep "Public key" | awk '{print $3}')
            local grandpa_pub=$(subkey inspect "${grandpa_seed}" 2>/dev/null | grep "Public key" | awk '{print $3}')
            
            echo "{\"aura\":\"${aura_pub}\",\"grandpa\":\"${grandpa_pub}\"}" > "${node_dir}/authority-keys.json"
            echo -e "${GREEN}Authority keys generated${NC}"
        else
            echo -e "${YELLOW}subkey not found, skipping authority key generation${NC}"
        fi
    fi
}

# Function to create node configuration
create_node_config() {
    local node_id=$1
    local node_dir="deployment/keys/node-${node_id}"
    local config_dir="deployment/config/node-${node_id}"
    
    echo -e "${YELLOW}Creating configuration for node ${node_id}...${NC}"
    
    mkdir -p "${config_dir}"
    
    # Create node configuration
    cat > "${config_dir}/config.toml" << EOF
# X3 Chain Node Configuration
# Node ID: ${node_id}

[node]
name = "node-${node_id}"
port = ${P2P_PORT}
rpc_port = ${RPC_PORT}
ws_port = ${WS_PORT}

[chain]
spec = "chain-specs/${CHAIN_SPEC}"
base_path = "/var/lib/x3/node-${node_id}"

[network]
bootnodes = [
    "/ip4/127.0.0.1/tcp/30333/p2p/211d3541d4b56a921adaf0b6629e48a09f9840968ebe590bace085ca5bff90d9"
]

[keystore]
path = "${node_dir}/keystore"
node_key = "${node_dir}/node-key"

[metrics]
enabled = true
port = 9615

[telemetry]
enabled = false
url = "wss://telemetry.x3-chain.io/submit"
EOF
    
    echo -e "${GREEN}Node configuration created${NC}"
}

# Function to create systemd service files
create_systemd_services() {
    local node_id=$1
    
    echo -e "${YELLOW}Creating systemd service for node ${node_id}...${NC}"
    
    cat > "deployment/systemd/x3-chain-node@${node_id}.service" << EOF
[Unit]
Description=X3 Chain Node ${node_id}
After=network.target

[Service]
Type=simple
User=x3
Group=x3
ExecStart=/usr/local/bin/x3-chain-node \
    --name "node-${node_id}" \
    --base-path /var/lib/x3/node-${node_id} \
    --chain chain-specs/${CHAIN_SPEC} \
    --port ${P2P_PORT} \
    --rpc-port ${RPC_PORT} \
    --ws-port ${WS_PORT} \
    --validator \
    --node-key $(cat deployment/keys/node-${node_id}/node-key) \
    --telemetry-url "wss://telemetry.x3-chain.io/submit 0" \
    --log info
Restart=always
RestartSec=10
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF
    
    echo -e "${GREEN}Systemd service created${NC}"
}

# Main setup
echo -e "${GREEN}=== Starting Multi-Node Testnet Setup ===${NC}"
echo ""

# Generate keys for all nodes
for ((i=1; i<=NODES; i++)); do
    generate_node_keys $i
    echo ""
done

# Create configurations
for ((i=1; i<=NODES; i++)); do
    create_node_config $i
    echo ""
done

# Create systemd services
for ((i=1; i<=NODES; i++)); do
    create_systemd_services $i
    echo ""
done

echo -e "${GREEN}=== Multi-Node Testnet Setup Complete ===${NC}"
echo ""
echo "To start the testnet:"
echo "  1. Copy chain-specs/${CHAIN_SPEC} to all nodes"
echo "  2. Copy deployment/keys/node-* to each node"
echo "  3. Copy deployment/systemd/x3-chain-node@*.service to /etc/systemd/system/"
echo "  4. Run: systemctl daemon-reload && systemctl start x3-chain-node@1"
echo ""
echo "To check node status:"
echo "  systemctl status x3-chain-node@1"
echo ""
