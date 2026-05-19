# X3 Chain - Critical Path PRD
## Minimum Viable Production (MVP) Requirements

**Version:** 1.0  
**Date:** February 13, 2026  
**Duration:** 4 Weeks  
**Goal:** Get core blockchain operational with dual-VM support

---

## Overview

This PRD focuses on the **critical path** to get X3 Chain's core functionality working end-to-end. These tasks must be completed before the comprehensive PRD can be fully executed.

**Success Criteria:** A working blockchain with WebSocket RPC, dual-VM execution (real, not mocks), and basic frontend for testing.

**Supplemental Planning Artifact:** Post-MVP hardening, swarm governance, operator control-plane, and outward-action gaps are tracked in [X3_END_TO_END_GAPS_MASTER_PLAN.md](../../X3_END_TO_END_GAPS_MASTER_PLAN.md).

**Supplemental Execution Status (2026-03-28):**
- [x] Phase 4.5 Ticket 1/2 reservation-routing foundation is now live in `cross-chain-position-manager`; `cargo test -p cross-chain-position-manager --lib` passes with `5/5` tests.

**Supplemental Execution Status (2026-03-29):**
- [x] Phase 4.5 Ticket 3 inventory manager groundwork is live in `cross-chain-position-manager`; `cargo test -p cross-chain-position-manager --lib` now passes with `11/11` tests including inventory and obligation transitions.

---

## Week 1: Core Infrastructure

### Critical Task 1.1: Fix Build Issues

- [x] Run `cargo clippy --workspace --all-targets --all-features` and fix all errors
- [x] Ensure `cargo build --release --workspace` completes successfully
- [x] Run `cargo test --workspace` and ensure all tests pass
- [x] Commit: "fix: resolve all compiler warnings and build errors"

### Critical Task 1.2: Enable WebSocket RPC

- [x] Implement WebSocket server in `node/src/rpc.rs`
- [x] Expose standard Substrate RPC methods (system_*, chain_*, state_*)
- [x] Test connection with Polkadot.js apps UI
- [x] Update README with WebSocket connection examples
- [x] Commit: "feat: add WebSocket RPC support for Polkadot.js integration"

### Critical Task 1.3: Complete X3 VM Core

- [x] Implement base calculation for nested calls (vm.rs:449)
- [x] Implement global variable storage (vm.rs:492, 500)
- [x] Implement rollback mechanism (vm.rs:894)
- [x] Add unit tests for all three features
- [x] Run `cargo test -p x3-vm` to verify
- [x] Commit: "feat: complete X3 VM core functionality with storage and rollback"

---

## Week 2: Dual-VM Integration (Real Implementation)

### Critical Task 2.1: Real EVM Integration

- [x] Remove mock EVM executor from runtime
- [x] Wire Frontier pallet into runtime properly
- [x] Enable Frontier RPC module (node/src/rpc.rs:1308)
- [x] Deploy and test a simple Solidity contract (ERC20 or Hello World)
- [x] Verify EVM state syncs with canonical ledger
- [x] Add integration test for EVM contract execution
- [x] Commit: "feat: integrate real Frontier EVM with contract deployment"

### Critical Task 2.2: Real SVM Integration

- [x] Remove mock SVM executor from runtime
- [x] Wire SVM pallet into runtime properly
- [x] Enable SVM program deployment
- [x] Deploy and test a simple Solana program (token or counter)
- [x] Verify SVM state syncs with canonical ledger
- [x] Add integration test for SVM program execution
- [x] Commit: "feat: integrate real SVM with program deployment"

### Critical Task 2.3: Cross-VM Communication

- [x] Implement EVM-to-SVM asset transfer
- [x] Implement SVM-to-EVM asset transfer
- [x] Test atomic cross-VM transaction
- [x] Add integration test for cross-VM bridge
- [x] Document cross-VM bridge usage
- [x] Commit: "feat: enable atomic cross-VM transactions"

---

### Critical Task 2.4: Production Bridge Adapters

- [x] Replace `InMemoryBalanceAdapter`/`InMemoryEscrowAdapter` simulation with real Substrate-backed adapters
- [x] Implement `SubstrateClientBalanceAdapter<C,Block>` using `AtlasKernelRuntimeApi` overlay pattern
- [x] Implement `PalletEscrowAdapter<C,Block,P>` with SHA-256 ticket generation and overflow-safe arithmetic
- [x] Implement `OffchainEscrowPersistence<O>` backed by `sp_core::offchain::OffchainStorage`
- [x] Re-export `pallet_x3_kernel::StateChange` for bundle receipt use
- [x] Write 21 unit tests (overlay, deltas, round-trips, double-spend, atomicity)
- [x] Run `cargo test -p x3-bridge-adapters` — 21/21 pass
- [x] Fix `.cargo/config.toml` linker to `clang-14` (LLVM 20.1 SIGSEGV workaround)
- [x] Commit: "feat(bridge-adapters): replace simulation with Substrate-backed adapters"

---

## Week 3: SDK & Frontend Essentials

### Critical Task 3.1: Complete TypeScript SDK

- [x] Implement SS58 decoding (utils.ts:206)
- [x] Add Base58 validation (utils.ts:271)
- [x] Implement all collateral RPC calls (collateral.ts:21,26,31,36)
- [x] Complete SHA256 in svm.ts (line 134)
- [x] Add SDK integration test with live node
- [x] Commit: "feat: complete TypeScript SDK core functionality"

### Critical Task 3.2: Minimal Wallet UI

- [x] Create basic wallet interface in `apps/wallet`
- [x] Implement account creation and key management
- [x] Add transaction signing UI
- [x] Connect to X3 node via WebSocket
- [x] Test sending transactions on testnet
- [x] Commit: "feat: add minimal functional wallet UI"

### Critical Task 3.3: Block Explorer MVP

- [x] Create basic explorer in `apps/explorer`
- [x] Display latest blocks
- [x] Show transaction details
- [x] Add account balance lookup
- [x] Deploy explorer to testnet
- [x] Commit: "feat: deploy basic block explorer"

---

## Week 4: Testing & Documentation

### Critical Task 4.1: Integration Testing

- [ ] Add E2E test for EVM contract deployment and calling
- [ ] Add E2E test for SVM program deployment and execution
- [ ] Add E2E test for cross-VM transaction
- [ ] Add E2E test for WebSocket RPC
- [ ] Ensure all E2E tests pass in CI
- [ ] Commit: "test: add comprehensive E2E test suite"

### Critical Task 4.2: Core Documentation

- [x] Update main README with current status
- [x] Document WebSocket RPC endpoints with examples
- [x] Document EVM contract deployment process
- [x] Document SVM program deployment process
- [x] Document cross-VM bridge usage
- [x] Create "Quick Start" guide
- [x] Commit: "docs: update all core documentation"

### Critical Task 4.3: Testnet Deployment

- [ ] Deploy updated node to testnet
- [x] Verify 3+ validators running
- [x] Document owned-hardware node allocation and lab-to-public topology in `docs/deployment/HARDWARE_ROLE_PLAN.md`
- [ ] Test all RPC endpoints on testnet
- [x] Deploy wallet and explorer frontends
- [ ] Announce testnet update to community
- [ ] Commit: "deploy: update testnet with dual-VM support"

---

## Acceptance Criteria (MVP Complete)

### Must Have
- ✅ WebSocket RPC working with Polkadot.js
- ✅ Real EVM contracts deployable and executable
- ✅ Real SVM programs deployable and executable
- ✅ Cross-VM transactions working atomically
- ✅ Basic wallet UI functional
- ✅ Block explorer operational
- ✅ E2E tests passing
- ✅ Documentation updated

### Should Have
- ✅ Zero critical bugs
- ✅ Testnet stable
- ✅ TypeScript SDK complete
- ✅ Basic monitoring in place

### Nice to Have
- ⚪ Python SDK complete
- ⚪ All frontend apps polished
- ⚪ Advanced features implemented

---

## After Critical Path

Once these critical tasks are complete:
1. Move to comprehensive PRD (docs/planning-artifacts/docs/planning-artifacts/PRD_COMPLETE_PROJECT.md)
2. Continue with Phase 5+ of full project completion
3. Focus on polish, security, and production readiness

---

## Dependencies & Blockers

### Pre-requisites
- Node.js 20+ installed
- Rust toolchain configured
- Docker available for testing
- Access to testnet infrastructure

### Known Blockers
- None currently - all critical path items are unblocked

### External Dependencies
- Frontier crate for EVM
- Solana dependencies for SVM
- Polkadot.js for testing

---

## Quick Reference

### Build Commands
```bash
# Full workspace build
cargo build --release --workspace

# Run all tests
cargo test --workspace

# Check for warnings
cargo clippy --workspace --all-targets --all-features

# Format code
cargo fmt --all
```

### Test Commands
```bash
# Unit tests
cargo test -p <crate-name>

# Integration tests
cargo test --test <test-name>

# E2E tests
./tests/e2e/start_test_environment.sh up
cargo test --test state_root_replay
```

### Node Operations
```bash
# Start development node
./run-dev-node.sh

# Start production node
./run-production-node.sh

# Check node logs
journalctl -u x3-chain-node -f
```

---

## Progress Tracking

Track progress by marking tasks complete in this PRD. Ralph will automatically:
- Mark completed tasks with [x]
- Move to next task sequentially
- Commit changes with descriptive messages
- Reference this PRD in commits

**Current Status:** Ready to begin Week 1

---

## Notes for Ralph

- **Priority:** Work strictly in order - Week 1 before Week 2, etc.
- **Testing:** Run tests after every task
- **Committing:** Make atomic commits with clear messages
- **Documentation:** Update docs as you go
- **Blocking Issues:** If stuck, mark task and move to next
- **Review:** Test each feature manually before marking complete

---

**Total Critical Path Tasks:** 30  
**Estimated Duration:** 4 weeks  
**Success Metric:** Functional blockchain with real dual-VM execution  
**Status:** Ready for Execution

---

## Cross-VM / Cross-Chain Hardening Backlog (2026-03-22)

- [x] Audit and locate runtime placeholders/stubs in atomic cross-VM + cross-chain path
- [ ] P0-1: Replace proof-validation stubs with real EVM/SVM/BTC proof verification
- [ ] P0-2: Replace fake keccak/hash placeholders in bridge light-client path
- [ ] P0-3: Replace mirror execution placeholders (EVM/SVM/BTC bridge actions)
- [ ] P1-1: Remove external-chain payload amount placeholders and encode real values
- [ ] P1-2: Replace adapter/Arbitrum placeholder defaults with production behavior
- [ ] P1-3: Remove SVM execution stub-success paths and propagate real execution results
- [ ] P2-1: Replace rollback fixed refund placeholder with executed-leg accounting
- [ ] P2-2: Implement relayer registry/path discovery/event processing in cross-chain pallet

Detailed tracking and resume checkpoint:
- `docs/reports/CROSS_VM_CROSS_CHAIN_100_TRACKER.md`

---

## Use Ralph With This PRD

To use this PRD with Ralph:
1. Open Ralph Control Panel in VS Code (click Ralph icon)
2. This file will be detected as `PRD_CRITICAL_PATH.md`
3. Click "Start" to begin autonomous execution
4. Ralph will work through tasks Week 1 → Week 4

For full project completion, switch to `docs/planning-artifacts/docs/planning-artifacts/PRD_COMPLETE_PROJECT.md` after finishing this critical path.
