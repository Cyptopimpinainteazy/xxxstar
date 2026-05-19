#!/bin/bash
# enable_tls.sh - Enable TLS/SSL encryption for all Phase 5 services

# Generates self-signed certificates for development
# Uses provided certificates for production
# Usage: ./enable_tls.sh [dev|prod]

set -euo pipefail

ENV="${1:-dev}"
CERT_DIR="./certs"
DOMAIN="${DOMAIN:-localhost}"
DAYS_VALID="${DAYS_VALID:-365}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔐 Enabling TLS/SSL for ${ENV} environment${NC}"

# Create certificate directory
mkdir -p "${CERT_DIR}"
chmod 700 "${CERT_DIR}"

if [ "${ENV}" = "dev" ]; then
  echo -e "${YELLOW}Generating self-signed certificates for ${DOMAIN}...${NC}"
  
  # Generate private key (RSA-2048)
  echo -e "${BLUE}→ Generating RSA-2048 private key...${NC}"
  openssl genrsa -out "${CERT_DIR}/jury.key" 2048 2>/dev/null
  chmod 600 "${CERT_DIR}/jury.key"
  
  # Generate self-signed certificate
  echo -e "${BLUE}→ Generating self-signed certificate...${NC}"
  openssl req -new -x509 \
    -key "${CERT_DIR}/jury.key" \
    -out "${CERT_DIR}/jury.crt" \
    -days "${DAYS_VALID}" \
    -subj "/C=US/ST=CA/L=SF/O=AtlasSphere/CN=${DOMAIN}" \
    2>/dev/null
  
  # Create certificate info file
  cat > "${CERT_DIR}/cert_info.txt" << EOF
Certificate Information
Generated: $(date)
Valid for: ${DAYS_VALID} days
Domain: ${DOMAIN}
Key: jury.key (RSA-2048)
Certificate: jury.crt
Fingerprint: $(openssl x509 -in "${CERT_DIR}/jury.crt" -noout -fingerprint 2>/dev/null)
EOF
  
  echo -e "${GREEN}✅ Self-signed certificates generated${NC}"
  echo -e "   Private Key:  ${CERT_DIR}/jury.key"
  echo -e "   Certificate:  ${CERT_DIR}/jury.crt"
  
elif [ "${ENV}" = "prod" ]; then
  echo -e "${YELLOW}Production TLS mode${NC}"
  echo -e "${RED}⚠️  OBTAIN PRODUCTION CERTIFICATES${NC}"
  echo ""
  echo -e "Recommended: Use Let's Encrypt (free, automatic renewal)"
  echo -e "  1. Install certbot: ${BLUE}apt-get install certbot${NC}"
  echo -e "  2. Generate cert: ${BLUE}certbot certonly --standalone -d ${DOMAIN}${NC}"
  echo -e "  3. Update .env.production:"
  echo -e "     ${BLUE}TLS_CERT_PATH=/etc/letsencrypt/live/${DOMAIN}/fullchain.pem${NC}"
  echo -e "     ${BLUE}TLS_KEY_PATH=/etc/letsencrypt/live/${DOMAIN}/privkey.pem${NC}"
  echo ""
  echo -e "Alternative: Use your own certificate authority"
  echo -e "  Copy certificates to ${CERT_DIR}/"
fi

# Generate dhparam for perfect forward secrecy (PFS)
if [ ! -f "${CERT_DIR}/dhparam.pem" ]; then
  echo -e "${BLUE}→ Generating DH parameters for perfect forward secrecy...${NC}"
  openssl dhparam -out "${CERT_DIR}/dhparam.pem" 2048 2>/dev/null || true
  chmod 644 "${CERT_DIR}/dhparam.pem"
fi

# Create NGINX TLS configuration snippet (if NGINX is used)
cat > "${CERT_DIR}/nginx_tls.conf" << 'EOF'
# NGINX TLS Configuration Snippet
# Include this in your nginx.conf for all Phase 5 services

ssl_certificate /etc/nginx/certs/jury.crt;
ssl_certificate_key /etc/nginx/certs/jury.key;
ssl_dhparam /etc/nginx/certs/dhparam.pem;

# Modern configuration (TLS 1.3 + 1.2)
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers on;

# HSTS (HTTP Strict-Transport-Security)
add_header Strict-Transport-Security "max-age=63072000; includeSubDomains" always;

# Session configuration
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;
ssl_session_tickets off;

# OCSP Stapling
ssl_stapling on;
ssl_stapling_verify on;
EOF

# Create docker-compose override for TLS
cat > "${CERT_DIR}/docker-compose.tls.yml" << 'EOF'
version: '3.8'

services:
  jury-service:
    environment:
      TLS_ENABLED: 'true'
      TLS_CERT_PATH: /etc/ssl/certs/jury.crt
      TLS_KEY_PATH: /etc/ssl/private/jury.key
    volumes:
      - ./jury.crt:/etc/ssl/certs/jury.crt:ro
      - ./jury.key:/etc/ssl/private/jury.key:ro
    ports:
      - "8443:8443"

  jury-anchorer:
    environment:
      TLS_ENABLED: 'true'
      TLS_CERT_PATH: /etc/ssl/certs/jury.crt
      TLS_KEY_PATH: /etc/ssl/private/jury.key
    volumes:
      - ./jury.crt:/etc/ssl/certs/jury.crt:ro
      - ./jury.key:/etc/ssl/private/jury.key:ro

  blockchain-node:
    environment:
      TLS_ENABLED: 'true'
    volumes:
      - ./jury.crt:/etc/ssl/certs/jury.crt:ro
      - ./jury.key:/etc/ssl/private/jury.key:ro
EOF

echo ""
echo -e "${BLUE}=== TLS Configuration Summary ===${NC}"
echo -e "Environment:  ${ENV}"
echo -e "Certificates: ${CERT_DIR}/"
echo -e "Domain:       ${DOMAIN}"
echo ""

# Verify certificates
if [ -f "${CERT_DIR}/jury.crt" ]; then
  echo -e "${BLUE}Certificate Details:${NC}"
  openssl x509 -in "${CERT_DIR}/jury.crt" -text -noout 2>/dev/null | grep -E "Subject:|Issuer:|Not Before|Not After|Public-Key:" | head -5
fi

echo ""
echo -e "${GREEN}✅ TLS Configuration Complete${NC}"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "1. Update .env.production with TLS paths"
echo "2. Set TLS_ENABLED=true in environment"
echo "3. Test TLS: ${BLUE}curl -k https://localhost:8443/api/health${NC}"
echo "4. Monitor certificates for expiration"
echo ""

# Certificate expiration warning (development only)
if [ "${ENV}" = "dev" ]; then
  EXPIRES=$(date -d "+${DAYS_VALID} days" +"%Y-%m-%d")
  echo -e "${YELLOW}⚠️  Dev certificates expire: ${EXPIRES}${NC}"
  echo "   (Regenerate with: ./enable_tls.sh dev)"
fi
