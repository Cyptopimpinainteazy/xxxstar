#!/usr/bin/env bash

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

WORKSPACE_ROOT="/home/lojak/Desktop/x3-chain"
KEYS_DIR="$WORKSPACE_ROOT/deployment/keys"

echo "🔑 X3 Chain Testnet v1 - Key Generation"
echo "============================================"
echo ""

# Ensure PATH includes cargo bin
export PATH="$HOME/.cargo/bin:$PATH"

# Check if subkey is installed
if ! command -v subkey &> /dev/null; then
    echo -e "${RED}❌ subkey not found in PATH${NC}"
    echo "Please ensure subkey is installed: cargo install --force --locked subkey --git https://github.com/paritytech/polkadot-sdk"
    exit 1
fi

echo "✅ Found subkey: $(which subkey)"
echo "   Version: $(subkey --version)"
echo ""

# Create keys directory
mkdir -p "$KEYS_DIR"

# Create .gitignore for keys directory
cat > "$KEYS_DIR/.gitignore" << 'EOF'
# CRITICAL: Never commit these keys to git!
# These are cryptographic keys that control your testnet validators
*
!.gitignore
!KEYS_MANIFEST.md
EOF

echo "📂 Keys will be stored in: $KEYS_DIR"
echo ""

# Ask how many validators
echo -e "${YELLOW}How many validators do you want to deploy? (recommended: 3-5)${NC}"
read -p "Number of validators [3]: " NUM_VALIDATORS
NUM_VALIDATORS=${NUM_VALIDATORS:-3}

echo ""
echo "🔑 Step 1/2: Generating keys for $NUM_VALIDATORS validators..."
echo ""

# Generate validator keys
for i in $(seq 1 $NUM_VALIDATORS); do
    VALIDATOR_NUM=$(printf "%02d" $i)
    SUMMARY_FILE="$KEYS_DIR/validator-$VALIDATOR_NUM-summary.txt"
    
    echo "Generating keys for Validator $VALIDATOR_NUM..."
    
    # Generate Aura key (Sr25519 - for block production)
    echo "  Generating Aura key (Sr25519)..."
    AURA_OUTPUT=$(subkey generate --scheme Sr25519 --output-type json)
    AURA_SS58=$(echo "$AURA_OUTPUT" | grep -oP '"ss58Address":\s*"\K[^"]+')
    AURA_PUBLIC=$(echo "$AURA_OUTPUT" | grep -oP '"publicKey":\s*"\K[^"]+')
    AURA_SECRET=$(echo "$AURA_OUTPUT" | grep -oP '"secretSeed":\s*"\K[^"]+')
    AURA_PHRASE=$(echo "$AURA_OUTPUT" | grep -oP '"secretPhrase":\s*"\K[^"]+')
    
    # Generate GRANDPA key (Ed25519 - for finality)
    echo "  Generating GRANDPA key (Ed25519)..."
    GRANDPA_OUTPUT=$(subkey generate --scheme Ed25519 --output-type json)
    GRANDPA_SS58=$(echo "$GRANDPA_OUTPUT" | grep -oP '"ss58Address":\s*"\K[^"]+')
    GRANDPA_PUBLIC=$(echo "$GRANDPA_OUTPUT" | grep -oP '"publicKey":\s*"\K[^"]+')
    GRANDPA_SECRET=$(echo "$GRANDPA_OUTPUT" | grep -oP '"secretSeed":\s*"\K[^"]+')
    GRANDPA_PHRASE=$(echo "$GRANDPA_OUTPUT" | grep -oP '"secretPhrase":\s*"\K[^"]+')
    
    # Save to summary file
    cat > "$SUMMARY_FILE" << EOF
# Validator $VALIDATOR_NUM - Authority Keys
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

## AURA KEY (Block Production - Sr25519)
SS58 Address:     $AURA_SS58
Public Key (Hex): $AURA_PUBLIC
Secret Seed:      $AURA_SECRET
Secret Phrase:    $AURA_PHRASE

## GRANDPA KEY (Finality - Ed25519)
SS58 Address:     $GRANDPA_SS58
Public Key (Hex): $GRANDPA_PUBLIC
Secret Seed:      $GRANDPA_SECRET
Secret Phrase:    $GRANDPA_PHRASE

## RPC KEY INSERTION COMMANDS (Run on validator-$VALIDATOR_NUM)

# Insert Aura key (block production)
curl -H "Content-Type: application/json" \\
  --data '{"jsonrpc":"2.0","method":"author_insertKey","params":["aura","$AURA_PHRASE","$AURA_PUBLIC"],"id":1}' \\
  http://localhost:9944

# Insert GRANDPA key (finality)
curl -H "Content-Type: application/json" \\
  --data '{"jsonrpc":"2.0","method":"author_insertKey","params":["gran","$GRANDPA_PHRASE","$GRANDPA_PUBLIC"],"id":1}' \\
  http://localhost:9944

## CHAIN SPEC ENTRY (Add to "initialAuthorities" in chain spec)
[
  "$AURA_SS58",     // Aura (Sr25519)
  "$GRANDPA_SS58"   // GRANDPA (Ed25519)
]

⚠️  SECURITY WARNING:
• NEVER share these keys publicly
• NEVER commit to git (already .gitignored)
• Store securely in password manager or vault
• Backup encrypted to multiple locations
EOF
    
    echo -e "  ${GREEN}✅ Validator $VALIDATOR_NUM keys saved to: $SUMMARY_FILE${NC}"
    echo ""
done

echo ""
echo "🔑 Step 2/2: Generating bootnode network key..."
echo ""

# Generate bootnode key using x3-chain-node
cd "$WORKSPACE_ROOT"
BOOTNODE_KEY_FILE="$KEYS_DIR/bootnode-node-key"
BOOTNODE_INFO_FILE="$KEYS_DIR/bootnode-info.txt"

# Generate the network key
./target/release/x3-chain-node key generate-node-key --file "$BOOTNODE_KEY_FILE"

# Read the generated key
BOOTNODE_KEY=$(cat "$BOOTNODE_KEY_FILE")

# Derive the peer ID
PEER_ID_OUTPUT=$(./target/release/x3-chain-node key inspect-node-key --file "$BOOTNODE_KEY_FILE")
PEER_ID=$(echo "$PEER_ID_OUTPUT" | grep -oP '\w{52}')

echo "  Generated bootnode network key"
echo "  Peer ID: $PEER_ID"

# Save bootnode info
cat > "$BOOTNODE_INFO_FILE" << EOF
# Bootnode Network Configuration
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")

## Network Key (Ed25519)
$BOOTNODE_KEY

## Peer ID
$PEER_ID

## Multiaddress Format
/ip4/<BOOTNODE_IP>/tcp/30333/p2p/$PEER_ID

## Example (replace <BOOTNODE_IP> with actual IP)
/ip4/10.0.1.100/tcp/30333/p2p/$PEER_ID
/dns4/bootnode.testnet.x3-chain.io/tcp/30333/p2p/$PEER_ID

## Systemd Service Configuration
Place the node key file at: /var/lib/x3/node-key
Set permissions: chmod 600 /var/lib/x3/node-key

## Chain Spec Entry (Add to "bootNodes")
"/ip4/<BOOTNODE_IP>/tcp/30333/p2p/$PEER_ID"
EOF

echo -e "${GREEN}✅ Bootnode info saved to: $BOOTNODE_INFO_FILE${NC}"
echo ""

# Generate keys manifest
MANIFEST_FILE="$KEYS_DIR/KEYS_MANIFEST.md"
cat > "$MANIFEST_FILE" << EOF
# X3 Chain Testnet v1 - Keys Manifest

**Generated**: $(date -u +"%Y-%m-%d %H:%M:%S UTC")  
**Validators**: $NUM_VALIDATORS  
**Status**: ⚠️ HIGHLY SENSITIVE - SECURE IMMEDIATELY

---

## 📁 Generated Keys

### Validator Authority Keys
EOF

for i in $(seq 1 $NUM_VALIDATORS); do
    VALIDATOR_NUM=$(printf "%02d" $i)
    echo "- \`validator-$VALIDATOR_NUM-summary.txt\` - Validator $VALIDATOR_NUM (Aura + GRANDPA)" >> "$MANIFEST_FILE"
done

cat >> "$MANIFEST_FILE" << 'EOF'

### Network Keys
- `bootnode-node-key` - Bootnode network identity (Ed25519)
- `bootnode-info.txt` - Bootnode peer ID and multiaddresses

---

## 🔐 Security Best Practices

### Immediate Actions Required

1. **Backup Keys (Encrypted)**
   ```bash
   # GPG encrypted backup
   tar czf - deployment/keys | gpg -e -r your@email.com \
     > x3-testnet-keys-$(date +%Y%m%d).tar.gz.gpg
   
   # Password-protected zip (requires zip package)
   zip -r -e x3-testnet-keys-$(date +%Y%m%d).zip deployment/keys/
   ```

2. **Store Backups in 3 Locations**
   - [ ] Cloud storage (encrypted)
   - [ ] USB drive (encrypted)
   - [ ] Password manager or secure vault

3. **Distribute to Validators Securely**
   - Use encrypted communication channels
   - Send each validator ONLY their own keys
   - Never send keys via plain email or chat

### Access Control

- **Who needs access**: Validator operators only
- **Who should NEVER have access**: Public, developers, support staff
- **Rotation schedule**: If compromised, generate new keys immediately

### Verification

After deployment, verify keys are loaded:
```bash
# On each validator node
curl -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"author_hasKey","params":["<AURA_PUBLIC_KEY>","aura"],"id":1}' \
  http://localhost:9944
```

Expected response: `{"jsonrpc":"2.0","result":true,"id":1}`

---

## ⚠️ Incident Response

If keys are compromised:
1. **Immediately stop all validators**
2. **Generate new keys** (run this script again)
3. **Update chain spec** with new authorities
4. **Redeploy validators** with new keys
5. **Notify team** via secure channel

---

## 📞 Emergency Contacts

- **Security Issues**: security@x3-chain.io
- **Deployment Support**: devops@x3-chain.io
- **On-Call**: [Add phone/pager]

---

**REMEMBER**: These keys control your entire testnet. Treat them like production private keys.
EOF

echo -e "${GREEN}✅ Keys manifest saved to: $MANIFEST_FILE${NC}"
echo ""

# Summary
echo "═══════════════════════════════════════════════════════"
echo -e "${GREEN}✅ Key generation complete!${NC}"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "Generated keys for:"
echo "  • $NUM_VALIDATORS validators (Aura + GRANDPA)"
echo "  • 1 bootnode (network identity)"
echo ""
echo "📂 Keys location: $KEYS_DIR"
echo ""
echo -e "${RED}⚠️  CRITICAL - BACKUP IMMEDIATELY:${NC}"
echo ""
echo "  # GPG encrypted backup (recommended)"
echo "  tar czf - deployment/keys | gpg -e -r your@email.com \\"
echo "    > x3-testnet-keys-\$(date +%Y%m%d).tar.gz.gpg"
echo ""
echo -e "${YELLOW}📋 Next steps:${NC}"
echo ""
echo "1. BACKUP the keys (see command above)"
echo "2. Update chain spec with validator authorities:"
echo "   • Edit: deployment/chain-specs/x3-testnet-plain.json"
echo "   • Add validator SS58 addresses to 'initialAuthorities'"
echo "   • Regenerate raw spec: ./target/release/x3-chain-node build-spec \\"
echo "       --chain deployment/chain-specs/x3-testnet-plain.json --raw \\"
echo "       > deployment/chain-specs/x3-testnet-raw.json"
echo ""
echo "3. Provision infrastructure (if not done yet):"
echo "   • Run: ./deployment/provision-digitalocean.sh (or AWS/manual guide)"
echo "   • Update: deployment/inventory.yaml with actual IPs"
echo ""
echo "4. Deploy nodes:"
echo "   • Run: ./deployment/deploy-nodes-day1.sh"
echo ""
echo "═══════════════════════════════════════════════════════"
