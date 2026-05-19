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
