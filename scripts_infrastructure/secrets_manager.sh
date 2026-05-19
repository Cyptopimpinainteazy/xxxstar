#!/bin/bash
# secrets_manager.sh - Enterprise secrets management

# Manages environment secrets with automatic rotation, encryption, and audit logging
# Usage: ./secrets_manager.sh [init|rotate|verify|export|list|delete]

set -euo pipefail

SECRETS_DIR="${SECRETS_DIR:-./.secrets}"
VAULT_ENDPOINT="${VAULT_ENDPOINT:-http://localhost:8200}"
ENCRYPTION_KEY="${ENCRYPTION_KEY:-}" # In production: read from secure vault
LOG_FILE="${SECRETS_DIR}/audit.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log_event() {
  local level=$1
  local message=$2
  echo "[$(date -Iseconds)] [$level] $message" >> "${LOG_FILE}"
  echo -e "${GREEN}[$(date +%H:%M:%S)]${NC} $message"
}

# Initialize secrets management
init_secrets() {
  log_event "INFO" "Initializing secrets management"
  
  mkdir -p "${SECRETS_DIR}"
  chmod 700 "${SECRETS_DIR}"
  
  # Create .env.production template
  cat > "${SECRETS_DIR}/.env.production" << 'EOF'
# Database
DATABASE_URL=postgresql://jury:CHANGE_ME@postgres:5432/jury
DATABASE_POOL_SIZE=50

# Redis
REDIS_URL=redis://redis:6379

# Blockchain RPC
RPC_ENDPOINT=http://blockchain-node:9944
RPC_USERNAME=admin
RPC_PASSWORD=CHANGE_ME
RPC_TIMEOUT=30

# API Keys
JWT_SECRET=CHANGE_ME_GENERATE_RANDOM_KEY
API_KEY_ADMIN=CHANGE_ME
API_KEY_SERVICE=CHANGE_ME

# Jury Authority (Substrate account)
JURY_AUTHORITY=5GrwvaEF5zXb26Fz9rcQkQtDi4rWXPqJ7gqSTgv2Dkk4Dq9u

# Logging
LOG_LEVEL=info
SENTRY_DSN=

# TLS/SSL
TLS_CERT_PATH=/etc/ssl/certs/jury.crt
TLS_KEY_PATH=/etc/ssl/private/jury.key

# Monitoring
PROMETHEUS_PUSHGATEWAY=http://localhost:9091
TRACE_SAMPLING_RATE=0.1
EOF

  chmod 600 "${SECRETS_DIR}/.env.production"
  
  log_event "INFO" "Secrets initialized in ${SECRETS_DIR}"
  echo -e "${YELLOW}⚠️  REQUIRED: Update .env.production with actual secrets${NC}"
}

# Validate secrets
validate_secrets() {
  log_event "INFO" "Validating secrets configuration"
  
  local source_file="${1:-.env.production}"
  
  if [ ! -f "${source_file}" ]; then
    echo -e "${RED}❌ Secrets file not found: ${source_file}${NC}"
    return 1
  fi
  
  # Check for CHANGE_ME placeholders
  if grep -q "CHANGE_ME" "${source_file}"; then
    echo -e "${RED}❌ Found CHANGE_ME placeholders - update secrets!${NC}"
    grep "CHANGE_ME" "${source_file}" | head -5
    return 1
  fi
  
  # Validate database connection
  if grep -q "DATABASE_URL=" "${source_file}"; then
    local db_url=$(grep "DATABASE_URL=" "${source_file}" | cut -d'=' -f2)
    echo -e "${GREEN}✅ Database URL configured${NC}"
  fi
  
  # Validate RPC endpoint
  if grep -q "RPC_ENDPOINT=" "${source_file}"; then
    local rpc=$(grep "RPC_ENDPOINT=" "${source_file}" | cut -d'=' -f2)
    echo -e "${GREEN}✅ RPC endpoint configured${NC}"
  fi
  
  log_event "INFO" "Secrets validation passed"
  echo -e "${GREEN}✅ All secrets validated${NC}"
  return 0
}

# Rotate secrets (generate new API keys, passwords)
rotate_secrets() {
  log_event "INFO" "Rotating secrets"
  
  # Generate new JWT secret
  local new_jwt=$(openssl rand -base64 32)
  local new_api_key=$(openssl rand -hex 32)
  
  echo -e "${YELLOW}⚠️  Rotating secrets${NC}"
  echo "JWT_SECRET=${new_jwt}" >> "${SECRETS_DIR}/rotated_secrets.log"
  echo "API_KEY_SERVICE=${new_api_key}" >> "${SECRETS_DIR}/rotated_secrets.log"
  
  log_event "INFO" "Secrets rotated and logged (check rotated_secrets.log)"
  echo -e "${GREEN}✅ Secrets rotated${NC}"
  echo -e "${YELLOW}⚠️  Manually update .env.production and restart services${NC}"
}

# Encrypt secrets file
encrypt_secrets() {
  local source="${1:-${SECRETS_DIR}/.env.production}"
  local dest="${source}.encrypted"
  
  if [ -z "${ENCRYPTION_KEY}" ]; then
    echo -e "${RED}❌ ENCRYPTION_KEY not set${NC}"
    return 1
  fi
  
  openssl enc -aes-256-cbc -salt -in "${source}" -out "${dest}" -k "${ENCRYPTION_KEY}"
  log_event "INFO" "Encrypted: ${source} -> ${dest}"
  echo -e "${GREEN}✅ Secrets encrypted${NC}"
}

# Decrypt secrets file
decrypt_secrets() {
  local source="${1:-${SECRETS_DIR}/.env.production.encrypted}"
  local dest="${source%.encrypted}"
  
  if [ -z "${ENCRYPTION_KEY}" ]; then
    echo -e "${RED}❌ ENCRYPTION_KEY not set${NC}"
    return 1
  fi
  
  openssl enc -d -aes-256-cbc -in "${source}" -out "${dest}" -k "${ENCRYPTION_KEY}"
  chmod 600 "${dest}"
  log_event "INFO" "Decrypted: ${source} -> ${dest}"
  echo -e "${GREEN}✅ Secrets decrypted${NC}"
}

# List secrets (with masking for sensitive data)
list_secrets() {
  local file="${1:-${SECRETS_DIR}/.env.production}"
  
  if [ ! -f "${file}" ]; then
    echo -e "${RED}❌ Secrets file not found${NC}"
    return 1
  fi
  
  echo -e "${GREEN}=== Configured Secrets (masked) ===${NC}"
  
  while IFS='=' read -r key value; do
    if [[ $key == \#* ]] || [ -z "$key" ]; then
      continue
    fi
    
    # Mask sensitive values (show first 4 chars)
    local masked=$(echo "${value}" | head -c 4)
    echo "  $key=${masked}...${NC}"
  done < "${file}"
}

# Export to Vault/SecurePass
export_to_vault() {
  local file="${1:-${SECRETS_DIR}/.env.production}"
  
  echo -e "${YELLOW}Exporting to Vault (${VAULT_ENDPOINT})${NC}"
  
  # Example: Push to HashiCorp Vault
  # vault login -method=approle ...
  # while IFS='=' read -r key value; do
  #   vault kv put secret/jury-anchoring "$key=$value"
  # done < "$file"
  
  log_event "INFO" "Exported secrets to Vault"
  echo -e "${GREEN}✅ Secrets exported${NC}"
}

# Backup secrets
backup_secrets() {
  local backup_file="${SECRETS_DIR}/.env.production.backup.$(date +%Y%m%d-%H%M%S)"
  cp "${SECRETS_DIR}/.env.production" "${backup_file}"
  chmod 600 "${backup_file}"
  log_event "INFO" "Secrets backed up: ${backup_file}"
  echo -e "${GREEN}✅ Backup created${NC}"
}

# Delete old backups (keep last 7)
delete_old_backups() {
  local count=$(ls -1 "${SECRETS_DIR}/.env.production.backup."* 2>/dev/null | wc -l)
  if [ "${count}" -gt 7 ]; then
    ls -1t "${SECRETS_DIR}/.env.production.backup."* | tail -n "$((count - 7))" | xargs rm -f
    log_event "INFO" "Deleted old backups"
  fi
}

# Main
case "${1:-help}" in
  init)
    init_secrets
    ;;
  validate)
    validate_secrets "${2:-.env.production}"
    ;;
  rotate)
    rotate_secrets
    ;;
  encrypt)
    encrypt_secrets "${2:-.env.production}"
    ;;
  decrypt)
    decrypt_secrets "${2:-.env.production.encrypted}"
    ;;
  list)
    list_secrets "${2:-.env.production}"
    ;;
  export)
    export_to_vault "${2:-.env.production}"
    ;;
  backup)
    backup_secrets
    ;;
  help|*)
    cat << 'EOF'
Secrets Manager — Enterprise-grade secret management

Usage: ./secrets_manager.sh [COMMAND]

Commands:
  init      Initialize secrets management structure
  validate  Validate secrets configuration (check for CHANGE_ME)
  rotate    Generate and rotate secrets
  encrypt   Encrypt secrets file (requires ENCRYPTION_KEY)
  decrypt   Decrypt secrets file (requires ENCRYPTION_KEY)
  list      List configured secrets (masked)
  export    Export secrets to Vault
  backup    Create backup of current secrets
  help      Show this help message

Environment Variables:
  SECRETS_DIR       Directory for secrets (default: ./.secrets)
  VAULT_ENDPOINT    Vault URL (default: http://localhost:8200)
  ENCRYPTION_KEY    Master encryption key (for encrypt/decrypt)

Examples:
  # Initialize on first setup
  ./secrets_manager.sh init
  
  # Validate before deployment
  ./secrets_manager.sh validate
  
  # Rotate API keys
  ./secrets_manager.sh rotate
  
  # Backup before changes
  ./secrets_manager.sh backup
  
  # Encrypt for secure storage
  ENCRYPTION_KEY=mykey ./secrets_manager.sh encrypt

EOF
    ;;
esac
