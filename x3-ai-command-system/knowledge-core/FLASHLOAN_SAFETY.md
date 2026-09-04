# Flashloan Safety — Knowledge Core

## Overview

Flashloans are a powerful tool for capital-efficient arbitrage, liquidations, and DeFi composability. They are also a significant attack vector when used improperly. This document defines the mandatory safety rules for all flashloan usage in the X3 ecosystem. Every flashloan operation — whether for arbitrage, liquidation, or any other purpose — must comply with these rules.

## Core Principle

A flashloan is a borrow-and-repay within a single transaction. If the repayment fails, the entire transaction reverts, and no state change persists. This atomicity is the primary safety property of flashloans. It must never be compromised.

## Rule FL-1: Borrow Only From Explicit Providers

- Flashloans must only be borrowed from known, audited flashloan providers.
- Approved providers include: Aave, DyDx, Uniswap V3 (flash callbacks), and X3-native flashloan modules that have been audited.
- Do not borrow from unknown or unaudited contracts. A malicious flashloan provider can manipulate the callback to steal funds.
- The provider address must be hardcoded or configured via governance. Do not accept the provider address as a user-supplied parameter.
- If a new provider is added, it must be audited and approved by governance before use.

## Rule FL-2: Validate the Initiator

- The flashloan callback must validate that the initiator is the expected contract (the flashloan provider).
- Do not assume that `msg.sender` in the callback is the flashloan provider. It could be a malicious contract calling the callback directly.
- Compare `msg.sender` against the known flashloan provider address.
- If the initiator is not the expected provider, revert the transaction.

## Rule FL-3: Validate the Callback

- The flashloan callback must validate all parameters: token, amount, route, deadline, slippage, and profit.
- **Token**: Verify that the borrowed token is the expected token. Do not accept arbitrary tokens.
- **Amount**: Verify that the borrowed amount is within the expected range. Do not accept amounts that are too large (could cause excessive slippage) or too small (could result in unprofitable execution).
- **Route**: Verify that the execution route matches the expected route. Do not accept arbitrary routes from user input.
- **Deadline**: Verify that the transaction is within the expected time window. Do not accept transactions with expired deadlines.
- **Slippage**: Verify that the slippage is within the expected range. Do not accept transactions with excessive slippage.
- **Profit**: Verify that the expected profit is above the minimum threshold. Do not execute unprofitable flashloans.

## Rule FL-4: Simulate Before Execute

- Before executing a flashloan on-chain, simulate it off-chain using `eth_call` or equivalent.
- Verify that the simulation succeeds (the flashloan is repaid) and the net profit is positive.
- If the simulation fails, do not execute the flashloan. Analyze the failure and fix the strategy.
- If the simulation succeeds but the net profit is below the minimum threshold, do not execute.
- Simulation must include gas cost estimation. A flashloan that is profitable before gas but unprofitable after gas must not be executed.

## Rule FL-5: Revert If Below Threshold

- If the flashloan execution results in a net profit below the minimum threshold, the transaction must revert.
- The minimum threshold must be configured by the operator and must be positive (greater than zero).
- The threshold must account for gas cost, slippage, and any fees charged by the flashloan provider.
- Do not execute flashloans with a zero or negative net profit, even if the repayment succeeds.

## Rule FL-6: Reentrancy Protection

- Flashloan callbacks are inherently reentrant. The callback function must be protected with `ReentrancyGuard` or equivalent.
- Do not make external calls from the flashloan callback to untrusted contracts.
- If the callback must call an external contract (e.g., a DEX), verify the contract address against a whitelist.
- Do not modify state after the callback returns. All state modifications must be completed before the callback.
- Use the checks-effects-interactions pattern: validate parameters, execute the strategy, then repay the flashloan.

## Rule FL-7: Emit Events

- Every flashloan operation must emit events for:
  - `FlashloanBorrowed(address indexed provider, address indexed token, uint256 amount)`
  - `FlashloanExecuted(address indexed strategy, uint256 profit, uint256 gasCost)`
  - `FlashloanRepaid(address indexed provider, address indexed token, uint256 amount)`
  - `FlashloanFailed(address indexed provider, address indexed token, uint256 amount, string reason)`
- Events must be emitted before the repayment (for success) or in the revert reason (for failure).
- Events must be consumable by off-chain monitors and PnL trackers.

## Rule FL-8: No Unpaid Debt Paths

- There must be no code path where the flashloan is not repaid.
- The repayment must be the last action in the transaction. All other actions must be completed before repayment.
- If any step before the repayment fails, the entire transaction must revert. This is the flashloan's atomicity guarantee.
- Do not use `try/catch` to suppress errors in the execution path. If an error occurs, let the transaction revert.
- Do not transfer funds out of the contract before repaying the flashloan, unless the transfer is part of the strategy and the repayment is guaranteed.

## Rule FL-9: Max Loss and Pause Controls

- Every flashloan strategy must have a maximum loss parameter. If the strategy loses more than this amount (including gas), it must not execute.
- Every flashloan strategy must have a pause mechanism. If the strategy is paused, no flashloans may be executed.
- The pause mechanism must be callable by a guardian role. The guardian can pause but cannot steal.
- The max loss parameter must be configurable by the operator.
- The pause mechanism must not block flashloan repayments. If a flashloan is in progress and the strategy is paused, the repayment must still succeed.

## Rule FL-10: PnL Logging

- Every flashloan execution must be logged with:
  - Timestamp
  - Strategy name
  - Borrowed token and amount
  - Execution route
  - Gross profit (before gas and fees)
  - Gas cost
  - Flashloan provider fee
  - Net profit (after gas and fees)
  - Success or failure
  - Failure reason (if failed)
- PnL logs must be stored on-chain (via events) and off-chain (via a monitoring system).
- PnL logs must be auditable. The total profit and loss must match the on-chain balance changes.

## Flashloan Attack Vectors and Defenses

| Attack | Description | Defense |
|--------|-------------|---------|
| Price manipulation | Borrow large amount, manipulate oracle price, profit from mispricing | Use TWAP oracles, validate prices against multiple sources |
| Reentrancy via callback | Malicious callback re-enters the strategy contract | ReentrancyGuard, whitelist external calls |
| Unpaid debt | Strategy fails to repay the flashloan | Atomic revert, no try/catch on repayment |
| Sandwitch attack | Attacker front-runs the flashloan | Private routing (Flashbots/MEV-Boost), slippage protection |
| Governance attack | Attacker changes flashloan parameters | Timelocked governance, guardian pause |
| Flashloan + governance | Attacker borrows to manipulate governance vote | Block-list flashloan borrowers from governance during voting period |

## Relationship to Other Knowledge Core Documents

- **ARBITRAGE_PLAYBOOK.md** — Flashloan-based arb must follow the execution stages defined in the playbook.
- **TRADING_SAFETY_KERNEL.md** — Flashloan strategies must pass the Trading Safety Kernel before execution.
- **MEV_DEFENSE.md** — Flashloan transactions must use private routing and MEV defense.
- **EVM_RULES.md** — Flashloan contracts must comply with EVM rules (reentrancy, events, access control).
- **SVM_RULES.md** — SVM flashloan programs must comply with SVM rules (account validation, CPI safety).
- **FORBIDDEN_PATTERNS.md** — Using flashloans for malicious purposes (manipulation, theft) is forbidden.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*