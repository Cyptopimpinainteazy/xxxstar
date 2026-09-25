# GHSA-3v94-mw7p-v465 / RUSTSEC-2026-0118 — hickory-proto NSEC3 unbounded loop

**Package:** `hickory-proto` (Rust)
**Severity as published:** high
**Advisory range as published:** `>=0.25.0-alpha.3, <=0.25.2`
**Status:** **unreachable in this graph.** A resolved version *is* inside the window; the code carrying the
defect is not compiled, because the advisory's own stated precondition is not met.
**Date:** 2026-09-25
**Record:** [`security/advisory-scope.toml`](../../security/advisory-scope.toml) (`status = "unreachable"`)
**Gate:** `scripts/check-advisory-scope.py` (`advisory scope`), which enforces the precondition below.

---

## 1. What is in the tree

Three copies of `hickory-proto` are resolved, reached through two independent networking backends:

| Version | Pulled by | In the node? | In the advisory window? |
| --- | --- | --- | --- |
| `0.24.4` | `hickory-resolver 0.24.4` ← `libp2p-dns 0.42.0` ← `libp2p 0.54.1` ← `sc-network` | yes | no |
| `0.25.2` | `hickory-resolver 0.25.2` ← `litep2p 0.13.3` ← `sc-network` | yes | **yes** |
| `0.26.3` | this repository's own `x3-dns-server` (pinned `=0.26.3`) | yes | no |

So the alert is not a false positive about the version: `0.25.2` really is inside the published
range, and it really is compiled into the node.

## 2. Why it is nevertheless unreachable

The advisory states its own precondition:

> The bug is reachable by any caller of `DnssecDnsHandle` — including the resolver, recursor, and
> client — **when built with the `dnssec-ring` or `dnssec-aws-lc-rs` feature and configured to
> perform DNSSEC validation.**

That precondition is a *feature*, and it is checkable. `hickory-proto` declares the module behind
exactly that cfg:

```rust
// hickory-proto 0.25.2, src/lib.rs
#[cfg(any(feature = "dnssec-aws-lc-rs", feature = "dnssec-ring"))]
pub mod dnssec;
```

```rust
// hickory-proto 0.24.4, src/xfer/mod.rs  (the 0.24 line gates the same code on `dnssec`)
#[cfg(feature = "dnssec")]
pub mod dnssec_dns_handle;
```

Measured against this workspace, with `cargo tree -e features`:

| Copy | Features enabled |
| --- | --- |
| `hickory-proto 0.24.4` | `mdns`, `socket2`, `tokio`, `tokio-runtime` |
| `hickory-proto 0.25.2` | `futures-io`, `std`, `tokio` |
| `hickory-proto 0.26.3` | `default`, `std` |

A workspace-wide search for any `dnssec*` feature name returns nothing. `DnssecDnsHandle`, and the
closest-encloser loop the advisory describes, are therefore not in any artifact this repository
builds. The `mdns` feature on the 0.24.4 copy is `libp2p-mdns`'s, which pulls the crate for message
*parsing*, not for DNSSEC.

## 3. Why no upgrade is possible

There is nothing to upgrade to on the affected line: the newest `0.25.x` release is `0.25.2` itself.
The fix was not backported — the affected implementation moved out of `hickory-proto` and into
`hickory-net` at the 0.26.0 release, and the advisory recommends `hickory-net` 0.26.1 "when the
implementation of that type is required". `libp2p-dns 0.42` requires `hickory-resolver ^0.24` and
`litep2p 0.13.3` requires `^0.25`, so neither can be moved to 0.26 by a lockfile change.

Since the requirement is exactly that the vulnerable type is *not* required here, there is nothing
to fix: the honest action is to record the precondition and keep it enforced.

## 4. What would change the answer

The gate fails the moment any of `dnssec-ring`, `dnssec-aws-lc-rs` or `dnssec-openssl` appears in
the enabled feature set of this workspace. That is the signal that this record has to be
re-verified — either because a consumer turned DNSSEC on (then the exposure is real and the file
has to say so), or because the feature name changed.

## 5. How to re-verify

```bash
python3 scripts/check-advisory-scope.py --list
python3 scripts/check-advisory-scope.py          # fails if a dnssec feature appears

# The features actually enabled, per copy.
cargo tree -e features -i hickory-proto@0.24.4 --workspace | grep -o 'hickory-proto feature "[^"]*"' | sort -u
cargo tree -e features -i hickory-proto@0.25.2 --workspace | grep -o 'hickory-proto feature "[^"]*"' | sort -u
cargo tree -e features --workspace | grep -o 'feature "dnssec[^"]*"' | sort -u   # expect: nothing

# The cfg that makes the module conditional.
grep -n -B1 'pub mod dnssec' ~/.cargo/registry/src/*/hickory-proto-0.25.2/src/lib.rs
grep -n -B1 'pub mod dnssec_dns_handle' ~/.cargo/registry/src/*/hickory-proto-0.24.4/src/xfer/mod.rs
```
