# X3 Lending Protocol

> **Aave V3-style Cross-Chain DeFi Lending** powered by X3 Kernel Comit transactions

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        X3 LENDING PROTOCOL                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          Pool.sol (Core)                            │   │
│  │  deposit() → withdraw() → borrow() → repay() → liquidationCall()   │   │
│  │                      └── flashLoan() ──┘                            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│         ┌──────────────────────────┼──────────────────────────┐            │
│         │                          │                          │            │
│         ▼                          ▼                          ▼            │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐        │
│  │   AToken.sol    │    │VariableDebt.sol │    │ StableDebt.sol  │        │
│  │ (Interest Recv) │    │(Compound Index) │    │ (Fixed Rate)    │        │
│  │  scaledBalance  │    │  scaledBalance  │    │   userRate      │        │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘        │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                    PoolConfigurator.sol                            │   │
│  │  initReserve() │ setReserveParams() │ emergencyAdmin actions       │   │
│  │                  ┌─── 2-Day Timelock ───┐                          │   │
│  └──────────────────┴──────────────────────┴──────────────────────────┘   │
│                                    │                                       │
│  ┌─────────────┬──────────────┬────┴────┬──────────────┬─────────────┐   │
│  │             │              │         │              │             │   │
│  ▼             ▼              ▼         ▼              ▼             ▼   │
│  ┌───────────┐ ┌────────────┐ ┌────────┐ ┌───────────┐ ┌───────────┐   │
│  │InterestRate│ │OracleRouter│ │Collat- │ │FlashLoan  │ │    Math   │   │
│  │  Model    │ │ (Chainlink)│ │Manager │ │ Receivers │ │ Libraries │   │
│  │           │ │  + TWAP    │ │ E-Mode │ │           │ │  WadRay   │   │
│  └───────────┘ └────────────┘ └────────┘ └───────────┘ └───────────┘   │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                     X3 KERNEL INTEGRATION                            │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              CrossChainLending.sol (Comit Bundle Interface)      │   │
│  │  • Deposit on Chain A → Borrow on Chain B (atomic, 6 seconds)   │   │
│  │  • Cross-chain liquidations with guaranteed settlement           │   │
│  │  • 103-chain collateral aggregation via canonical ledger         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 📁 Contract Structure

```
contracts/lending/
├── src/
│   ├── core/
│   │   ├── Pool.sol                 # Main lending pool (deposit/borrow/liquidate/flash)
│   │   ├── InterestRateModel.sol    # Kinked rate curves (stablecoin/ETH/volatile presets)
│   │   ├── OracleRouter.sol         # Chainlink + TWAP + manual fallback
│   │   ├── CollateralManager.sol    # LTV, liquidation thresholds, E-Mode, isolation
│   │   └── PoolConfigurator.sol     # Admin config with 2-day timelock
│   ├── tokens/
│   │   ├── AToken.sol               # Interest-bearing deposit receipts (scaled balance)
│   │   └── DebtTokens.sol           # VariableDebtToken + StableDebtToken
│   ├── helpers/
│   │   └── FlashLoanReceivers.sol   # Arbitrage, Liquidation, CollateralSwap strategies
│   ├── interfaces/
│   │   ├── IPool.sol                # Pool interface
│   │   ├── ITokens.sol              # Token interfaces
│   │   └── IProtocol.sol            # Protocol interfaces
│   └── libraries/
│       └── MathLibraries.sol        # WadRayMath, PercentageMath, ReserveLogic, HealthFactorLogic
├── script/
│   └── Deploy.s.sol                 # Foundry deployment scripts
├── test/
│   ├── Pool.t.sol                   # Core pool tests
│   └── Liquidation.t.sol            # Liquidation + interest accrual tests
└── foundry.toml                     # Foundry configuration
```

## 🔑 Key Features

| Feature                 | Description                                               |
| ----------------------- | --------------------------------------------------------- |
| **Dual Interest Rates** | Variable (compound index) + Stable (user-locked rate)     |
| **E-Mode Categories**   | Higher LTV for correlated assets (e.g., stablecoins: 95%) |
| **Isolation Mode**      | Risk-contained markets for new assets                     |
| **Flash Loans**         | 0.09% fee, atomic arbitrage/liquidation/swap              |
| **Supply/Borrow Caps**  | Per-asset limits to manage protocol risk                  |
| **Cross-Chain**         | X3 Kernel enables atomic multi-chain positions         |

## 📐 Mathematics

### Interest Rate Model (Kinked Curve)

```solidity
// Below optimal utilization (gentle slope)
if (utilization <= OPTIMAL) {
    rate = BASE_RATE + (utilization / OPTIMAL) × SLOPE_1
}
// Above optimal (steep slope to discourage over-borrowing)
else {
    excess = utilization - OPTIMAL
    rate = BASE_RATE + SLOPE_1 + (excess / (1 - OPTIMAL)) × SLOPE_2
}

// Example: Stablecoin at 90% utilization (80% optimal)
// rate = 0% + 4% + (10%/20%) × 75% = 41.5% APY
```

### Scaled Balance (Index-Based Accounting)

```solidity
// Deposits/withdrawals use scaled amounts
scaledBalance = amount / liquidityIndex

// Actual balance grows as index increases
actualBalance = scaledBalance × liquidityIndex

// Interest compounds automatically without per-user updates
```

### Health Factor Calculation

```solidity
healthFactor = (Σ collateral_i × price_i × liquidationThreshold_i) / (Σ debt_j × price_j)

// liquidationThreshold (e.g., 85%) < 1.0 triggers liquidation
// DEEP_LIQUIDATION (HF < 0.95): 100% can be liquidated
// PARTIAL_LIQUIDATION (0.95 ≤ HF < 1.0): 50% max
```

### Liquidation Bonus Calculation

```solidity
collateralToSeize = (debtToCover × debtPrice × liquidationBonus) / collateralPrice

// Example: Liquidating $5,000 USDC debt for ETH @ $2,500 with 5% bonus
// collateralToSeize = ($5,000 × $1 × 1.05) / $2,500 = 2.1 ETH
```

## 🧪 Testing

```bash
# Install dependencies
cd contracts/lending
forge install OpenZeppelin/openzeppelin-contracts
forge install foundry-rs/forge-std

# Run all tests
forge test -vvv

# Run specific test file
forge test --match-path test/Pool.t.sol -vvv

# Run with gas report
forge test --gas-report

# Fuzz testing (extended)
forge test --fuzz-runs 10000

# Coverage report
forge coverage
```

### Test Coverage

| Category            | Tests                                            | Status |
| ------------------- | ------------------------------------------------ | ------ |
| Math Libraries      | WadMul, RayMul, WadDiv, PercentMul, Interest     | ✅      |
| Oracle              | Set/Get Price, Update, TWAP, Staleness           | ✅      |
| Interest Rate Model | Base, Optimal, Above Optimal, Full Utilization   | ✅      |
| Collateral Manager  | Configure, LTV, Caps, E-Mode                     | ✅      |
| Health Factor       | Healthy, At-Risk, Liquidatable, Edge Cases       | ✅      |
| Liquidation         | Partial, Full, Collateral Seizure, Profitability | ✅      |
| Flash Loan          | Fee Calculation, Profitability                   | ✅      |
| Token Mechanics     | Mint, Burn, Scaled Balance, Credit Delegation    | ✅      |
| Fuzz Tests          | Math overflow, HF bounds, Seizure amounts        | ✅      |

## 🚀 Deployment

### Local Development

```bash
# Start local Anvil node
anvil

# Deploy full protocol
forge script script/Deploy.s.sol:DeployLendingProtocol \
  --rpc-url http://localhost:8545 \
  --private-key $PRIVATE_KEY \
  --broadcast
```

### Testnet Deployment

```bash
# Deploy to X3 Testnet
forge script script/Deploy.s.sol:DeployLendingProtocol \
  --rpc-url https://rpc.testnet.x3-chain.io \
  --private-key $PRIVATE_KEY \
  --broadcast \
  --verify

# Configure reserves
forge script script/Deploy.s.sol:ConfigureReserves \
  --rpc-url https://rpc.testnet.x3-chain.io \
  --private-key $PRIVATE_KEY \
  --broadcast
```

### Deployment Order

1. **MathLibraries** (library, linked)
2. **OracleRouter** (set prices)
3. **InterestRateModel** (per-asset type)
4. **CollateralManager** (risk parameters)
5. **Pool** (core lending)
6. **PoolConfigurator** (admin)
7. **AToken** (per reserve)
8. **DebtTokens** (per reserve)
9. **FlashLoanReceivers** (optional strategies)
10. **Wire Permissions** (pool ↔ configurator ↔ tokens)

## ⚠️ Security Considerations

### Implemented Protections

- **Reentrancy Guards**: All external entry points protected
- **Access Control**: Ownable + role-based (POOL_ADMIN, RISK_ADMIN)
- **Timelock**: 2-day delay on parameter changes
- **Oracle Staleness**: Configurable max age (default 1 hour)
- **Fallback Oracles**: Manual override for emergency pricing
- **Health Factor Invariants**: Borrow reverts if would create unhealthy position
- **Flash Loan Repayment Check**: Validates amount + premium returned

### Known Considerations

- **prepare_root**: Commits to inputs only (see H-1 in main pallet)
- **Isolation Mode**: New assets start isolated by default
- **Oracle Manipulation**: TWAP + Chainlink mitigate, but monitor
- **Interest Rate Spikes**: Emergency freeze capability in PoolConfigurator

## 📊 Gas Benchmarks

| Operation          | Gas   | Notes                               |
| ------------------ | ----- | ----------------------------------- |
| deposit()          | ~150k | First deposit higher (storage init) |
| withdraw()         | ~120k |                                     |
| borrow()           | ~180k | Includes health factor check        |
| repay()            | ~130k |                                     |
| liquidationCall()  | ~250k | Seize + transfer                    |
| flashLoan()        | ~80k  | + callback gas                      |
| Health Factor calc | ~3k   | Pure math                           |

## 🔗 Integration with X3 Kernel

The lending protocol integrates with X3 Kernel for cross-chain operations:

```solidity
// Example: Deposit USDC on Polygon, borrow ETH on Arbitrum
ComitPayload[] memory payloads = new ComitPayload[](2);

// Payload 1: Deposit on Polygon
payloads[0] = ComitPayload({
    targetVm: VM.EVM,
    targetChain: 137, // Polygon
    callData: abi.encodeCall(Pool.deposit, (USDC, 10000e6, msg.sender, 0))
});

// Payload 2: Borrow on Arbitrum
payloads[1] = ComitPayload({
    targetVm: VM.EVM,
    targetChain: 42161, // Arbitrum
    callData: abi.encodeCall(Pool.borrow, (WETH, 1e18, 2, 0, msg.sender))
});

// Execute atomically
atlasKernel.submitComit(payloads);
```

## 📝 License

MIT License - see [LICENSE](../../LICENSE)
