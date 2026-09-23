# TICKET-101, third pass: the six rows, measured from the tests rather than the numbers

Date: 2026-09-23.

## The correction

The second pass concluded that X3-XCHAIN-013/018/021 and X3-LANG-004/009/010 had "no named test or file
I could confirm on master", and that `tested=82` was therefore unbacked. That was wrong, and the cause
is instructive: **it searched for test names beginning with `test_`.** This repository names tests as
sentences — `authorizes_only_when_all_required_domains_are_finalized`,
`rejects_wrong_chain_or_vm_binding`, `rejects_reused_proof_across_required_domains` — precisely so the
name says what the test does. A prefix search finds none of them.

`crates/x3-atomic-swap/src/secret_release.rs` alone holds eleven tests, including the firewall and the
`SecretReleasePermit` the row's blocker said still had to "land together".

## What the rows say now

Each row's `required_tests` names real tests, and `feature_matrix.py check` **resolves every name
against the row's own `paths`** — so the claim is verified by the gate, not by this report:

| row | named tests |
| --- | --- |
| X3-XCHAIN-013 | firewall: required domains finalized, replay across domains, refunded destination, wrong preimage, stale intent |
| X3-XCHAIN-018 | binding: wrong chain/VM, missing destination domain, refunded destination, duplicate domain operation |
| X3-XCHAIN-021 | tx/block: wrong finality tx/block, included-but-not-finalized, insufficient confirmations |
| X3-LANG-004 | bridge: compiles source to runtime-loadable bytecode, rejects invalid source, executes, gas exhaustion |
| X3-LANG-009 | decoder: invalid magic, minimal module parse |
| X3-LANG-010 | envelope: invalid magic, execution, gas exhaustion |

`tested` stays where it was: the evidence now supports the number rather than raising it, and nothing
found here justifies lifting `mainnet_ready` past the live-proof gap (TICKET-095).

Two blockers were stale ("Pending merge/exact-head proof", "Must land firewall and unforgeable permit
together") and now name the real remaining gap: no live external-chain proof of the release path. Two
were sharpened, and sharpening them found a defect the scores could not express:

## TICKET-108 — the envelope's version and checksum are written and never checked

`crates/x3-backend/src/bc_format.rs` defines the format properly: magic `X3BC`, a packed semantic
version, a `min_version`, feature flags, and a checksum over the body — `compute_checksum`, written at
byte offset 12.

`crates/x3-integration/src/mini_x3.rs` re-implements the same envelope for `no_std` builds — and that
is the decoder `X3Executor::execute` uses when the `std` feature is off, which is the runtime's case.
Its parser does this:

```rust
let magic = r.read_bytes(4)?;
if magic != b"X3BC" { return Err(X3Error::InvalidMagic); }
r.skip(20)?; // version, flags, checksum, minversion, features
```

So the fields are read past, not checked. A module declaring a future version, one whose `min_version`
the loader cannot satisfy, or a body corrupted after compilation is accepted as long as its structure
parses. `x3-backend`'s own reader is no better on the checksum: it reads it into `_checksum` and moves
on.

Two decoders for one format, and the weaker one runs where the runtime does. That also means
X3-LANG-009's row name — "Authenticated bytecode decoder" — describes something with no signature or
digest behind it at all; the row now says so rather than leaving the name to imply otherwise.

**Acceptance for TICKET-108:** one decoder for the format, or the `no_std` one verifying the magic,
version, `min_version` and checksum the writer already emits, with a test each proving a corrupted body
and a future version are refused.

## Evidence

```
python3 scripts/feature_matrix.py check      PASS — 145 features, 7 warnings (the four X3-RT-* spread warnings are pre-existing)
bash scripts/check-readiness-consistency.sh  PASS
rg "fn (authorizes_only_when_all_required_domains_are_finalized|rejects_wrong_chain_or_vm_binding|...)"  -> all resolve under the rows' paths
```
