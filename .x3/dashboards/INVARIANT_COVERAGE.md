# X3 Invariant Coverage Dashboard

## Universal Asset Kernel
- Status: **NEEDS REVIEW**
- Files: 541
- Test files found: 96
- Risky files: 372

  - `.scripts/x3_graph_builder.py` -> atomic rollback risk, bridge risk, fake timestamp risk, known issue, local-only config risk, mock hash risk, panic risk, replay risk, replay/nonce risk, stub risk, supply invariant risk, unfinished logic, unsafe code
  - `.scripts/x3_smell_scan.sh` -> atomic rollback risk, bridge risk, fake timestamp risk, known issue, local-only config risk, mock hash risk, panic risk, replay risk, replay/nonce risk, stub risk, supply invariant risk, unfinished logic, unsafe code
  - `launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-201255.md` -> atomic rollback risk, bridge risk, replay risk
  - `launch-gates/reports/X3-MAINNET-GO-NO-GO-20260501-203300.md` -> atomic rollback risk, bridge risk, replay risk
  - `X3-contracts/shared/gpu-parity-core/src/lib.rs` -> panic risk
  - `X3-contracts/shared/parity-core/src/lib.rs` -> panic risk
  - `apps/dashboard/src/panelRegistry.tsx` -> bridge risk
  - `apps/dashboard/src/panels/adapters/SolanaAdapterPanel.tsx` -> bridge risk
  - `apps/dashboard/src/panels/security/AuditDashboardPanel.tsx` -> bridge risk
  - `apps/shared/config/chain.ts` -> local-only config risk
  - `apps/shared/providers/ChainProvider.tsx` -> replay/nonce risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/verifier/mod.rs` -> panic risk
  - `apps/x3-desktop/src-tauri/tauri.conf.json` -> local-only config risk, unsafe code
  - `apps/x3-desktop/src/components/panels/admin/AdminPanel.tsx` -> local-only config risk
  - `apps/x3-desktop/src/components/panels/explorer/BridgePanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/components/panels/explorer/CommunityPanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/components/panels/explorer/DevDocsPanel.tsx` -> bridge risk, local-only config risk, replay risk
  - `apps/x3-desktop/src/components/panels/explorer/X3SubPagesPanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/lib/substrate/client.ts` -> local-only config risk, replay/nonce risk
  - `apps/x3-desktop/src/lib/substrate/queries.ts` -> replay/nonce risk
  - `apps/x3-desktop/src/services/applicationService.ts` -> bridge risk, local-only config risk, replay risk
  - `apps/x3-desktop/src/services/x3ChainService.ts` -> atomic rollback risk, local-only config risk, panic risk, replay/nonce risk
  - `apps/x3-intelligence/src/pages/FloorRules.tsx` -> replay risk
  - `contracts/botchain-tri-vm-genesis/hardhat/test/BOT.test.js` -> panic risk
  - `contracts/botchain-tri-vm-genesis/hardhat/test/SimpleDEX.test.js` -> panic risk

## X3VM / Cross-VM
- Status: **NEEDS REVIEW**
- Files: 115
- Test files found: 11
- Risky files: 48

  - `apps/x3-desktop/tests/unit/operatorDashboard.test.ts` -> panic risk
  - `contracts/botchain-tri-vm-genesis/.github/workflows/ci.yml` -> local-only config risk
  - `contracts/botchain-tri-vm-genesis/hardhat/contracts/MarriageLicense.sol` -> replay risk
  - `contracts/botchain-tri-vm-genesis/hardhat/generated-contracts/MarriageLicense.sol` -> replay risk
  - `contracts/botchain-tri-vm-genesis/hardhat/package.json` -> local-only config risk
  - `contracts/botchain-tri-vm-genesis/hardhat/test/AtomicSwap.test.js` -> panic risk
  - `contracts/botchain-tri-vm-genesis/hardhat/test/ProjectHygiene.test.js` -> panic risk
  - `contracts/botchain-tri-vm-genesis/python/cli/simulate_lifecycle.py` -> local-only config risk
  - `contracts/botchain-tri-vm-genesis/python/utils/ipfs_client.py` -> local-only config risk
  - `contracts/botchain-tri-vm-genesis/tests/conftest.py` -> unsafe code
  - `contracts/botchain-tri-vm-genesis/tests/test_checker.py` -> unsafe code
  - `crates/cross-vm-coordinator/src/persistence.rs` -> panic risk, replay risk
  - `crates/gpu-swarm.backup/src/gpu_backends/cuda.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/sandbox_manager.rs` -> panic risk
  - `crates/gpu-swarm.backup/tests/blockchain_tests.rs` -> local-only config risk, panic risk
  - `crates/parallel-proposer/src/integration.rs` -> panic risk, replay/nonce risk
  - `crates/parallel-proposer/src/substrate.rs` -> replay/nonce risk
  - `crates/x3-backend/src/bc_format_helpers.rs` -> panic risk
  - `crates/x3-backend/src/lower.rs` -> atomic rollback risk, panic risk
  - `crates/x3-backend/src/opcode.rs` -> atomic rollback risk
  - `crates/x3-court/src/court.rs` -> panic risk, replay risk, replay/nonce risk
  - `crates/x3-court/src/lib.rs` -> replay risk
  - `crates/x3-court/src/replay.rs` -> replay risk
  - `crates/x3-evolution/src/fitness.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/deterministic.rs` -> replay risk

## Bridge / Router
- Status: **NEEDS REVIEW**
- Files: 265
- Test files found: 44
- Risky files: 182

  - `.scripts/x3_repomix_pack.sh` -> bridge risk
  - `crates/cross-vm-bridge/bridge/finality.rs` -> panic risk
  - `crates/cross-vm-bridge/tests/finality_verification_tests.rs` -> bridge risk, panic risk, replay/nonce risk
  - `apps/dashboard/src/panels/backend/SocialBackendPanel.tsx` -> bridge risk
  - `apps/dashboard/src/panels/docs/GovernanceProposalsPanel.tsx` -> bridge risk
  - `apps/dashboard/src/panels/infrastructure/CrossChainBridgePanel.tsx` -> bridge risk
  - `apps/dashboard/src/panels/security/BugBountyProgramPanel.tsx` -> bridge risk
  - `apps/dex/CLAUDE.md` -> unfinished logic
  - `apps/dex/PRPs/templates/prp_base.md` -> local-only config risk, panic risk
  - `apps/dex/README.md` -> bridge risk, local-only config risk, unfinished logic
  - `apps/inferstructor-dashboard/src-tauri/src/main.rs` -> panic risk
  - `apps/inferstructor-dashboard/src/api.test.ts` -> bridge risk, local-only config risk, panic risk
  - `apps/inferstructor-dashboard/src/api.ts` -> bridge risk, local-only config risk
  - `apps/inferstructor-dashboard/src/components/AdminDashboardTelemetryPanels.test.tsx` -> bridge risk, panic risk
  - `apps/inferstructor-dashboard/src/components/Dashboard.tsx` -> bridge risk, local-only config risk
  - `apps/inferstructor-dashboard/src/components/OrchestraOperationsPanel.test.ts` -> bridge risk, panic risk
  - `apps/inferstructor-dashboard/src/components/OrchestraOperationsPanel.ui.test.tsx` -> bridge risk, panic risk
  - `apps/inferstructor-dashboard/src/components/dashboard/DetailsGrid.tsx` -> bridge risk
  - `apps/inferstructor-dashboard/src/components/dashboard/StatsGrid.tsx` -> bridge risk
  - `apps/inferstructor-dashboard/src/components/orchestra-incidents.ts` -> bridge risk
  - `apps/inferstructor-dashboard/src/test/setup.ts` -> local-only config risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/signers/common/mod.rs` -> bridge risk
  - `apps/x3-desktop/src/components/panels/explorer/CommunitySubPanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/components/panels/governance/CrmGovernancePanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/components/panels/infrastructure/InfrastructurePanel.tsx` -> bridge risk, local-only config risk

## EVM Integration
- Status: **NEEDS REVIEW**
- Files: 178
- Test files found: 25
- Risky files: 90

  - `X3-contracts/shared/parity-core/tests/parity_vectors.rs` -> panic risk
  - `apps/x3-desktop/src-tauri/src/wallet.rs` -> panic risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/coordinator.rs` -> panic risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/signers/evm.rs` -> panic risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/signers/mod.rs` -> unsafe code
  - `apps/x3-desktop/src/components/panels/BlockchainConnectorPanel.tsx` -> replay/nonce risk
  - `apps/x3-desktop/src/components/panels/dex/DexPanel.tsx` -> atomic rollback risk
  - `apps/x3-desktop/src/components/panels/wallet/WalletPanel.test.tsx` -> panic risk
  - `apps/x3-desktop/src/components/panels/wallet/__tests__/HistoryView.test.tsx` -> panic risk
  - `apps/x3-desktop/src/stores/walletStore.test.ts` -> panic risk
  - `contracts/botchain-tri-vm-genesis/hardhat/contracts/AtomicSwapAdapter.sol` -> replay risk, replay/nonce risk
  - `contracts/botchain-tri-vm-genesis/hardhat/generated-contracts/AtomicSwapAdapter.sol` -> replay risk, replay/nonce risk
  - `contracts/botchain-tri-vm-genesis/hardhat/hardhat.config.js` -> local-only config risk
  - `contracts/botchain-tri-vm-genesis/hardhat/test/MarriageLicense.test.js` -> panic risk, replay risk
  - `contracts/botchain-tri-vm-genesis/python/utils/web3_client.py` -> local-only config risk, replay/nonce risk
  - `crates/confidential-gpu/src/enclave.rs` -> panic risk
  - `crates/cross-chain-gpu-validator/docs/security.md` -> atomic rollback risk, local-only config risk
  - `crates/cross-chain-gpu-validator/src/orchestrator.rs` -> atomic rollback risk, local-only config risk
  - `crates/cross-vm-coordinator/src/abi.rs` -> panic risk
  - `crates/cross-vm-coordinator/src/htlc.rs` -> replay/nonce risk
  - `crates/cross-vm-coordinator/src/rpc_client.rs` -> local-only config risk, panic risk
  - `crates/cross-vm-coordinator/src/state_machine.rs` -> panic risk, replay risk
  - `crates/evm-integration/src/frontier.rs` -> replay/nonce risk
  - `crates/evm-integration/src/mini_evm.rs` -> panic risk, replay/nonce risk
  - `crates/evm-integration/src/state.rs` -> panic risk, replay/nonce risk

## SVM Integration
- Status: **NEEDS REVIEW**
- Files: 41
- Test files found: 7
- Risky files: 17

  - `apps/x3-desktop/src-tauri/src/wallet_core/signers/svm.rs` -> panic risk
  - `crates/cross-chain-gpu-validator/src/svm_validator.rs` -> panic risk
  - `crates/svm-integration/src/interp.rs` -> panic risk, replay/nonce risk
  - `crates/svm-integration/src/lib.rs` -> panic risk
  - `crates/svm-integration/tests/counter_integration.rs` -> panic risk
  - `crates/x3-orchestrator/src/message.rs` -> replay risk, replay/nonce risk
  - `crates/x3-packet-schema/src/svm.rs` -> panic risk
  - `crates/x3-relayer/src/watchers/svm.rs` -> panic risk
  - `crates/x3-sdk/src/svm.rs` -> panic risk
  - `crates/x3-svm/src/anchor_idl_parser.rs` -> panic risk
  - `crates/x3-svm/src/solana_devnet_fork.rs` -> atomic rollback risk, local-only config risk, panic risk
  - `launch-gates/sources/pack-01-wiring/pallets/svm-runtime-lib.rs` -> panic risk, unsafe code
  - `launch-gates/sources/pack-04-invariant/integration-tests/svm-counter-test/tests/counter.rs` -> panic risk
  - `launch-gates/sources/pack-05-test-gap/pallets/svm-runtime-excerpt.rs` -> unsafe code
  - `packages/ts-sdk/tests/svm.test.ts` -> panic risk
  - `pallets/svm-runtime/fuzz/fuzz_targets/fuzz_codec_parsing.rs` -> panic risk, unfinished logic
  - `pallets/svm-runtime/src/lib.rs` -> panic risk, unsafe code

## X3 DEX
- Status: **NEEDS REVIEW**
- Files: 307
- Test files found: 21
- Risky files: 125

  - `apps/dex/app/lib/rpc-client.ts` -> local-only config risk
  - `apps/dex/app/page.tsx` -> local-only config risk
  - `apps/inferstructor-dashboard/TESTING.md` -> panic risk
  - `apps/x3-desktop/rag-bot/index.js` -> local-only config risk
  - `apps/x3-desktop/src-tauri/src/crm/agents.rs` -> bridge risk, local-only config risk, panic risk, replay risk
  - `apps/x3-desktop/src-tauri/src/wallet_core/signers/common/secure_memory.rs` -> unsafe code
  - `apps/x3-desktop/src/components/panels/IframePanel.tsx` -> local-only config risk
  - `apps/x3-desktop/src/components/panels/WorldMonitorPanel.tsx` -> local-only config risk
  - `apps/x3-desktop/src/components/panels/explorer/ExplorerDetailPanel.tsx` -> replay/nonce risk
  - `apps/x3-desktop/src/components/panels/wallet/RealTransactionSigningPanel.tsx` -> replay/nonce risk
  - `apps/x3-desktop/src/components/panels/wallet/WalletPanel.tsx` -> bridge risk
  - `apps/x3-desktop/src/services/agentService.ts` -> local-only config risk
  - `apps/x3-desktop/tests/unit/windowManager.test.ts` -> panic risk
  - `apps/x3-intelligence/server.js` -> local-only config risk
  - `crates/atomic-swap-orchestrator/src/atomic_lock.rs` -> replay/nonce risk
  - `crates/chronos-flash/src/mempool.rs` -> replay/nonce risk
  - `crates/chronos-flash/src/timewarp.rs` -> atomic rollback risk, bridge risk
  - `crates/chronos-flash/src/types.rs` -> atomic rollback risk
  - `crates/confidential-gpu/src/lib.rs` -> panic risk
  - `crates/confidential-gpu/src/threshold.rs` -> panic risk
  - `crates/contention-predictor/src/shard_planner.rs` -> panic risk
  - `crates/cross-chain-gpu-validator/docs/deployment.md` -> local-only config risk
  - `crates/cross-chain-gpu-validator/src/dashboard.rs` -> atomic rollback risk
  - `crates/gpu-swarm.backup/src/announcer.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/blockchain.rs` -> local-only config risk, panic risk

## DEX Liquidity
- Status: **NEEDS REVIEW**
- Files: 66
- Test files found: 4
- Risky files: 28

  - `apps/analytics/analytics-service/docker-compose.yml` -> local-only config risk
  - `apps/analytics/analytics-service/setup.sh` -> local-only config risk
  - `apps/analytics/analytics-service/src/main.rs` -> local-only config risk, panic risk
  - `apps/x3-desktop/src/components/panels/infrastructure/RpcStatsPanel.tsx` -> local-only config risk
  - `crates/gpu-swarm.backup/node-config.toml` -> local-only config risk
  - `crates/gpu-swarm.backup/src/crown/auditor.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/performance/memory_pooling.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/performance/mod.rs` -> panic risk
  - `crates/gpu-swarm.backup/tests/integration_tests.rs` -> local-only config risk, panic risk
  - `crates/import-queue-wrapper/src/lib.rs` -> panic risk, replay/nonce risk
  - `crates/private-mempool/src/encryption.rs` -> panic risk, replay/nonce risk
  - `crates/swarm-media/src/reputation.rs` -> panic risk
  - `crates/x3-backend/src/bc_format.rs` -> panic risk
  - `crates/x3-backend/src/emit.rs` -> atomic rollback risk, panic risk
  - `crates/x3-consensus/src/parallel_proposer.rs` -> replay/nonce risk
  - `crates/x3-economics/src/stake_compounding.rs` -> panic risk
  - `crates/x3-gateway/tests/loom_mempool_concurrency.rs` -> panic risk, replay/nonce risk
  - `crates/x3-gpu-validator-swarm/src/lib.rs` -> replay risk
  - `crates/x3-gpu-validator-swarm/tests/stress_harness.rs` -> panic risk
  - `crates/x3-marketplace/src/fee_distribution.rs` -> panic risk
  - `crates/x3-wallet/src/defi_tracker.rs` -> panic risk
  - `crates/x3-wallet/src/privacy_mixing.rs` -> panic risk
  - `deployment/kubernetes/gpu-swarm-production.yaml` -> local-only config risk
  - `infra-structure/services/cloudflare-tunnel/config.yml` -> local-only config risk
  - `launch-gates/sources/pack-01-wiring/pallets/private-execution-lib.rs` -> unsafe code

## Launchpad
- Status: **NEEDS REVIEW**
- Files: 117
- Test files found: 55
- Risky files: 71

  - `apps/x3-desktop/src/components/ipfsStorage/IpfsStoragePanel.test.tsx` -> panic risk
  - `apps/x3-desktop/src/components/systemMetrics/SystemMetricsPanel.test.tsx` -> panic risk
  - `apps/x3-desktop/tests/e2e/stress-tests.spec.ts` -> panic risk
  - `apps/x3-desktop/tests/e2e/tauri-backend.spec.ts` -> panic risk
  - `apps/x3-desktop/tests/e2e/world-monitor.spec.ts` -> local-only config risk, panic risk
  - `apps/x3-desktop/tests/unit/WorldMonitorPanel.test.tsx` -> panic risk
  - `apps/x3-desktop/tests/unit/applicationRegistry.test.ts` -> panic risk
  - `apps/x3-desktop/tests/unit/telemetryStream.test.tsx` -> panic risk
  - `crates/x3-launch-validator/src/checklist.rs` -> replay risk
  - `crates/x3-launch-validator/src/checks.rs` -> replay risk
  - `crates/x3-launch-validator/src/main.rs` -> panic risk
  - `crates/x3-relayer/RPC_FAILOVER_PROCEDURES.md` -> local-only config risk
  - `crates/x3-relayer/VALIDATOR_OPERATIONS.md` -> local-only config risk
  - `crates/x3-relayer/WAR_GAME_QUICK_START.md` -> bridge risk, local-only config risk
  - `launch-gates/embarrassment-scan.sh` -> known issue, local-only config risk, panic risk, stub risk, unfinished logic
  - `launch-gates/multi-node-testnet-proof.ROOT-CAUSE.md` -> unsafe code
  - `launch-gates/multi-node-testnet-proof.sh` -> local-only config risk, unsafe code
  - `launch-gates/run-proof-commands.sh` -> bridge risk, panic risk
  - `launch-gates/run-substrate-proof-pack.sh` -> replay risk
  - `launch-gates/sources/pack-01-wiring/pallets/agent-accounts-lib.rs` -> unsafe code
  - `launch-gates/sources/pack-01-wiring/pallets/agent-memory-lib.rs` -> unsafe code
  - `launch-gates/sources/pack-01-wiring/pallets/depin-marketplace-lib.rs` -> unsafe code
  - `launch-gates/sources/pack-01-wiring/pallets/fraud-proofs-lib.rs` -> unsafe code
  - `launch-gates/sources/pack-01-wiring/pallets/governance-lib.rs` -> atomic rollback risk, unsafe code
  - `launch-gates/sources/pack-01-wiring/pallets/meme-overlord-lib.rs` -> unsafe code

## Liquidity Locks
- Status: **MISSING**
- Files: 0
- Test files found: 0
- Risky files: 0

## Anti-Rug Mechanics
- Status: **MISSING**
- Files: 0
- Test files found: 0
- Risky files: 0

## Genesis / Chain Spec
- Status: **NEEDS REVIEW**
- Files: 35
- Test files found: 4
- Risky files: 21

  - `crates/poh-generator/src/lib.rs` -> panic risk
  - `crates/x3-consensus/src/ghost_fork_choice.rs` -> panic risk
  - `crates/x3-consensus/src/proof_of_history.rs` -> panic risk
  - `crates/x3-genesis-builder/src/lib.rs` -> panic risk
  - `deployment/build-and-keygen.sh` -> local-only config risk
  - `deployment/deploy-local-testnet.sh` -> local-only config risk
  - `deployment/deploy-nodes-day1.sh` -> local-only config risk
  - `deployment/public-rpc/run-public-rpc.sh` -> local-only config risk, unsafe code
  - `packages/py-sdk/tests/conftest.py` -> replay/nonce risk
  - `pallets/agent-accounts/src/mock.rs` -> panic risk
  - `pallets/agent-memory/src/mock.rs` -> panic risk
  - `pallets/swarm/src/mock.rs` -> panic risk
  - `pallets/treasury/src/mock.rs` -> panic risk
  - `pallets/x3-da/src/mock.rs` -> panic risk
  - `pallets/x3-inventory/src/mock.rs` -> panic risk
  - `pallets/x3-reservation/src/mock.rs` -> panic risk
  - `pallets/x3-sequencer/src/mock.rs` -> panic risk
  - `pallets/x3-slash/src/mock.rs` -> panic risk
  - `pallets/x3-solvency/src/mock.rs` -> panic risk
  - `pallets/x3-vrf/src/mock.rs` -> panic risk
  - `pallets/x3-wallet-pallet/src/mock.rs` -> panic risk

## Proof System
- Status: **NEEDS REVIEW**
- Files: 82
- Test files found: 4
- Risky files: 29

  - `X3-contracts/shared/gpu-parity-core/tests/parity_vectors.rs` -> panic risk
  - `apps/x3-desktop/tests/e2e/practical-integration.spec.ts` -> local-only config risk, panic risk
  - `apps/x3-intelligence/src/pages/SlashingPage.tsx` -> replay risk
  - `apps/x3-intelligence/src/services/api.ts` -> local-only config risk
  - `crates/custody-service/src/hsm.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/jobs/zk_proving.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/verification.rs` -> replay/nonce risk
  - `crates/x3-agent/src/identity.rs` -> replay/nonce risk
  - `crates/x3-agent/src/registry.rs` -> panic risk
  - `crates/x3-court/Cargo.toml` -> replay risk
  - `crates/x3-court/src/error.rs` -> replay risk
  - `crates/x3-gpu-validator-swarm/src/proof_aggregator.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/validator.rs` -> panic risk, replay risk
  - `crates/x3-intent/src/lifecycle.rs` -> panic risk
  - `crates/x3-parallel-executor/src/lib.rs` -> panic risk
  - `crates/x3-proof-dispute/src/lib.rs` -> panic risk
  - `crates/x3-proof/src/epoch.rs` -> panic risk
  - `crates/x3-proof/src/hasher.rs` -> replay risk
  - `crates/x3-sidecar/src/state.rs` -> atomic rollback risk
  - `crates/x3-slash/src/lib.rs` -> replay risk
  - `crates/x3-vrf/src/lib.rs` -> panic risk
  - `pallets/fraud-proofs/fuzz/fuzz_targets/fuzz_codec_parsing.rs` -> panic risk, unfinished logic
  - `pallets/fraud-proofs/src/lib.rs` -> unsafe code
  - `pallets/x3-inventory/src/inventory.rs` -> atomic rollback risk
  - `pallets/x3-vrf/fuzz/fuzz_targets/fuzz_codec_parsing.rs` -> panic risk

## TPS Benchmark Suite
- Status: **NEEDS REVIEW**
- Files: 209
- Test files found: 17
- Risky files: 48

  - `apps/inferstructor-dashboard/src-tauri/tauri.conf.json` -> local-only config risk, unsafe code
  - `apps/inferstructor-dashboard/src/components/AdminDashboard.tsx` -> bridge risk
  - `apps/inferstructor-dashboard/src/components/AdminDashboardTelemetryPanels.tsx` -> bridge risk
  - `apps/inferstructor-dashboard/src/components/DashboardCharts.test.tsx` -> panic risk
  - `apps/inferstructor-dashboard/src/components/OrchestraOperationsPanel.tsx` -> bridge risk
  - `apps/inferstructor-dashboard/src/components/TpsLeaderboard.test.tsx` -> panic risk
  - `apps/inferstructor-dashboard/src/utils/validation.test.ts` -> local-only config risk, panic risk
  - `apps/shared/config/__tests__/chain.test.ts` -> local-only config risk, panic risk
  - `apps/x3-desktop/src-tauri/src/social/activitypub.rs` -> panic risk
  - `crates/cross-chain-gpu-validator/deployment/deploy_testnet.sh` -> local-only config risk
  - `crates/gpu-swarm.backup/src/advanced/social_agents.rs` -> panic risk
  - `crates/tps-tracker/src/lib.rs` -> local-only config risk
  - `crates/tps-tracker/src/main.rs` -> local-only config risk
  - `crates/x3-gateway/src/graphql.rs` -> bridge risk, replay risk, replay/nonce risk
  - `crates/x3-gpu-validator-swarm/benches/e2e_tps.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/bin/x3_bench.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/bin/x3_swarm_orchestrator.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/bin/x3_validator.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/config.rs` -> replay risk
  - `crates/x3-gpu-validator-swarm/tests/chaos_stress_test.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/tests/stress_with_real_time_metrics.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/tests/tps_sliding_window_test.rs` -> panic risk
  - `crates/x3-opt/tests/loop_pack_integration_bench.rs` -> panic risk
  - `crates/x3-sidecar/src/gateway_client.rs` -> local-only config risk
  - `deployment/config/alertmanager.yml` -> local-only config risk

## GPU Validator Swarm
- Status: **NEEDS REVIEW**
- Files: 106
- Test files found: 16
- Risky files: 37

  - `apps/x3-desktop/tests/e2e/full-integration.spec.ts` -> local-only config risk, panic risk
  - `crates/confidential-gpu/src/attestation.rs` -> panic risk
  - `crates/contention-predictor/src/model.rs` -> panic risk, replay/nonce risk
  - `crates/gpu-sig-verifier/src/lib.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/admin.rs` -> bridge risk, panic risk, replay/nonce risk
  - `crates/gpu-swarm.backup/src/billing.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/bin/swarm-node.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/crown/scrapyard.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/jobs/funding_campaign.rs` -> bridge risk, panic risk
  - `crates/gpu-swarm.backup/src/jobs/model_training.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/monitoring/logging.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/monitoring/tracing.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/network.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/performance/batch_optimization.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/protocol.rs` -> replay/nonce risk
  - `crates/gpu-swarm.backup/src/warden/allocator.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/warden/metrics.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/warden/predictor.rs` -> panic risk
  - `crates/gpu-swarm.backup/src/warden/signals.rs` -> panic risk
  - `crates/gpu-swarm.backup/tests/admin_api.rs` -> panic risk
  - `crates/gpu-swarm.backup/tests/network_tests.rs` -> panic risk
  - `crates/gpu-swarm.backup/tests/wallet_derivation.rs` -> panic risk
  - `crates/quantum-swarm/src/quantum/backend.rs` -> panic risk
  - `crates/swarm-media/src/rpc_api.rs` -> panic risk
  - `crates/x3-gpu-validator-swarm/src/crypto.rs` -> panic risk

## Validator / LaunchOps
- Status: **NEEDS REVIEW**
- Files: 64
- Test files found: 13
- Risky files: 23

  - `apps/inferstructor-dashboard/src/hooks/useTokenRefresh.test.ts` -> panic risk
  - `apps/shared/hooks/useChainSubscription.ts` -> replay/nonce risk
  - `crates/dylint-determinism/src/lib.rs` -> panic risk
  - `crates/x3-consensus/src/network_partition_recovery.rs` -> panic risk
  - `crates/x3-pq/src/lib.rs` -> panic risk
  - `crates/x3-runtime-params/src/consensus_params.rs` -> atomic rollback risk
  - `crates/x3-staking-analytics/src/slash_tracker.rs` -> panic risk
  - `crates/x3-staking-analytics/src/staking_ledger.rs` -> panic risk
  - `crates/x3-staking-analytics/src/staking_simulator.rs` -> panic risk
  - `crates/x3-staking-analytics/src/validator_stats.rs` -> panic risk
  - `crates/x3-turbine/src/peer.rs` -> panic risk
  - `crates/x3-validator-attestation/src/lib.rs` -> panic risk
  - `deployment/deploy-to-testnet.sh` -> local-only config risk
  - `deployment/docker/config/prometheus.yml` -> local-only config risk
  - `deployment/docker/docker-compose.production.yml` -> bridge risk, local-only config risk
  - `deployment/kubernetes/production-deployment.yaml` -> local-only config risk
  - `deployment/manage-testnet.sh` -> local-only config risk
  - `infra-structure/validator/scripts/healthcheck.sh` -> local-only config risk
  - `packages/blockchain-connector/src/adapters/cosmos.ts` -> replay/nonce risk
  - `packages/blockchain-connector/src/adapters/near.ts` -> replay/nonce risk
  - `pallets/cross-chain-validator/fuzz/fuzz_targets/fuzz_codec_parsing.rs` -> panic risk, unfinished logic
  - `pallets/private-execution/src/tests.rs` -> panic risk
  - `pallets/x3-settlement-engine/src/atomic_lock.rs` -> replay/nonce risk
