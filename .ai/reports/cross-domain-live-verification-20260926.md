# Every live and cross-domain gate, run: the groups the "all gates pass" record never included

Date: 2026-09-26
Scope: `node/tests/x3vm_evm_live.rs`, `scripts/cross-domain-evm-gate.sh`, `X3-XVM-014`
Directive: ROADMAP PRIORITY 2 (live X3<->EVM) and the evidence standard — run the real harness.

## Why this ran

The default `local-ci` set is 85 gates and contains none of the `--live` or `--cross` groups, so
"81 gates, all PASS" was never a statement about the live paths. Earlier in this session the
`X3-native lifecycles` gate was run for the first time (PASS). This pass ran the rest.

## Results — all nine gates, first time in one session

```
--cross group
  PASS  X3-native lifecycles                          362s   (6 tests: compiled program -> finality ->
                                                             receipt; restart; replay; lock/claim;
                                                             timeout-refund; early refund refused)
  PASS  cross-domain EVM                              224s   (7 tests, below)
  PASS  cross-domain SVM                              194s   (2 tests against solana-test-validator
                                                             4.2.2 and the real SBF program
                                                             CshvQwFZVjWEXoKeyESYD915DgFbAU9nehHKGns5FB9k)
  PASS  cross-domain EVM (strict posture)             334s   (same seven, genesis policy flipped)
  PASS  cross-domain SVM (strict posture)             196s

--live group
  PASS  local node smoke                               16s
  PASS  local network smoke                            11s
  PASS  EVM contract lifecycle                         56s   (11 assertions, 0 failed)
  PASS  SVM contract lifecycle                        108s   (wrong-preimage, double-claim,
                                                             refund-before-timeout, refund-by-wrong-
                                                             authority all rejected)
```

## What was added

The EVM gate ran four tests: lock/claim, timeout-refund, the header attestation, and the receipt
proof. Its failure matrix had no live coverage — those tests only ever claim with the preimage they
locked with, so "the contract checks the secret", "it cannot pay twice" and "the timelock gates
refunds" were assumptions about `AtlasHTLC` rather than things a chain had shown. Three tests now
prove them through `LiveEvmExecutor`, the executor the product path uses:

* `real_evm_a_claim_with_the_wrong_secret_is_refused` — and the right preimage still claims, so the
  refusal is the secret and not a bricked lock;
* `real_evm_a_second_claim_on_the_same_lock_is_refused` — the property the lock exists for;
* `real_evm_an_early_refund_is_refused_and_succeeds_once_expired` — the guard is the clock, not a
  broken refund path.

Measured: `cross-domain EVM` PASS 224s with all seven, each new test 3.05s (they need no X3 node).

**Not double work:** `X3-contracts/evm/test-live-lifecycle.sh` already asserts double-claim,
premature refund and wrong-authority refund at the *contract* level through `cast` (11 passed). The
new tests are the same refusals one layer up, through the executor and its receipt handling, which is
where a caller actually meets them. The matrix row now says both.

## Matrix

`X3-XVM-014` ("X3 -> EVM route") asked for exactly this — its evidence said "exact named tests should
be added before score increase" — and now names the seven node-level tests, the gate that runs them,
the strict-posture run, and the contract-level script. Its scores were left alone.

That placeholder sentence turned out to be in **nine** rows of `cross-vm-atomic.toml`; one is now
real evidence and eight still ask for names.

## Remaining

* Eight rows still carry the placeholder note.
* The strict-posture gates only run when invoked (`--cross`), and the whole `--cross`/`--live` set is
  not part of the default run or of the "all gates pass" record the reports quote.
* PRIORITY 2's public-testnet half is untouched: everything above is anvil and a local node, which
  the directive explicitly does not accept as public-chain evidence.
