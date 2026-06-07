# Validator Quickstart Guide

## Overview
This document provides a step-by-step guide for setting up and running a validator node for the X3 Atomic Star network. It covers installation, configuration, and operational requirements.

## Prerequisites
- Linux x86_64 system (Ubuntu 22.04 LTS or equivalent)
- Minimum 8 CPU cores
- Minimum 32GB RAM
- Minimum 500GB SSD storage
- Static public IP address
- Open ports: 30333 (P2P), 9933 (RPC), 9443 (RPC TLS), 9615 (prometheus)

## Installation Steps

### 1. System Preparation
```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y build-essential curl git jq

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Install target for wasm32
rustup target add wasm32-unknown-unknown
```

### 2. Build X3 Node
```bash
# Clone the repository
git clone https://github.com/yourorg/x3-atomic-star.git
cd x3-atomic-star

# Checkout the latest stable branch
git checkout stable2512

# Build the node
cargo build --release -p x3-chain-node
```

### 3. Configuration
Create a configuration file at `~/.x3-node/config.toml`:

```toml
[network]
name = "x3-rc1-testnet"
bootnodes = [
    "enr://5678@bootnode1.x3testnet.com:30333"
]

[rpc]
enable = true
port = 9933
cors = ["all"]

[prometheus]
enable = true
port = 9615
```

### 4. Run the Validator
```bash
# Start the node
./target/release/x3-chain-node --config ~/.x3-node/config.toml
```

### 5. Verify Operation
Check that the node is synced and producing blocks:
```bash
# Check sync status
curl -s http://localhost:9933 | jq '.result[] | select(.blockNumber?)'

# Monitor logs
journalctl -u x3-node -f
```

## Operational Requirements
- Maintain at least 99.9% uptime
- Participate in governance votes
- Monitor node metrics via Prometheus
- Apply security updates promptly

## Troubleshooting
- Check logs for error messages
- Verify network connectivity to bootnodes
- Ensure time synchronization (NTP)
- Validate configuration file syntax

## Support
For assistance, contact the validator support team at validators@x3testnet.com or join the #validators channel on Matrix.