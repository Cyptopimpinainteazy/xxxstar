#!/bin/bash
#
# 🌐 X3 Chain - Multi-Server Testnet Deployment
#
# Deploy validators across multiple physical/virtual servers
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}X3 Chain Multi-Server Deployment${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY_DIR="$PROJECT_ROOT/deployment"
BINARY="$PROJECT_ROOT/target/release/x3-chain-node"
CHAIN_SPEC="$DEPLOY_DIR/chain-specs/x3-testnet-raw.json"
KEYS_DIR="$DEPLOY_DIR/keys"
SERVERS_ENV="$DEPLOY_DIR/servers.env"

# Server inventory - sourced from deployment/servers.env
echo -e "${YELLOW}Server Inventory:${NC}"
echo ""

if [ ! -f "$SERVERS_ENV" ]; then
    echo -e "${RED}Error: $SERVERS_ENV not found${NC}"
    echo "Copy deployment/servers.env.example to deployment/servers.env and fill in real SSH targets."
    exit 1
fi

# shellcheck disable=SC1090
source "$SERVERS_ENV"

declare -A SERVERS=(
    ["bootnode"]="${BOOTNODE_HOST:-}"
    ["validator-01"]="${VALIDATOR_01_HOST:-}"
    ["validator-02"]="${VALIDATOR_02_HOST:-}"
    ["validator-03"]="${VALIDATOR_03_HOST:-}"
)

for required_host in bootnode validator-01 validator-02 validator-03; do
    if [ -z "${SERVERS[$required_host]}" ]; then
        echo -e "${RED}Error: missing SSH target for $required_host in $SERVERS_ENV${NC}"
        echo "Set BOOTNODE_HOST, VALIDATOR_01_HOST, VALIDATOR_02_HOST, and VALIDATOR_03_HOST before running this script."
        exit 1
    fi
done

# Ports
BOOTNODE_PORT=30333
BOOTNODE_RPC=9944

# Show inventory
echo "Bootnode:     ${SERVERS[bootnode]}"
echo "Validator-01: ${SERVERS[validator-01]}"
echo "Validator-02: ${SERVERS[validator-02]}"
echo "Validator-03: ${SERVERS[validator-03]}"
echo ""

read -p "Is this correct? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Update deployment/servers.env with the correct SSH targets and rerun this script"
    exit 1
fi
echo ""

# Verify prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    exit 1
fi

if [ ! -f "$CHAIN_SPEC" ]; then
    echo -e "${RED}Error: Chain spec not found at $CHAIN_SPEC${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Prerequisites OK${NC}"
echo ""

# Test SSH connectivity
echo -e "${YELLOW}Testing SSH connectivity...${NC}"
for name in "${!SERVERS[@]}"; do
    server="${SERVERS[$name]}"
    if ssh -o ConnectTimeout=5 "$server" "echo ok" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ $name ($server) reachable${NC}"
    else
        echo -e "${RED}✗ $name ($server) NOT reachable${NC}"
        echo "Make sure SSH keys are set up and servers are online"
        exit 1
    fi
done
echo ""

# Deploy binary to all servers
echo -e "${YELLOW}Deploying binary to servers...${NC}"
for name in "${!SERVERS[@]}"; do
    server="${SERVERS[$name]}"
    echo "Copying to $name..."
    scp "$BINARY" "$server:/tmp/x3-chain-node"
    ssh "$server" "sudo mv /tmp/x3-chain-node /usr/local/bin/ && sudo chmod +x /usr/local/bin/x3-chain-node"
    echo -e "${GREEN}✓ $name done${NC}"
done
echo ""

# Deploy chain spec to all servers
echo -e "${YELLOW}Deploying chain spec...${NC}"
for name in "${!SERVERS[@]}"; do
    server="${SERVERS[$name]}"
    ssh "$server" "mkdir -p ~/x3-chain"
    scp "$CHAIN_SPEC" "$server:~/x3-chain/chain-spec.json"
    echo -e "${GREEN}✓ $name done${NC}"
done
echo ""

# Get bootnode peer ID
echo -e "${YELLOW}Generating bootnode peer ID...${NC}"
BOOTNODE_KEY=$(cat "$KEYS_DIR/bootnode-key.txt")
BOOTNODE_PEER_ID=$(x3-chain-node key inspect-node-key --file "$KEYS_DIR/bootnode-key.txt" 2>&1 | grep -oP '12D3[a-zA-Z0-9]+')
BOOTNODE_IP=$(echo "${SERVERS[bootnode]}" | cut -d'@' -f2)

echo -e "${GREEN}✓ Bootnode Peer ID: $BOOTNODE_PEER_ID${NC}"
echo -e "${GREEN}✓ Bootnode IP: $BOOTNODE_IP${NC}"
echo ""

# Deploy validator keys
echo -e "${YELLOW}Deploying validator keys...${NC}"
for i in 01 02 03; do
    server="${SERVERS[validator-$i]}"
    keystore="$KEYS_DIR/validator-$i-keys/keystore"
    
    if [ -d "$keystore" ]; then
        # Create remote directory
        ssh "$server" "mkdir -p /var/lib/x3-chain/chains/x3_testnet/"
        
        # Copy keystore
        scp -r "$keystore" "$server:/var/lib/x3-chain/chains/x3_testnet/"
        
        # Fix permissions
        ssh "$server" "sudo chown -R \$USER:\$USER /var/lib/x3-chain"
        
        echo -e "${GREEN}✓ Validator-$i keys deployed${NC}"
    else
        echo -e "${RED}✗ Validator-$i keystore not found!${NC}"
    fi
done
echo ""

# Create systemd service on bootnode
echo -e "${YELLOW}Setting up bootnode service...${NC}"
ssh "${SERVERS[bootnode]}" "sudo tee /etc/systemd/system/x3-bootnode.service > /dev/null" <<EOF
[Unit]
Description=X3 Chain Bootnode
After=network.target

[Service]
Type=simple
User=\$(whoami)
WorkingDirectory=\$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain \$HOME/x3-chain/chain-spec.json \\
  --base-path /var/lib/x3-chain/bootnode \\
  --name "X3-Bootnode" \\
  --node-key $BOOTNODE_KEY \\
  --port $BOOTNODE_PORT \\
  --rpc-port $BOOTNODE_RPC \\
  --validator \\
  --rpc-external \\
  --rpc-cors all \\
  --pruning archive
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

ssh "${SERVERS[bootnode]}" "sudo systemctl daemon-reload && sudo systemctl enable x3-bootnode && sudo systemctl start x3-bootnode"
echo -e "${GREEN}✓ Bootnode started${NC}"
echo ""

sleep 5

# Create validator services
for i in 01 02 03; do
    echo -e "${YELLOW}Setting up validator-$i service...${NC}"
    
    port=$((30333 + ${i#0}))
    rpc_port=$((9944 + ${i#0}))
    
    ssh "${SERVERS[validator-$i]}" "sudo tee /etc/systemd/system/x3-validator.service > /dev/null" <<EOF
[Unit]
Description=X3 Chain Validator $i
After=network.target

[Service]
Type=simple
User=\$(whoami)
WorkingDirectory=\$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain \$HOME/x3-chain/chain-spec.json \\
  --base-path /var/lib/x3-chain/validator \\
  --name "Validator-$i" \\
  --validator \\
  --port $port \\
  --rpc-port $rpc_port \\
  --bootnodes /ip4/$BOOTNODE_IP/tcp/$BOOTNODE_PORT/p2p/$BOOTNODE_PEER_ID \\
  --pruning archive
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

    ssh "${SERVERS[validator-$i]}" "sudo systemctl daemon-reload && sudo systemctl enable x3-validator && sudo systemctl start x3-validator"
    echo -e "${GREEN}✓ Validator-$i started${NC}"
    sleep 2
done
echo ""

# Check status
echo -e "${BLUE}=====================================${NC}"
echo -e "${BLUE}Deployment Complete!${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""

echo -e "${GREEN}Services started on:${NC}"
echo "  • Bootnode:     ${SERVERS[bootnode]}"
echo "  • Validator-01: ${SERVERS[validator-01]}"
echo "  • Validator-02: ${SERVERS[validator-02]}"
echo "  • Validator-03: ${SERVERS[validator-03]}"
echo ""

echo "📊 RPC Endpoint:"
echo "  http://$BOOTNODE_IP:$BOOTNODE_RPC"
echo ""

echo "🔍 Check logs on servers:"
echo "  ssh ${SERVERS[bootnode]} 'sudo journalctl -u x3-bootnode -f'"
echo "  ssh ${SERVERS[validator-01]} 'sudo journalctl -u x3-validator -f'"
echo ""

echo "🌐 Connect via Polkadot.js:"
echo "  https://polkadot.js.org/apps/?rpc=ws://$BOOTNODE_IP:$BOOTNODE_RPC"
echo ""

echo -e "${GREEN}Happy testing! 🚀${NC}"
