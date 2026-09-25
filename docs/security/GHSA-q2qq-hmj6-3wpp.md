# GHSA-q2qq-hmj6-3wpp / RUSTSEC-2026-0119 — hickory-proto O(n^2) name compression

**Package:** `hickory-proto` (Rust)
**Severity as published:** medium
**Advisory range as published:** `>=0.3.1, <=0.26.0` (first patched: `0.26.1`)
**Status:** **accepted risk**, with a per-consumer call-path analysis. Unlike the NSEC3 advisory this
one is not gated behind a feature, so the code is compiled; what is missing is a caller that hands
the encoder attacker-shaped input.
**Date:** 2026-09-25
**Record:** [`security/advisory-scope.toml`](../../security/advisory-scope.toml) (`status = "accepted_risk"`)
**Gate:** `scripts/check-advisory-scope.py` (`advisory scope`), which fails if every resolved copy
ever leaves the window — the acceptance cannot outlive the exposure it describes.

---

## 1. What the advisory says

> During message encoding, `hickory-proto`'s `BinEncoder` stores pointers to labels that are
> candidates for name compression in a `Vec<(usize, Vec<u8>)>`. The name compression logic then
> searches for matches with a linear scan. A malicious message with many records can both introduce
> many candidate labels, and invoke this linear scan many times. This can amplify CPU exhaustion in
> DoS attacks.

The cost is quadratic in *(candidate labels introduced) x (compression scans performed)*, which is
to say: in the size of a **message being encoded**.

## 2. Which copies are in the tree

Same three copies as the sibling advisory — `0.24.4` (via `libp2p-dns`), `0.25.2` (via `litep2p`),
`0.26.3` (this repository's `x3-dns-server`, pinned `=0.26.3`). The first two are inside the window;
`0.26.3` is past the 0.26.1 fix. Nothing to upgrade: `libp2p-dns 0.42` requires `^0.24` and
`litep2p 0.13.3` requires `^0.25`.

## 3. Reachability, consumer by consumer

Encoding happens in `hickory_proto::op::Message::emit` / `BinEncoder`, whose call sites are in
`op/message.rs` and `op/query.rs`. Each consumer of the vulnerable copies:

**`libp2p-mdns 0.46.0`** (the reason `mdns` is enabled on the 0.24.4 copy) — **parses only**. Every
message it handles goes through `Message::from_vec`:

```
src/behaviour/iface/query.rs:50:        let packet = Message::from_vec(buf)?;
```

A search of its non-test sources for `to_vec`, `BinEncoder` or `emit(` returns **nothing**. A
crafted mDNS packet is decoded, never re-encoded, so this consumer cannot reach the vulnerable code
at all.

**`hickory-resolver 0.24.4` / `0.25.2`** (the stub resolver inside `libp2p-dns` and `litep2p`) —
**encodes, but only its own query**. The resolver builds a `Message` with a single question and no
records, and that is what the encoder sees. The advisory's amplification needs *many records* to
build up candidate labels; one question contributes none. A long question name contributes labels
only, and a DNS name is capped at 255 bytes, so the quadratic term is bounded by roughly
`127^2` — not an amplification primitive.

The one attacker-influenced input on that path is the *name* to resolve: a peer can advertise a
`/dns4/...` multiaddr and the resolver will look it up. That reaches the encoder with a single
question, which is the bounded case above; it cannot turn into the "many records" shape the advisory
describes. If that ever changes — for example if a hickory **server** built from the 0.24/0.25 line
ever encodes responses inside the node — this record has to move to a real exposure.

**`x3-dns-server`** is the component that *does* encode responses to arbitrary queries, which is the
exact shape the advisory describes. It pins `hickory-proto = "=0.26.3"`, past the 0.26.1 fix, so it
is not affected; the finding is recorded here because it is the reason the pin matters and must not
be relaxed:

```toml
# crates/x3-dns-server/Cargo.toml
hickory-proto = "=0.26.3"
```

## 4. Residual risk

A remote peer that supplies a DNS name can cause one bounded, quadratic-in-name-length compression
pass on the resolution path. The practical impact is CPU work proportional to a legal DNS name, not
the record-count amplification the advisory is about. Accepted, with the ratchet that the gate
fails if the resolved copies change.

## 5. How to re-verify

```bash
python3 scripts/check-advisory-scope.py --list
python3 scripts/check-advisory-scope.py        # fails if no resolved copy is inside the window any more

# The encoding call sites, and the absence of one in libp2p-mdns.
grep -rn 'BinEncoder::new' ~/.cargo/registry/src/*/hickory-proto-0.25.2/src/ | head
grep -rn 'to_vec\|BinEncoder\|emit(' ~/.cargo/registry/src/*/libp2p-mdns-0.46.0/src/   # expect: nothing

# The pin that keeps our own server on the fixed line.
grep -n 'hickory-proto' crates/x3-dns-server/Cargo.toml
```
