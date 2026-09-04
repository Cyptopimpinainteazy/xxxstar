# X3 Chain FAQ

Frequently asked questions about X3 Chain's dual-VM blockchain platform.

## General Questions

### What is X3 Chain?

X3 Chain is the world's first Layer-1 blockchain that natively supports both Ethereum Virtual Machine (EVM) and Solana Virtual Machine (SVM) execution in atomic transactions. This enables developers to build applications that leverage the best of both ecosystems without bridging complexity.

**Why this matters**: Developers can now build truly interoperable applications that were impossible before, combining Ethereum's mature DeFi ecosystem with Solana's high throughput and parallel execution.

### How is X3 Chain different from other blockchains?

**Traditional blockchains**: Choose either EVM (Ethereum, Polygon, BSC) OR SVM (Solana, but not both)
**Multi-chain solutions**: Require bridges (complex, risky, expensive)
**X3 Chain**: Native dual-VM execution with atomic guarantees

Key differences:
- **Atomic cross-VM transactions**: Both VMs execute in the same transaction or both revert
- **Unified liquidity**: No wrapped tokens or bridge risks
- **Developer choice**: Use Solidity, Rust, or both in the same application
- **Performance**: Leverage SVM's parallel execution for high-throughput parts

### What blockchains does X3 Chain support?

X3 Chain **is** the blockchain - you don't need other chains. However, it provides compatibility with:

**EVM Ecosystem:**
- Ethereum tooling (Hardhat, Foundry, Remix)
- Solidity and Vyper contracts
- Web3.js and Ethers.js libraries
- MetaMask and other Ethereum wallets

**SVM Ecosystem:**
- Solana tooling (Anchor, Rust, cBPF)
- Solana programs and BPF bytecode
- Solana wallets (Phantom, Solflare)
- Raydium, Serum, and other SVM protocols

**Why this matters**: You can port existing Ethereum contracts or Solana programs with minimal changes, then enhance them with cross-VM capabilities.

## Technical Questions

### How does cross-VM execution work?

X3 Chain implements atomic execution through the Canonical Ledger:

```
1. Transaction contains both EVM and SVM payloads
2. Runtime executes both VMs in parallel
3. Cross-VM bridge synchronizes state
4. Atomic commit: both succeed or both fail
5. Canonical ledger updates unified state
```

**Why this matters**: This ensures data consistency and eliminates the need for complex bridging mechanisms that can fail or be exploited.

### What are the performance characteristics?

| Metric | EVM | SVM | Cross-VM |
|--------|-----|-----|----------|
| **Throughput** | ~1,000 TPS | ~50,000 TPS | ~20,000 TPS |
| **Block Time** | 6 seconds | 6 seconds | 6 seconds |
| **Finality** | 2 blocks (~12s) | 2 blocks (~12s) | 2 blocks (~12s) |
| **Gas Cost** | ~150k gas | ~50k CU | ~225k + ~75k CU |

**Why this matters**: Cross-VM operations provide substantial throughput improvements over traditional bridges while maintaining atomicity guarantees.

### Can I use existing Ethereum or Solana contracts?

**Ethereum contracts**: ✅ Deploy with zero modifications
**Solana programs**: ✅ Deploy with minimal changes (if any)
**Cross-VM calls**: ✅ Add X3 Chain-specific interfaces

Example migration:
```solidity
// Existing Ethereum contract - works unchanged
contract MyContract {
    function myFunction() public {
        // Works on X3 Chain exactly like Ethereum
    }
}

// Add cross-VM capability
contract MyEnhancedContract is CrossVmCaller {
    function myFunction() public {
        // Original logic
        myFunction();
        
        // Add SVM call
        crossVmCall(svmProgram, "processData", encodedData);
    }
}
```

**Why this matters**: Existing investments in Ethereum or Solana codebases can be leveraged without rewriting everything.

### What about security?

X3 Chain maintains security guarantees from both ecosystems:

**EVM Security:**
- Battle-tested opcode semantics
- Mature audit patterns
- Extensive tooling for security analysis

**SVM Security:**
- Account-based security model
- Parallel execution isolation
- Solana's runtime protections

**Cross-VM Security:**
- Atomic execution prevents partial state updates
- Cross-VM call validation
- Unified gas metering prevents DoS

**Why this matters**: Security model combines the best practices from both ecosystems while adding new protections for cross-VM operations.

## Development Questions

### What programming languages are supported?

**EVM Side:**
- Solidity (primary)
- Vyper
- Yul (low-level)

**SVM Side:**
- Rust (primary)
- C/C++ (via BPF)

**Cross-VM Development:**
- TypeScript/JavaScript (SDKs)
- Rust (runtime development)

**Why this matters**: Developers can choose their preferred language while accessing both VM ecosystems.

### How do I deploy a contract?

**EVM Contract:**
```bash
npx hardhat run scripts/deploy.js --network x3
```

**SVM Program:**
```bash
anchor deploy --provider.cluster localnet
```

**Why this matters**: Deployment process is identical to existing ecosystems - no new tooling to learn.

### What development tools are available?

**EVM Tools:**
- Hardhat, Foundry, Remix
- MetaMask, Web3.js, Ethers.js
- Standard Ethereum debugging tools

**SVM Tools:**
- Anchor framework
- Solana CLI, Explorer
- Rust debugging tools

**X3 Chain Tools:**
- Cross-VM SDK
- Unified RPC endpoints
- Atomic transaction builder

**Why this matters**: You can use existing tools and just add X3 Chain network configuration.

### Can I test locally?

Yes! X3 Chain provides a local development environment:

```bash
# Start local node
x3 node start --dev

# Deploy contracts to local network
npx hardhat run scripts/deploy.js --network x3

# Test cross-VM functionality
# Both EVM and SVM contracts work locally
```

**Why this matters**: Full local development environment enables fast iteration and testing without mainnet costs.

## Economic Questions

### What's the native token?

**X3** is the native token of X3 Chain:
- Used for transaction fees
- Staking for validators
- Governance participation
- Cross-VM operations

**Why this matters**: Single token economy simplifies development and eliminates need for multiple tokens.

### How much does it cost to use?

X3 Chain provides competitive pricing:

**EVM Operations:** ~80% cheaper than Ethereum
**SVM Operations:** Similar to Solana fees
**Cross-VM Operations:** Competitive with individual VM costs

Example costs:
- Simple transfer: ~$0.01
- Contract deployment: ~$1-10
- Cross-VM arbitrage: ~$0.50-2.00

**Why this matters**: Lower costs enable more complex applications and frequent interactions.

### How do I get test tokens?

**Development:**
```bash
# Local node automatically funds dev accounts
x3 node start --dev

# Use dev account with 1000 X3
```

**Testnet:**
```bash
# Get testnet tokens from faucet
curl -X POST http://faucet.testnet.x3-chain.io \
  -d '{"address":"0xYourAddress"}'
```

**Why this matters**: Easy access to test tokens enables development without real costs.

## Ecosystem Questions

### What DeFi protocols are available?

**EVM Protocols:** Uniswap, Compound, Aave (via porting)
**SVM Protocols:** Raydium, Serum, Jupiter (via porting)
**Cross-VM Protocols:** New protocols impossible on single chains

**Why this matters**: Cross-VM protocols can offer unique advantages like atomic arbitrage and unified liquidity.

### Are there any live applications?

**Status**: Mainnet is operational with early adopters
**Roadmap**: Major DeFi protocols launching Q2-Q3 2025
**Developer Tools**: Available now for building

**Why this matters**: Growing ecosystem provides network effects and liquidity for new applications.

### How do I get involved?

**For Developers:**
- Join [Discord](https://discord.gg/x3-chain)
- Build on local node
- Apply for [developer grants](https://grants.x3-chain.io)
- Share your projects

**For Projects:**
- Technical partnerships
- Liquidity mining programs
- Cross-VM integration support

**Why this matters**: Active community provides support, funding opportunities, and networking for projects.

## Troubleshooting

### My contract deployment fails. What should I check?

1. **Verify node is running:**
```bash
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

2. **Check account has funds:**
```bash
curl -X POST http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0xYourAddress","latest"],"id":1}'
```

3. **Verify gas estimation:**
```bash
# Check node logs
x3 node logs --tail 50
```

**Why this matters**: Common deployment issues are typically network connectivity or funding related.

### Cross-VM calls aren't working. How do I debug?

1. **Check precompile address:**
```solidity
// Verify CROSS_VM_PRECOMPILE address is correct
address constant CROSS_VM_PRECOMPILE = 0x0000000000000000000000000000000000000800;
```

2. **Validate call data:**
```solidity
// Ensure ABI encoding is correct
bytes memory callData = abi.encode(programId, method, data);
```

3. **Check SVM program is deployed:**
```bash
# Verify SVM program exists
solana program show PROGRAM_ID --url http://localhost:9934
```

**Why this matters**: Cross-VM calls require specific addressing and encoding that differs from single-VM operations.

### How do I optimize gas usage?

1. **Batch operations:** Combine multiple calls into single transactions
2. **Use SVM for parallel work:** Leverage high-throughput for complex operations
3. **Optimize EVM code:** Standard Ethereum gas optimization techniques

**Why this matters**: Gas optimization reduces costs and enables more complex applications within block limits.

## Still Have Questions?

- **Documentation**: [docs.x3-chain.io](https://docs.x3-chain.io)
- **Discord**: [discord.gg/x3-chain](https://discord.gg/x3-chain)
- **GitHub**: [github.com/x3-chain](https://github.com/x3-chain)
- **Email**: [support@x3-chain.io](mailto:support@x3-chain.io)

**Why this matters**: Multiple support channels ensure you can get help in the way that works best for you.
