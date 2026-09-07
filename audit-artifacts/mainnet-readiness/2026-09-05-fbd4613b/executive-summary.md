# X3 Atomic Star — Executive Summary

**Audit date:** 2026-09-05
**Audited commit:** `fbd4613bd8769ac7422278fae441af1b302a1c88` (branch `master`)
**Auditor:** Claude (Anthropic) — AI-assisted static analysis, live build/test execution, and independent multi-agent cross-verification. Not a substitute for a licensed, independent security-audit firm.
**Full detail:** `booklet.pdf` (61 pages) in this directory. Machine-readable detail: `findings.json`, `feature-matrix.csv`.

---

## Verdict

**Overall readiness: 49 / 100.** Public testnet: **NO-GO**. Mainnet: **NO-GO**.

The repository's own `LAUNCH_SCOPE.md` self-labels the project as a "v0.4 Internal Testnet Candidate" and does not claim public testnet readiness — this audit agrees with that self-assessment.

## Methodology

Seven independent, parallel domain investigations (consensus/networking, transaction lifecycle, state/storage, cryptography/keys, contracts/VM/cross-chain, tokenomics, and APIs/ops/proof-gates/performance), each citing exact `file:line` evidence, combined with live command execution: `cargo check --workspace` (clean, exit 0), `cargo audit` (0 blocking vulnerabilities), `forge test` (169/169 EVM tests passing), and targeted `cargo test -p <pallet>` runs for the highest-value pallets. The repository's own extensive prior self-audit documents were treated as unverified claims and independently re-checked — several were confirmed, several were found factually wrong.

## Findings Summary

| Severity | Count |
|---|---|
| Critical | 3 |
| High | 7 |
| Medium | 12 |
| Low | 7 |
| Informational | 4 |
| **Total** | **33** |

## Top Three Blocking Issues

1. **A live, unconditional RPC surface fabricates wallet data on every node** (CRIT-03, `crates/x3-rpc/src/wallet_service_rpc.rs`). `wallet_createWallet` falls back to the publicly-known default test mnemonic; `wallet_getBalance` returns hardcoded fake USD balances. No real cryptography or chain-state lookup backs either. Registered unconditionally on every node's live RPC surface.
2. **An unauthenticated public call can slash any validator with zero evidence** (CRIT-02, `pallets/x3-consensus/src/lib.rs:262`). `report_misbehavior` requires only a signed transaction — no proof, no rate limit, no dispute window — in stark contrast to the correctly proof-gated GRANDPA equivocation path a few hundred lines away.
3. **Validator authoring seeds and a plaintext EVM private key remain reachable in git history** (CRIT-01). Today's remediation commit only untracked the files from the working tree; the secrets themselves are still fetchable from any prior clone or mirror.

## Three Disproven Documentation Claims

- **"Node boots deterministically," cited via srtool checksums** — the checksummed `.log` files do not exist anywhere in the repo, reference a different machine, and are 4+ months stale (HIGH-05).
- **Specific `/health/<feature>` endpoints cited as readiness evidence in `FEATURE_REGISTRY.toml`** — these literal paths do not exist anywhere in the codebase (HIGH-06).
- **Treasury.sol's `routeFee()` described as "callable by anyone"** in the repository's own `TREASURY_POLICY.md` — the actual Solidity is `onlyOwner`-gated; the real (narrower) risk is single-EOA ownership, not an open call (MED-06).

## What Is Genuinely Strong

Aura block production and GRANDPA finality are real, unmodified Substrate/Polkadot-SDK crates. The cross-VM router (81 tests), settlement engine (23 property-based tests), and supply ledger (33 tests incl. a fuzz test) are genuinely tested and passing live. `ExternalBridgesEnabled` is a real, dispatch-time-enforced gate with an automatic circuit breaker. The EVM contract suite passes 169/169 Foundry tests with correct reentrancy and access-control patterns.

## What Must Be Fixed First

All three Critical findings are narrow, well-understood, bounded fixes requiring no new architecture: remove or gate the fabricated wallet RPC, add real evidence requirements to the validator-slashing call, and rewrite git history plus rotate the leaked keys. See `booklet.pdf` Chapter 13 for the full prioritized recovery plan (P0–P3) and Chapter 14 for objective, mostly non-waivable launch gates through mainnet.

## Scope & Limitations

This is a read-only, single-session audit. No files were modified, no contracts deployed, no transactions broadcast, no keys generated. Multi-node network tests were not re-executed (real, non-fabricated-looking loopback evidence exists in the repository and is cited as such). No external, licensed security audit has been engaged for this codebase — that remains the primary gate before any public-facing claim.
