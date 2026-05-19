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
