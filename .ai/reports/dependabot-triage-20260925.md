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
| `hickory-proto` | 0.24.4 | high (GHSA-3v94-mw7p-v465, unbounded NSEC3 loop) + medium | **yes** | `hickory-resolver` → `libp2p-dns` → `sc-network` |
| `evm` | 0.39.1 | medium (error return ignored) | **yes** | **our own** `crates/evm-integration` |
| `ethereum` | 0.14.0 | medium (malleability check) | **yes** | via that `evm` |
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
* `hickory-proto 0.24.4` arrives through `libp2p-dns`; the high-severity NSEC3 advisory has **no patched
  version** at all (`no-fix` in Dependabot, and the DB confirms).

Neither can be fixed by a lockfile update: the fixing versions are on new major lines (`yamux 0.13.x`,
`hickory 0.26.x`) that `libp2p-yamux 0.46` / `libp2p-dns 0.42` do not accept. The two honest paths are
an SDK/libp2p bump, or a **vendored backport** — which this repository already does for other pinned
crates (`patches/sc-executor-wasmtime`, `patches/core2`, `patches/rustix`, `patches/fastbloom`), so it
is established practice rather than a new mechanism.

The one gap that is *ours to fix* is `evm 0.39.1` / `ethereum 0.14.0`: they come from
`crates/evm-integration`'s own dependency, not from the SDK. Bumping it to the line frontier already
uses (`evm 0.43.4`, also present in the lock) aligns the two EVM implementations and clears both
advisories — a real upgrade of ~2,900 lines' dependency surface, so it needs its own verification pass.

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

## Follow-ups this produced

1. ~~**`yamux 0.12.1` — remote panic, reachable from peers.** Either bump libp2p via the SDK, or vendor a
   backported `yamux` under `patches/` with a regression test for the malformed Data frame.~~ **Closed
   2026-09-25: not exploitable.** The resolved versions (`0.12.1`, `0.13.8`, `0.13.10` across the
   tree's lockfiles) are all outside the real window `>=0.13.9,<0.13.10`; the published range was
   coarser than the regression. No patch is needed, and no backport was vendored. What was missing was
   a way to *record and enforce* that judgement, so `security/advisory-scope.toml` plus the
   `advisory scope` gate fail the build if a future lockfile bump resolves `0.13.9` or drops a
   version the record expects. See `docs/security/GHSA-vxx9-2994-q338.md`.
2. **`hickory-proto 0.24.4` — unbounded NSEC3 loop, no upstream fix.** Needs a decision: accept with a
   written reason (it is DNS resolution inside libp2p), or pin/take over the resolver.
3. **`evm 0.39.1` / `ethereum 0.14.0`** — ours; bump `crates/evm-integration` to the frontier line and
   verify the EVM path end to end (`cargo test -p x3-evm-integration`, the EVM lifecycle gate).
4. **A GitHub-advisory gate**, so the next high-severity GHSA in the Rust graph fails something.
5. **npm (87 alerts)** — `x3-app-store` frontend/backend and the smaller JS projects; lockfile bumps
   with `npm audit fix` plus their own tests. Not chain-critical, but 1 critical + several high.
