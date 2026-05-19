#!/bin/bash
# Compiler ECDSA Key Generation Script
# Generates secp256k1 keypair for manifest signing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEY_PATH="${SCRIPT_DIR}/key.pem"
PUB_PATH="${SCRIPT_DIR}/key.pub"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Botchain Compiler Key Generation ===${NC}"

# Check if keys already exist
if [ -f "$KEY_PATH" ]; then
    echo -e "${YELLOW}Warning: Key already exists at ${KEY_PATH}${NC}"
    read -p "Overwrite existing key? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${GREEN}Keeping existing key.${NC}"
        exit 0
    fi
    # Backup existing key
    BACKUP_PATH="${KEY_PATH}.backup.$(date +%s)"
    cp "$KEY_PATH" "$BACKUP_PATH"
    echo -e "${YELLOW}Backed up existing key to ${BACKUP_PATH}${NC}"
fi

# Check for OpenSSL
if ! command -v openssl &> /dev/null; then
    echo -e "${RED}Error: OpenSSL is required but not installed.${NC}"
    echo "Install with: apt-get install openssl (Debian/Ubuntu)"
    echo "          or: brew install openssl (macOS)"
    exit 1
fi

# Generate secp256k1 private key
echo "Generating secp256k1 private key..."
openssl ecparam -name secp256k1 -genkey -noout -out "$KEY_PATH"

# Extract public key
echo "Extracting public key..."
openssl ec -in "$KEY_PATH" -pubout -out "$PUB_PATH" 2>/dev/null

# Secure the private key
chmod 600 "$KEY_PATH"
chmod 644 "$PUB_PATH"

# Display key info
echo ""
echo -e "${GREEN}=== Key Generation Complete ===${NC}"
echo -e "Private key: ${KEY_PATH}"
echo -e "Public key:  ${PUB_PATH}"
echo ""
echo "Public key fingerprint:"
openssl ec -in "$KEY_PATH" -pubout -outform DER 2>/dev/null | sha256sum | cut -d' ' -f1
echo ""
echo -e "${YELLOW}IMPORTANT: Keep key.pem secure and never commit to version control!${NC}"
echo -e "${YELLOW}Add 'compiler/key.pem' to your .gitignore${NC}"
