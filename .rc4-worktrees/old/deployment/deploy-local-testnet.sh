#!/bin/bash
#
# 🚀 X3 Chain - Single-Server Testnet Deployment
# 
# This script deploys a full 3-validator testnet on ONE machine using systemd services
#

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}X3 Chain Local Testnet Setup${NC}"
echo -e "${BLUE}================================${NC}"
echo ""

# Get project root
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY_DIR="$PROJECT_ROOT/deployment"
BINARY="$PROJECT_ROOT/target/release/x3-chain-node"
CHAIN_SPEC="$DEPLOY_DIR/chain-specs/x3-testnet-raw.json"
KEYS_DIR="$DEPLOY_DIR/keys"

# Verify prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    echo "Run: cd $PROJECT_ROOT && SKIP_WASM_BUILD=1 cargo build --release"
    exit 1
fi

if [ ! -f "$CHAIN_SPEC" ]; then
    echo -e "${RED}Error: Chain spec not found at $CHAIN_SPEC${NC}"
    exit 1
fi

if [ ! -d "$KEYS_DIR" ]; then
    echo -e "${RED}Error: Keys directory not found at $KEYS_DIR${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Binary found${NC}"
echo -e "${GREEN}✓ Chain spec found${NC}"
echo -e "${GREEN}✓ Keys found${NC}"
echo ""

# Copy binary to system path
echo -e "${YELLOW}Installing binary...${NC}"
sudo cp "$BINARY" /usr/local/bin/x3-chain-node
sudo chmod +x /usr/local/bin/x3-chain-node
echo -e "${GREEN}✓ Binary installed to /usr/local/bin/x3-chain-node${NC}"
echo ""

# Create data directories
echo -e "${YELLOW}Creating data directories...${NC}"
sudo mkdir -p /var/lib/x3-chain/{bootnode,validator-01,validator-02,validator-03,rpc}
sudo chown -R $USER:$USER /var/lib/x3-chain
echo -e "${GREEN}✓ Data directories created${NC}"
echo ""

# Deploy validator keys
echo -e "${YELLOW}Deploying validator keys...${NC}"

for i in 01 02 03; do
    KEYSTORE_SRC="$KEYS_DIR/validator-$i-keys/keystore"
    KEYSTORE_DST="/var/lib/x3-chain/validator-$i/chains/x3_testnet/keystore"
    
    if [ -d "$KEYSTORE_SRC" ]; then
        mkdir -p "$(dirname "$KEYSTORE_DST")"
        cp -r "$KEYSTORE_SRC" "$(dirname "$KEYSTORE_DST")/"
        echo -e "${GREEN}✓ Validator $i keys deployed${NC}"
    else
        echo -e "${YELLOW}⚠ Validator $i keystore not found, skipping${NC}"
    fi
done
echo ""

# Get bootnode peer ID
echo -e "${YELLOW}Getting bootnode peer ID...${NC}"
BOOTNODE_KEY=$(cat "$KEYS_DIR/bootnode-node-key")
BOOTNODE_PEER_ID=$(grep -oP '12D3[a-zA-Z0-9]+' "$KEYS_DIR/bootnode-info.txt" | head -1)

if [ -z "$BOOTNODE_PEER_ID" ]; then
    echo -e "${RED}Error: Could not find bootnode peer ID in bootnode-info.txt${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Bootnode Peer ID: $BOOTNODE_PEER_ID${NC}"
echo ""

# Create systemd services
echo -e "${YELLOW}Creating systemd services...${NC}"

# Bootnode service
sudo tee /etc/systemd/system/x3-bootnode.service > /dev/null <<EOF
[Unit]
Description=X3 Chain Bootnode
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain $CHAIN_SPEC \\
  --base-path /var/lib/x3-chain/bootnode \\
  --name "X3-Bootnode" \\
  --node-key $BOOTNODE_KEY \\
  --port 30333 \\
  --rpc-port 9944 \\
  --validator \\
  --rpc-cors all \\
  --pruning archive
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Validator 01 service
sudo tee /etc/systemd/system/x3-validator-01.service > /dev/null <<EOF
[Unit]
Description=X3 Chain Validator 01
After=network.target x3-bootnode.service

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain $CHAIN_SPEC \\
  --base-path /var/lib/x3-chain/validator-01 \\
  --name "Validator-01" \\
  --validator \\
  --port 30334 \\
  --rpc-port 9945 \\
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/$BOOTNODE_PEER_ID \\
  --pruning archive
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Validator 02 service
sudo tee /etc/systemd/system/x3-validator-02.service > /dev/null <<EOF
[Unit]
Description=X3 Chain Validator 02
After=network.target x3-bootnode.service

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain $CHAIN_SPEC \\
  --base-path /var/lib/x3-chain/validator-02 \\
  --name "Validator-02" \\
  --validator \\
  --port 30335 \\
  --rpc-port 9946 \\
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/$BOOTNODE_PEER_ID \\
  --pruning archive
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Validator 03 service
sudo tee /etc/systemd/system/x3-validator-03.service > /dev/null <<EOF
[Unit]
Description=X3 Chain Validator 03
After=network.target x3-bootnode.service

[Service]
Type=simple
User=$USER
WorkingDirectory=$HOME
ExecStart=/usr/local/bin/x3-chain-node \\
  --chain $CHAIN_SPEC \\
  --base-path /var/lib/x3-chain/validator-03 \\
  --name "Validator-03" \\
  --validator \\
  --port 30336 \\
  --rpc-port 9947 \\
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/$BOOTNODE_PEER_ID \\
  --pruning archive
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

echo -e "${GREEN}✓ Systemd services created${NC}"
echo ""

# Reload systemd
echo -e "${YELLOW}Reloading systemd...${NC}"
sudo systemctl daemon-reload
echo -e "${GREEN}✓ Systemd reloaded${NC}"
echo ""

# Enable services
echo -e "${YELLOW}Enabling services...${NC}"
sudo systemctl enable x3-bootnode x3-validator-01 x3-validator-02 x3-validator-03
echo -e "${GREEN}✓ Services enabled${NC}"
echo ""

# Start services
echo -e "${YELLOW}Starting services...${NC}"
echo "Starting bootnode first, waiting 5 seconds..."
sudo systemctl start x3-bootnode
sleep 5

echo "Starting validators..."
sudo systemctl start x3-validator-01
sleep 2
sudo systemctl start x3-validator-02
sleep 2
sudo systemctl start x3-validator-03
sleep 3

echo -e "${GREEN}✓ All services started${NC}"
echo ""

# Check status
echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Service Status${NC}"
echo -e "${BLUE}================================${NC}"
echo ""

for service in x3-bootnode x3-validator-01 x3-validator-02 x3-validator-03; do
    if sudo systemctl is-active --quiet $service; then
        echo -e "${GREEN}✓ $service is running${NC}"
    else
        echo -e "${RED}✗ $service is NOT running${NC}"
    fi
done
echo ""

# Wait for RPC to be ready
echo -e "${YELLOW}Waiting for RPC to be ready...${NC}"
for i in {1..30}; do
    if curl -s http://localhost:9944 > /dev/null 2>&1; then
        echo -e "${GREEN}✓ RPC is ready!${NC}"
        break
    fi
    echo -n "."
    sleep 1
done
echo ""
echo ""

# Check block production
echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}Blockchain Status${NC}"
echo -e "${BLUE}================================${NC}"
echo ""

sleep 10  # Wait for blocks to start

BLOCK_NUM=$(curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' \
    http://localhost:9944 | jq -r '.result.block.header.number' 2>/dev/null || echo "0")

if [ "$BLOCK_NUM" != "0" ] && [ -n "$BLOCK_NUM" ]; then
    echo -e "${GREEN}✓ Blocks being produced! Current block: $BLOCK_NUM${NC}"
else
    echo -e "${YELLOW}⚠ Waiting for block production to start...${NC}"
fi
echo ""

# Check peers
PEERS=$(curl -s -H "Content-Type: application/json" \
    -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
    http://localhost:9944 | jq -r '.result.peers' 2>/dev/null || echo "0")

echo -e "Connected peers: ${GREEN}$PEERS${NC}"
echo ""

# Summary
echo -e "${BLUE}================================${NC}"
echo -e "${BLUE}🎉 Deployment Complete!${NC}"
echo -e "${BLUE}================================${NC}"
echo ""
echo -e "${GREEN}Your testnet is running!${NC}"
echo ""
echo "📊 RPC Endpoints:"
echo "  • Bootnode:    http://localhost:9944"
echo "  • Validator 01: http://localhost:9945"
echo "  • Validator 02: http://localhost:9946"
echo "  • Validator 03: http://localhost:9947"
echo ""
echo "🔍 Useful Commands:"
echo "  • Check logs:        sudo journalctl -u x3-bootnode -f"
echo "  • Check status:      sudo systemctl status x3-validator-01"
echo "  • Stop all:          sudo systemctl stop x3-{bootnode,validator-*}"
echo "  • Restart:           sudo systemctl restart x3-bootnode"
echo ""
echo "🌐 Connect via Polkadot.js:"
echo "  1. Go to https://polkadot.js.org/apps/"
echo "  2. Settings → Custom endpoint → ws://127.0.0.1:9944"
echo ""
echo -e "${GREEN}Happy testing! 🚀${NC}"
echo ""
