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
