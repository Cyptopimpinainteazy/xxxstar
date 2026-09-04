# X3 Atomic Star — Next 10 Tasks

**Updated: 2026-06-09** — P0 remediation complete. Next priority: P1 release engineering + P3 infrastructure.

## Next 10 Tasks

### 1. Create signed GitHub release with provenance
**Priority**: P1
**Effort**: 1–2 person-days
**Description**: Tag an RC-1 release candidate, build node + runtime binaries deterministically, generate checksums + SBOM + artifact attestations, publish as a GitHub Release with release notes.
**Exit criteria**: `gh release list` shows a tagged release with checksums and attestations. Docker images pushed to registry with multi-arch support.

### 2. Add CI secret scanning gate
**Priority**: P1
**Effort**: 0.5 person-days
**Description**: Add a GitHub Actions job (or pre-commit hook) that scans for leaked key patterns, API tokens, and mnemonic phrases. Use `trufflehog` or `git-secrets`.
**Exit criteria**: CI fails if any secret-like pattern is detected outside exempted directories.

### 3. Add Zombienet integration test to CI
**Priority**: P1
**Effort**: 2–3 person-days
**Description**: Create a Zombienet config (template exists in `docs/Zombienet-template.toml`), write smoke tests for block production + finality, wire into GitHub Actions as a CI job.
**Exit criteria**: CI runs a 3-node ephemeral testnet, validates 10+ blocks produced and finalized, reports pass/fail.

### 4. Add try-runtime upgrade rehearsal pipeline
**Priority**: P1
**Effort**: 1–2 person-days
**Description**: Add `try-runtime on-runtime-upgrade live` to CI. Create a staging upgrade script that exercises storage migrations against a network snapshot.
**Exit criteria**: CI dry-runs a runtime upgrade against a mock or snapshot state and validates no storage migration failures.

### 5. Add FRAME benchmarking CI job
**Priority**: P1
**Effort**: 1–3 person-days
**Description**: Run `cargo build --release -p x3-chain-runtime` in benchmarking mode, extract weight files, verify fee multipliers align with expected ranges. Add a regression gate that fails if weights drift beyond a threshold.
**Exit criteria**: CI produces benchmark-derived weights, checks weight files are in sync with runtime.

### 6. Deploy public testnet (alpha)
**Priority**: P1/P3
**Effort**: 2–4 weeks with infra
**Description**: Stand up 5–7 validator nodes on separate infrastructure. Deploy indexer, block explorer, public RPC gateway with rate limiting. Publish faucet + wallet onboarding docs.
**Exit criteria**: External users can connect via Polkadot.js, submit transactions, view blocks on explorer, and request test tokens from faucet.

### 7. Commission external audit (runtime + contracts)
**Priority**: P2
**Effort**: 6–10 weeks (auditor-dependent)
**Description**: Scope and engage an external security firm to audit runtime-critical pallets (cross-VM router, supply ledger, settlement engine, DEX, LP locker, launchpad), EVM contracts, SVM program, and DevSecOps.
**Exit criteria**: Audit report received, critical/high findings documented and remediated.

### 8. Harden observability stack
**Priority**: P2
**Effort**: 1–2 person-weeks
**Description**: Deploy Prometheus + Alertmanager with production rules (block production alerts, finality stalled, memory/disk thresholds). Add OpenTelemetry Collector for trace/metric/log correlation. Create Grafana dashboards.
**Exit criteria**: Alertmanager configured with PagerDuty/email/Slack routing, dashboards visible, runbooks documented.

### 9. Document governance framework
**Priority**: P2
**Effort**: 3–5 person-days
**Description**: Publish multi-sig governance doc (who controls upgrades, validator set changes, treasury). Create upgrade charter (proposal, voting, enactment, emergency hotfix process). Define treasury policy (spending categories, approval thresholds).
**Exit criteria**: Governance documents reviewed by stakeholders, stored in `docs/governance/`.

### 10. Create legal compliance package
**Priority**: P3
**Effort**: Ongoing
**Description**: Token classification memo, terms of service for hosted endpoints, privacy policy, sanctions/jurisdiction handling, validator operator agreements.
**Exit criteria**: Counsel-reviewed documents stored in `docs/legal/`.

## Backlog (lower priority)

11. **Public testnet beta** — Stress network, run upgrade drill, fix operator issues
12. **Mainnet genesis freeze** — Signed genesis, release checklist complete, rollback plan
13. **Mainnet launch** — Authority-set Aura + GRANDPA chain with governance-gated bridges disabled
14. **Add staking (post-launch)** — Implement `pallet-staking` with permissionless validator onboarding
15. **Enable external bridges (post-audit)** — Governance vote to enable bridge pallet

## Timeline Targets

| Phase | Target Duration | Cumulative |
|---|---|---|
| P0 Remediation | ✅ COMPLETE | Week 0 |
| P1 Release Engineering | 2–3 weeks | Week 3 |
| Public Testnet Alpha | 3–4 weeks | Week 7 |
| External Audit | 6–10 weeks (parallel) | Week 13 |
| Public Testnet Beta | 3–4 weeks | Week 11 |
| Mainnet Cutover | 2 weeks | Week 15 |

**Estimated earliest mainnet: ~15 weeks from now (mid-September 2026)**