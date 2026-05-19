#!/usr/bin/env bash

set -e

KEYS_DIR="deployment/keys"
BACKUP_NAME="x3-testnet-keys-$(date +%Y%m%d-%H%M%S)"

echo "🔐 X3 Chain Testnet - Keys Backup Utility"
echo "=============================================="
echo ""

# Check if keys directory exists
if [ ! -d "$KEYS_DIR" ]; then
    echo "❌ Error: Keys directory not found: $KEYS_DIR"
    exit 1
fi

# Count key files
KEY_COUNT=$(ls -1 "$KEYS_DIR"/*.txt "$KEYS_DIR"/*-key 2>/dev/null | wc -l)
if [ "$KEY_COUNT" -eq 0 ]; then
    echo "❌ Error: No key files found in $KEYS_DIR"
    exit 1
fi

echo "Found $KEY_COUNT key files to backup"
echo ""

# Check for available backup methods
HAS_OPENSSL=$(command -v openssl &> /dev/null && echo "yes" || echo "no")
HAS_GPG=$(command -v gpg &> /dev/null && echo "yes" || echo "no")
HAS_ZIP=$(command -v zip &> /dev/null && echo "yes" || echo "no")

echo "Available backup methods:"
echo "  1. Password-protected tar.gz with OpenSSL (recommended) - Available: $HAS_OPENSSL"
echo "  2. Password-protected zip - Available: $HAS_ZIP"
echo "  3. GPG encrypted (requires GPG key) - Available: $HAS_GPG"
echo "  4. Plain tar.gz (NOT ENCRYPTED - not recommended)"
echo ""

read -p "Choose backup method [1]: " METHOD
METHOD=${METHOD:-1}

case $METHOD in
    1)
        if [ "$HAS_OPENSSL" = "no" ]; then
            echo "❌ OpenSSL not found. Please choose another method."
            exit 1
        fi
        
        echo ""
        echo "Creating password-protected backup with OpenSSL..."
        read -s -p "Enter encryption password: " PASSWORD
        echo ""
        read -s -p "Confirm password: " PASSWORD2
        echo ""
        
        if [ "$PASSWORD" != "$PASSWORD2" ]; then
            echo "❌ Passwords don't match!"
            exit 1
        fi
        
        tar czf - "$KEYS_DIR" | openssl enc -aes-256-cbc -salt -pbkdf2 -pass pass:"$PASSWORD" -out "${BACKUP_NAME}.tar.gz.enc"
        
        echo "✅ Encrypted backup created: ${BACKUP_NAME}.tar.gz.enc"
        echo ""
        echo "To restore:"
        echo "  openssl enc -aes-256-cbc -d -pbkdf2 -pass pass:YOUR_PASSWORD -in ${BACKUP_NAME}.tar.gz.enc | tar xzf -"
        ;;
        
    2)
        if [ "$HAS_ZIP" = "no" ]; then
            echo "❌ zip not found. Installing..."
            sudo apt-get install -y zip || { echo "Failed to install zip"; exit 1; }
        fi
        
        echo ""
        echo "Creating password-protected zip..."
        zip -r -e "${BACKUP_NAME}.zip" "$KEYS_DIR"
        
        echo "✅ Encrypted backup created: ${BACKUP_NAME}.zip"
        echo ""
        echo "To restore:"
        echo "  unzip ${BACKUP_NAME}.zip"
        ;;
        
    3)
        if [ "$HAS_GPG" = "no" ]; then
            echo "❌ GPG not found. Please choose another method."
            exit 1
        fi
        
        echo ""
        echo "Available GPG keys:"
        gpg --list-keys 2>/dev/null || echo "No GPG keys found"
        echo ""
        read -p "Enter GPG key email or ID: " GPG_KEY
        
        if [ -z "$GPG_KEY" ]; then
            echo "❌ No GPG key specified"
            exit 1
        fi
        
        tar czf - "$KEYS_DIR" | gpg -e -r "$GPG_KEY" -o "${BACKUP_NAME}.tar.gz.gpg"
        
        if [ $? -eq 0 ]; then
            echo "✅ GPG encrypted backup created: ${BACKUP_NAME}.tar.gz.gpg"
            echo ""
            echo "To restore:"
            echo "  gpg -d ${BACKUP_NAME}.tar.gz.gpg | tar xzf -"
        else
            echo "❌ GPG encryption failed"
            exit 1
        fi
        ;;
        
    4)
        echo ""
        echo "⚠️  WARNING: Creating UNENCRYPTED backup!"
        echo "This is NOT recommended for production keys."
        read -p "Are you sure? (type 'yes' to continue): " CONFIRM
        
        if [ "$CONFIRM" != "yes" ]; then
            echo "Cancelled."
            exit 0
        fi
        
        tar czf "${BACKUP_NAME}.tar.gz" "$KEYS_DIR"
        
        echo "✅ Backup created: ${BACKUP_NAME}.tar.gz"
        echo "⚠️  This file is NOT ENCRYPTED - secure it immediately!"
        echo ""
        echo "To restore:"
        echo "  tar xzf ${BACKUP_NAME}.tar.gz"
        ;;
        
    *)
        echo "❌ Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "📂 Backup saved to: $(pwd)/${BACKUP_NAME}.*"
echo ""
echo "🔐 IMPORTANT - Secure Your Backup:"
echo "  1. Copy to USB drive and store securely"
echo "  2. Upload to encrypted cloud storage (Google Drive, Dropbox)"
echo "  3. Store password in password manager (LastPass, 1Password, Bitwarden)"
echo "  4. Keep backup in multiple locations"
echo "  5. Test restoration process to ensure backup works"
echo ""
echo "📋 Backup contains:"
ls -lh "$KEYS_DIR"
echo ""
echo "✅ Backup complete!"
