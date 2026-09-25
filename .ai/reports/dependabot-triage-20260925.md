# Dependabot triage — 115 open alerts on the default branch (2026-09-25)

Source: `gh api repos/Cyptopimpinainteazy/xxxstar/dependabot/alerts?state=open`, pulled after GitHub
reported the count on a push. This is the first time any of this signal has been readable here: the
repository's own dependency gate is `cargo-audit`/`cargo-deny` against the **RustSec** database, and
GitHub's Dependabot uses the **GitHub Advisory Database**. Those two sets are not the same, and this
file records the difference rather than the headline number.

## The headline, decomposed

| Ecosystem | Alerts | Where |
| --- | --- | --- |
| npm | 87 | `x3-app-store/backend` 41, `x3-app-store/frontend` 37, plus explorer/sdk/wallet-integration |
| Rust | 28 | `Cargo.lock` 19, `X3-contracts/svm/Cargo.lock` 6, `crates/gpu-swarm`, `crates/x3-gulfstream` |

Severities across both: 1 critical (`websocket-driver` in the app-store frontend), 32 high, 67 medium,
15 low.

## Rust: what is actually reachable

Measured per crate with `cargo tree -i <crate>@<version>` against this tree:

| Crate | Version | Severity | In the resolved graph? | Pulled by |
| --- | --- | --- | --- | --- |
| `yamux` | 0.12.1 | high (GHSA-vxx9-2994-q338, remote panic on a malformed Data frame) — **re-verified: not exploitable here, see below** | **yes** | `libp2p-yamux` → `libp2p 0.54.1` → `sc-network` |
| `hickory-proto` | 0.24.4 **and 0.25.2** | high (GHSA-3v94-mw7p-v465, unbounded NSEC3 loop) + medium (GHSA-q2qq-hmj6-3wpp, O(n^2) encoding) — **decided 2026-09-25, see below** | **yes** | `hickory-resolver` → `libp2p-dns` **and** → `litep2p` → `sc-network` |
| `evm` | 0.39.1 | medium (error return ignored) | **yes** — **closed 2026-09-25, see below** | **our own** `crates/evm-integration` |
| `ethereum` | 0.14.0 | medium (malleability check) | **yes** — **closed 2026-09-25, see below** | via that `evm` |
| `serde_with` | 3.21.0 | medium | yes | `sc-network-types` |
| `lru`, `rand` | several | low | yes (various copies) | tracing-log, libp2p-identify, jsonrpc, ark-std |
| `libp2p-gossipsub`, `libp2p-quic`, `rustls-webpki 0.101.7`, `ring 0.16.20`, `protobuf 2.28.0`, `ed25519-dalek 1.0.1`, `idna 0.1.5`, `borsh`, `jsonwebtoken`, `atty`, `git2`, `hickory-proto 0.26.3` | — | mixed | **no** (absent from the default graph) | stale lock entries or non-default features/targets |

The two that matter are the first two, and both are **transitive through polkadot-sdk's libp2p**:

* `libp2p-yamux 0.46.0` depends on `yamux 0.12.1` **and** `yamux 0.13.3` unconditionally (both
  `[dependencies.yamux012]` / `[dependencies.yamux013]`, neither optional) and its non-test code holds
  `Either<yamux012::Connection<C>, yamux013::Connection<C>>`, so both copies are compiled into
  the node. RustSec has **zero** advisories for `yamux`, which is why the repository's own gate never
  said anything.

  **Correction, added after this table was first written (2026-09-25).** The copy *is* compiled, but
  it is not vulnerable, and neither is anything else here. The defect is a guard-*ordering* bug: the
  oversized-body check moved to *after* `make_new_inbound_stream` in `0.13.9` (during the flow-control
  refactor of PR 221) and back before it in `0.13.10`. Every `0.12.x` release — including the `0.12.1`
  resolved here — checks the body length first, so it never carried the panic. The real vulnerable
  window is `>=0.13.9,<0.13.10`, one release wide; the published `<0.13.10` is simply coarser, which
  is why Dependabot reports a version that cannot be fixed by any lockfile change. Tags checked:
  `yamux-v0.12.0`, `-v0.12.1`, `-v0.13.8` (guard first, safe), `-v0.13.9` (stream first, vulnerable),
  `-v0.13.10` (guard first, fixed). Full evidence, including the lockfile-by-lockfile versions:
  `docs/security/GHSA-vxx9-2994-q338.md`. Guarded from now on by
  `security/advisory-scope.toml` + `scripts/check-advisory-scope.py` (fast gate `advisory scope`).
* `hickory-proto` arrives twice, through two independent backends: `0.24.4` via `libp2p-dns`, and
  `0.25.2` via `litep2p`. The high-severity NSEC3 advisory has **no patched version** on the affected
  0.25 line: the newest `0.25.x` release is `0.25.2` itself, and the implementation moved to
  `hickory-net` at 0.26.0.

  **Decided, 2026-09-25.** The two advisories are different problems and get different answers, both
  now recorded in `security/advisory-scope.toml` and enforced by the `advisory scope` gate:

  * `GHSA-3v94-mw7p-v465` (high) is `unreachable`, not merely "accepted". The advisory states its
    own precondition — reachable "when built with the `dnssec-ring` or `dnssec-aws-lc-rs` feature
    and configured to perform DNSSEC validation" — and `hickory-proto` declares the module behind
    exactly that cfg (`#[cfg(any(feature = "dnssec-aws-lc-rs", feature = "dnssec-ring"))] pub mod
    dnssec;`). `cargo tree -e features` shows those features are enabled **nowhere** in this graph,
    so `DnssecDnsHandle` and the closest-encloser loop are not compiled into any artifact we build.
    The gate fails if a dnssec feature ever appears. `docs/security/GHSA-3v94-mw7p-v465.md`.
  * `GHSA-q2qq-hmj6-3wpp` (medium) is an `accepted_risk` with a call-path analysis rather than an
    assertion: the cost is in the message **encoder**, `libp2p-mdns` only ever parses
    (`Message::from_vec`, and a search of its non-test sources for `to_vec`/`BinEncoder`/`emit(`
    is empty), and the stub resolver encodes its own single-question query — one question
    contributes no candidate labels, so the record-count amplification is not available. The
    component that *does* encode arbitrary responses, our own `x3-dns-server`, pins
    `hickory-proto = "=0.26.3"`, past the 0.26.1 fix.
    `docs/security/GHSA-q2qq-hmj6-3wpp.md`.

    The record is a ratchet in the other direction from the yamux one: it fails when **every**
    resolved copy leaves the window, so the acceptance cannot outlive the exposure.

Neither can be fixed by a lockfile update: the fixing versions are on new major lines (`yamux 0.13.x`,
`hickory 0.26.x`) that `libp2p-yamux 0.46` / `libp2p-dns 0.42` do not accept. The two honest paths are
an SDK/libp2p bump, or a **vendored backport** — which this repository already does for other pinned
crates (`patches/sc-executor-wasmtime`, `patches/core2`, `patches/rustix`, `patches/fastbloom`), so it
is established practice rather than a new mechanism.

The one gap that is *ours to fix* is `evm 0.39.1` / `ethereum 0.14.0`: they come from
`crates/evm-integration`'s own dependency, not from the SDK. Bumping it to the line frontier already
uses (`evm 0.43.4`, also present in the lock) aligns the two EVM implementations and clears both
advisories — a real upgrade of ~2,900 lines' dependency surface, so it needs its own verification pass.

**Done, 2026-09-25.** `crates/evm-integration` now depends on `rust-ethereum/evm.git` `branch =
"v0.x"` — Frontier's spec, verbatim — and the lock resolves a single `evm 0.43.4`. Matching the
*source spec* on the new organisation rather than pinning a `rev` is what unified the graph: a
rev-pinned URL is a different git SourceId from the one `pallet-evm` uses, so cargo compiled two
copies of the interpreter instead of one, which is also why the first attempt at this port hit 18
type errors and why `primitive-types` had to move to 0.13.1 (`evm::H160`/`U256` *are*
primitive-types types).

Verifying it turned up the more serious finding: **`mini_evm::execute_evm` — the function the
runtime actually executes EVM through — had no test**, so the interpreter could have changed
behaviour silently. The crate's unit tests covered config, gas estimation and the state-root helper;
`pallets/x3-kernel`'s tests drive `TestEvmAdapter`, a mock; and the only two files that ran EVM
bytecode, `crates/evm-integration/tests/{integration,erc20_integration}.rs`, each began with
`#![cfg(any())]` — a permanently false cfg, so `cargo test` reported "0 tests" for them rather than
"ignored", and both had drifted off the current `EvmExecutor::execute` signature so they could not
simply be un-commented. A report (`reports/rc3/…`) even cited `tests/integration.rs` as evidence that
EVM integration was "Complete".

Both files are real tests now, 13 unit tests drive real bytecode through `execute_evm` (including a
CREATE-then-CALL test and an identity-precompile test with a negative control), and two gates keep
them running: `test x3-evm-integration` and `test x3-evm-integration frontier`. The optional
`frontier` feature builds for the first time as a side effect of the unification, which also brings
two `frontier.rs` tests into existence that had never been compiled.

## The tooling gap this triage found

The repository's own dependency gate cannot see part of what GitHub reports. Checked by cloning
`github.com/rustsec/advisory-db` (1,251 advisories) and searching by identifier:

* `GHSA-vxx9-2994-q338` (yamux): **0 results** — GitHub-only; `yamux` has no RustSec advisories at all.
* `GHSA-gc42-3jg7-rxr2` (libp2p-gossipsub): **0 results**.
* `GHSA-27wg-99g8-2v4v` (evm): **0 results**.
* `GHSA-3v94-mw7p-v465` (hickory-proto), `GHSA-rhfx-m35p-ff5j` (lru), `GHSA-h97m-ww89-6jmq` (idna):
  present in RustSec.

So "33 allowed warnings" in `CURRENT_MAINNET_STATUS.md` and Dependabot's 115 are answers to different
questions, and neither is the whole picture. A gate that reads GitHub advisories (or a job that fails
on new Dependabot alerts of `high`+ in the Rust graph) is the missing piece; today nothing in the
repository would notice a new high-severity advisory that RustSec has not imported.

**Status of that, 2026-09-25.** The RustSec half is now a gate: `dependency audit` runs
`cargo-audit` against `.cargo/audit.toml`, which no gate had ever invoked — and its first run found
`RUSTSEC-2026-0285` (rustls 0.23.44, TLS 1.3 handshake confusion in the libp2p websocket transport)
unignored and shipping in the node. Fixed by a one-version bump to 0.23.45.
`.ai/reports/dependency-audit-20260925.md` records the finding, the before/after audit output, and
what the gate still does not cover. Advisory *records* for the GitHub-only half are enforced by
`advisory scope`; a gate that notices a new GitHub-only advisory that has no record is still a
ticket.

## Follow-ups this produced

1. ~~**`yamux 0.12.1` — remote panic, reachable from peers.** Either bump libp2p via the SDK, or vendor a
   backported `yamux` under `patches/` with a regression test for the malformed Data frame.~~ **Closed
   2026-09-25: not exploitable.** The resolved versions (`0.12.1`, `0.13.8`, `0.13.10` across the
   tree's lockfiles) are all outside the real window `>=0.13.9,<0.13.10`; the published range was
   coarser than the regression. No patch is needed, and no backport was vendored. What was missing was
   a way to *record and enforce* that judgement, so `security/advisory-scope.toml` plus the
   `advisory scope` gate fail the build if a future lockfile bump resolves `0.13.9` or drops a
   version the record expects. See `docs/security/GHSA-vxx9-2994-q338.md`.
2. ~~**`hickory-proto 0.24.4` — unbounded NSEC3 loop, no upstream fix.** Needs a decision: accept with a
   written reason (it is DNS resolution inside libp2p), or pin/take over the resolver.~~ **Closed
   2026-09-25.** The NSEC3 advisory is unreachable here — the vulnerable module is gated behind a
   `dnssec*` feature and none is enabled — so it is recorded as `unreachable` and enforced, rather
   than accepted. Its sibling `GHSA-q2qq-hmj6-3wpp` is a recorded, ratcheted `accepted_risk` with a
   per-consumer call-path analysis. Both live in `security/advisory-scope.toml`; the `advisory
   scope` gate also now requires each record's evidence document to exist and its RustSec id to be
   ignored in **both** `.cargo/audit.toml` and `deny.toml`, which nothing checked before.
3. ~~**`evm 0.39.1` / `ethereum 0.14.0`** — ours; bump `crates/evm-integration` to the frontier line and
   verify the EVM path end to end.~~ **Closed 2026-09-25.** Repointed to `rust-ethereum/evm.git`
   `v0.x` (Frontier's spec, so the graph unifies on one copy), `primitive-types` to 0.13.1, and the
   missing coverage written: `execute_evm` and both integration files are tested and gated. The
   runtime wasm hash was re-attested because `pallet-x3-kernel` is in the runtime's graph.
4. **A GitHub-advisory gate**, so the next high-severity GHSA in the Rust graph fails something.
5. **npm (87 alerts)** — `x3-app-store` frontend/backend and the smaller JS projects; lockfile bumps
   with `npm audit fix` plus their own tests. Not chain-critical, but 1 critical + several high.
