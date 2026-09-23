# The gateway URI has no default, and a live chain refuses the published seeds

Date: 2026-09-23. Closes TICKET-105.

## What was true

`--x3-gateway-uri`'s help text said it "Defaults to `//x3-atomic-gateway` for dev chains". The spawn
path did not implement that: it treated `None` as "do not spawn the atomic gateway service" and logged
a warning. So the documented default was wrong in the first place.

The real gap was the other direction. Nothing stopped an operator from passing one of the seeds this
repository publishes — `//x3-atomic-gateway`, `//x3-settlement-gateway`, `//Alice` … — on a live chain.
Since spec 19 the runtime accepts atomic calls only from accounts the chain's genesis authorizes, so
that service would have had every extrinsic rejected. Fail-closed, and completely silent: the operator
would see a service that does nothing and no reason why.

## What it is now

* The help text says the flag is **required** when `--enable-atomic-kernel` is set, and that there is no
  default.
* `crate::atomic_gateway::published_dev_seed(uri)` recognises the published phrases by exact match after
  trimming — deliberately exact, because `//x3-atomic-gateway-prod` is a different account and may be the
  operator's own secret.
* `refuses_published_seed_on_a_live_chain(chain_type, chain_id, uri)` is the rule, and the spawn path
  calls it before constructing the service. On `ChainType::Live` with a published seed the node logs one
  error naming the chain, the seed and the flag to pass, and does not start the service. On
  `Development`/`Local` it proceeds, because those genesis files name exactly those accounts.
* The rule is a pure function of three values, so it is unit-tested rather than asserted:
  `a_live_chain_refuses_the_published_seeds`, `a_dev_chain_may_use_them`,
  `a_live_chain_accepts_an_operator_account`, plus `published_dev_seeds_are_recognised_and_nothing_else_is`.

## Evidence

```
cargo test -p x3-chain-node --lib        70+ passed, including the four above
cargo clippy -p x3-chain-node --all-targets -- -D warnings    clean
cargo fmt --all -- --check                clean
check-runtime-hash-freshness.py           3 node files changed, none in the runtime graph — no re-attestation
```

## Why this is not "just a log line"

The origin work (spec 19) made the *chain* decide who may drive the atomic path. This makes the *node*
tell the operator when its own configuration cannot possibly satisfy that decision, before it spins up a
service that will silently fail every call. Together they are the two halves of "the privilege is
declared and the holder is named": the chain declares, the node checks.
