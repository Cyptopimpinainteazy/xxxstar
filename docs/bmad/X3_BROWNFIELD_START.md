# X3 × BMAD Method — Brownfield kickoff

**Prepared:** 2026-10-30  
**Source baseline:** master at 754ad4687f8b227e776a7a639064d1499dd06682  
**Disposition:** Methodology baseline and installation entrypoint only. The BMad CLI, agents, tests, and release checks **have not executed on X3 hosts**.  
**Release decision:** NO-GO pending security reconciliation and exact-head verification.

## 1. Product brief (Analysis and PM)

X3 Atomic Star is an established Substrate blockchain repository with native, EVM, and SVM internal execution domains, a Rust/Cargo workspace, a separate X3Lang workspace, smart contracts, applications, and a self-hosted CI gate. This is a **brownfield**, not a new blockchain scaffold.

**Goal:** bring one verifiable, secure internal testnet candidate through reproducible build, review, and validation. Keep the RC-1 scope narrow and fail-closed. Do not broaden external-value bridging, public staking, or production claims.

**Non-goals:** replacing consensus, rewriting the codebase, speculative AI/quantum features, enabling external bridges, or deploying public testnet/mainnet during this kickoff.

**Sources:** README.md; AGENTS.md; LAUNCH_SCOPE.md; Cargo.toml; package.json; FEATURE_REGISTRY.toml; RELEASE_GATES.md.

## 2. Existing architecture (Architect)

| System | Implementation area | Required proof |
| --- | --- | --- |
| Node, consensus, runtime | node/; runtime/; Cargo.toml | Compiles and runs at the exact candidate SHA; live finality evidence |
| Invariant-preserving atomic execution | pallets/x3-atomic-kernel/; pallets/x3-cross-vm-router/; pallets/x3-supply-ledger/; pallets/x3-settlement-engine/ | Supply invariants, replay rejection, rollback/refund, authorization and adverse-case tests |
| EVM/SVM integrations | X3-contracts/; crates/; programs/ | Authentic verifier receipts, non-synthetic liquidity, independent external-domain proof |
| Language compiler | x3-lang/ and its independent workflow | Compiler and VM parity; no assumption that root workspace tests cover this subtree |
| CI and release | scripts/local-ci.sh; .github/workflows/local-ci.yml; launch-gates/ | Exit-status-based, exact-head local CI and release gate evidence |

**Observed conflicts to resolve:**

- LAUNCH_SCOPE.md classifies X3 as an **internal v0.4 testnet candidate**, not a public testnet or mainnet-ready network. External EVM/SVM/BTC bridging is gated.
- README.md on master refers to main and .github/workflows/ci.yml; the actual default branch is master and the runner-of-record workflow is .github/workflows/local-ci.yml. Open draft PR #704 attempts this documentation correction.
- The 2026-10-26 master commit notes a pre-existing pallet-x3-automation build failure in its use of frame_system::Pallet::<T>::events(). It requires a fresh exact-head reproduction and repair.
- Open draft PR #703 is stacked on PR #694, not master, and reports unresolved integration with master security fixes plus missing same-head CI. It is **not** an accepted release candidate.
- Registry scores and older completion reports are not interchangeable with CI, security, or real-fleet deployment evidence.

## 3. X3 project context (BMad adoption rules)

1. **Preserve AGENTS.md.** Run BMad project-context in adopt mode; never replace existing agent instructions with generated defaults.
2. **No fake paths.** Never create mock relayers, fake proofs, artificial liquidity, unaudited bypasses, or claims of passing tests without exits and logs.
3. **No main-branch direct commits.** Work from a recorded SHA in a feature branch; peer review and merge under the existing required checks.
4. **Status discipline.** Only use NOT RUN, RUNNING, PASS, FAIL, BLOCKED. Only completed commands with exit codes and exact SHA can establish PASS.
5. **No silently broadened scope.** Do not enable external gateway, PQ, AI consensus, or experimental execution features without explicit audit and governance.
6. **Respect excluded workspaces.** Root Cargo tests do not validate x3-lang/ or independent Rust/contract projects.
7. **Keep security evidence separate from planning.** Any suspected leaked secret on #703 requires classification and containment before candidate promotion.
8. **Preserve the local CI gate.** BMAD planning, reviews, and scoring do not waive scripts/local-ci.sh or the protected x3/local-ci status.
9. **Bind all release evidence** to exact source commit, dependency set, binary, chain spec, genesis, validator network, and verification run.
10. **BMad is not an automatic deployer.** Human approval is required for releases, security-impacting changes, external network admission, and key rotation.

## 4. First deliverable PRD and acceptance criteria

**P0 — Trusted integrated source candidate.** The intended runtime, X3Lang, dependency, security, integration, and CI work exists at one candidate SHA. Reconcile master security commits without replacing integrated dependency changes. Build/test/security jobs execute at that same SHA, findings are triaged, and review evidence is attached. No skip may be treated as a pass.

**P1 — Internal validator evidence.** Fresh multi-host validators demonstrate finality, recovery after node loss, secure key rotation, supply invariants, rollback/refund, authentic cross-VM receipts, persistent state restoration, and operator observability. Historical single-host loopback results do not meet this bar.

**P2 — Separate public testnet admission.** Define audited scope, endpoint policy, rate limiting, anti-abuse protections, public operator documentation, supply/faucet handling, immutable release artifacts, and independent go/no-go approval before network exposure.

## 5. First sprint backlog (Scrum Master)

| Order | Story | Definition of done | Status at kickoff |
| --- | --- | --- | --- |
| S0 | Triage reported GitGuardian issue on draft #703 | Exact finding reviewed; real/false/already-remediated determined; if real, contain, rotate by explicit authorization and verify | BLOCKED — finding not inspected |
| S1 | Repair automation pallet compilation | Reproduce events() error, implement correct fix, add regression coverage, prove targeted and root builds | NOT RUN |
| S2 | Reconcile #703 candidate with master | Security and dependency changes retained, all intended source components integrated, exact candidate SHA frozen | NOT RUN |
| S3 | Re-run all required CI on the exact candidate SHA | Local CI, nested workspaces, X3Lang parity, cross-VM and atomic lifecycle, security, validator proofs with logs | NOT RUN |
| S4 | Review draft #704 and release claims | Documentation corrections match workflow and source; LAUNCH_SCOPE overlap with #703 reconciled | NOT RUN |
| S5 | Demonstrate multi-host internal staging and promotion controls | Proven validator finality/recovery, receipts, governance and release checklist with signed immutable evidence | NOT RUN |

Dependencies: S0 and S1 can proceed independently; S2 follows security triage; S3 follows S1/S2; S4 is required before promotion; S5 follows S3/S4. **No story is marked complete merely because this document exists.**

## 6. Executing BMad on an X3 development machine

On a separate **clean, reviewed feature branch**, from the root of the X3 checkout:

    bash scripts/bmad-x3.sh check
    bash scripts/bmad-x3.sh install
    bash scripts/bmad-x3.sh status

The installer is pinned to the stable BMad 6.12.4 npm package, selects core + BMad Method with the Codex tool target, and refuses the default branch, dirty tree, or an existing installation directory. It does **not** launch validators or edit release policy. Check the npm package provenance and generated diff before committing changes. Then run **bmad-project-context (adopt existing AGENTS.md)** and **bmad-help** inside the configured coding agent, and implement S0/S1 first.

Official upstream installer: https://github.com/bmad-code-org/BMAD-METHOD/blob/main/docs/start/install-bmad.md

## 7. Reporting contract

For every engineering story provide the X3-required fields: **Files changed; Commands run; Proof result; Remaining blockers; Next 10 tasks; Completion percent**. A completion percentage is not measurable without defined, tested acceptance criteria and an explicit denominator; do not estimate whole-project completion from documentation. Preserve evidence and disclose every failed, blocked, or never-run verification.
