# Getting Started with X3 Chain

This guide will get you up and running with X3 Chain in under 10 minutes. We'll cover local development setup, connecting your wallet, and deploying your first contract.

## Prerequisites

- Node.js 18+ or Rust 1.70+
- Git
- 4GB+ available disk space
- Basic knowledge of Solidity or Rust

## Quick Start (5 minutes)

### 1. Install X3 CLI

```bash
# Install via npm
npm install -g @x3-chain/cli

# Or via curl
curl -sSL https://x3-chain.io/install | bash

# Verify installation
x3 --version
```

**Why this matters**: The X3 CLI provides unified commands for both EVM and SVM development, simplifying your workflow.

### 2. Start Local Node

```bash
# Start development node (creates fresh chain)
x3 node start --dev

# Or with specific configuration
x3 node start --dev --port 9944 --rpc-port 9933
```

The node will:
- Start on `ws://localhost:9944` (WebSocket RPC)
- Expose HTTP RPC on `http://localhost:9933`
- Create a development account with 1000 X3 tokens
- Enable both EVM and SVM execution

**Expected output:**
```
2025-12-10 14:00:00 INFO X3 Chain Node v1.0.0
2025-12-10 14:00:01 INFO Dual-VM runtime initialized
2025-12-10 14:00:02 INFO EVM adapter: Ready
2025-12-10 14:00:02 INFO SVM adapter: Ready
2025-12-10 14:00:03 INFO RPC server started on ws://localhost:9944
2025-12-10 14:00:03 INFO Development account: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
```

### 3. Connect Your Wallet

#### MetaMask Setup
```javascript
// Add X3 Chain to MetaMask
await window.ethereum.request({
  method: 'wallet_addEthereumChain',
  params: [{
    chainId: '0x1234', // Replace with actual chain ID
    chainName: 'X3 Chain Dev',
    nativeCurrency: {
      name: 'X3',
      symbol: 'X3',
      decimals: 12
    },
    rpcUrls: ['http://localhost:9933'],
    blockExplorerUrls: ['http://localhost:3000']
  }]
});
```

#### X3 SDK Setup
```bash
npm install @x3-chain/sdk
```

```javascript
import { AtlasSphereProvider } from '@x3-chain/sdk';

const provider = new AtlasSphereProvider({
  network: 'local',
  rpcUrl: 'ws://localhost:9944',
  timeout: 30000
});

await provider.connect();
console.log('Connected to X3 Chain!');
```

**Why this matters**: X3 Chain provides both standard Ethereum Web3 compatibility and enhanced dual-VM features through the X3 SDK.

## Choose Your VM

X3 Chain supports three development approaches:

### EVM-Only Development
**Best for**: Existing Ethereum projects, DeFi protocols, familiar tooling

```solidity
// Deploy with standard tools
npx hardhat compile --network x3
npx hardhat run scripts/deploy.js --network x3
```

**Use cases**: 
- DeFi protocols (DEX, lending, derivatives)
- DAOs and governance
- NFT marketplaces
- Standard Ethereum patterns

### SVM-Only Development  
**Best for**: High-throughput apps, gaming, microtransactions

```bash
# Anchor project setup
anchor init my-svm-app
cd my-svm-app

# Configure for X3 Chain
anchor set provider cluster http://localhost:9933
anchor set provider wallet ~/.config/solana/id.json

# Build and deploy
anchor build
anchor deploy
```

**Use cases**:
- Real-time gaming
- High-frequency trading
- Social media applications
- Microtransaction platforms

### Cross-VM Development
**Best for**: Maximum flexibility, atomic operations, arbitrage

```solidity
// EVM contract calling SVM program
pragma solidity ^0.8.0;

import "@x3-chain/contracts/CrossVM.sol";

contract AtomicArb is CrossVM {
    function executeArb(uint256 amount) external {
        // 1. Check EVM balances
        require(balanceOf(msg.sender) > amount, "Insufficient balance");
        
        // 2. Call SVM program atomically
        bytes32 result = crossVMCall(
            SVM_PROGRAM_ID,
            "executeTrade",
            abi.encode(amount)
        );
        
        // 3. Process SVM result in EVM
        uint256 profit = decodeProfit(result);
        if (profit > 0) {
            _settleProfit(profit);
        }
    }
}
```

**Use cases**:
- Cross-chain arbitrage
- Unified liquidity protocols
- Multi-domain applications
- Atomic swaps and trades

## Deploy Your First Contract

### EVM Contract Example

**contracts/HelloWorld.sol**
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract HelloWorld {
    string public message;
    uint256 public counter;
    
    event MessageChanged(string oldMessage, string newMessage, uint256 timestamp);
    
    constructor(string memory _message) {
        message = _message;
        counter = 0;
    }
    
    function setMessage(string memory _message) external {
        string memory oldMessage = message;
        message = _message;
        counter++;
        
        emit MessageChanged(oldMessage, _message, block.timestamp);
    }
    
    function getMessage() external view returns (string memory) {
        return message;
    }
    
    function getCounter() external view returns (uint256) {
        return counter;
    }
}
```

**Deploy script (scripts/deploy.js)**
```javascript
const hre = require("hardhat");

async function main() {
    console.log("Deploying HelloWorld contract to X3 Chain...");
    
    const HelloWorld = await hre.ethers.getContractFactory("HelloWorld");
    const helloWorld = await HelloWorld.deploy("Hello X3 Chain!");
    
    await helloWorld.deployed();
    
    console.log("HelloWorld deployed to:", helloWorld.address);
    
    // Interact with the contract
    const message = await helloWorld.getMessage();
    console.log("Initial message:", message);
    
    const tx = await helloWorld.setMessage("Updated via X3 Chain!");
    await tx.wait();
    
    const updatedMessage = await helloWorld.getMessage();
    const counter = await helloWorld.getCounter();
    
    console.log("Updated message:", updatedMessage);
    console.log("Transaction count:", counter.toString());
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
```

**Deploy with Hardhat**
```bash
# Install dependencies
npm install --save-dev hardhat @nomiclabs/hardhat-ethers

# Configure Hardhat (hardhat.config.js)
require("@nomiclabs/hardhat-ethers");

module.exports = {
  solidity: "0.8.0",
  networks: {
    x3: {
      url: "http://localhost:9933",
      chainId: 1234,
      accounts: ["0x..."] // Your private key
    }
  }
};

# Deploy
npx hardhat run scripts/deploy.js --network x3
```

### SVM Program Example

**programs/hello-world/src/lib.rs**
```rust
use anchor_lang::prelude::*;

declare_id!("YourProgramIDHere");

#[program]
pub mod hello_world {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, message: String) -> Result<()> {
        let account = &mut ctx.accounts.message_account;
        account.message = message;
        account.counter = 0;
        account.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn update_message(ctx: Context<UpdateMessage>, new_message: String) -> Result<()> {
        let account = &mut ctx.accounts.message_account;
        let old_message = account.message.clone();
        account.message = new_message;
        account.counter += 1;
        
        msg!("Updated message from '{}' to '{}'", old_message, account.message);
        Ok(())
    }

    pub fn get_message(ctx: Context<GetMessage>) -> Result<String> {
        Ok(ctx.accounts.message_account.message.clone())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 8 + 4 + 200)]
    pub message_account: Account<'info, MessageAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMessage<'info> {
    #[account(mut, seeds = [b"message", authority.key().as_ref()], bump)]
    pub message_account: Account<'info, MessageAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetMessage<'info> {
    #[account(seeds = [b"message", authority.key().as_ref()], bump)]
    pub message_account: Account<'info, MessageAccount>,
    pub authority: Signer<'info>,
}

#[account]
pub struct MessageAccount {
    pub message: String,
    pub counter: u64,
    pub authority: Pubkey,
}
```

**Deploy with Anchor**
```bash
# Build and deploy
anchor build
anchor deploy

# Interact via CLI
anchor run initialize --message "Hello X3 Chain!"
anchor run update_message --new_message "Updated via Anchor!"
```

## Test Your Deployment

### EVM Testing
```javascript
// test/hello-world.test.js
const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("HelloWorld", function () {
  it("Should deploy and interact with HelloWorld", async function () {
    const HelloWorld = await ethers.getContractFactory("HelloWorld");
    const helloWorld = await HelloWorld.deploy("Hello X3!");
    await helloWorld.deployed();

    expect(await helloWorld.getMessage()).to.equal("Hello X3!");
    
    await helloWorld.setMessage("Updated!");
    expect(await helloWorld.getMessage()).to.equal("Updated!");
    expect(await helloWorld.getCounter()).to.equal(1);
  });
});
```

```bash
# Run tests
npx hardhat test --network x3
```

### SVM Testing
```rust
// tests/hello-world.ts
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { HelloWorld } from "../target/types/hello_world";
import { expect } from "chai";

describe("hello-world", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.HelloWorld as Program<HelloWorld>;

  it("Initialize message account", async () => {
    const message = "Hello X3 Chain!";
    
    await program.methods
      .initialize(message)
      .accounts({
        messageAccount: anchor.web3.Keypair.generate().publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const storedMessage = await program.methods.getMessage().rpc();
    expect(storedMessage).to.equal(message);
  });
});
```

```bash
# Run tests
anchor test
```

## Next Steps

### Explore Examples
- **[Cross-VM Atomic Operations](./tutorials/cross-vm-atomic.md)** - Build apps that use both VMs
- **[RPC Integration Example](./examples/rpc_integration.rs)** - Connect to chain endpoints and flows
- **[Arbitrage Bot Example](./examples/arbitrage_bot_config.rs)** - Explore advanced strategy configuration

### Advanced Features
- **[RPC API Reference](./rpc.md)** - Detailed API documentation
- **[Cross-VM Tutorial](./tutorials/cross-vm-atomic.md)** - End-to-end cross-VM development flow
- **[Gas Model](./x3-lang/spec/gas-model.md)** - Understand execution and fee model

### Join the Community
- **[Discord](https://discord.gg/x3-chain)** - Get help and share projects
- **[GitHub](https://github.com/x3-chain)** - Contribute and report issues
- **[Twitter](https://twitter.com/atlassphere)** - Follow updates

## Troubleshooting

### Common Issues

**Node won't start**
```bash
# Check if port is already in use
lsof -i :9944

# Kill existing processes
pkill -f x3-chain-node

# Start with different ports
x3 node start --dev --port 9945 --rpc-port 9934
```

**Wallet connection fails**
```bash
# Verify node is running
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Check WebSocket connection
wscat -c ws://localhost:9944
```

**Contract deployment fails**
```bash
# Check gas estimation
x3 node logs --tail 50

# Verify account has funds
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0xYourAddress","latest"],"id":1}'
```

**Why this matters**: X3 Chain provides detailed logging and debugging tools to help you resolve issues quickly and understand what's happening under the hood.

---

*Ready to build something amazing? Check out our [tutorials](./tutorials/) for step-by-step guides to common patterns and use cases.*
