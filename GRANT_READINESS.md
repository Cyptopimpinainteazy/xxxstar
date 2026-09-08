# Grant Readiness

**Evidence date:** 2026-09-08  
**Evidence branch:** `master` at `7ddf0f27fa922a7af25d278a4376b05ca631b055`  
**Project phase:** v0.4 internal testnet candidate

X3 Atomic Star is a Substrate-based blockchain research and engineering project focused on atomic execution across native, EVM, and SVM domains. The repository contains working runtime pallets, contracts, relayer code, test infrastructure, operator tooling, and experimental components. It is not mainnet-ready, and external bridges remain disabled by default.

## Current completion

The repository's canonical `FEATURE_REGISTRY.toml` tracks 15 implemented features and assigns a readiness score only when the registry points to real code. Its current unweighted average is **51%**.

This percentage measures engineering readiness, not percent of source code written. A higher score requires implementation, integration, tests, CI enforcement, security controls, and operational evidence. Experimental files and design documents do not earn completion credit.

| Area | Readiness | Evidence-backed status |
|---|---:|---|
| Internal cross-VM router | 88% | Six-route Native/EVM/SVM matrix, replay protection, refund paths, and supply checks are represented by named tests |
| DEX | 75% | Guarded testnet implementation |
| Token factory | 75% | Guarded testnet implementation |
| Atomic lock lifecycle | 68% | Testnet implementation; broader network evidence remains limited |
| Runtime evolution layer | 65% | Guarded testnet implementation |
| Gateway service | 65% | Real service code; external bridge activation remains gated |
| Launch gates | 55% | Evidence and release tooling exist; active-branch CI targeting needs correction |
| Wallet pallet | 55% | Testnet implementation; security review remains open |
| Sentinel | 50% | Real pallet and tests; guarded deployment |
| Atomic kernel | 40% | Core code exists; earlier fictional test names were removed and missing invariant tests remain open |
| Benchmark reactor | 40% | Implemented tooling with incomplete critical-path enforcement |
| Bitcoin gateway | 25% | Simulation/regtest stage; no production signer quorum |
| Swarm core | 25% | Experimental and guarded |
| Repository scanner | 25% | Development tooling |
| Tauri OS | 15% | Early application layer |

## What is working

- A Substrate node and runtime with Aura block production and GRANDPA finality.
- Internal X3Native, X3Evm, and X3Svm domain routing.
- Supply accounting, replay protection, timeout handling, and refund paths in the internal router.
- An enforced genesis kill switch for external bridges.
- EVM gateway contracts and receipt-proof verification code with test evidence recorded in the repository.
- Gateway HTTP, GraphQL, database, and readiness endpoints.
- SVM HTLC tests and a corrected 32-byte program identifier.
- Four-validator cold-start and one-validator-loss recovery drill automation.
- Dependency, static-analysis, provenance, and release-hardening workflows in the repository.

## What is not ready

- Trust-minimized external EVM, SVM, or Bitcoin value transfer in production.
- Multi-validator bridge quorum and production key custody.
- Independent bridge-root proof validation; one current path only proves that the submitted proof is non-empty.
- Production Bitcoin signer quorum.
- Public staging infrastructure with independent operators.
- External security audits and a live bug bounty.
- A clean active-branch CI story: the default branch is `master`, while the critical-path workflow currently targets `main`.

## Funding use

Grant funding would be applied to work that can be independently verified:

1. production bridge quorum, key custody, and failure recovery;
2. external EVM/SVM/BTC proof validation;
3. public multi-operator staging infrastructure;
4. third-party runtime and contract audits;
5. reproducible benchmarks, release artifacts, and operator drills;
6. documentation and evidence that stay synchronized with code.

## Claims policy

Public grant material should use these rules:

- Describe X3 as an **internal testnet candidate**.
- Describe internal cross-VM routing separately from external cross-chain bridging.
- Do not call external bridges production-ready or mainnet-ready.
- Cite a commit, test, workflow, report, or source path for technical claims.
- Treat `FEATURE_REGISTRY.toml`, `LAUNCH_SCOPE.md`, and the current failure ledger as the source of truth.
- Lower a completion score when evidence disappears. Never preserve a number for marketing.
