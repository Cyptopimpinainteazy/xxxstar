#!/bin/bash
# X3 Chain Testnet v1 - Node Deployment
# Day 1: Deploy bootnode + validators

set -e

update_chain_spec_bootnodes() {
    local spec_path="$1"
    local multiaddr="$2"
    python3 - "$spec_path" "$multiaddr" <<'PY'
import json
import sys

spec_path, multiaddr = sys.argv[1], sys.argv[2]
with open(spec_path, "r", encoding="utf-8") as fh:
    spec = json.load(fh)

boot_nodes = spec.get("bootNodes") or []
if multiaddr not in boot_nodes:
    boot_nodes = [multiaddr]
spec["bootNodes"] = boot_nodes

with open(spec_path, "w", encoding="utf-8") as fh:
    json.dump(spec, fh, indent=2)
    fh.write("\n")
PY
}

echo "🚀 X3 Chain Testnet v1 - Node Deployment (Day 1)"
echo "===================================================="
echo ""

# Configuration
DEPLOYMENT_DIR="$(pwd)/deployment"
BINARY="$DEPLOYMENT_DIR/x3-chain-node"
CHAIN_SPEC="$DEPLOYMENT_DIR/chain-specs/x3-testnet-raw.json"
KEYS_DIR="$DEPLOYMENT_DIR/keys"
INVENTORY="$DEPLOYMENT_DIR/inventory.yaml"

# Check prerequisites
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found: $BINARY"
    echo "   Copy from target/release/x3-chain-node"
    exit 1
fi

if [ ! -f "$CHAIN_SPEC" ]; then
    echo "❌ Chain spec not found: $CHAIN_SPEC"
    exit 1
fi

echo "✅ Prerequisites checked"
echo ""

# Step 1: Deploy bootnode
echo "═══════════════════════════════════════════════════"
echo "Step 1/3: Deploy Bootnode"
echo "═══════════════════════════════════════════════════"
echo ""

read -p "Bootnode IP address: " BOOTNODE_IP
read -p "Bootnode SSH user [x3]: " BOOTNODE_USER
BOOTNODE_USER=${BOOTNODE_USER:-x3}

echo ""
echo "Deploying to bootnode ($BOOTNODE_USER@$BOOTNODE_IP)..."

# Create bootnode setup script
cat > "$DEPLOYMENT_DIR/setup-bootnode.sh" << 'BOOTNODE_SCRIPT'
#!/bin/bash
set -e

# Create user and directories
sudo useradd -m -s /bin/bash x3 || true
sudo mkdir -p /var/lib/x3/{data,node-key}
sudo chown -R x3:x3 /var/lib/x3
sudo mkdir -p /etc/x3
sudo chown x3:x3 /etc/x3

# Install systemd service
sudo tee /etc/systemd/system/x3-bootnode.service > /dev/null << 'EOF'
[Unit]
Description=X3 Chain Bootnode
After=network.target

[Service]
Type=simple
User=x3
WorkingDirectory=/var/lib/x3
ExecStart=/usr/local/bin/x3-chain-node \
    --base-path /var/lib/x3/data \
    --chain /etc/x3/x3-testnet-raw.json \
    --name "X3 Bootnode" \
    --node-key-file /var/lib/x3/node-key \
    --port 30333 \
    --prometheus-external \
    --prometheus-port 9615
Restart=always
RestartSec=10
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

echo "✅ Bootnode systemd service created"
BOOTNODE_SCRIPT

# Deploy files
echo "📦 Copying files to bootnode..."
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'bash -s' < "$DEPLOYMENT_DIR/setup-bootnode.sh"
scp "$BINARY" "$BOOTNODE_USER@$BOOTNODE_IP:/tmp/x3-chain-node"
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo mv /tmp/x3-chain-node /usr/local/bin/ && sudo chmod +x /usr/local/bin/x3-chain-node'
scp "$CHAIN_SPEC" "$BOOTNODE_USER@$BOOTNODE_IP:/tmp/x3-testnet-raw.json"
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo mv /tmp/x3-testnet-raw.json /etc/x3/'

# Copy bootnode key
BOOTNODE_KEY=$(cat "$KEYS_DIR/bootnode-key.txt")
echo "$BOOTNODE_KEY" | ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'cat > /tmp/node-key && sudo mv /tmp/node-key /var/lib/x3/node-key && sudo chown x3:x3 /var/lib/x3/node-key'

# Start bootnode
echo "🚀 Starting bootnode..."
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo systemctl daemon-reload && sudo systemctl enable x3-bootnode && sudo systemctl start x3-bootnode'

sleep 5

# Check status
echo ""
echo "📊 Bootnode status:"
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo systemctl status x3-bootnode --no-pager -l'

echo ""
echo "📝 Bootnode logs (last 20 lines):"
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo journalctl -u x3-bootnode -n 20 --no-pager'

# Get peer ID
echo ""
echo "🔍 Extracting peer ID from logs..."
PEER_ID=$(ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo journalctl -u x3-bootnode | grep "Local node identity"' | grep -oP '12D3[A-Za-z0-9]+' | head -1)

if [ -n "$PEER_ID" ]; then
    echo "✅ Bootnode Peer ID: $PEER_ID"
    echo ""
    echo "📋 Bootnode Multiaddr:"
    echo "  /ip4/$BOOTNODE_IP/tcp/30333/p2p/$PEER_ID"
    echo "  /dns/bootnode.testnet.x3-chain.io/tcp/30333/p2p/$PEER_ID"
    echo ""
    echo "🛠 Updating local chain spec with the live bootnode multiaddr..."
    BOOTNODE_MULTIADDR="/ip4/$BOOTNODE_IP/tcp/30333/p2p/$PEER_ID"
    update_chain_spec_bootnodes "$CHAIN_SPEC" "$BOOTNODE_MULTIADDR"
    scp "$CHAIN_SPEC" "$BOOTNODE_USER@$BOOTNODE_IP:/tmp/x3-testnet-raw.json"
    ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo mv /tmp/x3-testnet-raw.json /etc/x3/'
    echo "✅ Chain spec updated: $BOOTNODE_MULTIADDR"
    echo "$PEER_ID" > "$DEPLOYMENT_DIR/bootnode-peer-id.txt"
else
    echo "⚠️  Could not extract peer ID. Check logs manually."
    exit 1
fi

# Step 2: Deploy validators
echo ""
echo "═══════════════════════════════════════════════════"
echo "Step 2/3: Deploy Validators"
echo "═══════════════════════════════════════════════════"
echo ""

read -p "Number of validators to deploy [3]: " NUM_VALIDATORS
NUM_VALIDATORS=${NUM_VALIDATORS:-3}

for i in $(seq 1 $NUM_VALIDATORS); do
    echo ""
    echo "───────────────────────────────────────────────────"
    echo "Deploying Validator $i"
    echo "───────────────────────────────────────────────────"
    
    read -p "Validator $i IP address: " VALIDATOR_IP
    read -p "Validator $i SSH user [x3]: " VALIDATOR_USER
    VALIDATOR_USER=${VALIDATOR_USER:-x3}
    
    echo ""
    echo "Deploying to validator-0$i ($VALIDATOR_USER@$VALIDATOR_IP)..."
    
    # Create validator setup script
    cat > "$DEPLOYMENT_DIR/setup-validator.sh" << 'VALIDATOR_SCRIPT'
#!/bin/bash
set -e

# Create user and directories
sudo useradd -m -s /bin/bash x3 || true
sudo mkdir -p /var/lib/x3/data
sudo chown -R x3:x3 /var/lib/x3
sudo mkdir -p /etc/x3
sudo chown x3:x3 /etc/x3

# Install systemd service
sudo tee /etc/systemd/system/x3-validator.service > /dev/null << 'EOF'
[Unit]
Description=X3 Chain Validator
After=network.target

[Service]
Type=simple
User=x3
WorkingDirectory=/var/lib/x3
ExecStart=/usr/local/bin/x3-chain-node \
    --base-path /var/lib/x3/data \
    --chain /etc/x3/x3-testnet-raw.json \
    --validator \
    --name "VALIDATOR_NAME" \
    --port 30333 \
    --rpc-port 9944 \
    --rpc-cors all \
    --prometheus-external \
    --prometheus-port 9615 \
    --bootnodes BOOTNODE_MULTIADDR
Restart=always
RestartSec=10
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

echo "✅ Validator systemd service created"
VALIDATOR_SCRIPT
    
    # Update service with validator name and bootnode
    VALIDATOR_NAME="X3-Validator-$(printf %02d $i)"
    BOOTNODE_MULTIADDR="/ip4/$BOOTNODE_IP/tcp/30333/p2p/$PEER_ID"
    
    # Deploy files
    echo "📦 Copying files to validator-0$i..."
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'bash -s' < "$DEPLOYMENT_DIR/setup-validator.sh"
    scp "$BINARY" "$VALIDATOR_USER@$VALIDATOR_IP:/tmp/x3-chain-node"
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'sudo mv /tmp/x3-chain-node /usr/local/bin/ && sudo chmod +x /usr/local/bin/x3-chain-node'
    scp "$CHAIN_SPEC" "$VALIDATOR_USER@$VALIDATOR_IP:/tmp/x3-testnet-raw.json"
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'sudo mv /tmp/x3-testnet-raw.json /etc/x3/'
    
    # Update systemd service with actual values
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" "sudo sed -i 's/VALIDATOR_NAME/$VALIDATOR_NAME/g' /etc/systemd/system/x3-validator.service"
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" "sudo sed -i 's|BOOTNODE_MULTIADDR|$BOOTNODE_MULTIADDR|g' /etc/systemd/system/x3-validator.service"
    
    # Start validator
    echo "🚀 Starting validator-0$i..."
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'sudo systemctl daemon-reload && sudo systemctl enable x3-validator && sudo systemctl start x3-validator'
    
    sleep 3
    
    # Check status
    echo ""
    echo "📊 Validator-0$i status:"
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'sudo systemctl status x3-validator --no-pager -l | head -20'
    
    echo ""
    echo "⏳ Waiting 10 seconds before inserting keys..."
    sleep 10
    
    # Insert authority keys
    echo ""
    echo "🔑 Inserting authority keys for validator-0$i..."
    
    # Read keys from summary file
    AURA_SEED=$(grep "Secret Seed:" "$KEYS_DIR/validator-0$i-summary.txt" | head -1 | awk '{print $3}')
    AURA_PUBKEY=$(grep "Public Key:" "$KEYS_DIR/validator-0$i-summary.txt" | head -1 | awk '{print $3}')
    GRANDPA_SEED=$(grep "Secret Seed:" "$KEYS_DIR/validator-0$i-summary.txt" | tail -1 | awk '{print $3}')
    GRANDPA_PUBKEY=$(grep "Public Key:" "$KEYS_DIR/validator-0$i-summary.txt" | tail -1 | awk '{print $3}')
    
    # Insert Aura key
    echo "  Inserting Aura key..."
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" "curl -s http://localhost:9944 -H 'Content-Type: application/json' -d '{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"author_insertKey\",\"params\":[\"aura\",\"$AURA_SEED\",\"$AURA_PUBKEY\"]}'" > /dev/null
    
    # Insert GRANDPA key
    echo "  Inserting GRANDPA key..."
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" "curl -s http://localhost:9944 -H 'Content-Type: application/json' -d '{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"author_insertKey\",\"params\":[\"gran\",\"$GRANDPA_SEED\",\"$GRANDPA_PUBKEY\"]}'" > /dev/null
    
    echo "✅ Keys inserted for validator-0$i"
    
    # Verify keys loaded
    echo ""
    echo "🔍 Checking if keys loaded (from logs)..."
    sleep 3
    ssh "$VALIDATOR_USER@$VALIDATOR_IP" 'sudo journalctl -u x3-validator -n 50 --no-pager | grep -i "key"' || true
    
    echo ""
    echo "✅ Validator-0$i deployment complete!"
    echo ""
done

# Step 3: Verify network
echo ""
echo "═══════════════════════════════════════════════════"
echo "Step 3/3: Verify Network"
echo "═══════════════════════════════════════════════════"
echo ""

echo "⏳ Waiting 30 seconds for network to initialize..."
sleep 30

echo ""
echo "🔍 Checking bootnode peer count..."
ssh "$BOOTNODE_USER@$BOOTNODE_IP" 'sudo journalctl -u x3-bootnode -n 20 --no-pager | grep -i "peer"' || echo "No peer info in logs yet"

echo ""
echo "🔍 Checking validator-01 for block production..."
VALIDATOR_1_IP=$(echo "$VALIDATOR_IP" | head -1)  # Use first validator IP entered
ssh "$VALIDATOR_USER@$VALIDATOR_1_IP" 'sudo journalctl -u x3-validator -n 30 --no-pager | grep -E "(Imported|Finalized)"' || echo "No blocks produced yet"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "✅ Day 1 Node Deployment Complete!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "📊 Deployed:"
echo "  • 1 bootnode at $BOOTNODE_IP"
echo "  • $NUM_VALIDATORS validators"
echo ""
echo "🔍 Next steps:"
echo ""
echo "1. Monitor logs for block production:"
echo "   ssh $VALIDATOR_USER@VALIDATOR_IP 'sudo journalctl -u x3-validator -f'"
echo ""
echo "2. Check network health:"
echo "   curl http://VALIDATOR_IP:9944 -H 'Content-Type: application/json' \\"
echo "     -d '{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"system_health\",\"params\":[]}'"
echo ""
echo "3. Proceed to Day 2: Deploy RPC nodes + faucet"
echo ""
echo "════════════════════════════════════════════════════════════════"
