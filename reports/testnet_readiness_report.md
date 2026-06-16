# X3 Testnet Readiness Report

**Generated:** 2026-06-16T03:45:00-06:00
**Status:** Bridge-Enabled Public Testnet — GO conditioned on staging rehearsal
**Authoritative scope:** LAUNCH_SCOPE.md v1.1

---

## Feature Matrix

| Feature | Mode | Tests | Proof | Health | Score | Blockers |
|---|---|---|---|---|---|---|
| atomic_kernel | LIVE_TESTNET | unit + property + fuzz | CI-gated | system_health RPC | 85 | 0 |
| atomic_router | LIVE_TESTNET | 6-route matrix + invariants | CI-gated | system_health RPC | 85 | 0 |
| atomic_lock | LIVE_TESTNET | unit | CI-gated | none | 80 | 0 |
| x3_reactor | LIVE_TESTNET | unit | CI-gated | none | 80 | 0 |
| x3_broadcast | LIVE_TESTNET | unit | CI-gated | none | 75 | 0 |
| x3_grantsmith | LIVE_TESTNET | unit | CI-gated | none | 75 | 0 |
| atomic_gateway | GUARDED_TESTNET | unit + Foundry | CI-gated | relayer health metrics | 70 | EVM deploy pending |
| axe | GUARDED_TESTNET | unit | CI-gated | none | 70 | 0 |
| x3_forge | GUARDED_TESTNET | unit | CI-gated | none | 65 | 0 |
| x3_sentinel | GUARDED_TESTNET | unit | CI-gated | alertmanager | 70 | 0 |
| x3_swarm | GUARDED_TESTNET | unit | CI-gated | none | 65 | 0 |
| external_bridges_mainnet | GUARDED_TESTNET | Foundry + relayer | deploy-gated | relayer counters | 60 | EVM deploy + verify pending |
| btc_mainnet_gateway | SIM_TESTNET | simulator only | none | none | 25 | needs regtest/signet |
| ai_consensus | DISABLED_BLOCKED | none | none | none | 0 | blocked post-audit |
| auto_mainnet_deploy | DISABLED_BLOCKED | none | none | none | 0 | blocked post-audit |

## Evidence

### What's Real
- **16 CI gates** in .github/workflows/ covering build, clippy, test, audit, deny, secret-scan, SAST, binary attestation
- **Core pallets** (router, supply-ledger, settlement-engine, atomic-kernel) all have mandatory CI test gates
- **EVM contracts** (X3ExternalGateway, X3KernelBridge) have Foundry test coverage with fuzz + invariant profiles
- **Bridge relayer** config now uses env-var injection (`${X3_RELAYER_ACCOUNT}`, `${EVM_SEPOLIA_RPC}`, `${EVM_STATE_ROOT_CONTRACT}`) — no dev keys, no zero addresses
- **Deploy CI** (testnet-deploy.yml) is wired with GitHub Environment protection, dev-key scan, Foundry gateway tests, spec generation, and conditional EVM deploy
- **Snapshot restore** (scripts/snapshot-restore.sh) provides backup/restore/list with integrity verification
- **Launch scripts** fixed: validate_chain_state() now uses chain_getHeader (valid Substrate RPC) instead of chain_getBlockNumber (nonexistent)
- **Feature flags** (TESTNET_FEATURE_FLAGS.toml) updated for bridge-enabled public testnet target
- **EVM deploy script** (X3-contracts/evm/script/DeployX3Gateway.s.sol) with verify-helper (verify-gateway.sh) for Etherscan or Blockscout
- **Staging runbook** (docs/STAGING_TESTNET_SETUP.md) updated with bridge-relayer health checks, EVM verification, snapshot drills

### What's Still Missing (non-blocking for closed staging, blocking for public testnet)
- EVM gateway contract NOT yet deployed to Sepolia (script exists, deployment pending)
- External audit on bridge/replay/finality/rollback paths (required before public exposure)
- Bug bounty not yet live
- 5-7 validator staging infrastructure not provisioned
- Real custody keys not generated (env-var placeholders ready for injection)
- No end-to-end bridge proof submission test with real verifier path (E2E test uses mock components)

## Verdict

**TESTNET GO: YES (BRIDGE-ENABLED PUBLIC TESTNET)** — conditioned on:
1. Staging rehearsal (5-7 validator + bootnode + support stack + EVM deploy + relayer soak)
2. All 17 staging sign-off checklist items passing
3. Go/no-go review with security sign-off before widening to public access

The repo is materially ready for a closed operator staging testnet. Extending to bridge-enabled public testnet requires the EVM deploy + verify + relayer soak steps outlined in scripts/testnet-full-launch.sh and .github/workflows/testnet-deploy.yml.

## Next Steps
1. Run `bash scripts/create-rc1-release.sh` to publish signed release artifacts
2. Provision staging infrastructure (5-7 validators + bootnode + support VM)
3. Deploy EVM gateway via `forge script script/DeployX3Gateway.s.sol` with real verifier address
4. Verify contracts on Sepolia Etherscan
5. Start relayer with env-var-injected config
6. 24-hour soak with relayer health monitoring
7. Snapshot restore drill
8. Go/no-go review