# X3 Trading Safety Kernel

No trading model may recommend mainnet execution unless the route has all of these controls:

## Required Controls

1. **dry_run mode** — No mainnet execution without dry-run first
2. **simulate_before_execute** — Simulate every route before signing
3. **max_trade_size** — Cap per-trade exposure
4. **max_daily_loss** — Circuit breaker on daily loss
5. **max_gas_spend** — Cap gas expenditure
6. **max_failed_tx_per_hour** — Stop after N failed transactions
7. **min_profit_after_fees** — Minimum net profit threshold (gas + fees + slippage + latency risk)
8. **liquidity_depth_check** — Verify sufficient liquidity before execution
9. **slippage_limit** — Maximum acceptable slippage
10. **private_route_preference** — Use private relay where available (Flashbots, MEV-Boost, secure RPC)
11. **nonce_manager** — Prevent nonce conflicts and stuck transactions
12. **RPC health check** — Verify RPC availability before routing
13. **emergency_pause** — Kill switch for all trading activity
14. **PnL logging** — Log every trade, profit, loss, gas, slippage
15. **explicit route rejection reasons** — Log why routes were rejected

## Route Verdicts

Every route analysis must produce one of these verdicts:

- **EXECUTE** — All checks pass, net EV positive, risks acceptable
- **SIMULATE_ONLY** — Route looks promising but needs simulation verification
- **WATCH** — Route is borderline, monitor for better conditions
- **REJECT_LOW_PROFIT** — Net EV below minimum threshold after all costs
- **REJECT_BAD_FINALITY** — Finality risk too high for the route
- **REJECT_BRIDGE_RISK** — Bridge custody/settlement risk unacceptable
- **REJECT_SLIPPAGE** — Expected slippage exceeds limit
- **REJECT_NO_LIQUIDITY** — Insufficient liquidity at expected price
- **REJECT_MEV_EXPOSURE** — MEV exposure risk too high
- **REJECT_UNSAFE_CONTRACT** — Contract has known vulnerabilities
- **REJECT_UNKNOWN_PROOF** — Proof requirements unclear or unverifiable
- **REJECT_NO_REFUND_PATH** — No timeout/refund path in case of failure

## Absolute Rule

**Profitable does not mean safe.**

Every route must be evaluated on risk-adjusted net EV, not gross profit.

## Execution Modes

Trading bots must progress through these stages in order:

1. **DRY_RUN** — No real transactions, simulation only
2. **SIM_ONLY** — Quote and simulate, no signing
3. **PAPER_TRADE** — Track fake balance, no real execution
4. **SMALL_CAP** — Tiny real trades with tight limits
5. **GUARDED_MAIN** — Full mainnet with circuit breakers and max loss
6. **FULL_AUTO** — Only after proven logs, audits, and operator approval

**Never skip stages.** Bots that jump to FULL_AUTO become donation machines.

## Forbidden Trading Patterns

The X3 AI model pack must never produce:

- Malicious sandwich bots targeting retail users
- Approval-draining contracts
- Honeypot tokens
- Fake WETH or asset clones
- Rug-pull mechanics
- Hidden tax tokens
- Phishing approval flows
- Unauthorized exploit execution
- DAO vote hijacking tools
- Bridge-drain exploit bots
- User-targeted MEV extraction

## Allowed Trading Patterns

- Legal same-chain DEX arbitrage
- Legal triangular arbitrage
- Protocol-permitted liquidations
- Risk-controlled flashloan arbitrage
- Cross-VM X3 atomic arbitrage (when X3 coordinates finality)
- MEV defense (private routing, slippage protection)
- Inventory rebalancing
- CEX/DEX basis research (not execution without all controls)

## Monitoring Requirements

Every trading system must have:

- Real-time PnL dashboard
- Failure rate tracking
- Route rejection logging with reasons
- Gas spend tracking
- Emergency pause status
- RPC health monitoring
- Position/exposure tracking
- Circuit breaker status