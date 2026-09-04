# Arbitrage Playbook — Knowledge Core

## Overview

This document defines the types of arbitrage opportunities available on X3, the risk model for each, and the execution stages that must be followed. All arbitrage strategies on X3 must comply with the Trading Safety Kernel and must never cross into forbidden territory (see FORBIDDEN_PATTERNS.md).

## Arbitrage Types

### Type 1: Same-Chain DEX Arbitrage

Buy an asset on one DEX pool and sell it on another DEX pool on the same chain.

- **Example**: Buy ETH on Uniswap V3, sell ETH on SushiSwap, both on X3's EVM.
- **Atomicity**: Potentially atomic within a single transaction (flashloan + multi-DEX swap).
- **Risk**: Low (if executed atomically). Gas cost must be less than profit.
- **Model requirements**: Gas cost, slippage on both pools, price impact, liquidity depth, MEV exposure (front-running on the same chain).

### Type 2: Triangular Arbitrage

Trade through three or more assets in a cycle to capture a price discrepancy.

- **Example**: ETH -> USDC -> WBTC -> ETH, where the cycle yields more ETH than started.
- **Atomicity**: Potentially atomic within a single transaction on the same chain.
- **Risk**: Low to medium. Slippage compounds across multiple pools. Gas cost scales with the number of hops.
- **Model requirements**: Gas cost (per hop), slippage (per hop, compounding), liquidity depth (per pool), price impact (cumulative), MEV exposure.

### Type 3: Cross-DEX Route Arbitrage

Exploit price differences between DEXs across different routes on the same chain.

- **Example**: Buy on Curve, sell on Uniswap V3, on X3's EVM.
- **Atomicity**: Potentially atomic within a single transaction.
- **Risk**: Low to medium. Different DEXs have different fee structures, slippage models, and gas costs.
- **Model requirements**: Gas cost, slippage per DEX, liquidity depth per DEX, fee structure per DEX, MEV exposure.

### Type 4: Cross-VM X3 Atomic Arbitrage

Exploit price differences between assets on different VMs within X3's consensus boundary.

- **Example**: Buy ETH on X3's EVM DEX, sell on X3's SVM DEX, within a single X3 block.
- **Atomicity**: Coordinated atomic via X3's cross-VM message passing.
- **Risk**: Medium. Cross-VM routing adds latency, and the atomicity depends on X3's coordination layer.
- **Model requirements**: Gas/compute cost per VM, slippage per VM, liquidity depth per VM, cross-VM routing latency, X3 block time, MEV exposure (cross-VM MEV is harder but not impossible).

### Type 5: Cross-Chain Delayed-Settlement Arbitrage

Exploit price differences between assets on different chains, settled via bridges.

- **Example**: Buy ETH on X3, bridge to Ethereum L1, sell on Uniswap V3 on L1.
- **Atomicity**: Not atomic. Requires finality on both chains. Subject to bridge latency, finality delays, and bridge risk.
- **Risk**: High. Bridge latency means the price may move against the arb during settlement. Bridge risk means the funds may be stuck or lost.
- **Model requirements**: Gas cost (both chains), bridge fees, slippage (both chains), bridge latency, finality delay (both chains), bridge risk (custody, proof, timeout), price drift during settlement, MEV exposure on both chains.

### Type 6: CEX/DEX Basis Arbitrage

Exploit price differences between centralized exchanges and on-chain DEXs.

- **Example**: Buy ETH on Binance, withdraw to X3, sell on X3's DEX.
- **Atomicity**: Not atomic. Requires CEX withdrawal, on-chain transaction, and finality.
- **Risk**: Medium to high. CEX withdrawal latency, on-chain gas cost, slippage, counterparty risk (CEX), and price drift.
- **Model requirements**: CEX fees, withdrawal latency, on-chain gas cost, slippage, liquidity depth, CEX counterparty risk, price drift during withdrawal.

### Type 7: Protocol-Allowed Liquidations

Liquidate undercollateralized positions on lending protocols where liquidations are a designed feature.

- **Example**: Liquidate a margin position on an X3 lending protocol and claim the liquidation bonus.
- **Atomicity**: Potentially atomic within a single transaction.
- **Risk**: Low to medium. Gas cost, slippage on the collateral sale, and competition from other liquidators.
- **Model requirements**: Gas cost, liquidation bonus, slippage on collateral sale, competition (MEV), health factor of the position, protocol-specific rules.

### Type 8: Funding-Rate Arbitrage

Capture the difference between spot price and perpetual futures funding rate.

- **Example**: Go long on spot, go short on perps, earn the funding rate.
- **Atomicity**: Not atomic. Requires managing two positions on different venues.
- **Risk**: Medium. Funding rates change, positions need maintenance, and liquidation risk on the short side.
- **Model requirements**: Funding rate history, spot slippage, perp slippage, margin requirements, liquidation risk, position management cost, gas cost.

## Cost Model

Every arbitrage opportunity must model the following costs:

| Cost | Description | When It Applies |
|------|-------------|----------------|
| Gas | Transaction gas/compute cost | All on-chain transactions |
| Slippage | Price impact of the trade on the pool | All DEX trades |
| Liquidity | Depth of the order book or pool | All trades |
| Bridge fees | Fees charged by the bridge for cross-chain transfers | Cross-chain routes |
| Finality delay | Time to wait for confirmation on source and destination | Cross-chain routes |
| Bridge risk | Probability of bridge failure, hack, or timeout | Cross-chain routes |
| MEV | Probability of being front-run or sandwiched | All on-chain transactions |
| Price drift | Price movement during settlement | Delayed-settlement routes |
| Failure cost | Cost of a failed transaction (gas lost) | All transactions |
| Inventory | Cost of holding inventory between trades | CEX/DEX, funding rate |

## Net Expected Value

The net expected value (EV) of an arbitrage opportunity must be calculated as:

```
net_EV = gross_profit - gas_cost - slippage_cost - bridge_fees - expected_MEV_cost - expected_failure_cost - inventory_cost
```

If `net_EV < min_profit_after_fees`, the opportunity must be rejected (see TRADING_SAFETY_KERNEL.md).

## Execution Stages

No arbitrage execution may skip stages. The stages are:

### Stage 1: DRY_RUN

- Simulate the opportunity off-chain with historical data.
- Calculate the net EV using the cost model.
- Verify that all routes are valid and all parameters are within bounds.
- No on-chain transactions. No real funds at risk.

### Stage 2: SIM_ONLY

- Simulate the opportunity on-chain using `eth_call` or equivalent.
- Verify that the simulation succeeds and the net EV is positive.
- No real transactions. Gas estimation is performed but not consumed.

### Stage 3: PAPER_TRADE

- Execute the opportunity with paper trading (record the trade but do not send real funds).
- Track the hypothetical PnL over time.
- Verify that the paper PnL matches the simulated PnL.

### Stage 4: SMALL_CAP

- Execute the opportunity with a small amount of real funds (e.g., 1% of the target capital).
- Verify that the real PnL matches the simulated and paper PnL.
- Verify that all safety checks pass (slippage, deadline, max loss, etc.).

### Stage 5: GUARDED_MAIN

- Execute the opportunity with the target capital but with all safety checks enabled:
  - Max trade size
  - Max daily loss
  - Max gas spend
  - Max failed transactions per hour
  - Min profit after fees
  - Slippage limit
  - Private route preference
  - Emergency pause
- Monitor the PnL in real time.
- If any safety check is triggered, halt execution and alert the operator.

### Stage 6: FULL_AUTO

- Execute the opportunity with the target capital and all safety checks enabled.
- Automated monitoring, alerting, and emergency pause.
- Continuous PnL logging.
- Regular re-evaluation of the strategy's profitability and risk.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Arb strategies must respect the canonical supply invariant and cross-VM routing constraints.
- **UNIVERSAL_ASSET_KERNEL.md** — Cross-chain arb must not create phantom assets or violate the UAK.
- **CROSS_VM_ROUTING.md** — Cross-VM arb must follow route specifications and account for finality delays.
- **TRADING_SAFETY_KERNEL.md** — All arb strategies must pass the Trading Safety Kernel before execution.
- **FLASHLOAN_SAFETY.md** — Flashloan-based arb must comply with flashloan safety rules.
- **MEV_DEFENSE.md** — Arb strategies must use private routing and MEV defense.
- **FORBIDDEN_PATTERNS.md** — Malicious MEV, sandwich attacks, and user exploitation are forbidden.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*