# Trading Safety Kernel — Knowledge Core

## Overview

The Trading Safety Kernel (TSK) is the mandatory safety enforcement layer for all trading, arbitrage, and financial operations in the X3 ecosystem. No trading model may recommend mainnet execution unless the route passes the TSK. This document defines the safety checks, the route verdicts, and the absolute rule that governs all trading decisions.

## Absolute Rule

**Profitable does not mean safe.**

A route that is profitable is not necessarily safe. A route that is safe is not necessarily profitable. The TSK evaluates both profitability and safety. If a route is profitable but unsafe, the verdict is REJECT. If a route is safe but unprofitable, the verdict is REJECT. Only routes that are both profitable and safe receive a verdict of EXECUTE or SIMULATE_ONLY.

## Mandatory Safety Checks

No route may be recommended for mainnet execution unless all of the following checks pass:

### Check 1: dry_run

- The route must be simulated off-chain with historical data.
- The simulation must show a positive net EV (gross profit minus all costs).
- The simulation must include gas cost, slippage, liquidity, bridge fees, and MEV exposure.
- If the simulation fails or shows a negative net EV, the route must not be executed.

### Check 2: simulate_before_execute

- The route must be simulated on-chain using `eth_call` or equivalent before execution.
- The simulation must succeed (no revert) and the net profit must be positive.
- The simulation must include gas cost estimation.
- If the simulation reverts or shows a negative net profit, the route must not be executed.

### Check 3: max_trade_size

- The route must have a maximum trade size parameter.
- The trade size must not exceed the maximum, regardless of profitability.
- The maximum trade size must be configurable by the operator.
- Trades that exceed the maximum must be split or rejected.

### Check 4: max_daily_loss

- The route must have a maximum daily loss parameter.
- The cumulative loss for the day must not exceed the maximum, regardless of individual trade profitability.
- The maximum daily loss must be configurable by the operator.
- If the daily loss limit is reached, all trading must halt for the remainder of the day.

### Check 5: max_gas_spend

- The route must have a maximum gas spend parameter.
- The cumulative gas spend for the day must not exceed the maximum.
- The maximum gas spend must be configurable by the operator.
- If the gas spend limit is reached, all trading must halt for the remainder of the day.

### Check 6: max_failed_tx_per_hour

- The route must have a maximum failed transaction count per hour.
- If the failed transaction count exceeds the maximum, trading must halt for a cooldown period.
- The maximum failed transaction count and cooldown period must be configurable by the operator.
- Failed transactions must be analyzed for root cause. Trading must not resume until the root cause is identified and fixed.

### Check 7: min_profit_after_fees

- The route must have a minimum profit threshold after all fees (gas, slippage, bridge fees, MEV, etc.).
- If the net profit is below the threshold, the route must not be executed, regardless of the gross profit.
- The minimum profit threshold must be configurable by the operator and must be positive (greater than zero).

### Check 8: liquidity_depth_check

- The route must verify that there is sufficient liquidity at each step.
- The liquidity depth must be sufficient to absorb the trade without exceeding the slippage limit.
- If the liquidity depth is insufficient, the route must not be executed.
- Liquidity depth must be checked in real time, not from cached data.

### Check 9: slippage_limit

- The route must have a maximum slippage limit.
- If the estimated slippage exceeds the limit, the route must not be executed.
- The slippage limit must be configurable by the operator.
- Slippage must account for cross-VM and cross-chain latency.

### Check 10: private_route_preference

- The route must prefer private routing (Flashbots, MEV-Boost, X3 Secure RPC) over public routing.
- If private routing is available and the route can be executed privately, it must be.
- If private routing is not available, the route must account for the additional MEV exposure in the risk score.

### Check 11: nonce_manager

- The route must use a nonce manager to prevent transaction replay and ordering issues.
- Nonces must be monotonically increasing and must not be reused.
- For cross-chain routes, the nonce must include the source chain ID and destination chain ID.

### Check 12: rpc_health_check

- The route must verify that the RPC endpoint is healthy before execution.
- If the RPC endpoint is slow, unreliable, or returning stale data, the route must not be executed.
- RPC health must be checked before every execution, not just at startup.

### Check 13: emergency_pause

- The route must have an emergency pause mechanism.
- The pause must be callable by a guardian role.
- The pause must halt all new trades immediately.
- The pause must not block in-flight trades from completing.
- The pause must log an event and alert the operator.

### Check 14: PnL_logging

- Every trade must be logged with:
  - Timestamp
  - Route
  - Trade size
  - Gross profit
  - Gas cost
  - Slippage cost
  - Bridge fees
  - MEV cost (if applicable)
  - Net profit
  - Success or failure
  - Failure reason (if failed)
- PnL logs must be stored on-chain (via events) and off-chain (via a monitoring system).
- PnL logs must be auditable. The total profit and loss must match the on-chain balance changes.

### Check 15: explicit_rejection_reasons

- Every rejected route must have an explicit rejection reason.
- Rejection reasons must be logged and auditable.
- Rejection reasons must be actionable. The operator must be able to fix the issue or adjust the parameters.

## Route Verdicts

The TSK evaluates each route and produces one of the following verdicts:

| Verdict | Meaning | Action |
|---------|---------|--------|
| `EXECUTE` | The route is both profitable and safe. All safety checks pass. | Execute the route on mainnet. |
| `SIMULATE_ONLY` | The route passes safety checks but the profit is marginal or the risk is elevated. | Simulate the route on-chain but do not execute. |
| `WATCH` | The route is close to passing but one or more checks are borderline. | Monitor the route for future opportunities but do not execute. |
| `REJECT_LOW_PROFIT` | The route is safe but the net profit is below the minimum threshold. | Do not execute. Log the rejection reason. |
| `REJECT_BAD_FINALITY` | The route requires finality on a chain that has not reached the required confirmation depth. | Do not execute. Wait for finality. |
| `REJECT_BRIDGE_RISK` | The route involves a bridge that has unacceptable risk (custody, proof, timeout). | Do not execute. Log the rejection reason. |
| `REJECT_SLIPPAGE` | The estimated slippage exceeds the slippage limit. | Do not execute. Adjust the trade size or route. |
| `REJECT_NO_LIQUIDITY` | There is insufficient liquidity at one or more steps in the route. | Do not execute. Monitor for liquidity changes. |
| `REJECT_MEV_EXPOSURE` | The route has unacceptable MEV exposure (public routing, no privacy, high front-running risk). | Do not execute. Use private routing or adjust the route. |
| `REJECT_UNSAFE_CONTRACT` | One or more contracts in the route are unaudited, have known vulnerabilities, or are not on the approved list. | Do not execute. Audit the contract or find an alternative route. |
| `REJECT_UNKNOWN_PROOF` | The route requires a proof type that is not supported or not verified (e.g., unverified bridge proof). | Do not execute. Verify the proof or find an alternative route. |
| `REJECT_NO_REFUND_PATH` | The route does not have a deterministic refund path in case of failure. | Do not execute. Ensure the route has a refund path. |

## Verdict Decision Flow

```
1. Run dry_run simulation.
   - Fails? -> REJECT (with reason from simulation)

2. Run simulate_before_execute on-chain.
   - Reverts? -> REJECT (with revert reason)

3. Check max_trade_size.
   - Exceeds? -> REJECT_LOW_PROFIT or split the trade

4. Check max_daily_loss.
   - Exceeded? -> REJECT_LOW_PROFIT (daily loss limit reached)

5. Check max_gas_spend.
   - Exceeded? -> REJECT_LOW_PROFIT (gas spend limit reached)

6. Check max_failed_tx_per_hour.
   - Exceeded? -> REJECT (with reason: too many failed transactions)

7. Check min_profit_after_fees.
   - Below threshold? -> REJECT_LOW_PROFIT

8. Check liquidity_depth.
   - Insufficient? -> REJECT_NO_LIQUIDITY

9. Check slippage_limit.
   - Exceeded? -> REJECT_SLIPPAGE

10. Check finality.
    - Not reached? -> REJECT_BAD_FINALITY

11. Check bridge_risk.
    - Unacceptable? -> REJECT_BRIDGE_RISK

12. Check MEV_exposure.
    - Unacceptable? -> REJECT_MEV_EXPOSURE

13. Check contract_safety.
    - Unsafe? -> REJECT_UNSAFE_CONTRACT

14. Check proof_requirements.
    - Unknown? -> REJECT_UNKNOWN_PROOF

15. Check refund_path.
    - Missing? -> REJECT_NO_REFUND_PATH

16. All checks pass?
    - Yes, with high confidence? -> EXECUTE
    - Yes, with marginal profit or elevated risk? -> SIMULATE_ONLY
    - Close to passing? -> WATCH
```

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — The TSK enforces architectural constraints (finality, proof, refund paths).
- **UNIVERSAL_ASSET_KERNEL.md** — The TSK ensures that trading operations preserve the canonical supply invariant.
- **CROSS_VM_ROUTING.md** — The TSK evaluates route specifications for safety and profitability.
- **ARBITRAGE_PLAYBOOK.md** — Arb strategies must pass the TSK before execution.
- **FLASHLOAN_SAFETY.md** — Flashloan strategies must pass both the TSK and flashloan safety checks.
- **MEV_DEFENSE.md** — The TSK includes MEV risk scoring in its evaluation.
- **MAINNET_READINESS.md** — The TSK must be deployed and tested before mainnet.
- **FORBIDDEN_PATTERNS.md** — The TSK must reject routes that involve forbidden patterns.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*