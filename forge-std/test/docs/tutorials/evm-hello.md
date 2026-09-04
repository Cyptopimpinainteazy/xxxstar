# EVM Hello World Tutorial

This tutorial will guide you through deploying your first Solidity contract on X3 Chain using the EVM. We'll create a simple counter contract and interact with it.

## Prerequisites

- Node.js 18+ installed
- X3 Chain local node running (see [Getting Started](../getting-started.md))
- Basic understanding of Solidity and Ethereum development

**Why this matters**: This tutorial demonstrates that you can deploy existing Ethereum contracts on X3 Chain with zero modifications, leveraging the full EVM compatibility.

## Step 1: Set Up Your Project

```bash
# Create a new directory for your project
mkdir x3-evm-hello
cd x3-evm-hello

# Initialize npm project
npm init -y

# Install dependencies
npm install --save-dev hardhat @nomiclabs/hardhat-ethers ethers
npm install --save @x3-chain/sdk

# Initialize Hardhat
npx hardhat
```

Choose "Create a basic sample project" and select TypeScript when prompted.

## Step 2: Configure Hardhat for X3 Chain

Update `hardhat.config.ts`:

```typescript
import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.19",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200
      }
    }
  },
  networks: {
    x3: {
      url: "http://localhost:9933",
      chainId: 1234,
      accounts: [
        "0x" + "1".repeat(64) // Replace with your private key
      ]
    }
  }
};

export default config;
```

**Why this matters**: X3 Chain's EVM is compatible with standard Ethereum tooling. The configuration is identical to any Ethereum network configuration.

## Step 3: Create Your First Contract

Create `contracts/Counter.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title Counter
 * @dev A simple counter contract demonstrating EVM compatibility
 * @notice This contract works identically on Ethereum, Polygon, BSC, and X3 Chain
 */
contract Counter {
    uint256 private count;
    address public owner;
    
    event CounterIncremented(uint256 newCount, address indexed sender);
    event CounterDecremented(uint256 newCount, address indexed sender);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner can call this function");
        _;
    }
    
    constructor() {
        owner = msg.sender;
        count = 0;
    }
    
    /**
     * @dev Increment the counter
     * @return The current count after incrementing
     */
    function increment() external returns (uint256) {
        count++;
        emit CounterIncremented(count, msg.sender);
        return count;
    }
    
    /**
     * @dev Decrement the counter
     * @return The current count after decrementing
     */
    function decrement() external returns (uint256) {
        require(count > 0, "Counter cannot be negative");
        count--;
        emit CounterDecremented(count, msg.sender);
        return count;
    }
    
    /**
     * @dev Get the current count
     * @return The current count value
     */
    function getCount() external view returns (uint256) {
        return count;
    }
    
    /**
     * @dev Reset the counter to zero
     */
    function reset() external onlyOwner {
        count = 0;
    }
    
    /**
     * @dev Transfer ownership
     * @param newOwner The address to transfer ownership to
     */
    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "New owner cannot be zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
}
```

**Why this matters**: This contract uses standard Solidity patterns that work across all EVM-compatible blockchains. X3 Chain maintains full compatibility with Ethereum standards.

## Step 4: Create Deployment Script

Create `scripts/deploy.ts`:

```typescript
import { ethers } from "hardhat";
import { Counter } from "../typechain-types";

async function main() {
  console.log("🚀 Deploying Counter contract to X3 Chain...");
  
  // Get the contract factory
  const Counter = await ethers.getContractFactory("Counter");
  
  // Deploy the contract
  console.log("📡 Sending deployment transaction...");
  const counter = await Counter.deploy();
  
  // Wait for deployment to be mined
  await counter.deployed();
  
  console.log("✅ Counter contract deployed to:", counter.address);
  console.log("📊 Owner address:", await counter.owner());
  console.log("🔢 Initial count:", (await counter.getCount()).toString());
  
  // Demonstrate contract interaction
  console.log("\n🔧 Testing contract interactions...");
  
  // Increment the counter
  console.log("➕ Incrementing counter...");
  const tx1 = await counter.increment();
  await tx1.wait();
  console.log("New count:", (await counter.getCount()).toString());
  
  // Increment again
  console.log("➕ Incrementing counter again...");
  const tx2 = await counter.increment();
  await tx2.wait();
  console.log("New count:", (await counter.getCount()).toString());
  
  // Decrement the counter
  console.log("➖ Decrementing counter...");
  const tx3 = await counter.decrement();
  await tx3.wait();
  console.log("Final count:", (await counter.getCount()).toString());
  
  // Listen to events (optional - requires WebSocket connection)
  console.log("\n📡 Setting up event listeners...");
  counter.on("CounterIncremented", (newCount: BigInt, sender: string) => {
    console.log(`🎉 Counter incremented to ${newCount} by ${sender}`);
  });
  
  counter.on("CounterDecremented", (newCount: BigInt, sender: string) => {
    console.log(`📉 Counter decremented to ${newCount} by ${sender}`);
  });
  
  console.log("🎯 Contract deployed and tested successfully!");
  console.log("💡 Contract address for future use:", counter.address);
  
  return {
    address: counter.address,
    owner: await counter.owner(),
    finalCount: await counter.getCount()
  };
}

// Execute the deployment
main()
  .then((result) => {
    console.log("\n🎊 Deployment completed successfully!");
    console.log("📋 Summary:", result);
    process.exit(0);
  })
  .catch((error) => {
    console.error("❌ Deployment failed:", error);
    process.exit(1);
  });
```

## Step 5: Create Interaction Script

Create `scripts/interact.ts`:

```typescript
import { ethers } from "hardhat";
import { Counter } from "../typechain-types";

async function interactWithCounter() {
  // Contract address from deployment
  const contractAddress = "YOUR_DEPLOYED_CONTRACT_ADDRESS";
  
  if (contractAddress === "YOUR_DEPLOYED_CONTRACT_ADDRESS") {
    console.error("❌ Please update the contract address in scripts/interact.ts");
    process.exit(1);
  }
  
  console.log("🔗 Connecting to Counter contract at:", contractAddress);
  
  // Get the contract instance
  const counter = await ethers.getContractAt("Counter", contractAddress);
  
  // Read current state
  console.log("📊 Current owner:", await counter.owner());
  console.log("🔢 Current count:", (await counter.getCount()).toString());
  
  // Perform operations
  console.log("\n🔧 Performing operations...");
  
  // Increment
  console.log("➕ Incrementing...");
  const tx1 = await counter.increment();
  await tx1.wait();
  console.log("New count:", (await counter.getCount()).toString());
  
  // Multiple increments
  console.log("➕ Incrementing 3 more times...");
  for (let i = 0; i < 3; i++) {
    await counter.increment();
  }
  console.log("Final count:", (await counter.getCount()).toString());
  
  // Check if we can decrement
  console.log("➖ Attempting to decrement...");
  try {
    await counter.decrement();
    console.log("Decrement successful. New count:", (await counter.getCount()).toString());
  } catch (error: any) {
    console.log("Decrement failed (expected if count is 0):", error.message);
  }
}

interactWithCounter()
  .then(() => {
    console.log("🎯 Interaction completed!");
    process.exit(0);
  })
  .catch((error) => {
    console.error("❌ Interaction failed:", error);
    process.exit(1);
  });
```

## Step 6: Deploy and Test

```bash
# Compile the contract
npx hardhat compile

# Deploy to X3 Chain
npx hardhat run scripts/deploy.ts --network x3

# Interact with the deployed contract
npx hardhat run scripts/interact.ts --network x3
```

**Expected output:**
```
🚀 Deploying Counter contract to X3 Chain...
📡 Sending deployment transaction...
✅ Counter contract deployed to: 0x1234567890abcdef1234567890abcdef12345678
📊 Owner address: 0x742d35Cc6BF4e8B5e2C1C7d1A3E9c4F8d5A2B1C3
🔢 Initial count: 0

🔧 Testing contract interactions...
➕ Incrementing counter...
New count: 1
➕ Incrementing counter again...
New count: 2
➖ Decrementing counter...
Final count: 1
🎯 Contract deployed and tested successfully!
```

## Step 7: Create Tests

Create `test/Counter.test.ts`:

```typescript
import { expect } from "chai";
import { ethers } from "hardhat";

describe("Counter Contract", function () {
  let counter: any;
  let owner: any;
  let user: any;

  beforeEach(async function () {
    // Get signers
    [owner, user] = await ethers.getSigners();
    
    // Deploy contract
    const Counter = await ethers.getContractFactory("Counter");
    counter = await Counter.deploy();
    await counter.deployed();
  });

  describe("Deployment", function () {
    it("Should set the right owner", async function () {
      expect(await counter.owner()).to.equal(owner.address);
    });

    it("Should start with count of 0", async function () {
      expect(await counter.getCount()).to.equal(0);
    });
  });

  describe("Increment", function () {
    it("Should increment the count", async function () {
      await counter.increment();
      expect(await counter.getCount()).to.equal(1);
    });

    it("Should emit CounterIncremented event", async function () {
      await expect(counter.increment())
        .to.emit(counter, "CounterIncremented")
        .withArgs(1, owner.address);
    });
  });

  describe("Decrement", function () {
    beforeEach(async function () {
      await counter.increment();
      await counter.increment();
    });

    it("Should decrement the count", async function () {
      await counter.decrement();
      expect(await counter.getCount()).to.equal(1);
    });

    it("Should emit CounterDecremented event", async function () {
      await expect(counter.decrement())
        .to.emit(counter, "CounterDecremented")
        .withArgs(1, owner.address);
    });

    it("Should revert when count is 0", async function () {
      await counter.decrement();
      await counter.decrement();
      
      await expect(counter.decrement()).to.be.revertedWith(
        "Counter cannot be negative"
      );
    });
  });

  describe("Access Control", function () {
    it("Should allow owner to reset", async function () {
      await counter.increment();
      await counter.increment();
      
      await counter.reset();
      expect(await counter.getCount()).to.equal(0);
    });

    it("Should prevent non-owner from resetting", async function () {
      await expect(counter.connect(user).reset()).to.be.revertedWith(
        "Only owner can call this function"
      );
    });
  });

  describe("Ownership Transfer", function () {
    it("Should transfer ownership", async function () {
      await expect(counter.transferOwnership(user.address))
        .to.emit(counter, "OwnershipTransferred")
        .withArgs(owner.address, user.address);
      
      expect(await counter.owner()).to.equal(user.address);
    });

    it("Should prevent transferring to zero address", async function () {
      await expect(
        counter.transferOwnership(ethers.constants.AddressZero)
      ).to.be.revertedWith("New owner cannot be zero address");
    });
  });
});
```

Run tests:
```bash
npx hardhat test --network x3
```

## Step 8: Verify Contract on X3 Chain

```bash
# Verify contract (if X3 Chain has a block explorer with verification)
npx hardhat verify --network x3 CONTRACT_ADDRESS
```

**Why this matters**: Contract verification allows others to inspect your contract code and interact with it safely through the block explorer interface.

## Next Steps

### Explore Further
- **[Cross-VM Operations](./cross-vm-atomic.md)** - Learn to call SVM programs from your EVM contract
- **[RPC Integration Example](../examples/rpc_integration.rs)** - Explore endpoint integrations and request flows
- **[Gas Model](../x3-lang/spec/gas-model.md)** - Learn execution and fee mechanics

### Common Patterns
```solidity
// Cross-VM call from EVM
function callSVMProgram(bytes32 programId, bytes calldata data) external {
    // This will be covered in the cross-VM tutorial
    bytes32 result = atlasCrossVmCall(programId, data);
    // Process result
}

// Event emission for off-chain monitoring
emit CrossVmOperationExecuted(
    txHash,
    programId,
    success,
    gasUsed
);
```

### Troubleshooting

**Transaction fails with "out of gas"**
```bash
# Increase gas limit in deployment
const tx = await counter.deploy({
    gasLimit: 1000000  // 1M gas
});
```

**Contract address shows as empty**
```bash
# Check if node is running
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Verify network configuration in hardhat.config.ts
```

**Events not firing**
```bash
# Ensure WebSocket connection is active for event subscriptions
# Check block explorer for confirmed transactions
```

**Why this matters**: Understanding these common issues will help you debug deployment problems and ensure your contracts work reliably on X3 Chain.

---

*This tutorial demonstrates X3 Chain's full EVM compatibility. For advanced features like cross-VM calls, see our [Cross-VM Atomic Operations](./cross-vm-atomic.md) guide.*
