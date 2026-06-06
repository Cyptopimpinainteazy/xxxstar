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
4. Add A records:
   - Name: `rpc.testnet`, IPv4: `x.x.x.x`, Proxy: OFF
   - Name: `rpc2.testnet`, IPv4: `x.x.x.x`, Proxy: OFF
   - Name: `bootnode.testnet`, IPv4: `x.x.x.x`, Proxy: OFF
   - Name: `faucet.testnet`, IPv4: `x.x.x.x`, Proxy: OFF
   - Name: `metrics.testnet`, IPv4: `x.x.x.x`, Proxy: OFF

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
        "ResourceRecords": [{"Value": "x.x.x.x"}]
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
