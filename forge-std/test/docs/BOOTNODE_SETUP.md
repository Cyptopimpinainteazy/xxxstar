# X3 Chain Bootnode Setup Guide

This document describes the bootnode configuration and setup process for the X3 Chain network.

## Overview

The bootnode serves as the initial entry point for nodes joining the X3 Chain network. It provides:
- Peer discovery via Kademlia DHT
- Network bootstrapping for new nodes
- Secure peer ID verification using ed25519 cryptography

## Bootnode Configuration

### Key Files

The bootnode configuration is stored in `deployment/keys/`:

| File | Description |
|------|-------------|
| `bootnode-info.txt` | Human-readable configuration with peer ID and multiaddresses |
| `bootnode-node-key` | Raw hex-encoded ed25519 private key for Substrate |
| `bootnode-config.json` | Structured JSON configuration for programmatic use |

### Key Format

The bootnode uses **ed25519** keys for network identity:

- **Private Key**: 64 bytes, hex-encoded
- **Public Key**: 32 bytes, hex-encoded  
- **Peer ID**: Base58-encoded multihash (SHA2-256 of public key)

### Peer ID Derivation

The Peer ID is derived using the standard libp2p format:

```
PeerID = base58(0x1220 || sha256(public_key))
```

Where:
- `0x12` = SHA2-256 hash code
- `0x20` = 32 bytes hash length
- `||` = concatenation

## Multiaddress Format

Bootnode multiaddresses follow the libp2p format:

```
/ip4/<IP_ADDRESS>/tcp/<PORT>/p2p/<PEER_ID>
```

### Examples

```bash
# With IP address
/ip4/10.0.1.100/tcp/30333/p2p/QmQxQLmeH5t4CqteqF924NxuKQfHUJMRoGWe8zmvZMFpgB

# With DNS
/dns4/bootnode.x3-chain.io/tcp/30333/p2p/QmQxQLmeH5t4CqteqF924NxuKQfHUJMRoGWe8zmvZMFpgB
```

## Usage

### Starting a Bootnode

1. Copy the node key to the server:

```bash
scp deployment/keys/bootnode-node-key user@bootnode-server:/var/lib/x3/node-key
```

2. Set proper permissions:

```bash
chmod 600 /var/lib/x3/node-key
```

3. Start the node with the bootnode key:

```bash
x3-chain-node \
  --node-key-file /var/lib/x3/node-key \
  --listen-addr /ip4/0.0.0.0/tcp/30333 \
  --name "X3 Bootnode" \
  --chain=x3_chain_testnet
```

### Connecting to a Bootnode

Other nodes connect to the bootnode using the multiaddress:

```bash
x3-chain-node \
  --bootnodes /ip4/10.0.1.100/tcp/30333/p2p/QmQxQLmeH5t4CqteqF924NxuKQfHUJMRoGWe8zmvZMFpgB \
  --name "X3 Validator" \
  --chain=x3_chain_testnet
```

## Security Considerations

### Key Management

1. **Never commit keys to git**: The `deployment/keys/` directory is gitignored
2. **Use strong randomness**: Keys are generated using cryptographically secure random number generators
3. **Secure storage**: Store keys in encrypted storage or hardware security modules (HSM)
4. **Access control**: Limit file permissions to `600` (owner read/write only)

### Network Security

1. **Firewall rules**: Only allow connections on the configured port (default: 30333)
2. **IP whitelisting**: Consider restricting bootnode access to known validator IPs
3. **Monitoring**: Monitor bootnode connections for anomalies

## Generating New Keys

To generate new bootnode keys:

```bash
python3 scripts/generate-bootnode-keys.py
```

This will:
1. Generate a new ed25519 keypair
2. Derive the Peer ID from the public key
3. Update `bootnode-info.txt` and `bootnode-node-key`
4. Create `bootnode-config.json` for programmatic access

## Troubleshooting

### Peer ID Mismatch

If you see "WrongPeerId" errors:

1. Verify the node key file contains the correct private key
2. Check that the Peer ID in `bootnode-info.txt` matches the key
3. Ensure all nodes use the same bootnode multiaddress format

### Connection Issues

1. Verify firewall allows port 30333
2. Check that the bootnode is running and listening
3. Confirm the multiaddress format is correct (include `/p2p/<PEER_ID>`)

## Related Documentation

- [`CHAIN_SPEC.md`](CHAIN_SPEC.md) - Chain specification details
- [`VALIDATOR_SETUP.md`](VALIDATOR_SETUP.md) - Validator node setup
- [`DEPLOYMENT.md`](DEPLOYMENT.md) - Deployment guide
