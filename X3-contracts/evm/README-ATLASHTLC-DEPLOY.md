# AtlasHTLC Deployment Guide

## Overview

`AtlasHTLC` is a Hashed Timelock Contract (HTLC) deployed on EVM chains, used as the on-chain
settlement layer for X3 cross-chain atomic swaps. It supports:

- **Lock**: Create an HTLC with a hashlock and timelock.
- **Claim**: Redeem locked funds by revealing the preimage.
- **Refund**: Reclaim locked funds after the timelock expires.

The contract has **no constructor arguments**, making deployment identical across all chains.

---

## Prerequisites

- [Foundry](https://book.getfoundry.sh/getting-started/installation) installed
- An `.env` file or exported environment variables:

```bash
# Required for deployment
DEPLOYER_PRIVATE_KEY=0x...

# RPC endpoint for the target chain
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_KEY
# or
HOLESKY_RPC_URL=https://holesky.infura.io/v3/YOUR_KEY
# or (for local)
LOCAL_RPC_URL=http://localhost:8545

# Required for verification
ETHERSCAN_API_KEY=your_etherscan_api_key
```

---

## Step-by-Step Deployment

### 1. Build the contract

```bash
cd /home/x3star/Desktop/xxxstar-main/X3-contracts/evm
forge build --contracts contracts/AtlasHTLC.sol
```

### 2. Run tests

```bash
forge test --match-path test/AtlasHTLC.t.sol -vvv
```

### 3. Deploy to Sepolia (default)

```bash
CHAIN_ID=11155111 DEPLOYER_PRIVATE_KEY=$DEPLOYER_PRIVATE_KEY \
  forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
  --rpc-url "$SEPOLIA_RPC_URL" \
  --broadcast \
  --verify \
  --etherscan-api-key "$ETHERSCAN_API_KEY" \
  -vvvv
```

### 4. Deploy to Holesky

```bash
CHAIN_ID=17000 DEPLOYER_PRIVATE_KEY=$DEPLOYER_PRIVATE_KEY \
  forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
  --rpc-url "$HOLESKY_RPC_URL" \
  --broadcast \
  --verify \
  --etherscan-api-key "$ETHERSCAN_API_KEY" \
  -vvvv
```

### 5. Deploy to Localhost (Anvil)

Start anvil in a separate terminal:

```bash
anvil
```

Then deploy:

```bash
CHAIN_ID=31337 DEPLOYER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 \
  forge script script/DeployAtlasHTLC.s.sol:DeployAtlasHTLC \
  --rpc-url http://localhost:8545 \
  --broadcast \
  -vvvv
```

---

## Contract Address Tracking

After deployment, record the address output in the deployment logs:

```
=== AtlasHTLC Deployed ===
AtlasHTLC: 0x...
Chain ID: 11155111
```

Update the relevant configuration files:

| File | Field |
|---|---|
| `x3-relayer/relayer-config.testnet.yaml` | `atlas_htlc_address` |
| `crates/x3-atomic-swap/src/evm_htlc.rs` | `ATLAS_HTLC_ADDRESS` constant (testnet) |
| CI/CD pipelines | `ATLAS_HTLC_ADDRESS` environment variable |

---

## Verification Instructions

### Using the verification script

```bash
# Verify on Sepolia via Etherscan
ATLAS_HTLC_ADDRESS=0x... ETHERSCAN_API_KEY=$ETHERSCAN_API_KEY \
  bash script/verify-atlas-htlc.sh 11155111 etherscan

# Verify on Holesky via Etherscan
ATLAS_HTLC_ADDRESS=0x... ETHERSCAN_API_KEY=$ETHERSCAN_API_KEY \
  bash script/verify-atlas-htlc.sh 17000 etherscan

# Verify on a Blockscout instance
ATLAS_HTLC_ADDRESS=0x... VERIFIER_URL=https://blockscout.example.com/api/ \
  bash script/verify-atlas-htlc.sh 31337 blockscout
```

### Manual verification with Forge

```bash
forge verify-contract \
  --chain 11155111 \
  --verifier etherscan \
  --etherscan-api-key "$ETHERSCAN_API_KEY" \
  <ATLAS_HTLC_ADDRESS> \
  contracts/AtlasHTLC.sol:AtlasHTLC
```

> **Note**: `AtlasHTLC` has no constructor arguments, so `--constructor-args` is not needed.

---

## Integration with x3-relayer

The `x3-relayer` watches AtlasHTLC events on EVM chains and forwards them to the X3 kernel.
After deployment:

1. Update `x3-relayer/relayer-config.testnet.yaml`:

```yaml
watchers:
  evm:
    sepolia:
      rpc_url: "https://sepolia.infura.io/v3/YOUR_KEY"
      atlas_htlc_address: "0x<DEPLOYED_ADDRESS>"
      start_block: <DEPLOYMENT_BLOCK>
```

2. Restart the relayer:

```bash
cargo run -p x3-relayer -- --config relayer-config.testnet.yaml
```

The relayer will begin indexing `Locked`, `Claimed`, and `Refunded` events from the
AtlasHTLC contract and submitting corresponding intents to the X3 network.

---

## Multi-Chain Quick Reference

| Chain     | Chain ID | Explorer                      | RPC Endpoint (example)                 |
|-----------|----------|-------------------------------|----------------------------------------|
| Sepolia   | 11155111 | sepolia.etherscan.io          | https://sepolia.infura.io/v3/...       |
| Holesky   | 17000    | holesky.etherscan.io          | https://holesky.infura.io/v3/...       |
| Localhost | 31337    | N/A (anvil logs)              | http://localhost:8545                   |

---

## Troubleshooting

### "Failed to get EIP-1559 fees"

Add `--legacy` to the forge script command:

```bash
forge script ... --legacy
```

### "insufficient funds"

Ensure the deployer account has ETH on the target chain. For testnets:

- [Sepolia Faucet](https://sepoliafaucet.com/)
- [Holesky Faucet](https://holesky-faucet.pk910.de/)

### Verification fails with "Already Verified"

The contract may already be verified. Check on the block explorer or re-run with `--force`.
