# Trading Core v1 — verification report (2026-09-18)

Scope: the trading-core-v1 language surface added across PRs #133, #212, #216,
#223 — a dedicated single/cross-chain atomic-trade DSL inside x3-lang
(parser → semantic checker → IR lowering → bytecode verifier → VM execution →
signed receipts), plus its runtime pallet, `pallets/atomic-trade-engine`.

This report only claims what was independently re-run and observed passing
in this pass. It does not carry forward pass/fail claims from prior PR
descriptions without re-verification.

## x3-lang layer (compiler + VM + CLI)

```
cd x3-lang
cargo test -p x3-lang-compiler --test test_trading_core_e2e \
  --test test_trading_verifier --test test_trading_semantic_hardening \
  --test test_trading_ir --test test_trading_types
  → 45 passed; 0 failed

cargo test -p x3-lang-vm --test trading_execution \
  --test trading_properties --test trading_receipts
  → 58 passed; 0 failed

cargo test -p x3-tools --test cli --test cli_integration
  → 28 passed; 0 failed
```

131/131 tests passing. Coverage includes, by real test name (not aspirational):

- **Gas ceiling**: `gas_ceiling_within_policy_still_commits`,
  `gas_ceiling_exceeded_rejects_even_though_profit_and_debts_are_fine`,
  `gas_ceiling_is_enforced_at_commit_even_with_no_other_guard_operations`
- **Cross-chain rejection**: `swap_across_declared_chains_is_rejected`,
  `borrow_on_a_different_chain_than_the_swap_is_rejected`,
  `bridge_source_must_match_the_trade_chain`
- **Solvency invariant**: `solvent_invariant_passes_when_every_touched_asset_nets_non_negative`,
  `solvent_invariant_catches_hidden_cost_in_an_asset_the_trade_never_touches`
- **Quotes/slippage**: `output_matching_or_beating_the_quote_is_never_slippage`,
  `slippage_within_policy_ceiling_still_commits`,
  `slippage_beyond_policy_ceiling_is_rejected`
- **Oracle firewall**: `oracle_firewall_fails_closed_when_required_but_host_reports_no_sources`,
  `oracle_firewall_rejects_a_source_that_deviates_beyond_ceiling`,
  `oracle_firewall_catches_a_source_quoting_higher_too`
- **Simulate (no side effects)**: `simulate_never_commits_even_when_every_guard_passes`,
  `simulate_does_not_disturb_a_later_real_execution`,
  `simulating_a_lossy_trade_never_updates_the_cumulative_ledger`
- **Cumulative loss ceiling**: `cumulative_loss_ceiling_trips_only_once_prior_trades_are_summed_in`,
  `cumulative_loss_ledger_only_grows_from_trades_that_actually_commit`
- **Replay protection**: `replay_ledger_rejects_the_identical_receipt_presented_twice`,
  `economic_replay_rejects_forged_reported_profit_even_with_rehashed_receipt`
- **Receipt CLI**: `cli_receipt_execute_compiles_runs_and_emits_a_verifiable_receipt`,
  `cli_receipt_verify_accepts_valid_receipt`,
  `cli_receipt_verify_rejects_tampered_receipt`
- **Bridge wiring**: `bridge_credits_the_destination_asset_and_debits_the_source`,
  `bridge_against_a_host_with_no_bridging_support_fails_closed`,
  `bridge_pipeline_executes_and_verifies_receipt`

## Runtime pallet layer (`pallets/atomic-trade-engine`)

```
cargo test -p pallet-atomic-trade-engine
  → 48 passed; 0 failed
```

Covers multi-leg (EVM/SVM/X3) batch execution, slippage/deadline/nonce
guards, AMM adapter registration, liquidity-pool oracle observations, and
rollback-on-kernel-failure (`kernel_comit_failure_is_rolled_back_but_batch_is_marked_failed`).

This is a separate compilation unit from the x3-lang crates above (no shared
`crate_or_service` root), so it is tracked as its own registry row
(`[atomic_trade_engine]`) rather than folded into `[trading_core_v1]`.

## Known, un-fixed gap (not addressed by this report)

`x3c audit`'s `RiskScorer` (the "Risk score"/"Risk details" section of audit
output) still speaks only the older intent-DSL's vocabulary and does not
recognize `Item::AtomicTrade`/`Item::TradeRiskPolicy`. Flagged explicitly in
PR #216's description as a separate, likely-larger follow-up; still open as
of this report. A trading-core-v1 program's audit risk score cannot
currently be trusted as a signal — treat it as unscored, not as "safe".

## What was not verified in this pass

- No multi-validator / live-network run of `pallets/atomic-trade-engine`.
- No fuzzing or property-based testing beyond the two tests in
  `trading_properties.rs`.
- `cargo clippy` / `cargo fmt --check` were not re-run for this report;
  scope was limited to test correctness.
