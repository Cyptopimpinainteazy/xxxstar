//! # E2E Settlement / Atomic Kernel Test Plan (Placeholder)
//!
//! ⚠️  THIS FILE IS A DESIGN DOCUMENT — NOT AN EXECUTABLE TEST.
//!
//! The scenarios below describe the intended end-to-end settlement and atomic
//! kernel integration paths.  They are written as documentation to guide future
//! implementation.  They are NOT executable assertions.
//!
//! ## Test Scenarios (Design)
//!
//! ### Scenario 1: Happy-path atomic bundle → settlement
//!
//! 1. Fund two accounts (payer, receiver) with X3 tokens.
//! 2. Submit an atomic bundle with 1 EVM leg + 1 SVM leg.
//! 3. Assign executor → execute → finalize via `finalize_with_settlement`.
//! 4. Verify `BundleFinalized` event and PoAE proof stored.
//! 5. Verify settlement intent created in settlement engine.
//!
//! ### Scenario 2: Rollback atomic bundle with bond slash
//!
//! 1. Submit bundle, let deadline expire via `run_to_block`.
//! 2. Verify `BundleRolledBack(DeadlineExceeded)` event.
//! 3. Verify bond was slashed by 5%.
//!
//! ### Scenario 3: Unauthorized `finalize_with_settlement` rejected
//!
//! 1. Submit bundle as gateway account.
//! 2. Call `finalize_with_settlement` from a non-settlement account.
//! 3. Verify error `BadOrigin` (or equivalent).
//! 4. Call `finalize_with_settlement` from the settlement gateway account.
//! 5. Verify success.
//!
//! → **Executable regression tests for this scenario live at**
//! `pallets/x3-atomic-kernel/tests/e2e_settlement.rs`
//!
//! ### Scenario 4: Rollback with VM state reversion
//!
//! 1. Submit bundle, execute legs (populate state diffs via
//!    `record_leg_execution_receipt`).
//! 2. Rollback via `rollback_atomic_bundle(ExecutionFailed)`.
//! 3. Verify `IncompleteVmRevert` event (or no event if reverts succeed).
//!
//! ### Scenario 5: Cross-VM packet validation rejects bad packets
//!
//! 1. Build a packet with wrong domain mask for EVM (e.g. mask=0b0010).
//! 2. Submit via `submit_comit` with this packet as evm_payload.
//! 3. Verify `InvalidEvmPacket` error.
//!
//! See also:
//! - `integration-tests/cross-vm-atomic-test.rs` — node-level tests
//! - `integration-tests/cross-vm-pallet-test.rs` — pallet-level integration tests
