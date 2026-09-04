# Mainnet Readiness — Knowledge Core

## Overview

This document defines the mandatory readiness checklist for X3 mainnet deployment. No component, contract, program, runtime, or system may be deployed to mainnet unless all items in this checklist are verified. This checklist is the final gate before mainnet. It is non-negotiable and cannot be waived.

## Checklist

### 1. All Tests Passing

- [ ] Unit tests: 100% pass rate. No skipped tests. No commented-out tests.
- [ ] Integration tests: 100% pass rate. All cross-VM and cross-chain interactions tested.
- [ ] Invariant tests: All critical invariants verified (canonical supply, balance, access control, replay protection).
- [ ] Fuzz tests: No crashes, no panics, no unexpected states after minimum 10,000 iterations per function.
- [ ] Stress tests: System performs correctly under high load (maximum block capacity, maximum transaction throughput).
- [ ] Upgrade tests: Runtime and contract upgrades succeed without breaking state.
- [ ] Migration tests: Storage migrations produce correct state from previous versions.
- [ ] End-to-end tests: Full user flows (bridge, swap, stake, govern) work correctly on testnet.

### 2. All Invariants Verified

- [ ] **Canonical supply invariant**: `canonical_supply == native + evm + svm + x3vm + cosmwasm + btc_locked + external_locked + pending`. Verified on-chain for every block.
- [ ] **No phantom minting**: No asset can be minted without a corresponding lock or native creation.
- [ ] **No unbacked bridging**: No wrapped token can be minted without a corresponding lock on the source chain.
- [ ] **No double-counting**: No asset can appear in two terms simultaneously.
- [ ] **No stuck pending**: The `pending` term must converge to zero. No route may remain in pending state indefinitely.
- [ ] **Access control invariant**: No unauthorized address can call a restricted function.
- [ ] **Replay protection invariant**: No nonce can be used twice.
- [ ] **Finality invariant**: No cross-chain asset is considered final until the source chain has reached finality AND the destination chain has confirmed receipt.

### 3. All Audits Complete

- [ ] Internal audit: Completed by the X3 security team. All findings addressed.
- [ ] External audit: Completed by at least one independent security auditor. All findings addressed or explicitly accepted with documented risk.
- [ ] Formal verification: Critical invariants (canonical supply, access control, replay protection) are formally verified or proven.
- [ ] Bug bounty: Active bug bounty program with defined scope, rewards, and disclosure policy.
- [ ] Audit reports: Published and accessible. No redacted findings.

### 4. No TODOs in Critical Paths

- [ ] No `TODO`, `FIXME`, `HACK`, `XXX`, or `UNIMPLEMENTED` comments in any code path that handles funds, access control, consensus, bridging, or staking.
- [ ] No placeholder implementations in production code. Every function must have a real implementation.
- [ ] No mock contracts in production paths. Test mocks are acceptable in test code only.
- [ ] No disabled security checks. Every `require`, `ensure`, `assert`, and access control check is active.
- [ ] No hardcoded test values in production code (test addresses, test keys, test amounts).

### 5. No Mocks in Production Paths

- [ ] No mock oracle in production. Use the real oracle with real price feeds.
- [ ] No mock bridge in production. Use the real bridge with real custody and real proofs.
- [ ] No mock token in production. Use the real token with real supply and real accounting.
- [ ] No mock RPC in production. Use the real RPC with real node connectivity.
- [ ] No mock finality in production. Use real finality with real confirmation counts.

### 6. Deployment Addresses Defined

- [ ] All contract addresses are predefined and documented.
- [ ] All program IDs are predefined and documented.
- [ ] All RPC endpoints are predefined and documented.
- [ ] All custody addresses are predefined and documented.
- [ ] All oracle addresses are predefined and documented.
- [ ] All governance addresses are predefined and documented.
- [ ] No deployment uses dynamic address discovery without a verified on-chain registry.

### 7. Secrets Managed

- [ ] No private keys, mnemonics, or RPC tokens in the repository.
- [ ] All secrets are stored in a secure vault (HashiCorp Vault, AWS Secrets Manager, etc.).
- [ ] All secrets are rotated on a regular schedule.
- [ ] All secrets are accessible only to authorized personnel and services.
- [ ] All secrets are backed up and recoverable.
- [ ] `.env` files are in `.gitignore`. No `.env` files are committed to the repository.

### 8. Monitoring Active

- [ ] On-chain monitoring: All events are indexed and monitored in real time.
- [ ] Node monitoring: All validator and node health metrics are monitored.
- [ ] Bridge monitoring: All bridge operations (lock, release, mint, burn, refund) are monitored.
- [ ] UAK monitoring: The canonical supply invariant is checked on every block. Any violation triggers an alert.
- [ ] PnL monitoring: All trading PnL is tracked and logged.
- [ ] MEV monitoring: All sandwich attacks, front-running, and other MEV events are monitored.
- [ ] Gas monitoring: Gas costs are tracked and anomalies are alerted.
- [ ] Finality monitoring: Cross-chain finality is monitored. Delayed finality triggers an alert.

### 9. Alerting Configured

- [ ] Critical alerts: Bridge hack, supply invariant violation, unauthorized access, node downtime.
- [ ] High alerts: Unusual bridge activity, price oracle deviation, gas spike, MEV attack.
- [ ] Medium alerts: Test failure, deployment error, config drift.
- [ ] Low alerts: Performance degradation, minor log anomaly.
- [ ] Alert channels: Defined and tested (PagerDuty, Slack, email, SMS).
- [ ] Alert escalation: Defined and tested (L1 -> L2 -> L3 -> management).
- [ ] Alert runbooks: Documented and accessible for each alert type.

### 10. Kill Switches Tested

- [ ] Circuit breaker: Pause mechanism tested and verified. Pauses all trading but does not block withdrawals.
- [ ] Bridge pause: Bridge pause mechanism tested and verified. Pauses all new bridge operations but does not block refunds.
- [ ] Emergency shutdown: Full shutdown mechanism tested and verified. Stops all operations and enables safe withdrawal.
- [ ] Key rotation: Custody key rotation mechanism tested and verified. Old keys are revoked, new keys are active.
- [ ] Oracle kill switch: Oracle pause mechanism tested and verified. Pauses all oracle updates and falls back to the last known good price.

### 11. Rollback Plan Documented

- [ ] Rollback procedure for each component (contracts, programs, runtime, bridge, oracle).
- [ ] Rollback triggers: What conditions trigger a rollback?
- [ ] Rollback steps: Step-by-step procedure for executing a rollback.
- [ ] Rollback verification: How to verify that the rollback succeeded.
- [ ] Rollback testing: Each rollback procedure has been tested on testnet.
- [ ] Data recovery: How to recover data if a rollback results in state loss.

### 12. PnL Logging Active

- [ ] All trades are logged with full details (route, size, profit, cost, gas, slippage, result).
- [ ] PnL logs are stored on-chain (via events) and off-chain (via a monitoring system).
- [ ] PnL logs are auditable. The total profit and loss must match the on-chain balance changes.
- [ ] PnL anomalies (unexpected losses, unusually high profits) trigger alerts.
- [ ] PnL reports are generated daily and reviewed by the operator.

### 13. Emergency Contacts Defined

- [ ] On-call rotation: Who is on call for each type of emergency?
- [ ] Escalation path: Who to contact if the on-call person is unavailable?
- [ ] Communication channels: Slack, PagerDuty, phone, email.
- [ ] Response time SLAs: Maximum time to respond for each alert severity.
- [ ] Post-incident process: How to conduct a post-mortem and implement learnings.

## Deployment Process

1. **Pre-deployment**: All checklist items verified. All audits signed off. All tests green.
2. **Staging deployment**: Deploy to staging environment. Run full test suite. Verify all monitoring and alerting.
3. **Testnet deployment**: Deploy to testnet. Run full test suite. Run chaos tests (network partitions, node failures, oracle failures).
4. **Mainnet deployment**: Deploy to mainnet. Verify all contract addresses, program IDs, and custody addresses.
5. **Post-deployment**: Verify all monitoring is active. Verify all alerting is configured. Verify all kill switches are tested.
6. **Burn-in period**: Run with limited capital for 1-2 weeks. Monitor for anomalies. No unsupervised trading.
7. **Full operation**: After burn-in, enable full capital and automated trading.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — The architecture defines the invariants that must be verified.
- **UNIVERSAL_ASSET_KERNEL.md** — The UAK invariant must be verified on every block.
- **CROSS_VM_ROUTING.md** — All routes must have timeout/refund paths verified.
- **EVM_RULES.md** through **COSMWASM_IBC_RULES.md** — Each VM's rules must be tested and verified.
- **TRADING_SAFETY_KERNEL.md** — The TSK must be deployed and tested before mainnet trading.
- **FORBIDDEN_PATTERNS.md** — No forbidden patterns may be present in production code.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*