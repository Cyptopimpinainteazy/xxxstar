# X3 Atomic Star — Mainnet Readiness

**Status: NOT MAINNET READY.**

This document is the canonical mainnet-readiness statement required by
`scripts/mainnet_release_gate.py`. It records the honest state of the chain. It
is deliberately conservative: a capability is only listed as ready when a
committed command in this repository reproduces that claim.

Authoritative scope lives in [`LAUNCH_SCOPE.md`](./LAUNCH_SCOPE.md). Readiness
scores derive from [`FEATURE_REGISTRY.toml`](./FEATURE_REGISTRY.toml) and are
validated by `scripts/check-readiness-consistency.sh`.

## Current phase

v0.4 **Internal Testnet Candidate (RC-1)** — closed-operator, internal-only, not
a public testnet and not a mainnet candidate. External bridges, relayers,
parallel execution, GPU validator acceleration, and post-quantum cryptography
are gated out of RC-1.

## Gate command

```bash
make mainnet-check   # -> scripts/mainnet_release_gate.py + check-readiness-consistency.sh
```

Exit 0 = PASS. Exit 1 = FAIL — do not cut a release. A mainnet-ready claim is
forbidden unless every gate passes **and** every `FEATURE_REGISTRY.toml` feature
scores >= 95%.

## Known blockers

| ID | Blocker | Severity |
|---|---|---|
| MR-1 | No verified reproducible build at the audited SHA; `srtool` + `docker` are required by the gate and are not part of this tree | P0 |
| ~~MR-2~~ | **RESOLVED 2026-09-10** — `cargo check --workspace` and `cargo check -p x3-chain-node --features mainnet-rc1` both PASS at SHA `83bc5254f`. The "pre-existing compile error" recorded in `CURRENT_MAINNET_STATUS.md` no longer reproduces | — |
| MR-3 | No multi-validator (4-node) finality run has been archived for this revision | P0 |
| MR-4 | External bridges and relayers are intentionally disconnected in RC-1 (`ExternalBridgesEnabled = false`); bridge/mint paths are audit-ready design only | P0 |
| MR-5 | Validator attestation signatures are not yet cryptographically verified before quorum is counted | P0 |
| MR-6 | `crates/confidential-gpu` attestation is a simulation, not a TEE/NVIDIA CC round trip | P0 |
| ~~MR-7~~ | **RESOLVED 2026-09-10** — the kernel's FRAME mock was dead code; it is now wired in and repaired, the halt check precedes nonce mutation, and 4 named `EconomicHalt` tests pass (`cargo test -p pallet-x3-atomic-kernel`) | — |
| MR-5b | ~~Validator attestation signatures~~ **RESOLVED 2026-09-10** — Ed25519 verification with forgery/truncation/zero-key/mismatch negative tests (`cargo test -p x3-validator-attestation`, 8 passed) | — |
| MR-8 | Runtime/contract/SVM audits not yet performed externally | P1 |
| MR-9 | Genesis ceremony not executed; no signed release tag | P1 |
| MR-10 | `x3-lang` is a separate Cargo workspace and is therefore excluded from `cargo test --workspace` | P2 |

## Required before any mainnet claim

- [ ] Reproducible WASM build via `srtool` with published hashes
- [ ] `cargo check --workspace` and `cargo test --workspace` green at a pinned SHA
- [ ] `cargo check -p x3-chain-node --features mainnet-rc1` green
- [ ] 4-validator finality smoke archived (`zombienet-integration.yml`)
- [ ] Invariant tests for `EconomicHalt` (mint/transfer/swap blocking, refund/recovery allowance)
- [ ] Cryptographic verification of validator attestations with forgery negative tests
- [ ] External audit — runtime pallets, EVM contracts, SVM programs, infra
- [ ] Migration/upgrade rehearsal (`try-runtime`) enforced
- [ ] Rollback plan, monitoring, and governance approval recorded

## Evidence policy

Every readiness claim must be reproducible by a committed command and must be
stored as an artifact (CI log, `launch-gates/evidence/`, `.ai/runlogs/`).
Documentation alone is not evidence. Scores in `FEATURE_REGISTRY.toml` are the
single source of truth for per-feature readiness.

## Baseline evidence — 2026-09-10

| Command | Result |
|---|---|
| `cargo check --workspace --offline` | PASS (7m28s) |
| `cargo check -p x3-chain-node --features mainnet-rc1 --offline` | PASS (4m11s), 9 unused-import warnings in `runtime/src/lib.rs` |
| `cargo test -p x3-validator-attestation --offline` | PASS, 8/8 (includes forgery rejection) |
| `cargo test -p pallet-x3-atomic-kernel --offline` | PASS, 61 lib + 23 integration tests |
| `cargo test --manifest-path crates/confidential-gpu/Cargo.toml --offline` | PASS, 8/8 in fail-closed mode |
| `bash scripts/check-readiness-consistency.sh` | PASS |

Still not verified: reproducible `srtool` WASM build, `cargo test --workspace`,
multi-validator finality, external audits, genesis ceremony.

## Revision policy

Update this file whenever a blocker is closed or a new one is found. Do not
raise the readiness percentage without a committed proof command.
