# Round 9 — 2026-09-26 — what the finality row still cannot prove

Spawn payloads are unreliable, so read this file. Claim the workstream, and say in your first output
which one you took.

## Verified facts (measured — do not re-derive)

* `crates/x3-atomic-swap/src/finality.rs` no longer decides finality from caller-supplied integers.
  `FinalityCertificate` (`159`) has private fields and two constructors, `observe` (`193`) and
  `with_reported_confirmations` (`218`); `confirmations` is derived by
  `confirmations_at(block_height, observed_at)` (`173`), so a count the anchor does not imply is
  refused (`CertificateConfirmationsDisagree`). `FinalityOracle::verify_finality` (`304`) and
  `is_finalized` (`311`) take the certificate and refuse `CertificateChainMismatch`,
  `CertificateRewindsAcceptedAnchor`, `CertificateStale`, `CertificateBlockAfterObservation`.
  `InMemoryFinalityOracle` (`325`) tracks `accepted_tip` (`346`) and `seen_tip` (`351`) — **in
  memory, per process**.
* `TICKET-152` in `TESTNET_GAP_LEDGER.md` is the acceptance list for what is left. Its three open
  items are: no producer, no persistence, no live reorg evidence.
* anvil and cast are at `/home/lojak/.foundry/bin/` (add to PATH). `solana-test-validator` is at
  `~/.local/share/solana/install/active_release/bin/`.
* `cargo test -p x3-atomic-swap` was **668 + 2 + 31 + 44 passed, 0 failed** at `9a1ab5c37`.
* `crates/x3-atomic-swap` now depends on nothing that can reach an RPC, on purpose: it is a
  `no_std`-ish library. A *producer* that talks to a chain belongs in a crate that already talks to
  chain data, or behind a trait this crate defines and a caller implements. Do not drag
  `tokio`/`reqwest` into `x3-atomic-swap` — check `cargo tree` before adding any dependency.

## Workstream F — nothing builds a certificate from a real chain (`X3-XCHAIN-005`)

The certificate is a checked **shape**. `block_hash` is a field someone sets; nothing binds it to
the chain named in `chain`, nothing ties it to `block_height`, and nothing derives `observed_at` from
a tip the reader did not choose freely. So a caller can present a well-formed certificate for a block
that never existed.

Deliverable (TICKET-152's criteria 1 and 2):

1. **A producer.** A reader that takes real chain data and constructs a `FinalityCertificate`, with
   the `block_hash` ↔ `chain` binding proven by data the reader did not invent: for EVM, the receipt
   at that block (`eth_getTransactionReceipt` gives `blockNumber` + `blockHash` for a `tx_id`), and
   the `observed_at` taken from the *chain's* tip at read time, not from the caller. Where the
   existing `x3-verification-router` / `x3-evm-integration` code already speaks to anvil, reuse it
   rather than writing a second client.
2. **Every refusal reachable from real data.** At minimum: a certificate whose `block_hash` is not
   the block at that height is refused, not repaired; a certificate built from a *different* chain's
   receipt is refused for the chain, not for the depth; a certificate whose anchor moves backwards
   between two reads is refused as a rewind.
3. **Persistence.** The accepted/witnessed tips must survive a process restart, with a test that
   accepts tip `T`, drops the oracle, reloads it, presents an older certificate, and requires
   `CertificateRewindsAcceptedAnchor`. Put the storage behind a trait in `x3-atomic-swap` and
   implement it in the crate that owns the process — the point is the reload, not the file format.
4. **A drill.** One live run against a real anvil chain that starts from *this* repository's code:
   deploy or reuse a contract, send a transaction, wait for the depth the config requires, build the
   certificate from RPC data, settle on it, then produce a certificate from a rewound fork (anvil
   supports `anvil_reorg`/`evm_snapshot`+`evm_revert`) and require the refusal **from real data**.
   A drill script under `scripts/drills/` with a `<name>_live.sh` + `<name>_live_driver.cjs` pair is
   the established shape; see `scripts/drills/wallet_recovery_live.sh`.

Show each check red with it removed, then restore the file byte-identically (the pattern used by
`round8_finality_cert`: save a copy, remove one guard, run, paste back, re-run).

Own `crates/x3-atomic-swap/**`, the new producer crate or module you choose, and
`scripts/drills/<your-name>*`. If you must touch a runtime crate, say so before you do it.

## Ground rules (unchanged, and load-bearing)

1. Own only your workstream's files; re-check `git status` and `git log --oneline -5` before editing.
2. `git add` your own paths only. **Never `git add -A`.** Broad adds have swept other agents' staged
   files twice this session. Never `git reset`.
3. Do not edit `feature-matrix/*.toml`, `scripts/local-ci.sh`, `reports/rc6/*`,
   `reports/panic_unwrap_audit.md`, `FETCH_HEAD`, `libproto_lib/`, `docs/audit/**`,
   `audit-artifacts/**`, or `runtime/src/tests.rs` — the primary agent applies row deltas and
   regenerates the derived artifacts.
4. Unit, integration and single-node drill tests only. No multi-node or fixed-port gates — they
   collide; the primary agent runs those.
5. No fake green: never weaken, delete, skip or `#[ignore]` a test. If a claim is not proven, say so
   in your report instead of scoring it.
6. Commit your own files with a focused message and report the hash. **Do not push.**

## Not in scope this round

GPU measurement (no device on this box), the seven physical servers, the 72-hour soak, public testnet
hosting, the live runtime upgrade: external blockers, unchanged.
