# A matrix citation could name a test that does not exist, and every gate still passed

Date: 2026-09-26
Scope: `feature-matrix/*.toml` citations, `scripts/ci/check-matrix-tests-exist.py` (new gate)
Directive: evidence standard — a citation is evidence, so it has to resolve.

## Finding

`scripts/check-readiness-consistency.sh` proves every `required_tests` name in `FEATURE_REGISTRY.toml`
exists as a real test function. The **matrix fragments carry the same field and nothing checked
them**. Measured before writing anything: adding

```
required_tests = ["a_test_that_does_not_exist_anywhere", ...]
```

to `X3-XVM-006` left `readiness consistency` PASS. A row could cite invented tests and the green
record would not notice.

Separately, eight rows in `cross-vm-atomic.toml` carried the placeholder
`"note: audited test/CI evidence score=82; exact named tests should be added before score increase"`
instead of naming anything. One of them (`X3-XVM-014`) was fixed in the previous commit.

## Fix

* `scripts/ci/check-matrix-tests-exist.py` reads every `feature-matrix/*.toml` fragment, takes each
  row's `required_tests` and `paths`, and requires `fn <name>` to exist under one of those paths (a
  file path means its directory, so a row may cite `.../src/lib.rs` while its tests live in
  `.../src/tests.rs`). Wired into `local-ci` as `matrix tests exist`.
* The eight placeholder notes now name the tests that cover those rows, and seven of them (all but
  `X3-XVM-014`, which was already reworked) carry a `required_tests` array so the citation is
  checked rather than described. 26 citations added; all resolve.

| row | tests now cited |
|---|---|
| X3-XVM-006 Expired transfer refund | `expired_transfer_refunds_source`, `test_expired_transfer_refunds_to_source`, `edge_expired_transfer_rejected`, `test_cannot_cancel_before_expiry` |
| X3-XVM-008 Failed destination credit refund | `test_failed_destination_credit_refunds_pending_supply`, `failed_second_leg_rolls_back_first_leg`, `ixl_abort_after_lock_restores_ledger` |
| X3-XVM-010 Message replay rejection | `test_duplicate_message_replay_rejected`, `replay_message_rejected_no_state_change`, `duplicate_message_id_rejected` |
| X3-XVM-011 Nonce replay rejection | `test_duplicate_nonce_rejected`, `six_internal_routes_strict_invariants_and_replay_guards` |
| X3-XVM-012 EVM -> X3 route | `six_internal_routes_strict_invariants_and_replay_guards` (drives `X3Evm -> X3Native`), `test_all_six_internal_routes_succeed`, `vm_adapter_six_routes_preserve_supply_and_clear_pending` |
| X3-XVM-013 SVM -> X3 route | the same six-route test driving `X3Svm -> X3Native`, plus `xvm_router_svm_to_evm_full_round_trip` |
| X3-XVM-016 Cross-VM atomic router | the six-route invariants, the state-machine transitions test, and the three origin guards |
| X3-XVM-017 External bridge pause-at-genesis | `external_bridges_are_paused_at_genesis`, `external_bridges_disabled_at_genesis`, `test_external_route_rejected_in_mvp`, `register_external_root_rejected_at_genesis`, `enabling_external_bridges_requires_documented_audit_gate` |

All eight name tests in `pallets/x3-cross-vm-router/src/tests.rs`, which the
`test x3-cross-vm-router` gate runs.

## Evidence

```
$ python3 scripts/ci/check-matrix-tests-exist.py
check-matrix-tests-exist: OK - 104 required_tests citation(s) resolve to real fn names

$ (with "a_test_that_does_not_exist_anywhere" injected)
check-matrix-tests-exist: FAIL: citations that resolve to nothing:
  X3-XVM-006: required_tests cites 'a_test_that_does_not_exist_anywhere' but no fn ... exists
checker exit=1

$ bash scripts/local-ci.sh --jobs 4
86 gates, 0 failures     (was 85; `matrix tests exist` is the new one)
```

The 104 citations cover every row in the matrix that lists `required_tests`, not just the eight rows
touched here — the other 78 already resolved, which is a useful independent check on work earlier
sessions recorded.

## Remaining

* `test_evidence` prose notes are still unvalidated; only `required_tests` is checked. Rows whose
  evidence is a sentence rather than a citation can still drift.
* The `--live` and `--cross` gate groups are not part of the default run, so a report that quotes the
  86-gate result is still not quoting the live evidence (previous commit's report has that set).
* PRIORITY 2's public-testnet half remains untouched: everything here is anvil, a local X3 node and a
  local Solana validator.
