# Implementation Plan: X3 Chain Production Go-Live

[Overview]
Deploy X3 Chain to production testnet/mainnet by completing the critical-path gaps identified in X3_COMPLETION.md and X3_GAPS_REPORT.md. The system is a Substrate-based blockchain with dual-VM architecture (EVM + SVM), 79+ workspace crates, 22+ pallets, and extensive frontend applications. Current status is approximately 70% ready for testnet and 40% for mainnet, with 25 critical/high-priority gaps remaining across RPC, dual-VM integration, security, testing, deployment, and observability. This plan addresses all blockers in a 12-week sprint focused on integration-first development, automated enforcement via existing audit infrastructure, and progressive deployment from local devnet through testnet to mainnet.

[Types]
No new type system changes are required. Existing types across runtime, pallets, and crates are sufficient. Key types involved in this implementation:
- `X3RuntimeApi` in `runtime/src/lib.rs` — needs `submit_evm_transaction` verification
- `AgentRecord` in `crates/x3-agent/src/types.rs` — existing, needs integration testing
- `CrossVmBridge` types in `crates/cross-vm-bridge/src/lib.rs` — needs state sync implementation
- `FlashFinalityConfig` in `node/src/flash_finality.rs` — needs multi-node validation
- `GenesisConfig` in `node/src/chain_spec.rs` — needs testnet/mainnet variants

[Files]
Detailed breakdown of all files to create, modify, and configure:

**New files to create:**
- `testnet/genesis.json` — Testnet genesis configuration with 3+ validators
- `testnet/chain-spec.json` — Testnet chain specification
- `deployment/docker/Dockerfile.node` — Production-optimized node Dockerfile with health checks
- `deployment/docker/Dockerfile.rpc` — RPC service Dockerfile
- `deployment/docker/docker-compose.testnet.yml` — Full testnet stack (3 validators + RPC + monitoring)
- `deployment/kubernetes/x3-node-statefulset.yaml` — K8s StatefulSet for validators
- `deployment/kubernetes/x3-rpc-deployment.yaml` — K8s Deployment for RPC nodes
- `deployment/kubernetes/x3-monitoring-stack.yaml` — Prometheus + Grafana K8s manifests
- `scripts/deploy-testnet.sh` — Automated testnet deployment script
- `scripts/health-check.sh` — Node health validation script
- `tests/integration/cross_vm_test.rs` — Cross-VM integration test suite
- `tests/integration/multi_node_consensus_test.rs` — Multi-node consensus tests
- `tests/integration/rpc_websocket_test.rs` — WebSocket RPC integration tests
- `tests/load/tps_benchmark.rs` — TPS benchmarking test suite
- `tests/e2e/full_lifecycle_test.rs` — End-to-end lifecycle tests
- `.github/workflows/deploy-testnet.yml` — CI/CD pipeline for testnet deployment

**Existing files to modify:**
- `node/src/rpc.rs` — Complete WebSocket server implementation, add health endpoint
- `node/src/rpc_frontier.rs` — Wire Frontier JSON-RPC endpoints fully
- `node/src/service.rs` — Add graceful shutdown, telemetry hooks, multi-node boot validation
- `node/src/chain_spec.rs` — Add testnet/mainnet chain spec builders
- `crates/cross-vm-bridge/src/lib.rs` — Implement atomic cross-VM asset transfers
- `crates/evm-integration/src/lib.rs` — Complete EVM-to-ledger state synchronization
- `crates/svm-integration/src/lib.rs` — Complete SVM-to-ledger state synchronization
- `crates/x3-gpu-validator-swarm/src/lib.rs` — Complete GPU job scheduler, proof verification
- `runtime/src/lib.rs` — Verify WASM compilation, add missing weight annotations
- `pallets/x3-atomic-kernel/src/lib.rs` — Verify DeadlineIndex, slashing logic
- `X3_COMPLETION.md` — Mark completed items as ✅ throughout implementation
- `docker-compose.yml` — Update with production configuration
- `docker-compose.production.yml` — Harden with resource limits, secrets management
- `k8s-deployment.yaml` — Complete with StatefulSets, PVCs, health checks
- `prometheus.yml` — Add blockchain-specific metrics, alerting rules
- `grafana-dashboards.yml` — Create blockchain health dashboard
- `.github/workflows/x3-audit.yml` — Ensure Phase 3+4 gate alignment
- `deny.toml` — Verify dependency audit compliance
- `Makefile` — Add testnet deploy, health check, and coverage targets

**Files to delete or move:**
- `_unused/` directory — Remove or archive (orphaned experimental folders)
- Remove duplicate `docker-compose.staging.yml` if redundant with production

**Configuration file updates:**
- `rust-toolchain.toml` — Verify pinned nightly version
- `.cargo/audit.toml` — Ensure audit configuration matches CI
- `scripts/x3_audit.sh` — Verify Phase 3+4 gate commands
- `scripts/x3_coverage_gate.sh` — Verify coverage thresholds match Cargo.toml

[Functions]
Detailed breakdown of function modifications:

**New functions:**
- `fn deploy_testnet_validators()` in `scripts/deploy-testnet.sh` — Orchestrates 3-validator testnet deployment
- `fn run_health_checks()` in `scripts/health-check.sh` — Validates node RPC, consensus, and peer connectivity
- `fn test_cross_vm_transfer()` in `tests/integration/cross_vm_test.rs` — Tests atomic EVM→SVM→Ledger transfer
- `fn test_multi_node_consensus()` in `tests/integration/multi_node_consensus_test.rs` — Tests 3-node consensus
- `fn test_websocket_rpc()` in `tests/integration/rpc_websocket_test.rs` — Tests WebSocket subscription
- `fn benchmark_tps()` in `tests/load/tps_benchmark.rs` — Measures transactions per second under load
- `fn test_full_lifecycle()` in `tests/e2e/full_lifecycle_test.rs` — Tests deploy→transact→finality→query lifecycle

**Modified functions:**
- `fn create_full_rpc()` in `node/src/rpc.rs` — Add WebSocket server, health endpoint, subscription support
- `fn new_full()` in `node/src/service.rs` — Add graceful shutdown signal handler, telemetry opt-in
- `fn local_testnet_config()` in `node/src/chain_spec.rs` — Add `testnet_config()` and `mainnet_config()` builders
- `fn execute_cross_vm_transfer()` in `crates/cross-vm-bridge/src/lib.rs` — Implement atomic settlement with rollback
- `fn sync_to_ledger()` in `crates/evm-integration/src/lib.rs` — Implement EVM state→canonical ledger sync
- `fn sync_to_ledger()` in `crates/svm-integration/src/lib.rs` — Implement SVM state→canonical ledger sync
- `fn schedule_gpu_job()` in `crates/x3-gpu-validator-swarm/src/lib.rs` — Implement job queue with priority scheduling
- `fn verify_gpu_proof()` in `crates/x3-gpu-validator-swarm/src/lib.rs` — Implement proof verification logic
- `fn on_initialize()` in `pallets/x3-atomic-kernel/src/lib.rs` — Verify deadline expiry with DeadlineIndex

**Removed functions:**
- None — all existing functions remain, some get enhanced implementations

[Classes]
No new classes are required. Existing Rust struct/trait modifications:

**Modified structs/traits:**
- `RpcConfig` in `node/src/rpc.rs` — Add WebSocket port, TLS config, rate limiting params
- `NodeConfig` in `node/src/service.rs` — Add shutdown timeout, telemetry endpoint
- `TestnetGenesisConfig` in `node/src/chain_spec.rs` — New builder for testnet genesis
- `CrossVmBridge` in `crates/cross-vm-bridge/src/lib.rs` — Add `atomic_transfer()`, `rollback()` methods
- `EvmAdapter` in `crates/evm-integration/src/lib.rs` — Add `sync_state()` trait method
- `SvmAdapter` in `crates/svm-integration/src/lib.rs` — Add `sync_state()` trait method
- `GpuJobScheduler` in `crates/x3-gpu-validator-swarm/src/lib.rs` — Implement job queue, priority, status tracking
- `GpuProofVerifier` in `crates/x3-gpu-validator-swarm/src/lib.rs` — Implement proof verification trait

[Dependencies]
No new external dependencies required. All needed crates are already in workspace:
- `jsonrpsee` (v0.22.5) — Already in workspace deps for WebSocket RPC
- `tokio` (v1.0) — Already in workspace for async runtime
- `serde_json` (v1.0.111) — Already in workspace for JSON serialization
- `prometheus` — Via `substrate-prometheus-endpoint` (patched)
- `sc-service`, `sc-rpc`, `sc-network` — All Substrate client deps already pinned

**Dependency verification tasks:**
- Verify `cargo deny check` passes with current `deny.toml`
- Ensure all 30+ patches in `[patch.crates-io]` remain compatible
- Confirm Substrate rev `948fbd2` and Frontier `polkadot-v1.1.0` are stable

[Testing]
Comprehensive testing approach covering all layers:

**Unit tests (existing, verify passing):**
- `cargo test --workspace --release --locked` — All 200+ existing tests must pass
- `pallets/x3-kernel/src/tests.rs` — 98/98 tests verified passing
- `packages/ts-sdk/` — 185 tests verified passing

**New integration tests:**
- `tests/integration/cross_vm_test.rs` — EVM→SVM→Ledger atomic transfer (5 tests)
- `tests/integration/multi_node_consensus_test.rs` — 3-node Aura+GRANDPA consensus (4 tests)
- `tests/integration/rpc_websocket_test.rs` — WebSocket subscriptions, health checks (6 tests)
- `tests/integration/flash_finality_test.rs` — Flash Finality under network partitions (3 tests)

**New load tests:**
- `tests/load/tps_benchmark.rs` — Sustained TPS measurement (target: 500+ TPS initial)
- `tests/load/concurrent_users.rs` — 100+ concurrent RPC connections
- `tests/load/memory_stress.rs` — Long-running state accumulation test

**New E2E tests:**
- `tests/e2e/full_lifecycle_test.rs` — Deploy→Transact→Finality→Query (2 tests)
- `tests/e2e/disaster_recovery_test.rs` — Node crash→Recovery→State integrity (1 test)
- `tests/e2e/validator_rotation_test.rs` — Validator set change mid-consensus (1 test)

**Coverage validation:**
- `bash scripts/x3_coverage_gate.sh` — Verify all subsystems meet thresholds:
  - runtime: 95%, pallets: 90%, x3-constitution: 90%, x3-proof: 90%
  - x3-agent: 80%, x3-sdk: 80%, daemon: 85%, vm: 90%

**Security testing:**
- `cargo audit` — Verify no new advisories
- `rg 'unwrap\(\)|expect\(' --glob '!**/tests/**'` — Verify production code is panic-free
- Unsafe block review — Add SAFETY comments to all 7+ unsafe blocks in atomic-swap-orchestrator

[Implementation Order]
Numbered steps showing the logical order of changes to minimize conflicts and ensure successful integration:

**Phase 1: Build Verification & Cleanup (Week 1)**
1. Run `cargo check --workspace` — Fix any compilation errors
2. Run `cargo fmt --all -- --check` — Fix formatting
3. Run `cargo test --workspace --locked` — Verify all existing tests pass
4. Run `bash scripts/x3_audit.sh --ci` — Verify audit infrastructure
5. Remove `_unused/` directory if empty or archive contents
6. Update `X3_COMPLETION.md` with Phase 3 gate items marked ✅

**Phase 2: RPC Completion (Weeks 2-3)**
7. Implement WebSocket server in `node/src/rpc.rs` using jsonrpsee
8. Add health check endpoint (`/health`) returning node status
9. Wire Frontier JSON-RPC endpoints in `node/src/rpc_frontier.rs`
10. Test WebSocket connections with Polkadot.js
11. Add rate limiting middleware to RPC endpoints
12. Write `tests/integration/rpc_websocket_test.rs`
13. Update `X3_COMPLETION.md` Section 1.3 items to ✅

**Phase 3: Dual-VM State Synchronization (Weeks 3-5)**
14. Implement `sync_to_ledger()` in `crates/evm-integration/src/lib.rs`
15. Implement `sync_to_ledger()` in `crates/svm-integration/src/lib.rs`
16. Implement `atomic_transfer()` in `crates/cross-vm-bridge/src/lib.rs`
17. Add rollback mechanism for failed cross-VM transfers
18. Write `tests/integration/cross_vm_test.rs` (5 test cases)
19. Verify EVM contract deployment works end-to-end
20. Verify SVM program execution works end-to-end
21. Update `X3_COMPLETION.md` Section 4 items to ✅

**Phase 4: Consensus Hardening (Weeks 5-6)**
22. Create `testnet/chain-spec.json` with 3 validator identities
23. Implement multi-node boot validation in `node/src/service.rs`
24. Add graceful shutdown with configurable timeout
25. Write `tests/integration/multi_node_consensus_test.rs`
26. Write `tests/integration/flash_finality_test.rs`
27. Verify Aura block production + GRANDPA finality on 3-node cluster
28. Update `X3_COMPLETION.md` Section 2 items to ✅

**Phase 5: GPU Validator Completion (Weeks 6-7)**
29. Implement job queue in `crates/x3-gpu-validator-swarm/src/lib.rs`
30. Implement proof verification logic
31. Add reward distribution mechanism
32. Write unit tests for scheduler and verifier
33. Update `X3_COMPLETION.md` Section 6 items to ✅

**Phase 6: Security Hardening (Weeks 7-8)**
34. Add SAFETY comments to all unsafe blocks in `crates/atomic-swap-orchestrator/src/lib.rs`
35. Replace remaining `panic!()` in `crates/x3-flashloan/src/executor.rs` with proper error handling
36. Replace remaining `.unwrap()` in `crates/x3-economics/src/` with `.ok_or()` or `.map_err()`
37. Add CORS restrictions to RPC configuration
38. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
39. Run `rg 'unwrap\(\)|expect\(' --glob '!**/tests/**' .` and verify zero hits in production code
40. Update `X3_COMPLETION.md` Section 10 items to ✅

**Phase 7: Deployment Infrastructure (Weeks 8-9)**
41. Create `deployment/docker/Dockerfile.node` with multi-stage build, health checks
42. Create `deployment/docker/docker-compose.testnet.yml` (3 validators + RPC + monitoring)
43. Create `deployment/kubernetes/x3-node-statefulset.yaml` with PVCs
44. Create `deployment/kubernetes/x3-rpc-deployment.yaml` with HPA
45. Create `scripts/deploy-testnet.sh` — automated 3-validator deployment
46. Create `scripts/health-check.sh` — node health validation
47. Harden `docker-compose.production.yml` with resource limits, secrets
48. Update `X3_COMPLETION.md` Section 3.1, 3.2, 11.2 items to ✅

**Phase 8: Monitoring & Observability (Weeks 9-10)**
49. Update `prometheus.yml` with blockchain-specific metrics (block time, TPS, peer count)
50. Create `grafana-llm-dashboard.json` → rename to `grafana-blockchain-dashboard.json`
51. Add alerting rules (block time > 12s, peer count < 3, disk > 80%)
52. Verify Prometheus scrapes node metrics correctly
53. Create Grafana dashboard panels for: block production, finality time, TPS, peer count
54. Update `X3_COMPLETION.md` Section 9 items to ✅

**Phase 9: Load Testing & TPS Validation (Weeks 10-11)**
55. Create `tests/load/tps_benchmark.rs` — sustained transaction throughput
56. Create `tests/load/concurrent_users.rs` — RPC connection stress test
57. Create `tests/load/memory_stress.rs` — long-running state accumulation
58. Run benchmarks on 3-node testnet cluster
59. Document baseline TPS numbers in `docs/bench-results/`
60. Verify TPS meets minimum threshold (500+ TPS for testnet)
61. Update `X3_COMPLETION.md` Section 2.3, 5.1 items to ✅

**Phase 10: E2E Testing & Disaster Recovery (Weeks 11-12)**
62. Create `tests/e2e/full_lifecycle_test.rs` — deploy→transact→finality→query
63. Create `tests/e2e/disaster_recovery_test.rs` — crash→restart→state integrity
64. Create `tests/e2e/validator_rotation_test.rs` — validator set change
65. Run full test suite: `cargo test --workspace --release --locked`
66. Run `bash scripts/x3_coverage_gate.sh` — verify all thresholds met
67. Run `bash scripts/x3_audit.sh --ci` — verify all Phase 3+4 gates pass
68. Create `.github/workflows/deploy-testnet.yml` — CI/CD pipeline
69. Update `X3_COMPLETION.md` — mark all remaining in-scope items ✅

**Phase 11: Testnet Launch (Week 12)**
70. Execute `scripts/deploy-testnet.sh` on target infrastructure
71. Run `scripts/health-check.sh` — validate all nodes healthy
72. Run full TPS benchmark on live testnet
73. Monitor for 48 hours with Grafana dashboards
74. Document testnet status in `docs/deployment/testnet-status.md`
75. Final `X3_COMPLETION.md` audit — all Phase 3 items ✅

**Phase 12: Mainnet Preparation (Post-Week 12)**
76. Create `testnet/mainnet-genesis.json` with production tokenomics
77. Create mainnet chain specification with 5+ validator identities
78. Implement rollback procedures documented in `docs/disaster-recovery.md`
79. Conduct external security audit of critical paths
80. Run 7-day sustained load test on mainnet-equivalent infrastructure
81. Execute mainnet launch with governance approval