# Volunteer consent and manifest input hardening

Base: `eff2ec8b98ea133125bd9d081ab5d1d0668c0a5a` (PR #627, stacked on
PR #626). No dependencies were merged. This patch remains a fixed CPU smoke
executor; it does not implement or prove GPU isolation or GPU execution.

At the base, `consent="false"` authorized execution because it is truthy, and
`version=true`/`version=1.0` compared equal to integer version 1. Duplicate JSON
keys also selected the last value. These inputs could consume a signed lease
and execute despite invalid consent or an ambiguous manifest.

Both execution entrypoints now share exact boolean control validation and exact
integer manifest-version validation. Signed dispatch rejects duplicate keys before
consuming the durable nonce. Existing authenticated fixed-workload execution and
lease replay/restart tests remain intact.

Validation on 2026-10-18: `python3 -m unittest discover -s
services/x3-volunteer-worker -p test_strict_inputs.py -v` reproduced 18 failing
subcases across five new tests before the fix. After the fix,
`python3 -m unittest discover -s services/x3-volunteer-worker -v` exited 0 with
28 tests passed. Invalid-input tests also verify no lease nonce was consumed.
Proof logs are in `.ai/runlogs/volunteer-strict-20261018/`. `git diff --check`
passed. No Rust or frontend sources changed, so their build suites are outside
this patch's validation. Exact-head CI must pass before merging the stack.

Remaining requirements include persistent worker enrollment/revocation,
independently verifiable signatures, crash recovery beyond nonce consumption,
enforced hardware limits, a reviewed isolation backend and real GPU pilot tests.
No machines were enrolled, workloads deployed or compensation promised.
