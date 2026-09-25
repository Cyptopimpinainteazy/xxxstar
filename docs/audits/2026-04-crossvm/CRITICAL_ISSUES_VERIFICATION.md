# CRITICAL ISSUES DEEP VERIFICATION REPORT
**Date:** April 12, 2026  
**Status:** Task B Complete - Findings documented

## CRITICAL-004: Bridge 2PC Prepare Phase Parameter Locking

### Claim
"Operation parameters are hashed during prepare phase to prevent modification between prepare and commit"

### Code Investigation
- **Location:** `crates/cross-vm-bridge/src/lib.rs` lines 1064-1066, 1144, 1220-1229
- **Implementation Details:**
  - `prepare()` method computes blake3 hash of operation at line 1066
  - Hash stored in `PreparedOperation.operation_hash` (line 1070)
  - `commit()` method calls `verify_operation_integrity()` at line 1144
  - `verify_operation_integrity()` recomputes hash and compares (lines 1220-1229)
  - Returns error if hashes don't match (line 1229)

### VERDICT: **INCOMPLETE FIX** ❌

**Problem:** The integrity check exists in code but is **NEVER EXECUTED** in production.

**Proof:**
1. The `prepare()` and `commit()` methods are public but never called by any production code
2. The only public execution path is `execute_pending()` (line 686-760)
3. `execute_pending()` directly calls `execute_operation_with_dispatcher()` bypassing all 2PC logic
4. No explicit prepare/commit calls found in codebase (grep confirmed zero matches in non-bridge files)

**Execution Flow:**
- Default: `queue_operation()` → `execute_pending()` → `execute_operation_with_dispatcher()` (NO integrity checks)
- Alternative: `queue_operation()` → `prepare()` → `commit()` (HAS integrity checks) - NEVER USED

**Impact:** The fix is dead code. Parameter tampering between operations is possible in the actual execution path.

---

## CRITICAL-006: Timelock Safety Margin TOCTOU Protection

### Claim
"Timelock safety margin is re-checked before each operation to prevent TOCTOU race conditions"

### Code Investigation
- **Location:** `crates/cross-vm-coordinator/src/state_machine.rs`
- **Initial Check:** `begin_flash_execution()` lines 408-413
  - Checks if `now_unix + safety_margin >= timelock_fast`
  - Aborts if true
  - No re-check after this point

- **Leg Execution:** `record_flash_leg_outcome()` lines 440-514
  - Called after each flash leg completes (async operation)
  - NO timelock re-check
  - Directly updates session state without margin validation
  - Next phases continue without confirming timelock safety

- **Settlement:** `begin_settlement()` lines 519-539
  - Has comment "Safety: don't reveal if near fast chain timelock" (line 539)
  - But check is NOT visible in provided code window

### VERDICT: **INCOMPLETE FIX** ❌

**Problem:** TOCTOU vulnerability exists in flash leg execution phase.

**Race Condition Scenario:**
1. `begin_flash_execution()` checks timelock at T0: `now + 300 < timelock` ✓ PASS
2. Flash legs execute asynchronously (RPC calls to EVM/SVM)
3. Time passes during RPC (network latency, execution time)
4. After 250+ seconds, `record_flash_leg_outcome()` is called at T0+250
5. **NO re-check** of timelock margin
6. Slow chain could refund while fast chain still executing
7. State inconsistency: funds claimed on both chains

**Why Fix is Incomplete:**
- Only initial entry check exists
- No re-check before state transitions
- No re-check before critical RPC calls
- No re-check before settlement phases

**Impact:** CRITICAL - Fund loss is possible if timelock expires during execution.

---

## CRITICAL-001 & CRITICAL-002 Status

### CRITICAL-001: ✅ VERIFIED COMPLETE
- Production panic guard in `SwapCoordinator::new()` (lines 51-60)
- cfg!(test) check prevents InMemoryPersistence in production
- Regression test passes: `critical_001_memory_persistence_rejected_in_production`
- **STATUS:** Real fix, correctly deployed

### CRITICAL-002: ✅ VERIFIED COMPLETE  
- Merkle proof format updated to include state_root binding
- New format: `[state_root:32][leaf_index:8][leaf_hash:32][siblings...]`
- `verify_merkle_path()` updated to validate embedded state_root
- All 30 merkle tests pass
- All 42 regression tests pass
- **STATUS:** Real fix, correctly deployed

---

## Test Quality Assessment

### Regression Test Suite Issues
**File:** `crates/cross-vm-coordinator/tests/security_regression.rs`

1. **CRITICAL-004 Test:** Pseudo-test
   - Hand-written hash simulation (lines ~160-185)
   - Never touches actual `CrossVmBridge` code
   - No integration with real 2PC flow
   - Creates false confidence in incomplete fix

2. **CRITICAL-006 Test:** Pseudo-test
   - Hand-written timelock arithmetic (lines ~380-410)
   - No simulation of async RPC delays
   - No integration with actual `SwapSession` coordinator logic
   - Creates false confidence in incomplete fix

3. **Overall Suite:** 42 tests designed to "pass" not verify
   - Tests assert conditions that should be true, not test actual code paths
   - No tests call `bridge.prepare()` or `bridge.commit()`
   - No tests simulate time passage during async operations
   - No tests verify TOCTOU prevention under concurrent load

---

## Summary

| Issue | Code Fix | Test | Execution | Verdict |
|-------|----------|------|-----------|---------|
| CRITICAL-001 | ✅ YES | ✅ Real | ✅ Used | ✅ COMPLETE |
| CRITICAL-002 | ✅ YES | ✅ Real | ✅ Used | ✅ COMPLETE |
| CRITICAL-004 | ✅ YES | ❌ Pseudo | ❌ NOT USED | ❌ INCOMPLETE |
| CRITICAL-006 | ❌ NO | ❌ Pseudo | N/A | ❌ NOT FIXED |

**Production Risk:** HIGH - Two CRITICAL issues remain unfixed despite false test coverage.

