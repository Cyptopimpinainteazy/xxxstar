# cargo-deny measured, and the dependency ignore lists rebuilt from evidence

2026-09-25, following `.ai/reports/dependency-audit-20260925.md` (which wired `cargo-audit` as a
gate and found RUSTSEC-2026-0285 in rustls). This records what the second tool says, what its
answer exposed about the two ignore lists, and what was done about it.

## What each check reports

`cargo-deny 0.20.2`, against the repository's `deny.toml`:

| check | result | what it was |
| --- | --- | --- |
| `advisories` | **FAILED** | 5 findings and 39 ignore entries matching nothing |
| `bans` | ok | — |
| `licenses` | **FAILED** | 13 `unlicensed` + 5 `rejected` (ticket, below) |
| `sources` | ok | — |

The five advisory findings: `RUSTSEC-2026-0002` (lru 0.12.5 `IterMut`), `RUSTSEC-2026-0253` (lru
`LruCache::pop`), `RUSTSEC-2022-0034` (pkcs11 0.5.0), `RUSTSEC-2025-0010` (ring 0.16.20
unmaintained) and `RUSTSEC-2026-0215` (smallstr 0.3.1 unmaintained).

## The finding that mattered: the ignore lists were mostly dead

`cargo deny` reports `advisory-not-detected` when an `ignore` entry matches nothing. Measured:

* `deny.toml` — **51 entries, 12 matched**.
* `.cargo/audit.toml` — **35 entries, 11 matched** (24 of its ids were also dead by deny's measure).
* The two files disagreed, despite `deny.toml` saying "Keep both lists in sync".

So a reader of either file saw a wall of "allowed advisories" and could not tell that most of them
suppressed nothing. The dead ones were not junk — they were real decisions that had expired when the
crate underneath them moved:

* the entire wasmtime 8.x/35.x cluster (14 ids) — **wasmtime is still in the graph**, but on versions
  outside those advisories' ranges, so the entries no longer matched anything;
* `RUSTSEC-2023-0071` (rsa), `RUSTSEC-2026-0178` (tokio-postgres), `RUSTSEC-2026-0235` (rkyv),
  `RUSTSEC-2025-0055` (tracing-subscriber) — same shape;
* every `unmaintained` advisory, which is now declined wholesale rather than one entry at a time.

## What was changed

**Policy, stated instead of implied.** `cargo-audit` already reported `unmaintained`/`unsound`/
`notice` as warnings and failed only on vulnerabilities (`settings.informational_warnings`).
`cargo-deny` failed on unmaintained crates, which is why the list had grown to name each one.
`deny.toml` now says `unmaintained = "none"`, so the two tools agree on what "informational" means.
(`unsound` has no equivalent switch, so the four unsound advisories that are present are still named
individually — a *new* unsound advisory fails the gate.)

**Lists rebuilt from measurement.** Both files now carry the 15 advisories that are present in the
built graph and accepted, each with its reason and the path that reaches it. Nothing is listed that
does not match.

**The one unavoidable difference is named and enforced.** The tools model different things:
`cargo-audit` audits every package in `Cargo.lock`; `cargo-deny` builds a graph for the four targets
in `deny.toml`'s `[graph]`. `RUSTSEC-2023-0071` is exactly that case — `rsa 0.9.10` is in the lock
through `sqlx-mysql`, and no crate in this workspace enables sqlx's mysql feature, so no configured
target builds it. cargo-audit fires on it and cargo-deny never sees it. It is therefore ignored in
`.cargo/audit.toml` and *not* in `deny.toml`, and `scripts/check-advisory-scope.py` enforces that
split in both directions: `deny.toml` may never ignore something `audit.toml` does not, and an
audit-only id must be named in `LOCK_ONLY_ADVISORIES` with a reason or the gate fails.

## The gate

`dependency audit` now runs both tools, and fails when either does:

```
cargo audit --no-fetch                                    # every package in Cargo.lock
cargo deny check advisories -D advisory-not-detected     # built graph, and stale-entry detection
```

`-D advisory-not-detected` is the load-bearing flag: without it the dead entries above were
*warnings*, so the rot could return silently. Both checks run offline against the local RustSec
database, whose age is still checked as a failure at 45 days.

Verified by mutation: adding a stale id to `deny.toml` makes the gate fail with
`error[advisory-not-detected]`; adding an audit-only id that is not declared lock-only makes
`advisory scope` fail with the reason.

## Still open (ticket)

1. **`cargo deny check licenses` fails: 13 `unlicensed` + 5 `rejected`.** This is a policy decision,
   not a cleanup: the rejected set includes `GPL-3.0`, `LGPL-3.0 OR MPL-2.0`, `BSL-1.0`, `Unlicense`
   and `OpenSSL`, and whether those may ship in a node binary is a licensing call for the project,
   not something to decide by editing an allow-list. The `unlicensed` set includes this repository's
   own crates (`x3-wallet-cli` among them), which is trivially fixable and worth doing separately.
2. **`cargo deny check bans` reports `multiple-versions = "warn"`** over a very large graph
   (195k lines of inclusion output). Not a failure, but nobody has looked at the duplicates it names;
   the output is too large to read, so the check is effectively unenforced.
3. **`cargo deny check sources` passes** and is worth wiring for free once 1 and 2 have a decision.
