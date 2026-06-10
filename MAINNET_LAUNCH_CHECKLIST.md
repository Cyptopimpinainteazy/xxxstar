# X3 Mainnet Launch Checklist

## Phase 0 — P0 Critical Remediation
All items must be green before public testnet.

| # | Item | Status | Verified By |
|---|---|---|---|
| 0.1 | Rotate all committed secrets, purge git history | ✅ DONE | `grep -r secret_key deployment/keys/` — only `REPLACED_` |
| 0.2 | Remove unsafe Docker defaults | ✅ DONE | `grep -c "Unsafe\|--tmp" Dockerfile.validator` = 0 |
| 0.3 | Align toolchain across all build images | ✅ DONE | All Dockerfiles and rust-toolchain.toml use 1.90.0 |
| 0.4 | Create deny.toml for cargo-deny | ✅ DONE | `deny.toml` at repo root with license/bans/advisory policy |
| 0.5 | Document secret management policy | ✅ DONE | `docs/SECRET_MANAGEMENT_POLICY.md` |
| 0.6 | Add deployment/keys/ to .gitignore | ✅ DONE | Pattern added |
| 0.7 | Create deployment/keys/README.md | ✅ DONE | Template warning + rotation instructions |
| 0.8 | Verify mainnet-check Dockerfile builds | ⬜ TODO | `docker build -f Dockerfile.mainnet-check -t x3-mainnet-check .` |

## Phase 1 — Staging Testnet

| # | Item | Status | Verified By |
|---|---|---|---|
| 1.1 | CI secret scanning gate | ✅ DONE | TruffleHog + placeholder check in full-ci.yml |
| 1.2 | Create signed GitHub release tag | ⬜ TODO | `gh release create v0.4.0-rc.1` |
| 1.3 | Artifact attestation + SBOM pipeline | ✅ DONE | release-provenance.yml + release-hardening.yml |
| 1.4 | Zombienet CI tests (3-node smoke) | ✅ DONE | zombienet-integration.yml with 4-validator smoke |
| 1.5 | try-runtime upgrade pipeline | ✅ DONE | try-runtime-upgrade.yml + release-candidate-rehearsal.yml |
| 1.6 | CodeQL + Semgrep SAST gates | ✅ DONE | release-hardening.yml with CodeQL + Semgrep + SBOM |
| 1.7 | FRAME benchmarking CI | ⬜ TODO | CI generates + validates weights |
| 1.8 | Multi-node staging testnet (5-7 nodes) | ⬜ TODO | Separate infra, regular tests |
| 1.9 | Indexer + PostgreSQL deployment | ⬜ TODO | Health checks passing, block ingestion |

## Phase 2 — Public Testnet Alpha

| # | Item | Status | Verified By |
|---|---|---|---|
| 2.1 | Public RPC gateway with rate limiting | ⬜ TODO | External users can query |
| 2.2 | Block explorer deployment | ⬜ TODO | Blocks visible, searchable |
| 2.3 | Faucet with address-level caps | ⬜ TODO | Test tokens claimable |
| 2.4 | Wallet onboarding docs | ⬜ TODO | Polkadot.js + custom wallet guide |
| 2.5 | Prometheus + Alertmanager + dashboards | ⬜ TODO | Block production alerts, uptime SLO |
| 2.6 | OpenTelemetry collector deployment | ⬜ TODO | Logs, metrics, traces pipeline |
| 2.7 | Known-issues page published | ⬜ TODO | Public bug tracker or page |
| 2.8 | Public scope statement published | ⬜ TODO | Bridges disabled, authority-set consensus |

## Phase 3 — External Audit

| # | Item | Status | Verified By |
|---|---|---|---|
| 3.1 | Audit scope document signed by team | ⬜ TODO | Auditor engagement letter |
| 3.2 | Runtime-critical Rust audit engaged | ⬜ TODO | Report received |
| 3.3 | EVM contracts audit engaged | ⬜ TODO | Report received |
| 3.4 | SVM/Anchor program audit engaged | ⬜ TODO | Report received |
| 3.5 | DevSecOps audit engaged | ⬜ TODO | Report received |
| 3.6 | Critical/high findings remediated | ⬜ TODO | Re-test pass |
| 3.7 | Pre-launch assurance memo received | ⬜ TODO | Go/no-go recommendation |

## Phase 4 — Public Testnet Beta

| # | Item | Status | Verified By |
|---|---|---|---|
| 4.1 | 6+ weeks of stable uptime | ⬜ TODO | Uptime dashboard |
| 4.2 | Runtime upgrade drill performed | ⬜ TODO | Governance vote → upgrade enacted |
| 4.3 | Validator onboarding tested | ⬜ TODO | External validator joins set |
| 4.4 | Incident response drill performed | ⬜ TODO | Runbook validated |
| 4.5 | Bug bounty program running | ⬜ TODO | Public program |
| 4.6 | Economics validated (fees, rewards) | ⬜ TODO | Fee quotes match expectations |

## Phase 5 — Mainnet Cutover

| # | Item | Status | Verified By |
|---|---|---|---|
| 5.1 | Governance documents published | ⬜ TODO | Multi-sig, treasury, upgrade charter |
| 5.2 | Legal compliance package finalized | ⬜ TODO | Counsel-reviewed |
| 5.3 | Mainnet genesis spec finalized | ⬜ TODO | Signed genesis file |
| 5.4 | Genesis ceremony checklist complete | ⬜ TODO | `scripts/mainnet/genesis_ceremony.sh` |
| 5.5 | Release runbook approved | ⬜ TODO | Go/no-go sign-off |
| 5.6 | Validator onboarding runbook | ⬜ TODO | Authority set configured |
| 5.7 | Treasury account funded | ⬜ TODO | Genesis allocation |
| 5.8 | Faucet stopped, bridge gate verified closed | ⬜ TODO | Fail-closed verification |
| 5.9 | Block explorer updated to mainnet | ⬜ TODO | Mainnet chain visible |
| 5.10 | Mainnet genesis announced | ⬜ TODO | Public announcement |
| 5.11 | Rollback plan documented | ⬜ TODO | Emergency downgrade path |
| 5.12 | Incident response channels active | ⬜ TODO | On-call, escalation paths |

## Phase 6 — Post-Launch

| # | Item | Status | Verified By |
|---|---|---|---|
| 6.1 | Post-launch monitoring (72h watch) | ⬜ TODO | Duty roster |
| 6.2 | Security incident retro (30-day) | ⬜ TODO | Lessons learned document |
| 6.3 | Block production / finality monitoring SLO | ⬜ TODO | SLO dashboard |
| 6.4 | Runtime upgrade governance process | ⬜ TODO | First proposal → vote → enactment |

## Sign-Off Gates

| Gate | Required Approvals | Status |
|---|---|---|
| G1: P0 remediation complete | Engineering Lead | ✅ PASS |
| G2: Staging testnet healthy | DevOps Lead | ⬜ TODO |
| G3: Public testnet stable | Engineering Lead | ⬜ TODO |
| G4: Audit remediated | Security Lead | ⬜ TODO |
| G5: Governance docs approved | Stakeholder Council | ⬜ TODO |
| G6: Legal package signed | Legal Counsel | ⬜ TODO |
| G7: Final go/no-go | All Leads | ⬜ TODO |