# Cross-VM Atomic Operations Tutorial

This tutorial demonstrates X3 Chain's unique cross-VM capabilities by building an atomic arbitrage contract that executes operations on both EVM and SVM in a single transaction.

## Prerequisites

- X3 Chain local node running
- Completed [EVM Hello World](./evm-hello.md) tutorial
- Completed [SVM Hello World](./svm-hello.md) tutorial
- Understanding of both EVM and SVM development patterns

**Why this matters**: Cross-VM atomic operations are the core innovation of X3 Chain, enabling use cases impossible on traditional blockchains like trustless arbitrage, unified liquidity, and cross-chain DeFi strategies.

## Overview

We'll build a simple arbitrage system that:
1. Executes a trade on an EVM AMM (simulated)
2. Simultaneously executes a trade on an SVM DEX (simulated)
3. Settles profits atomically - both succeed or both fail
4. Demonstrates cross-VM state synchronization

```
┌─────────────────────────────────────────────────────────────────┐
│              ATOMIC CROSS-VM ARBITRAGE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  EVM Side                    │  SVM Side                       │
│  ┌─────────────────────┐     │  ┌─────────────────────┐       │
│  │ AMM Trade           │     │  │ DEX Trade           │       │
│  │ • Buy 100 USDC      │     │  │ • Sell 100 USDC     │       │
│  │ • Cost: 0.05 ETH    │     │  │ • Receive 0.051 ETH │       │
│  └─────────┬───────────┘     │  └─────────┬───────────┘       │
│           │                   │           │                   │
│           └───────────────────┼───────────┘                   │
│                               │                               │
│                    ┌──────────▼──────────┐                    │
│                    │ ATOMIC COORDINATOR  │                    │
│                    │ • Validate both     │                    │
│                    │ • Calculate profit  │                    │
│                    │ • Settle atomically │                    │
│                    └──────────┬───────────┘                    │
│                               │                               │
│                    ┌──────────▼──────────┐                    │
│                    │ CANONICAL LEDGER    │                    │
│                    │ • EVM: +0.001 ETH   │                    │
│                    │ • SVM: -0.001 ETH   │                    │
│                    │ • Total: 0          │                    │
│                    └─────────────────────┘                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Step 1: Create Cross-VM Project Structure

```bash
# Create project directory
mkdir x3-cross-vm-arb
cd x3-cross-vm-arb

# Create subdirectories
mkdir evm-contract svm-program shared

# Initialize Node.js project
npm init -y
npm install --save-dev hardhat @nomiclabs/hardhat-ethers ethers
npm install --save @x3-chain/sdk

# Initialize Rust project for SVM
cargo init --lib svm-program
cd svm-program
cargo add anchor-lang --features derive
cargo add anchor-client
cd ..
```

**Why this matters**: The project structure separates EVM and SVM components while sharing common types and interfaces through the `shared` directory.

## Step 2: Create Shared Types and Interfaces

Create `shared/types.ts`:

```typescript
/**
 * Shared types for cross-VM arbitrage system
 * These types are used by both EVM and SVM components
 */

export interface ArbitrageParams {
  evmAmount: bigint;
  svmAmount: bigint;
  minProfit: bigint;
  deadline: number;
}

export interface CrossVmResult {
  success: boolean;
  evmProfit: bigint;
  svmProfit: bigint;
  totalProfit: bigint;
  gasUsed: number;
  computeUnitsUsed: number;
}

export interface TradeParams {
  amountIn: bigint;
  minAmountOut: bigint;
  tokenIn: string;
  tokenOut: string;
}

export enum VmOperation {
  EVM_TRADE = 0,
  SVM_TRADE = 1,
  SETTLEMENT = 2,
}

// EVM ABI for cross-VM calls
export const ARBITRAGE_ABI = [
  {
    "inputs": [
      { "internalType": "uint256", "name": "evmAmount", "type": "uint256" },
      { "internalType": "uint256", "name": "svmAmount", "type": "uint256" },
      { "internalType": "uint256", "name": "minProfit", "type": "uint256" },
      { "internalType": "uint256", "name": "deadline", "type": "uint256" }
    ],
    "name": "executeAtomicArbitrage",
    "outputs": [
      {
        "components": [
          { "internalType": "bool", "name": "success", "type": "bool" },
          { "internalType": "int256", "name": "evmProfit", "type": "int256" },
          { "internalType": "int256", "name": "svmProfit", "type": "int256" },
          { "internalType": "int256", "name": "totalProfit", "type": "int256" },
          { "internalType": "uint256", "name": "gasUsed", "type": "uint256" }
        ],
        "internalType": "struct CrossVmResult",
        "name": "result",
        "type": "tuple"
      }
    ],
    "stateMutability": "payable",
    "type": "function"
  }
];
```

Create `shared/constants.ts`:

```typescript
/**
 * Shared constants for cross-VM operations
 */

export const CROSS_VM_CONSTANTS = {
  // Cross-VM call precompile address (X3 Chain specific)
  CROSS_VM_PRECOMPILE: "0x0000000000000000000000000000000000000800",
  
  // Gas limits
  MAX_EVM_GAS: 500000n,
  MAX_SVM_COMPUTE_UNITS: 300000,
  
  // Protocol addresses (example)
  EVM_AMM_ROUTER: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", // Uniswap V2
  SVM_DEX_PROGRAM: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
  
  // Token addresses
  EVM_USDC: "0xA0b86a33E6416d0E1b5C4c1e5b3D4E2F1A0B9C8D7",
  EVM_ETH: "0x0000000000000000000000000000000000000000",
  SVM_USDC: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  SVM_WSOL: "So11111111111111111111111111111111111111112",
} as const;

export const ARBITRAGE_CONFIG = {
  MIN_PROFIT_THRESHOLD: 1000n, // 0.001 X3
  SLIPPAGE_TOLERANCE: 50n, // 0.5%
  EXECUTION_TIMEOUT: 300, // 5 minutes
} as const;
```

## Step 3: Create EVM Arbitrage Contract

Create `evm-contract/contracts/AtomicArbitrage.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@x3-chain/contracts/cross-vm/CrossVmCaller.sol";

/**
 * @title AtomicArbitrage
 * @dev Demonstrates cross-VM atomic operations on X3 Chain
 * @notice Executes trades on both EVM and SVM with atomic settlement
 */
contract AtomicArbitrage is CrossVmCaller {
    
    struct ArbitrageParams {
        uint256 evmAmount;
        uint256 svmAmount;
        uint256 minProfit;
        uint256 deadline;
    }
    
    struct CrossVmResult {
        bool success;
        int256 evmProfit;
        int256 svmProfit;
        int256 totalProfit;
        uint256 gasUsed;
    }
    
    // Events
    event ArbitrageExecuted(
        address indexed executor,
        uint256 evmAmount,
        uint256 svmAmount,
        int256 totalProfit,
        bool success
    );
    
    event CrossVmCallExecuted(
        uint8 vmType,
        bool success,
        bytes result
    );
    
    // State
    mapping(address => uint256) public authorizedCallers;
    address public svmProgramAddress;
    uint256 public totalArbitrages;
    uint256 public totalProfit;
    
    modifier onlyAuthorized() {
        require(authorizedCallers[msg.sender] > 0, "Unauthorized");
        _;
    }
    
    constructor(address _svmProgramAddress) {
        svmProgramAddress = _svmProgramAddress;
        authorizedCallers[msg.sender] = 1;
        totalArbitrages = 0;
        totalProfit = 0;
    }
    
    /**
     * @dev Execute atomic arbitrage between EVM and SVM
     * @param params Arbitrage parameters
     * @return result Cross-VM execution result
     */
    function executeAtomicArbitrage(
        ArbitrageParams calldata params
    ) external onlyAuthorized returns (CrossVmResult memory result) {
        require(block.timestamp <= params.deadline, "Transaction expired");
        require(params.evmAmount > 0, "Invalid EVM amount");
        require(params.svmAmount > 0, "Invalid SVM amount");
        
        uint256 gasBefore = gasleft();
        
        // Step 1: Execute EVM trade (simulated AMM interaction)
        int256 evmProfit = _executeEvmTrade(params.evmAmount);
        
        // Step 2: Execute SVM trade via cross-VM call
        bytes memory svmCallData = abi.encode(
            params.svmAmount,
            params.minProfit
        );
        
        (bool svmSuccess, bytes memory svmResult) = 
            crossVmCall(svmProgramAddress, "executeArbitrageTrade", svmCallData);
        
        emit CrossVmCallExecuted(1, svmSuccess, svmResult);
        
        // Decode SVM result
        int256 svmProfit = 0;
        if (svmSuccess) {
            svmProfit = abi.decode(svmResult, (int256));
        }
        
        // Step 3: Validate profitability
        int256 totalProfit = evmProfit + svmProfit;
        require(totalProfit >= int256(params.minProfit), "Insufficient profit");
        
        // Step 4: Execute settlement (could involve additional logic)
        _executeSettlement(evmProfit, svmProfit);
        
        uint256 gasUsed = gasBefore - gasleft();
        
        result = CrossVmResult({
            success: true,
            evmProfit: evmProfit,
            svmProfit: svmProfit,
            totalProfit: totalProfit,
            gasUsed: gasUsed
        });
        
        // Update state
        totalArbitrages++;
        totalProfit += uint256(totalProfit);
        
        emit ArbitrageExecuted(
            msg.sender,
            params.evmAmount,
            params.svmAmount,
            totalProfit,
            true
        );
    }
    
    /**
     * @dev Simulate EVM AMM trade
     * @param amount Amount to trade
     * @return profit Profit from the trade (positive/negative)
     */
    function _executeEvmTrade(uint256 amount) internal returns (int256 profit) {
        // Simulate AMM trade logic
        // In real implementation, this would interact with actual AMMs
        
        // Simulate getting 1% better rate on EVM
        uint256 expectedOut = (amount * 101) / 100;
        
        // Simulate market conditions (could be better or worse)
        // For demo, assume we get exactly expected rate
        profit = int256(expectedOut) - int256(amount);
        
        emit CrossVmCallExecuted(0, true, abi.encode(profit));
        
        return profit;
    }
    
    /**
     * @dev Execute settlement logic
     * @param evmProfit Profit from EVM side
     * @param svmProfit Profit from SVM side
     */
    function _executeSettlement(int256 evmProfit, int256 svmProfit) internal {
        // Settlement logic could include:
        // - Fee distribution
        // - Profit sharing
        // - Risk management
        
        // For demo, just log the settlement
        emit CrossVmCallExecuted(2, true, abi.encode(evmProfit + svmProfit));
    }
    
    /**
     * @dev Get contract statistics
     * @return totalArb Total arbitrages executed
     * @return totalProf Total profit generated
     */
    function getStats() external view returns (uint256 totalArb, uint256 totalProf) {
        return (totalArbitrages, totalProfit);
    }
    
    /**
     * @dev Authorize a caller for cross-VM operations
     * @param caller Address to authorize
     */
    function authorizeCaller(address caller) external onlyAuthorized {
        authorizedCallers[caller] = 1;
    }
    
    /**
     * @dev Revoke authorization from a caller
     * @param caller Address to revoke authorization from
     */
    function revokeCaller(address caller) external onlyAuthorized {
        delete authorizedCallers[caller];
    }
}
```

Create the CrossVmCaller base contract in `shared/CrossVmCaller.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title CrossVmCaller
 * @dev Base contract for cross-VM operations on X3 Chain
 * @notice Provides standardized interface for calling SVM programs
 */
abstract contract CrossVmCaller {
    
    // Cross-VM precompile address on X3 Chain
    address constant CROSS_VM_PRECOMPILE = 0x0000000000000000000000000000000000000800;
    
    /**
     * @dev Execute cross-VM call to SVM program
     * @param programId SVM program address
     * @param method Method name to call
     * @param data Encoded method parameters
     * @return success Call success status
     * @return result Encoded result data
     */
    function crossVmCall(
        address programId,
        string memory method,
        bytes memory data
    ) internal returns (bool success, bytes memory result) {
        // Encode the cross-VM call
        bytes memory callData = abi.encode(programId, method, data);
        
        // Call the precompile
        (success, result) = CROSS_VM_PRECOMPILE.call(callData);
        
        require(success, "Cross-VM call failed");
    }
    
    /**
     * @dev Get cross-VM execution status
     * @param txHash Transaction hash to check
     * @return status Execution status (0=pending, 1=success, 2=failure)
     */
    function getCrossVmStatus(bytes32 txHash) external view returns (uint8 status) {
        bytes memory callData = abi.encode("getStatus", txHash);
        (success, result) = CROSS_VM_PRECOMPILE.call(callData);
        require(success, "Status query failed");
        return abi.decode(result, (uint8));
    }
}
```

## Step 4: Create SVM Program

Create `svm-program/src/lib.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("YourSVMProgramIDHere");

#[program]
pub mod cross_vm_arbitrage {
    use super::*;

    /// Execute arbitrage trade on SVM side
    /// This would typically interact with SVM DEXs like Serum, Raydium, etc.
    pub fn execute_arbitrage_trade(
        ctx: Context<ExecuteArbitrage>,
        amount: u64,
        min_profit: i64,
    ) -> Result<i64> {
        let arbitrage_account = &mut ctx.accounts.arbitrage_account;
        
        // Simulate SVM DEX trade
        // In real implementation, this would:
        // 1. Interact with SVM DEXs
        // 2. Execute parallel trades across accounts
        // 3. Calculate profit/loss
        
        msg!("Executing SVM arbitrage trade for amount: {}", amount);
        
        // Simulate getting slightly better rate on SVM (1.5% advantage)
        let simulated_profit = (amount as i64) * 15 / 1000; // 1.5% profit
        
        // Validate minimum profit requirement
        require!(simulated_profit >= min_profit, ArbitrageError::InsufficientProfit);
        
        // Update account state
        arbitrage_account.last_profit = simulated_profit;
        arbitrage_account.trade_count += 1;
        arbitrage_account.total_profit = arbitrage_account.total_profit
            .checked_add(simulated_profit as u64)
            .unwrap();
        
        // Emit event for cross-VM coordination
        emit!(ArbitrageExecuted {
            authority: ctx.accounts.authority.key(),
            amount,
            profit: simulated_profit,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        msg!("SVM arbitrage completed with profit: {}", simulated_profit);
        Ok(simulated_profit)
    }

    /// Initialize arbitrage account for an authority
    pub fn initialize_arbitrage_account(
        ctx: Context<InitializeArbitrage>,
    ) -> Result<()> {
        let arbitrage_account = &mut ctx.accounts.arbitrage_account;
        
        arbitrage_account.authority = ctx.accounts.authority.key();
        arbitrage_account.trade_count = 0;
        arbitrage_account.total_profit = 0;
        arbitrage_account.last
