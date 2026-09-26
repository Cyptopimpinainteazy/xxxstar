# The other half of the evidence standard: names cited in `test_evidence` prose

Date: 2026-09-26
Scope: `feature-matrix/*.toml` `test_evidence` notes, `scripts/ci/check-matrix-test-evidence.py` (new gate)
Follows: `.ai/reports/matrix-tests-exist-gate-20260926.md`

## Finding

The previous commit made the structured `required_tests` field resolvable. The free-text
`test_evidence` notes were still unchecked, and they carry most of the citations — a note that says
"`node/tests/x3vm_live_lifecycle.rs` runs `a_compiled_x3_program_is_finalized_and_its_receipt_is_readable`"
could name a test that does not exist, or a path that moved, and nothing would notice.

## The rule, and why it is narrow

An identifier in a `test_evidence` note counts as a citation when it starts with `test_` **or**
contains at least two underscores; English prose does not carry two underscores, so paragraphs stay
quiet. A citation must resolve to a `fn <name>` anywhere in the tree, or to a file name, file stem or
directory name anywhere in the tree. Measured before writing the gate: with that rule the notes
produce **one** candidate, `x3_atomic_swap`, which is a real directory
(`programs/svm/x3_atomic_swap/`) — i.e. the prose was already clean, and the gate can be a plain one
rather than a ratchet.

## Evidence

```
$ python3 scripts/ci/check-matrix-test-evidence.py
check-matrix-test-evidence: OK - 95 citation(s) in test_evidence notes resolve          (1.8s)

$ (with "the_absent_regression_test" injected into a note)
check-matrix-test-evidence: FAIL: citations that resolve to nothing:
  X3-XVM-006: test_evidence cites 'the_absent_regression_test', which is not a fn name, file,
  stem or directory anywhere in the tree
checker exit=1

$ bash scripts/local-ci.sh --jobs 4
87 gates, 0 failures     (was 86; `matrix test evidence` is the new one)
```

## The evidence standard, as it now stands

| layer | checked by | result |
|---|---|---|
| `FEATURE_REGISTRY.toml` `required_tests` | `check-readiness-consistency.sh` (pre-existing) | PASS |
| `feature-matrix/*.toml` `required_tests` | `matrix tests exist` (added 2026-09-26) | 104/104 resolve |
| `feature-matrix/*.toml` `test_evidence` names | `matrix test evidence` (this commit) | 95/95 resolve |
| registry crates actually run by a gate | `check-registry-tests-are-gated.py` (pre-existing) | PASS |

Every layer of "this row is backed by a test" is now machine-checked rather than asserted.

## Remaining

* The rule is a heuristic on shape: a citation written as a plain word ("checked by the kernel
  suite") is not caught, and a prose phrase with two underscores would be flagged. The trade is
  documented in the checker's docstring.
* The `--live` and `--cross` gate groups still are not part of the default 87, so a report quoting
  that number is not quoting the live evidence (see `.ai/reports/cross-domain-live-verification-20260926.md`
  for that set).
* PRIORITY 2's public-testnet half remains the standing external blocker.
