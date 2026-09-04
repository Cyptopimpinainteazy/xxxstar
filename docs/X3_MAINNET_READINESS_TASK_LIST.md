# X3 Mainnet Readiness — Critical Remediation Task List

## P0 — Security & Key Hygiene
- [ ] Rotate committed bootnode secrets and purge from git history
- [ ] Remove unsafe validator defaults from Dockerfile.validator
- [ ] Align toolchain versions across rust-toolchain.toml and Dockerfiles
- [ ] Create deny.toml for cargo-deny and fix Dockerfile.mainnet-check path
- [ ] Document key rotation and secret management policy

## P1 — Release Engineering
- [ ] Create GitHub release artifacts with provenance
- [ ] Add secret scanning to CI pipeline
- [ ] Expand CI with Zombienet/Moonwall integration tests
- [ ] Add try-runtime upgrade rehearsal pipeline
- [ ] Add FRAME benchmarking to CI

## P2 — Documentation & Status
- [ ] Update CURRENT_MAINNET_STATUS.md with accurate P0 remediation status
- [ ] Create X3_PROOF_LEDGER.md
- [ ] Create X3_COMPLETION_STATUS.md
- [ ] Create X3_NEXT_TASKS.md
- [ ] Fix broken deployment doc links
- [ ] Create mainnet launch checklist

## P3 — Infrastructure Hardening
- [ ] Public RPC gateway policy and rate limiting
- [ ] Validator TLS and production defaults
- [ ] Prometheus + Alertmanager + OTel observability stack
- [ ] Explorer + Faucet + Wallet docs
- [ ] Multi-node staging testnet configuration

## P4 — Governance & Legal
- [ ] Multi-sig governance documentation
- [ ] Treasury and upgrade charter
- [ ] Legal compliance package