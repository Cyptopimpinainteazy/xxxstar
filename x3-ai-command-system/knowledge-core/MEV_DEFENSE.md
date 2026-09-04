# MEV Defense — Knowledge Core

## Overview

Maximal Extractable Value (MEV) refers to the profit that can be extracted by manipulating transaction ordering within a block. On X3, MEV is a reality across all VMs — EVM, SVM, X3VM, and cross-VM routes. This document defines the mandatory defensive measures that all X3 trading, bridging, and arb systems must implement, and explicitly prohibits malicious MEV practices.

## Defensive Measures

### Measure MEV-1: Private Routing

All transactions that could be front-run must use private routing:

- **Flashbots Protect**: Submit transactions to Flashbots Protect RPC to prevent mempool exposure.
- **MEV-Boost**: Use MEV-Boost for Ethereum transactions to access a competitive builder market while benefiting from MEV-Share refunds.
- **X3 Secure RPC**: Use X3's private RPC endpoint for X3-native transactions. Transactions submitted via the secure RPC are not broadcast to the public mempool.
- **Solana**: Use Jupiter's private routing or Solana's priority fees with secure RPC.
- **Cosmos**: Use the CosmWasm private mempool or direct submission to validators.

Transactions that are not routed privately must be assumed to be visible to MEV searchers. Do not submit transactions that could be front-run via public RPC.

### Measure MEV-2: Bundle Simulation

Before submitting a transaction (or a bundle of transactions), simulate it:

- Use `eth_call` or `debug_traceTransaction` to simulate the transaction on the current state.
- Verify that the simulation succeeds and the expected profit is realized.
- Simulate with multiple state variants (current block, next block with potential state changes) to estimate MEV risk.
- If the simulation shows that the transaction could be sandwiched (price moves significantly after the trade), consider using private routing or adjusting the slippage.
- Bundle simulation must include gas cost estimation. A bundle that is profitable before gas but unprofitable after gas must not be submitted.

### Measure MEV-3: Slippage Protection

Every trade must have slippage protection:

- Set a maximum slippage percentage (e.g., 0.5% for stablecoins, 1-2% for volatile assets, 5% for long-tail assets).
- Use `amountOutMinimum` (EVM) or equivalent minimum output parameters (SVM, CosmWasm).
- Slippage protection must account for cross-VM latency. A cross-VM trade takes longer to settle, so slippage must be wider.
- Slippage protection must account for bridge latency. A cross-chain trade has significant latency, so slippage must be even wider.
- Do not set slippage to 100%. A 100% slippage means the trade will execute at any price, which is equivalent to no slippage protection.

### Measure MEV-4: Revert Protection

Transactions that revert cost gas without producing any benefit. To minimize revert costs:

- Simulate every transaction before submission. If the simulation reverts, do not submit the transaction.
- Use `eth_call` to check for revert conditions. If the call reverts, fix the condition before submitting.
- Use Flashbots bundles to ensure that reverted transactions are not included in the block (Flashbots does not charge for reverted bundles).
- For X3-native transactions, use the secure RPC to submit transactions that will not revert.
- Keep a log of all reverted transactions and their causes. Analyze patterns to prevent future reverts.

### Measure MEV-5: Transaction Privacy

Transaction privacy is critical for MEV defense:

- Do not submit transactions to the public mempool if they reveal trading intent (price, amount, route).
- Use encrypted mempools (if available) for sensitive transactions.
- Use private routing (Measure MEV-1) for all arbitrage and liquidation transactions.
- For governance transactions, consider using commit-reveal schemes to prevent vote manipulation.
- For cross-VM transactions, use X3's secure cross-VM message passing, which does not expose the transaction content to the public mempool.

### Measure MEV-6: Anti-Sandwich Execution

Sandwich attacks front-run a victim's trade (buy before) and back-run it (sell after) to extract value. Defenses:

- Use private routing for all trades (Measure MEV-1).
- Set slippage limits to prevent large price impacts (Measure MEV-3).
- Split large trades into smaller trades over multiple blocks to reduce price impact.
- Use TWAP (time-weighted average price) or VWAP (volume-weighted average price) strategies for large trades.
- Use DEX aggregators that split trades across multiple pools to reduce per-pool price impact.
- Monitor the mempool for potential sandwich attacks. If a sandwich is detected, cancel or adjust the trade.
- For X3-native cross-VM trades, use atomic execution within a single block to prevent sandwiching between VMs.

### Measure MEV-7: MEV-Aware Risk Scoring

Every route must be scored for MEV risk:

- **Low risk**: Private routing, atomic execution, no mempool exposure (e.g., cross-VM arb on X3 with secure RPC).
- **Medium risk**: Semi-private routing, short latency, limited mempool exposure (e.g., same-VM DEX arb with Flashbots).
- **High risk**: Public routing, long latency, full mempool exposure (e.g., cross-chain arb with bridge latency).
- **Critical risk**: Public routing, no slippage protection, long latency, no revert protection (e.g., unprotected cross-chain arb).

Routes with critical MEV risk must not be executed. Routes with high MEV risk must be approved by the operator before execution.

### Measure MEV-8: Defensive Monitoring

All MEV-exposed systems must have monitoring:

- Monitor the mempool for transactions that could be front-running or sandwiching your trades.
- Monitor on-chain events for sandwich attacks (transactions before and after your trade on the same asset pair).
- Monitor PnL for unexpected losses that could be caused by MEV extraction.
- Alert the operator if MEV extraction is detected.
- Maintain a dashboard of MEV-related metrics: front-running frequency, sandwich attack frequency, revert rate, gas cost due to MEV, PnL impact.

## Prohibited MEV Practices

The following MEV practices are **explicitly prohibited** in the X3 ecosystem:

1. **Malicious sandwich attacks**: Front-running and back-running a victim's trade to extract value at the victim's expense.
2. **User exploitation**: Using privileged access (e.g., mempool visibility, RPC access) to extract value from users.
3. **Theft**: Using MEV to steal funds from users or protocols.
4. **Phishing**: Using MEV to trick users into signing malicious transactions.
5. **Approval draining**: Using MEV to exploit token approvals that users have granted to compromised contracts.
6. **Front-running governance votes**: Using MEV to manipulate governance outcomes.
7. **Cross-chain MEV exploitation**: Using bridge latency to exploit users on either side of a bridge.

## Allowed MEV Practices

The following MEV practices are **allowed** in the X3 ecosystem:

1. **Defensive audits**: Identifying and mitigating MEV vulnerabilities in X3 protocols.
2. **Testnet exploit reproduction**: Reproducing MEV attacks on testnet to understand and fix vulnerabilities.
3. **Invariant testing**: Testing MEV invariants (e.g., "no sandwich attack can extract more than X% of the trade value").
4. **Simulation**: Simulating MEV scenarios to understand risk and develop defenses.
5. **MEV defense**: Implementing the defensive measures defined in this document.
6. **Private routing for protection**: Using private RPCs to protect user transactions from MEV.
7. **Legal arbitrage**: Exploiting legitimate price differences between markets without harming users (same-chain DEX arb, triangular arb, protocol-allowed liquidations).
8. **Protocol-permitted liquidations**: Liquidating undercollateralized positions as designed by the protocol.
9. **Risk-controlled flashloan arbitrage**: Using flashloans for arbitrage with all safety measures in place (see FLASHLOAN_SAFETY.md).

## Relationship to Other Knowledge Core Documents

- **ARBITRAGE_PLAYBOOK.md** — Arb strategies must use MEV defense measures.
- **TRADING_SAFETY_KERNEL.md** — The Trading Safety Kernel includes MEV risk scoring.
- **FLASHLOAN_SAFETY.md** — Flashloan transactions must use private routing and MEV defense.
- **EVM_RULES.md** — EVM contracts must implement slippage and deadline checks (MEV defense).
- **CROSS_VM_ROUTING.md** — Cross-VM routes must account for MEV exposure.
- **FORBIDDEN_PATTERNS.md** — Malicious MEV practices are explicitly forbidden.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*