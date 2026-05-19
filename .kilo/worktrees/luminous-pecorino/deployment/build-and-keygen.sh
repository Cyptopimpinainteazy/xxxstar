#!/bin/bash
# X3 Chain Testnet v1 - Build and Key Generation
# Day -1: Build binary, generate chain spec, create keys

set -e

ensure_no_loopback_bootnodes() {
    local spec_path="$1"
    if grep -Eq '"(/ip4/(127\.0\.0\.1|0\.0\.0\.0)|localhost)' "$spec_path"; then
        echo "❌ Refusing to keep loopback bootnodes in $spec_path"
        echo "   Update bootNodes to public multiaddrs or leave them empty until the bootnode exists."
        exit 1
    fi
}

echo "🔨 X3 Chain Testnet v1 - Build & Key Generation"
echo "===================================================="
echo ""

# Configuration
PROJECT_ROOT="$(pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
KEYS_DIR="$PROJECT_ROOT/deployment/keys"
CHAIN_SPEC_DIR="$PROJECT_ROOT/deployment/chain-specs"

mkdir -p "$KEYS_DIR"
mkdir -p "$CHAIN_SPEC_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Build release binary
echo "📦 Step 1/4: Building release binary..."
echo "  This will take 10-30 minutes depending on your hardware..."
echo ""

START_TIME=$(date +%s)
SKIP_WASM_BUILD=1 cargo build --release

END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))
BUILD_MINUTES=$((BUILD_TIME / 60))
BUILD_SECONDS=$((BUILD_TIME % 60))

if [ -f "$BUILD_DIR/x3-chain-node" ]; then
    echo -e "${GREEN}✅ Build successful in ${BUILD_MINUTES}m ${BUILD_SECONDS}s${NC}"
    echo "   Binary: $BUILD_DIR/x3-chain-node"
    echo "   Size: $(du -h $BUILD_DIR/x3-chain-node | cut -f1)"
else
    echo "❌ Build failed! Binary not found at $BUILD_DIR/x3-chain-node"
    exit 1
fi

# Test binary
echo ""
echo "🧪 Testing binary..."
$BUILD_DIR/x3-chain-node --version
echo -e "${GREEN}✅ Binary works!${NC}"

# Step 2: Generate chain specifications
echo ""
echo "📋 Step 2/4: Generating chain specifications..."

# Development chain spec (for reference)
echo "  Generating dev chain spec..."
$BUILD_DIR/x3-chain-node build-spec --disable-default-bootnode --chain dev \
    > "$CHAIN_SPEC_DIR/x3-dev-plain.json"
echo -e "${GREEN}✅ Created: x3-dev-plain.json${NC}"

# Local testnet chain spec (base for customization)
echo "  Generating local testnet chain spec..."
$BUILD_DIR/x3-chain-node build-spec --disable-default-bootnode --chain local \
    > "$CHAIN_SPEC_DIR/x3-testnet-plain.json"
echo -e "${GREEN}✅ Created: x3-testnet-plain.json${NC}"

# Staging chain spec (more production-like)
echo "  Generating staging chain spec..."
$BUILD_DIR/x3-chain-node build-spec --disable-default-bootnode --chain staging \
    > "$CHAIN_SPEC_DIR/x3-staging-plain.json"
echo -e "${GREEN}✅ Created: x3-staging-plain.json${NC}"

echo ""
echo -e "${YELLOW}⚠️  IMPORTANT: Edit x3-testnet-plain.json before converting to raw format${NC}"
echo "   Things to customize:"
echo "   • name: \"X3 Chain Testnet\""
echo "   • id: \"x3-testnet\""
echo "   • chainType: \"Live\""
echo "   • bootNodes: Leave empty now or set real public bootnode multiaddrs only"
echo "   • Add validator initial authorities (after generating keys below)"
echo ""
read -p "Press Enter after editing the chain spec, or Ctrl+C to exit and edit later..."

ensure_no_loopback_bootnodes "$CHAIN_SPEC_DIR/x3-testnet-plain.json"

# Convert to raw format (after user edits)
echo ""
echo "  Converting to raw format..."
$BUILD_DIR/x3-chain-node build-spec \
    --chain "$CHAIN_SPEC_DIR/x3-testnet-plain.json" \
    --raw \
    > "$CHAIN_SPEC_DIR/x3-testnet-raw.json"
ensure_no_loopback_bootnodes "$CHAIN_SPEC_DIR/x3-testnet-raw.json"
echo -e "${GREEN}✅ Created: x3-testnet-raw.json (use this for deployment)${NC}"

# Step 3: Generate validator keys
echo ""
echo "🔑 Step 3/4: Generating validator authority keys..."
echo ""

# Check if subkey is available
if ! command -v subkey &> /dev/null; then
    echo "Installing subkey..."
    cargo install --force --git https://github.com/paritytech/substrate subkey
fi

NUM_VALIDATORS=${NUM_VALIDATORS:-3}
echo "Generating keys for $NUM_VALIDATORS validators..."
echo ""

for i in $(seq 1 $NUM_VALIDATORS); do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Validator $i keys:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Generate Aura key (Sr25519)
    echo ""
    echo "🔐 Aura (Block Production) Key - Sr25519:"
    AURA_OUTPUT=$(subkey generate --scheme Sr25519)
    echo "$AURA_OUTPUT" | tee "$KEYS_DIR/validator-0$i-aura.txt"
    
    AURA_SEED=$(echo "$AURA_OUTPUT" | grep "Secret seed" | awk '{print $3}')
    AURA_PUBKEY=$(echo "$AURA_OUTPUT" | grep "Public key" | awk '{print $4}')
    AURA_SS58=$(echo "$AURA_OUTPUT" | grep "SS58 Address" | awk '{print $3}')
    
    # Generate GRANDPA key (Ed25519)
    echo ""
    echo "🔐 GRANDPA (Finality) Key - Ed25519:"
    GRANDPA_OUTPUT=$(subkey generate --scheme Ed25519)
    echo "$GRANDPA_OUTPUT" | tee "$KEYS_DIR/validator-0$i-grandpa.txt"
    
    GRANDPA_SEED=$(echo "$GRANDPA_OUTPUT" | grep "Secret seed" | awk '{print $3}')
    GRANDPA_PUBKEY=$(echo "$GRANDPA_OUTPUT" | grep "Public key" | awk '{print $4}')
    GRANDPA_SS58=$(echo "$GRANDPA_OUTPUT" | grep "SS58 Address" | awk '{print $3}')
    
    # Create summary file
    cat > "$KEYS_DIR/validator-0$i-summary.txt" << EOF
Validator $i Key Summary
========================

AURA (Block Production) - Sr25519
---------------------------------
Secret Seed:  $AURA_SEED
Public Key:   $AURA_PUBKEY
SS58 Address: $AURA_SS58

GRANDPA (Finality) - Ed25519
----------------------------
Secret Seed:  $GRANDPA_SEED
Public Key:   $GRANDPA_PUBKEY
SS58 Address: $GRANDPA_SS58

Key Insertion Commands (run on validator node):
-----------------------------------------------
curl http://localhost:9944 -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "author_insertKey",
    "params": [
      "aura",
      "$AURA_SEED",
      "$AURA_PUBKEY"
    ]
  }'

curl http://localhost:9944 -H "Content-Type: application/json" \\
  -d '{
    "id": 1,
    "jsonrpc": "2.0",
    "method": "author_insertKey",
    "params": [
      "gran",
      "$GRANDPA_SEED",
      "$GRANDPA_PUBKEY"
    ]
  }'

EOF
    
    echo ""
    echo -e "${GREEN}✅ Saved summary: $KEYS_DIR/validator-0$i-summary.txt${NC}"
    echo ""
done

# Step 4: Generate bootnode key
echo ""
echo "🔑 Step 4/4: Generating bootnode network key..."

# Generate a random node key
BOOTNODE_KEY=$($BUILD_DIR/x3-chain-node key generate-node-key 2>&1)
echo "$BOOTNODE_KEY" > "$KEYS_DIR/bootnode-key.txt"
echo -e "${GREEN}✅ Bootnode key: $BOOTNODE_KEY${NC}"

# Derive peer ID from node key
PEER_ID=$($BUILD_DIR/x3-chain-node key inspect-node-key --file <(echo -n "$BOOTNODE_KEY") 2>&1)
echo -e "${GREEN}✅ Bootnode peer ID: $PEER_ID${NC}"

# Create bootnode multiaddr template
BOOTNODE_IP="x.x.x.x"  # Will be replaced with actual IP
BOOTNODE_MULTIADDR="/ip4/$BOOTNODE_IP/tcp/30333/p2p/$PEER_ID"

cat > "$KEYS_DIR/bootnode-info.txt" << EOF
Bootnode Configuration
=====================

Node Key: $BOOTNODE_KEY

Peer ID: $PEER_ID

Multiaddr (update x.x.x.x with actual IP):
$BOOTNODE_MULTIADDR

DNS-based Multiaddr (after DNS setup):
/dns/bootnode.testnet.x3-chain.io/tcp/30333/p2p/$PEER_ID

Deployment:
-----------
1. Copy node key to bootnode server:
   echo "$BOOTNODE_KEY" | ssh x3@BOOTNODE_IP 'cat > /var/lib/x3/node-key'

2. Update chain spec bootNodes with the multiaddr above (never localhost/127.0.0.1)

3. Start bootnode with:
   x3-chain-node \\
     --base-path /var/lib/x3/data \\
     --chain /etc/x3/x3-testnet-raw.json \\
     --name "X3 Bootnode" \\
     --node-key-file /var/lib/x3/node-key \\
     --port 30333
EOF

echo ""
echo -e "${GREEN}✅ Saved: $KEYS_DIR/bootnode-info.txt${NC}"

# Generate sudo key (for development, will be removed later)
echo ""
echo "🔑 Generating sudo (development) key..."
SUDO_OUTPUT=$(subkey generate --scheme Sr25519)
echo "$SUDO_OUTPUT" | tee "$KEYS_DIR/sudo-key.txt"

SUDO_SS58=$(echo "$SUDO_OUTPUT" | grep "SS58 Address" | awk '{print $3}')
echo ""
echo -e "${YELLOW}⚠️  Sudo account (for development): $SUDO_SS58${NC}"
echo "   Add this to chain spec 'sudo' field"
echo ""

# Create deployment manifest
cat > "$KEYS_DIR/KEYS_MANIFEST.md" << 'EOF'
# X3 Chain Testnet - Keys Manifest

## ⚠️ SECURITY WARNING

**These files contain SECRET KEYS. NEVER commit to git or share publicly!**

This directory is `.gitignored` to prevent accidental commits.

## Key Files

### Validator Keys
Each validator has:
- `validator-0X-aura.txt` - Aura (block production) key (Sr25519)
- `validator-0X-grandpa.txt` - GRANDPA (finality) key (Ed25519)
- `validator-0X-summary.txt` - Combined summary with insertion commands

### Bootnode Keys
- `bootnode-key.txt` - Bootnode network key
- `bootnode-info.txt` - Bootnode configuration and multiaddr

### Sudo Key (Development)
- `sudo-key.txt` - Sudo account for development (remove before mainnet)

## Key Distribution

### Secure Distribution Methods

**Option 1: Encrypted Files (Recommended)**
```bash
# Encrypt for recipient
gpg --encrypt --recipient validator@email.com validator-01-summary.txt

# Recipient decrypts
gpg --decrypt validator-01-summary.txt.gpg > validator-01-summary.txt
```

**Option 2: Password-Protected Archive**
```bash
# Create encrypted archive
zip -e validator-01-keys.zip validator-01-*.txt
# Enter strong password when prompted

# Distribute via secure channel (Signal, encrypted email)
# Share password via different channel (voice call, SMS)
```

**Option 3: Hardware Security Module (HSM)**
For production, store keys in HSM or hardware wallet.

## Key Insertion on Validators

After validators are running, insert keys via RPC:

```bash
# Example for validator 1
ssh x3@validator-01 << 'ENDSSH'
# Insert Aura key
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["aura","SEED","PUBKEY"]}'

# Insert GRANDPA key  
curl http://localhost:9944 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"author_insertKey","params":["gran","SEED","PUBKEY"]}'
ENDSSH
```

See individual `validator-0X-summary.txt` files for exact commands.

## Key Backup

**CRITICAL: Backup these keys immediately!**

```bash
# Create encrypted backup
tar czf - deployment/keys | gpg --encrypt --recipient admin@x3-chain.io \
  > x3-testnet-keys-backup-$(date +%Y%m%d).tar.gz.gpg

# Store in multiple secure locations:
# 1. Encrypted cloud storage (Dropbox, Google Drive)
# 2. Hardware encrypted USB drive (offline)
# 3. Password manager vault (1Password, Bitwarden)
```

## Key Rotation (Future)

For mainnet, implement key rotation:
1. Generate new keys
2. Submit session key change transaction
3. Wait for era/epoch change
4. Verify new keys active
5. Securely destroy old keys
EOF

echo -e "${GREEN}✅ Created: $KEYS_DIR/KEYS_MANIFEST.md${NC}"

# Add .gitignore to keys directory
cat > "$KEYS_DIR/.gitignore" << 'EOF'
# Ignore all secret keys
*.txt
!KEYS_MANIFEST.md
!.gitignore

# Ignore GPG encrypted files
*.gpg

# Ignore archives
*.zip
*.tar.gz
EOF

echo -e "${GREEN}✅ Created: $KEYS_DIR/.gitignore (keys directory protected)${NC}"

# Summary
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "✅ Day -1 Build & Key Generation Complete!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "📦 Binary:"
echo "  • $BUILD_DIR/x3-chain-node"
echo "  • Version: $($BUILD_DIR/x3-chain-node --version)"
echo ""
echo "📋 Chain Specs:"
echo "  • $CHAIN_SPEC_DIR/x3-testnet-raw.json (for deployment)"
echo "  • $CHAIN_SPEC_DIR/x3-testnet-plain.json (human-readable)"
echo ""
echo "🔑 Keys Generated ($NUM_VALIDATORS validators):"
for i in $(seq 1 $NUM_VALIDATORS); do
    echo "  • Validator $i: $KEYS_DIR/validator-0$i-summary.txt"
done
echo "  • Bootnode: $KEYS_DIR/bootnode-info.txt"
echo "  • Sudo: $KEYS_DIR/sudo-key.txt"
echo ""
echo -e "${YELLOW}⚠️  CRITICAL SECURITY:${NC}"
echo "  1. Backup keys immediately (encrypted)"
echo "  2. Store in multiple secure locations"
echo "  3. NEVER commit keys to git"
echo "  4. Distribute to validators via encrypted channel"
echo ""
echo "🚀 Next steps:"
echo ""
echo "1. Backup keys:"
echo "   tar czf - deployment/keys | gpg -e -r admin@x3-chain.io \\"
echo "     > x3-testnet-keys-\$(date +%Y%m%d).tar.gz.gpg"
echo ""
echo "2. Copy binary to deployment server:"
echo "   scp $BUILD_DIR/x3-chain-node admin@deploy-server:/opt/x3/"
echo ""
echo "3. Copy chain spec to deployment server:"
echo "   scp $CHAIN_SPEC_DIR/x3-testnet-raw.json admin@deploy-server:/opt/x3/"
echo ""
echo "4. Distribute keys to validators (encrypted!):"
echo "   • See $KEYS_DIR/KEYS_MANIFEST.md for secure methods"
echo ""
echo "5. Proceed to Day 1: Deploy bootnode + validators"
echo ""
echo "════════════════════════════════════════════════════════════════"
