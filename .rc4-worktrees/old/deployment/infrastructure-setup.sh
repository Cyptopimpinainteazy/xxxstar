#!/bin/bash
# X3 Chain Testnet v1 - Infrastructure Setup Script
# Day -2: Provision infrastructure (VMs, DNS)

set -e

echo "🚀 X3 Chain Testnet v1 - Infrastructure Setup"
echo "=================================================="
echo ""

# Configuration
PROJECT_NAME="x3-testnet"
REGION="${REGION:-us-west-2}"  # Change to your preferred region
VM_PROVIDER="${VM_PROVIDER:-local}"  # Options: local, digitalocean, aws, gcp

# VM Specifications (adjust based on your provider)
VALIDATOR_SIZE="4GB RAM, 2 vCPU, 50GB SSD"
RPC_SIZE="8GB RAM, 4 vCPU, 100GB SSD"
BOOTNODE_SIZE="2GB RAM, 1 vCPU, 20GB SSD"
MONITOR_SIZE="4GB RAM, 2 vCPU, 50GB SSD"

# DNS Configuration (update with your domain)
DOMAIN="${DOMAIN:-testnet.x3-chain.io}"
DNS_PROVIDER="${DNS_PROVIDER:-cloudflare}"  # Options: cloudflare, route53, manual

echo "Configuration:"
echo "  Project: $PROJECT_NAME"
echo "  Region: $REGION"
echo "  Domain: $DOMAIN"
echo "  VM Provider: $VM_PROVIDER"
echo ""

# Step 1: Check prerequisites
echo "📋 Step 1/5: Checking prerequisites..."
if ! command -v ssh &> /dev/null; then
    echo "❌ ssh not found. Please install OpenSSH client."
    exit 1
fi
echo "✅ SSH client available"

# Step 2: Generate SSH key for deployment (if not exists)
echo ""
echo "🔑 Step 2/5: Setting up SSH keys..."
SSH_KEY="$HOME/.ssh/x3-testnet-deploy"
if [ ! -f "$SSH_KEY" ]; then
    ssh-keygen -t ed25519 -f "$SSH_KEY" -N "" -C "x3-testnet-deploy"
    echo "✅ Generated new SSH key: $SSH_KEY"
else
    echo "✅ SSH key already exists: $SSH_KEY"
fi

# Step 3: Create infrastructure inventory file
echo ""
echo "📝 Step 3/5: Creating infrastructure inventory..."
INVENTORY_FILE="deployment/inventory.yaml"
mkdir -p deployment

cat > "$INVENTORY_FILE" << 'EOF'
# X3 Chain hardware-backed deployment inventory
#
# Fill in the null addresses with real values after provisioning or racking hosts.
# Do not replace them with invented placeholders.

metadata:
  generated_for: x3-chain testnet planning
  generated_by: deployment/infrastructure-setup.sh
  updated_at: 2026-04-04
  status: partial

local_lab:
  authorities:
    - logical_name: validator-01
      host_id: x3-lab-val-01
      role: bootnode + validator
      ssh_user: null
      management_ip: null
      public_ip: null
    - logical_name: validator-02
      host_id: x3-lab-val-02
      role: validator
      ssh_user: null
      management_ip: null
      public_ip: null
    - logical_name: validator-03
      host_id: x3-lab-val-03
      role: validator during lab proving only
      ssh_user: null
      management_ip: null
      public_ip: null

  support_nodes:
    - logical_name: rpc-01
      host_id: x3-lab-rpc-01
      role: rpc + prometheus + grafana
      ssh_user: null
      management_ip: null
      public_ip: null
    - logical_name: dr-01
      host_id: x3-lab-dr-01
      role: restore target + spare
      ssh_user: null
      management_ip: null
      public_ip: null

public_testnet_minimum:
  authorities:
    - logical_name: validator-01
      host_id: x3-lab-val-01
      site: local-site
      public_ip: null
    - logical_name: validator-02
      host_id: remote-vps-a
      site: remote-site-a
      public_ip: null
    - logical_name: validator-03
      host_id: remote-vps-b
      site: remote-site-b
      public_ip: null

dns_records:
  - name: bootnode.testnet.x3-chain.io
    type: A
    value: null
  - name: rpc.testnet.x3-chain.io
    type: A
    value: null

firewall_rules:
  p2p:
    port: 30333
    protocol: tcp
    source: validator-peers-and-approved-seeds
  rpc:
    port: 9944
    protocol: tcp
    source: public-only-on-rpc-hosts
  metrics:
    port: 9615
    protocol: tcp
    source: vpn-or-management-network-only
  ssh:
    port: 22
    protocol: tcp
    source: operator-source-ips-only
EOF

echo "✅ Created inventory file: $INVENTORY_FILE"
echo "   ⚠️  IMPORTANT: Update this file with actual IPs after provisioning VMs!"

# Step 4: Create VM provisioning guides for different providers
echo ""
echo "📚 Step 4/5: Creating provider-specific guides..."

# DigitalOcean guide
cat > "deployment/provision-digitalocean.sh" << 'EOF'
#!/bin/bash
# DigitalOcean VM Provisioning for X3 Chain Testnet

# Prerequisites: Install doctl (DigitalOcean CLI)
# https://docs.digitalocean.com/reference/doctl/how-to/install/

set -e

# Configuration
REGION="nyc3"  # Change to your preferred region
SSH_KEY_ID="your-ssh-key-id"  # Get from: doctl compute ssh-key list
IMAGE="ubuntu-22-04-x64"
TAG="x3-testnet"

echo "🌊 Provisioning VMs on DigitalOcean..."

# Create validators
for i in {1..3}; do
    echo "Creating validator-0$i..."
    doctl compute droplet create "x3-validator-0$i" \
        --region "$REGION" \
        --size "s-2vcpu-4gb" \
        --image "$IMAGE" \
        --ssh-keys "$SSH_KEY_ID" \
        --tag-names "$TAG" \
        --wait
done

# Create RPC nodes
for i in {1..2}; do
    echo "Creating rpc-0$i..."
    doctl compute droplet create "x3-rpc-0$i" \
        --region "$REGION" \
        --size "s-4vcpu-8gb" \
        --image "$IMAGE" \
        --ssh-keys "$SSH_KEY_ID" \
        --tag-names "$TAG" \
        --wait
done

# Create bootnode
echo "Creating bootnode..."
doctl compute droplet create "x3-bootnode-01" \
    --region "$REGION" \
    --size "s-1vcpu-2gb" \
    --image "$IMAGE" \
    --ssh-keys "$SSH_KEY_ID" \
    --tag-names "$TAG" \
    --wait

# Create monitoring server
echo "Creating monitoring server..."
doctl compute droplet create "x3-monitoring-01" \
    --region "$REGION" \
    --size "s-2vcpu-4gb" \
    --image "$IMAGE" \
    --ssh-keys "$SSH_KEY_ID" \
    --tag-names "$TAG" \
    --wait

echo ""
echo "✅ All VMs created! Getting IP addresses..."
doctl compute droplet list --tag-name "$TAG" --format Name,PublicIPv4,PrivateIPv4

echo ""
echo "⚠️  Update deployment/inventory.yaml with these IPs!"
EOF
chmod +x "deployment/provision-digitalocean.sh"
echo "✅ Created: deployment/provision-digitalocean.sh"

# AWS guide
cat > "docs/docs/deployment/provision-aws.md" << 'EOF'
# AWS EC2 VM Provisioning for X3 Chain Testnet

## Prerequisites
- AWS CLI installed: `aws --version`
- AWS credentials configured: `aws configure`

## Instance Types
- Validators: t3.medium (2 vCPU, 4GB RAM)
- RPC Nodes: t3.large (2 vCPU, 8GB RAM)
- Bootnode: t3.small (2 vCPU, 2GB RAM)
- Monitoring: t3.medium (2 vCPU, 4GB RAM)

## Provisioning Steps

### 1. Create Security Groups
```bash
# Create security group
aws ec2 create-security-group \
    --group-name x3-testnet \
    --description "X3 Chain Testnet security group"

# Allow P2P (port 30333)
aws ec2 authorize-security-group-ingress \
    --group-name x3-testnet \
    --protocol tcp \
    --port 30333 \
    --cidr 0.0.0.0/0

# Allow SSH (port 22) - restrict to your IP!
aws ec2 authorize-security-group-ingress \
    --group-name x3-testnet \
    --protocol tcp \
    --port 22 \
    --cidr YOUR_IP/32
```

### 2. Launch Instances
```bash
# Launch validators (repeat 3 times)
aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1f0 \
    --instance-type t3.medium \
    --key-name your-key-pair \
    --security-groups x3-testnet \
    --count 3 \
    --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=x3-validator},{Key=Project,Value=x3-testnet}]'

# Launch RPC nodes (repeat 2 times)
aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1f0 \
    --instance-type t3.large \
    --key-name your-key-pair \
    --security-groups x3-testnet \
    --count 2 \
    --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=x3-rpc},{Key=Project,Value=x3-testnet}]'

# Launch bootnode
aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1f0 \
    --instance-type t3.small \
    --key-name your-key-pair \
    --security-groups x3-testnet \
    --count 1 \
    --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=x3-bootnode},{Key=Project,Value=x3-testnet}]'
```

### 3. Get IP Addresses
```bash
aws ec2 describe-instances \
    --filters "Name=tag:Project,Values=x3-testnet" \
    --query 'Reservations[*].Instances[*].[Tags[?Key==`Name`].Value|[0],PublicIpAddress,PrivateIpAddress]' \
    --output table
```

### 4. Update inventory.yaml with IPs
EOF
echo "✅ Created: docs/docs/deployment/provision-aws.md"

# Local/Manual guide
cat > "docs/docs/deployment/provision-manual.md" << 'EOF'
# Manual VM Provisioning for X3 Chain Testnet

## If using VPS provider (Hetzner, Linode, Vultr, etc.)

### VM Requirements

#### Validators (need 3-5)
- **CPU**: 2 vCPU
- **RAM**: 4GB
- **Storage**: 50GB SSD
- **OS**: Ubuntu 22.04 LTS
- **Network**: Public IP + 30333/tcp open

#### RPC Nodes (need 2+)
- **CPU**: 4 vCPU
- **RAM**: 8GB
- **Storage**: 100GB SSD
- **OS**: Ubuntu 22.04 LTS
- **Network**: Public IP + 9944/tcp open

#### Bootnode (need 1)
- **CPU**: 1 vCPU
- **RAM**: 2GB
- **Storage**: 20GB SSD
- **OS**: Ubuntu 22.04 LTS
- **Network**: Public IP + 30333/tcp open

#### Monitoring (need 1)
- **CPU**: 2 vCPU
- **RAM**: 4GB
- **Storage**: 50GB SSD
- **OS**: Ubuntu 22.04 LTS
- **Network**: Public IP + 3000/tcp open (Grafana)

## Provisioning Steps

### 1. Create VMs
Using your provider's web interface or CLI:
- Create VMs with specs above
- Use Ubuntu 22.04 LTS
- Add your SSH public key (`~/.ssh/x3-testnet-deploy.pub`)

### 2. Record IP Addresses
Get public and private IPs for each VM and update `deployment/inventory.yaml`

### 3. Test SSH Access
```bash
# Test each VM
ssh -i ~/.ssh/x3-testnet-deploy x3@VALIDATOR_IP
ssh -i ~/.ssh/x3-testnet-deploy x3@RPC_IP
ssh -i ~/.ssh/x3-testnet-deploy x3@BOOTNODE_IP
ssh -i ~/.ssh/x3-testnet-deploy x3@MONITORING_IP
```

### 4. Basic Server Hardening
Run on each VM:
```bash
# Update system
sudo apt-get update && sudo apt-get upgrade -y

# Install essentials
sudo apt-get install -y curl wget git build-essential ufw

# Configure firewall (example for validator)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 30333/tcp # P2P
sudo ufw enable
```

## Alternative: Local Testing

### Using Docker Compose (for testing only)
Create `deployment/docker-compose.yml`:
```yaml
version: '3.8'
services:
  validator-01:
    image: ubuntu:22.04
    ports:
      - "30333:30333"
      - "9944:9944"
    volumes:
      - ./data/validator-01:/data
  
  rpc-01:
    image: ubuntu:22.04
    ports:
      - "9945:9944"
    volumes:
      - ./data/rpc-01:/data
```

### Using Local VMs (VirtualBox, QEMU, etc.)
- Create VMs with specs above
- Set up host-only or bridged networking
- Configure port forwarding as needed
EOF
echo "✅ Created: docs/docs/deployment/provision-manual.md"

# Step 5: Create DNS configuration template
echo ""
echo "🌐 Step 5/5: Creating DNS configuration..."

cat > "docs/docs/deployment/dns-config.md" << 'EOF'
# DNS Configuration for X3 Chain Testnet

## Required DNS Records

After provisioning VMs, create these DNS records in your DNS provider:

### A Records
```
rpc.testnet.x3-chain.io      → RPC_LOAD_BALANCER_IP
rpc2.testnet.x3-chain.io     → BACKUP_RPC_IP
bootnode.testnet.x3-chain.io → BOOTNODE_IP
faucet.testnet.x3-chain.io   → FAUCET_SERVER_IP
metrics.testnet.x3-chain.io  → GRAFANA_IP
```

## Provider-Specific Guides

### Cloudflare
1. Log in to Cloudflare apps/dash-legacy-2-legacy-2board
2. Select your domain
3. Go to DNS → Records
4. Add A records using the real public IPs from `deployment/inventory.yaml`:
  - Name: `rpc.testnet`, IPv4: value for the public RPC host, Proxy: OFF
  - Name: `rpc2.testnet`, IPv4: value for the backup RPC host if you have one, Proxy: OFF
  - Name: `bootnode.testnet`, IPv4: value for the seed or bootnode host, Proxy: OFF
  - Name: `faucet.testnet`, IPv4: value for the faucet host if you deploy one, Proxy: OFF
  - Name: `metrics.testnet`, IPv4: value for the Grafana or monitoring host, Proxy: OFF

### AWS Route53
```bash
# Create hosted zone (if not exists)
aws route53 create-hosted-zone --name testnet.x3-chain.io --caller-reference $(date +%s)

# Add A records
aws route53 change-resource-record-sets \
    --hosted-zone-id YOUR_ZONE_ID \
    --change-batch file://dns-records.json
```

`dns-records.json`:
```json
{
  "Changes": [
    {
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "rpc.testnet.x3-chain.io",
        "Type": "A",
        "TTL": 300,
        "ResourceRecords": [{"Value": "<real-public-ip-from-deployment-inventory>"}]
      }
    }
  ]
}
```

### Manual (most DNS providers)
1. Log in to your DNS provider (Namecheap, GoDaddy, etc.)
2. Find DNS management page
3. Add A records as listed above
4. Set TTL to 300 seconds (5 minutes)
5. Wait 5-15 minutes for propagation

## Verify DNS
```bash
# Check DNS resolution
dig rpc.testnet.x3-chain.io
dig bootnode.testnet.x3-chain.io

# Should return the IPs you configured
```
EOF
echo "✅ Created: docs/docs/deployment/dns-config.md"

# Create firewall configuration script
cat > "deployment/configure-firewall.sh" << 'EOF'
#!/bin/bash
# Configure firewall on X3 Chain testnet nodes

set -e

NODE_TYPE="${1:-validator}"  # validator, rpc, bootnode, or monitoring
ADMIN_IP="${ADMIN_IP:-0.0.0.0/0}"  # Restrict SSH to admin IP
MONITORING_IP="${MONITORING_IP:-$ADMIN_IP}"  # Restrict metrics scraping to monitoring host/CIDR

echo "🔒 Configuring firewall for $NODE_TYPE node..."

# Update system
sudo apt-get update
sudo apt-get install -y ufw

# Default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# SSH access (restrict to admin IP in production!)
sudo ufw allow from "$ADMIN_IP" to any port 22 proto tcp

# Common: P2P port for all nodes
sudo ufw allow 30333/tcp comment 'X3 P2P'

# Node-type specific rules
case "$NODE_TYPE" in
    validator)
        echo "Validator: Opening RPC port (localhost only)"
        # RPC only accessible locally
        ;;
    rpc)
        echo "RPC Node: Opening public RPC port"
        sudo ufw allow 9944/tcp comment 'X3 RPC'
        ;;
    bootnode)
        echo "Bootnode: P2P only (already configured)"
        ;;
    monitoring)
        echo "Monitoring: Opening Prometheus and Grafana ports"
        sudo ufw allow 9090/tcp comment 'Prometheus'
        sudo ufw allow 3000/tcp comment 'Grafana'
        ;;
esac

# Metrics port (accessible from monitoring server only)
sudo ufw allow from "$MONITORING_IP" to any port 9615 proto tcp comment 'Prometheus metrics (restricted)'

# Enable firewall
sudo ufw --force enable

# Show status
sudo ufw status verbose

echo "✅ Firewall configured for $NODE_TYPE node"
EOF
chmod +x "deployment/configure-firewall.sh"
echo "✅ Created: deployment/configure-firewall.sh"

# Summary
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "✅ Day -2 Infrastructure Setup Complete!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "📁 Created files:"
echo "  • deployment/inventory.yaml (UPDATE WITH ACTUAL IPs)"
echo "  • deployment/provision-digitalocean.sh"
echo "  • docs/docs/deployment/provision-aws.md"
echo "  • docs/docs/deployment/provision-manual.md"
echo "  • docs/docs/deployment/dns-config.md"
echo "  • deployment/configure-firewall.sh"
echo ""
echo "🚀 Next steps:"
echo ""
echo "1. Provision VMs using your preferred method:"
echo "   • DigitalOcean: ./deployment/provision-digitalocean.sh"
echo "   • AWS: Follow docs/docs/deployment/provision-aws.md"
echo "   • Manual: Follow docs/docs/deployment/provision-manual.md"
echo ""
echo "2. Update deployment/inventory.yaml with actual IPs"
echo ""
echo "3. Configure DNS records using docs/docs/deployment/dns-config.md"
echo ""
echo "4. Run firewall setup on each node:"
echo "   ssh x3@NODE_IP 'bash -s' < deployment/configure-firewall.sh validator"
echo ""
echo "5. Proceed to Day -1: Build binary and generate keys"
echo ""
echo "════════════════════════════════════════════════════════════════"
