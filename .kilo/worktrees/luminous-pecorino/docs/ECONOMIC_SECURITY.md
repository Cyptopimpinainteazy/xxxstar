# X3 Economic Security Gate

Last updated: 2026-04-28

This document describes the economic-attack scenarios enforced by ProofForge Economic Gate.

## Scope

The gate currently runs 9 core scenarios across flashloans, DEX/MEV, oracle safety, cross-VM settlement, and governance.

## Attack Matrix

| # | Scenario | Expected Outcome | Mitigation Target | Test |
|---|---|---|---|---|
| 1 | Flash-loan oracle manipulation | Atomic execution reverts and pool capital is restored | TWAP oracle + observation window + atomic revert | crates/x3-flashloan/src/tests/attack_oracle_manipulation.rs :: flash_loan_oracle_manipulation_fails |
| 2 | Flash-loan reentrancy / concurrent same-asset borrow | Second borrow rejected while first is outstanding | Reentrancy/concurrency guard at pool level | crates/x3-flashloan/src/tests/attack_reentrancy.rs :: flash_loan_reentrancy_rejected |
| 3 | Flash-loan repayment bypass | Underpayment rejected; capital not restored prematurely | Strict principal+premium repayment check | crates/x3-flashloan/src/tests/attack_repayment_bypass.rs :: flash_loan_repayment_bypass_reverts |
| 4 | Sandwich MEV extraction | Attacker profit bounded by configured slippage ceiling | Dynamic slippage + MEV protection | crates/x3-dex/src/tests/attack_sandwich.rs :: sandwich_attack_profit_bounded_by_slippage |
| 5 | Liquidation front-run bonus theft | Ordering/fairness invariant prevents second caller theft | Fair ordering in atomic batch execution | crates/x3-dex/src/tests/attack_liquidation_frontrun.rs :: liquidation_frontrun_eliminated_by_fair_ordering |
| 6 | Single-block TWAP manipulation | TWAP deviation remains below threshold | Time-weighted execution window | crates/x3-dex/src/tests/attack_twap_manipulation.rs :: twap_resists_single_block_price_spike |
| 7 | Oracle update front-run / stale-read window | Update is applied atomically with no stale intermediate read | Atomic state transition during update | crates/x3-dex/src/tests/attack_oracle_frontrun.rs :: oracle_update_is_atomic_no_stale_window |
| 8 | Cross-VM arbitrage via asymmetric settlement | Commit without prepare/lock is rejected | Two-phase commit atomicity | crates/cross-vm-bridge/src/tests/attack_arbitrage.rs :: cross_vm_arbitrage_blocked_by_atomic_state |
| 9 | Flash-loan governance takeover | Votes acquired after snapshot are rejected/zeroed | Historical balance snapshot enforcement | pallets/governance/src/tests.rs :: flash_loan_governance_takeover_fails |

## Current Reality

- Scenario 2 is enforced by a same-chain/same-asset concurrent-borrow rejection in FlashloanPool.
- Scenario 9 is enforced by proposal-time voter balance snapshots in pallet-governance.
- Economic Gate currently reports all four runner areas as VERIFIED in strict mode.

## Gate Command

Run all scenarios:

```bash
x3-proof economic-gate --strict --fail-hard
```

- Exit code is non-zero when any required mitigation is missing or any attack test fails.
- CI uses this command to block merges on unmitigated economic attack vectors.

## Real-World Exploit Precedents

| Protocol | Year | Approximate Loss | Primary Vector |
|---|---|---:|---|
| bZx | 2020 | $1M | Flash loan + oracle manipulation |
| Harvest Finance | 2020 | $34M | Flash-loan arbitrage and pricing abuse |
| Cream Finance | 2021 | $130M | Reentrancy + flash-loan amplification |
| Mango Markets | 2022 | $114M | Oracle manipulation / mark price abuse |

These incidents motivate the gate as a release blocker for DeFi-critical code paths.
