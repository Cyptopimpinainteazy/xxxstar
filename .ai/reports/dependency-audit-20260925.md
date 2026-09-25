# The dependency audit had no gate, and the first run found a real vulnerability

2026-09-25. While deciding the remaining Rust Dependabot alerts, the repository's own dependency
configuration turned out to be dead weight, and running it immediately produced a finding that every
gate in the fast set was green with.

## The gap

`.cargo/audit.toml` carries a long ignore list with a reason per entry. `deny.toml` mirrors it and
says so ("cargo-deny uses its own list below ... Keep both lists in sync"). `.cargo/README.md`
explains the vendoring setup. But:

```console
$ bash scripts/local-ci.sh --list | grep -iE 'audit|deny|advisory'
  - advisory scope
  - audit matrix freshness
```

No gate ran `cargo-audit` or `cargo-deny`. Concretely, nothing in the repository would have:

* noticed a **new** advisory in the lockfile, RustSec or not;
* noticed that an **ignored** advisory was fixed and its entry now suppresses nothing;
* checked that the two ignore lists still agree.

The configuration was written for a tool the CI never invoked.

## What the first run found

```console
$ cargo audit
Crate:     rustls
Version:   0.23.44
ID:        RUSTSEC-2026-0285
Title:     TLS 1.3 handshake messages incorrectly accepted across encryption level boundaries
...
error: 1 vulnerability found!
warning: 26 allowed warnings found
```

`rustls 0.23.44` is inside the node: `futures-rustls 0.26.0` -> `libp2p-websocket 0.44.0` ->
`libp2p 0.54.1` -> `sc-network` -> `x3-chain-node`. The advisory is unignored — it was simply not
accounted for, and `advisory scope` could not have caught it either, because it *does* have a
RustSec id and lives in RustSec rather than only in the GitHub database.

## The fix

One version, nothing else in the lockfile:

```diff
-name = "rustls"
-version = "0.23.44"
+name = "rustls"
+version = "0.23.45"
```

`cargo update -p rustls@0.23.44 --precise 0.23.45`; the only other changes in `Cargo.lock` are the
packages that record a dependency on the new version. After the bump:

```console
$ cargo audit
warning: 26 allowed warnings found
$ echo $?
0
```

Verified with the full fast gate set, which compiles and tests the node the crate ships in. The
runtime WASM is unaffected — `python3 scripts/check-runtime-hash-freshness.py --base origin/master`
reports "6 file(s) changed, none in the runtime's dependency graph (129 packages)" — because rustls
is a std-only networking dependency.

## The gate this produced

`dependency audit` now runs `scripts/check-dependency-audit.sh`:

* audits with `--no-fetch`, so it is hermetic and takes about a second;
* **fails when the local RustSec database is older than 45 days** rather than warning, because a
  database nobody refreshed would turn this gate into exactly the quiet false green the repository
  keeps finding;
* names the install commands when `cargo-audit` is absent (the GNU prebuilt wants GLIBC 2.38+; the
  musl one is static, and the binary is a cargo multicall that must be invoked as `cargo audit`).

`local-ci.sh` reports `cargo-audit` in its prerequisite line and prints a note when it is missing.

`dependency audit` and `advisory scope` are complementary, not redundant: cargo-audit sees RustSec
and cannot see a GitHub-only advisory, and `advisory scope` enforces the records for advisories that
cargo-audit cannot represent.

## Still not covered (ticket)

1. **`cargo-deny` is still not run.** `deny.toml` remains configuration without a gate. `advisory
   scope` cross-checks the two ignore lists *for advisories it has records for*, so a divergence on
   any other id is still invisible.
2. **A new GitHub-only advisory still starts invisible.** `advisory scope` fails when a recorded
   advisory drifts; it cannot notice one that was never recorded. A gate that lists open Dependabot
   alerts for the Rust ecosystem and fails on `high`+ that is neither recorded nor ignored would close
   this, and needs `gh` and network.
3. **The 26 allowed warnings are unexamined as a set.** They are all RustSec `unmaintained` /
   `unsound` entries whose ignore entries predate this gate. Now that the gate runs, a stale entry is
   worth pruning, but each needs a look rather than a bulk edit.

## The 26 allowed warnings, as measured

`unmaintained`: bincode 1.3.3, bitmaps 2.1.0, derivative 2.2.0, fxhash 0.2.1, im 15.1.0, instant
0.1.13, libsecp256k1 0.6.0 and 0.7.2, paste 1.0.15, proc-macro-error 1.0.4, proc-macro-error2 2.0.1,
ring 0.16.20, rustls-pemfile 1.0.4, sized-chunks 0.6.5, smallstr 0.3.1, yaml-rust 0.4.5.

`unsound`: im 15.1.0 (`OrdSet` aliasing), lru 0.7.8 / 0.12.5 / 0.16.4 (panic-safety and `IterMut`),
memmap2 0.5.10, pkcs11 0.5.0, rand 0.7.3, sized-chunks 0.6.5, solana_rbpf 0.8.5.

These are the RustSec-visible subset. The Dependabot-visible subset with no RustSec id — the
`hickory-proto` pair, `yamux`, `libp2p-gossipsub`, `libp2p-quic`, `jsonwebtoken`, `serde_with`,
`git2`, `atty` and the rest — is tracked in `.ai/reports/dependabot-triage-20260925.md`.
