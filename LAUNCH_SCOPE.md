# X3 Atomic Star — Launch Scope (Authoritative)

**Version:** 1.1  
**Date:** 2026-06-10  
**Status:** v0.4 Internal Testnet Candidate  
**Supersedes:** README.md status claims, CURRENT_MAINNET_STATUS.md, MAINNET_RC1_SCOPE.md

---

## Purpose

This document is the single authoritative source of truth for X3 Atomic Star's launch scope. All public messaging, release notes, audit engagement, operator onboarding, and security assessments MUST reference this document. Contradictory claims in other repo documents are superseded by this file.

---

## Current Phase: Internal Staged Testnet (RC-1)

X3 Atomic Star is in **v0.4 Internal Testnet Candidate** phase. This is an internal-only, closed-operator staged testnet. It is NOT a public testnet, NOT a mainnet candidate, and NOT production-ready for external bridging or public-value settlement.

---

## Enabled in RC-1

The following capabilities are part of the shipped RC-1 product surface:

| Capability | Status | Assurance Level |
|---|---|---|
| Universal Asset Kernel with supply-ledger invariants | Active | Unit + property + fuzz tested |
| Internal X3Native / X3Evm / X3Svm domains | Active | Unit tested |
| Internal cross-VM routing (atomic source-debit / destination-credit) | Active | 6-route matrix, invariants, replay protection |
| Packet standard lifecycle (message commitment, replay protection, timeout) | Active | Unit + E2E tested |
| IXL MVP bundle execution and receipt emission | Active | Unit tested |
| LiquidityCore spot swap path and LP lock behavior | Active | Unit tested |
| Kernel invariants and atomic rollback/refund logic | Active | Unit + property tested |
| Internal proof-backed launch gate automation | Active | CI-gated |
| Launch validator and operator tooling | Active | Scripts + docs |
| Proof taxonomy and receipt generation | Active | CI-gated |
| Internal E2E test harnesses (happy path + RC1 + live node) | Active | CI-gated |
| CI gate matrix (fmt, clippy, test, audit, deny, secret-scan, binary) | Active | 20+ CI gates |
| CodeQL + Semgrep semantic analysis (SAST) | Active | CI-gated — release-hardening.yml |
| SBOM + artifact attestation + checksums | Active | CI-gated — release-provenance.yml, release-hardening.yml |
| Zombienet multi-node integration tests | Active | CI-gated — zombienet-integration.yml |
| try-runtime upgrade rehearsal | Active | CI-gated — try-runtime-upgrade.yml, release-candidate-rehearsal.yml |
| Multi-owner CODEOWNERS by domain | Active | 10+ domain teams with backup owners |
| Release creation script | Active | scripts/create-rc1-release.sh |
| Staging testnet setup guide | Active | docs/STAGING_TESTNET_SETUP.md |

## Explicitly Gated Out of RC-1

These features are **intentionally disabled** for RC-1 and MUST remain off until a later audited phase with explicit governance enablement:

| Feature | Guard Mechanism | Target Phase |
|---|---|---|
| `external-gateway` — External EVM/SVM/Bitcoin bridge gateway | Compile-time `compile_error!` with `mainnet-rc1` | Post-audit |
| `parallel-executor` — Parallel block execution | Compile-time `compile_error!` with `mainnet-rc1` | Post-audit |
| `appzone-factory` — Application zone contract factory | Compile-time guard | Post-audit |
| `pq-experimental` — Post-quantum experimental features | Compile-time guard | Post-audit |
| `advanced-dex` — Advanced DEX routing | Compile-time guard | Post-audit |
| `ai-optimizer` — AI consensus optimizer | Compile-time guard | Post-audit |
| `gpu-acceleration` — GPU-critical validator acceleration | Compile-time guard | Post-audit |

## Status Correction

The following claims in `CURRENT_MAINNET_STATUS.md` are **INCORRECT** for the current RC-1 scope:

| Incorrect Claim | Correction |
|---|---|
| "External EVM chains — 100% production" | Code exists but is gated off by `compile_error!` + `ExternalBridgesEnabled = false`; NOT production, NOT enabled in RC-1. Gateway contracts, verification router, and relayer infrastructure are present as **audit-ready design**, not active production paths. |
| "External SVM (Solana) — 100% production" | SVM token adapter code exists but is NOT wired into any active RC-1 production path. Present for audit review only. |
| "Bitcoin — 100% production" | SPV verifier and vault code exists but is NOT active. Present for audit review only. |
| "Relayer infrastructure — 100% production" | Relayer code is present but NOT operational in RC-1. External bridge flows are disabled at genesis. |

## Launched vs Pre-Launch Artifacts

### Fully Launched and Active
- Internal cross-VM routing (native ↔ evm ↔ svm)
- Supply ledger with invariant enforcement
- Settlement engine with refund path
- Packet standard lifecycle
- Internal testnet tooling and scripts
- CI gate matrix (fmt, clippy, tests, audit, deny, secret-scan, SAST, binary)
- CodeQL + Semgrep SAST in CI
- SBOM + artifact attestation pipeline
- Zombienet multi-node integration tests
- try-runtime upgrade rehearsal
- Monitoring and alerting rules
- Validator systemd units
- Docker support stack (explorer, indexer, monitoring, faucet)
- Secret management policy and security scanning
- Documentation (deployment policy, incident runbook, operator guide, RPC policy, staging setup)
- Multi-owner CODEOWNERS by domain
- Release creation script (scripts/create-rc1-release.sh)

### Pre-Launch / Audit-Ready Only (NOT active in RC-1)
- External EVM gateway contracts (`X3ExternalGateway`, `X3VmERC20`, `X3KernelBridge`)
- Verification router strategies (`EvmReceiptVerifier`, `ValidatorQuorumVerifier`, `SolanaFinalizedVerifier`, `BitcoinSpvVerifier`)
- SVM token adapter (`programs/svm/x3_svm_token_adapter`)
- Bitcoin vault and SPV verifier
- Relayer infrastructure
- All external bridge deposit/withdrawal flows

### Requires Manual Trigger
- Signed release tag (run `bash scripts/create-rc1-release.sh`)

## Audit Status

| Area | Audit Status | Target |
|---|---|---|
| Runtime pallets (router, supply-ledger, settlement, kernel) | Not yet externally audited | Pre-public-beta |
| EVM contracts | Not yet externally audited | Pre-public-beta |
| SVM/Anchor programs | Not yet externally audited | Pre-public-beta |
| DevSecOps / Infrastructure | Not yet externally audited | Pre-public-beta |
| Bug bounty | Not yet launched | Pre-public-beta |

## Public Messaging Rules

1. **DO NOT** claim "mainnet-ready," "production," or "100% complete" for any feature not listed as "Fully Launched and Active" above.
2. **DO NOT** claim external bridges, parallel execution, AI consensus, GPU acceleration, or PQ features as operational or production-ready.
3. **DO** describe the project as "v0.4 Internal Testnet Candidate — features gated for audit."
4. **DO** reference this document as the authoritative scope statement.
5. **DO** update this document with every scope expansion or phase transition.

## Next Phase: Public Staged Testnet

The following gates MUST pass before transitioning to a public staged testnet:

| Gate | Criterion | Current Status |
|---|---|---|
| Scope reconciliation | This document is the sole truth source | ✅ COMPLETE |
| Broken docs fixed | README deployment link points to existing doc | ✅ COMPLETE |
| Feature-gate enforcement | `external-gateway` removed from `mainnet-rc1` + `compile_error!` guard added | ✅ COMPLETE |
| CI semantic analysis | CodeQL + Semgrep in CI | ✅ COMPLETE |
| CI release hardening | SBOM + provenance attestation in CI | ✅ COMPLETE |
| Multi-owner review model | CODEOWNERS with domain teams and backup owners | ✅ COMPLETE |
| Staging testnet setup guide | docs/STAGING_TESTNET_SETUP.md | ✅ COMPLETE |
| Release creation script | scripts/create-rc1-release.sh | ✅ COMPLETE |
| Signed release tag published | `gh release create v0.4.0-rc.1` with hashes, SBOM, attestations | ⬜ TODO (run script) |
| FRAME benchmarking CI | CI generates + validates weights | ⬜ TODO |
| External audit — Runtime | Runtime pallets audit complete | ❌ TODO |
| External audit — Contracts | EVM + SVM contracts audit complete | ❌ TODO |
| External audit — DevSecOps | Infrastructure audit complete | ❌ TODO |
| try-runtime mandatory | Snapshot-based upgrade rehearsal enforced in promotion policy | ❌ TODO |
| Zombienet mandatory enforcement | Multi-node tests enforced as stage-exit criterion | ❌ TODO |
| Bug bounty live | Public program operational | ❌ TODO |
| Staging infrastructure deployed | 5-7 validator staging testnet live | ❌ TODO |
| Operator docs validated | Deploy, restore, incident, upgrade, rollback runbooks | ❌ TODO |