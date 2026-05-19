# Cargo Build Issues - gpu-swarm Crate

**Status**: 73 compilation errors, 61 warnings  
**Affected Crate**: `crates/gpu-swarm`  
**Date Documented**: February 8, 2026  
**Frontend Status**: ✅ Working with mock data at http://localhost:5174/

---

## Overview

The `gpu-swarm` crate fails to compile due to:
1. **Breaking changes** in dependencies (libp2p, prometheus)
2. **Missing type definitions** (TaskResult, error variants)
3. **Type mismatches** in struct field assignments
4. **API breaking changes** from external libraries

The frontend (swarm-dashboard) is fully functional with mock data fallback, so this does not block development.

---

## Error Categories & Fixes Required

### 1. Missing libp2p Imports (8 errors)

**Files**: `crates/gpu-swarm/src/network.rs`

**Errors**:
```
error[E0432]: unresolved import `libp2p::gossipsub::Publish`
error[E0432]: unresolved import `libp2p::identify::Identify`
error[E0432]: unresolved import `libp2p::kad::{Kademlia, KademliaConfig, QueryResult, store::MemoryStore}`
error[E0432]: unresolved import `libp2p::ping::Ping`
error[E0432]: unresolved import `libp2p::swarm::IntoConnectionHandler`
error[E0603]: trait `ConnectionHandler` is private
```

**Root Cause**: libp2p v0.51+ breaking changes - these types/traits were reorganized or removed.

**Fix**: 
- Review libp2p release notes for v0.51+
- Update imports to new module paths
- Replace removed traits with newer equivalents
- Update `Cargo.toml` libp2p version if needed

**Files to Update**:
- `crates/gpu-swarm/src/network.rs` (lines 15-23, 320-346)

---

### 2. Missing Prometheus Types (30+ errors)

**Files**: `crates/gpu-swarm/src/monitoring.rs`

**Errors**:
```
error[E0433]: failed to resolve: could not find `CounterOpts` in `prometheus`
error[E0433]: failed to resolve: could not find `GaugeOpts` in `prometheus`
error[E0599]: no method named `encode` found for struct `TextEncoder`
```

**Root Cause**: Prometheus crate v0.13+ removed `CounterOpts` and `GaugeOpts` (deprecated in favor of `CounterBuilder`, `GaugeBuilder`).

**Fix**:
- Replace `prometheus::CounterOpts::new()` with appropriate builder pattern
- Replace `prometheus::GaugeOpts::new()` calls
- Update `TextEncoder::encode()` call signature
- Update `Cargo.toml` prometheus version or revert to v0.12

**Files to Update**:
- `crates/gpu-swarm/src/monitoring.rs` (lines 63, 71, 79, 96, 101, 106, 111, 116, 121, 126, 134, 142, 156, 164, 172, 180, 197, 202, 207, 215, 220, 225, 230, 269)

---

### 3. Missing Task Type Definition (7 errors)

**Files**: `crates/gpu-swarm/src/gpu_backends/mod.rs`, `cuda.rs`, `opencl.rs`, `vulkan.rs`, `metal.rs`, `webgpu.rs`, `x3_vm.rs`

**Errors**:
```
error[E0432]: unresolved import `crate::task::TaskResult`
```

**Root Cause**: `TaskResult` type is missing from `crate::task` module.

**Fix**:
- Define `TaskResult` in `crates/gpu-swarm/src/task.rs`
- Should likely be an enum with variants for success/failure states
- Check what fields/variants are needed from usage in these files

**Files to Update**:
- `crates/gpu-swarm/src/task.rs` (add TaskResult definition)
- All gpu_backends files (update imports)

---

### 4. Missing Error Enum Variants (12 errors)

**Files**: `crates/gpu-swarm/src/gpu_backends/*.rs`, `blockchain.rs`, `x3_vm.rs`

**Errors**:
```
error[E0599]: no variant or associated item named `BlockchainError` found for enum `SwarmError`
error[E0599]: no variant or associated item named `ExecutionError` found for enum `SwarmError`
```

**Root Cause**: `SwarmError` enum is missing these variants.

**Fix**:
- Add to `crates/gpu-swarm/src/error.rs` (or wherever SwarmError is defined):
  ```rust
  pub enum SwarmError {
      // ... existing variants ...
      BlockchainError(String),
      ExecutionError(String),
  }
  ```

**Files to Update**:
- `crates/gpu-swarm/src/error.rs` (add missing variants)
- No code changes needed in other files if variants are added

---

### 5. Type Mismatches - memory_bandwidth_gbs (5 errors)

**Files**: `crates/gpu-swarm/src/gpu_backends/cuda.rs`, `opencl.rs`, `vulkan.rs`, `metal.rs`, `webgpu.rs`

**Errors** (example from cuda.rs:91):
```
error[E0308]: mismatched types
memory_bandwidth_gbs: *bandwidth,
```

**Root Cause**: Attempting to assign pointer to non-pointer field, or f32/f64 mismatch.

**Fix**:
- Dereference pointer: `*bandwidth` → `*bandwidth` (if field expects f32)
- Or change field type definition
- Check `GpuDeviceInfo` struct definition for expected type

**Files to Update**:
- `crates/gpu-swarm/src/gpu_backends/cuda.rs:91`
- `crates/gpu-swarm/src/gpu_backends/opencl.rs:57`
- `crates/gpu-swarm/src/gpu_backends/vulkan.rs:62`
- `crates/gpu-swarm/src/gpu_backends/metal.rs:57`
- `crates/gpu-swarm/src/gpu_backends/webgpu.rs:46`

---

### 6. Type Mismatches - task_id field (8 errors)

**Files**: `crates/gpu-swarm/src/gpu_backends/cuda.rs`, `opencl.rs`, `vulkan.rs`, `metal.rs`, `webgpu.rs`

**Errors** (example from cuda.rs:148):
```
error[E0308]: mismatched types
task_id: task.id.clone(),
```

**Root Cause**: `task.id` type doesn't match struct field type (likely String vs u64 or similar).

**Fix**:
- Check `Task` struct `id` field type
- Check `TaskResult` or output struct field type
- Convert types as needed (e.g., `.parse()?`, `.to_string()`, etc.)

**Files to Update**:
- All gpu_backends files (cuda.rs:148, 191, opencl.rs:126, vulkan.rs:131, metal.rs:126, webgpu.rs:115)

---

### 7. Type Mismatches - achieved_gflops (5 errors)

**Files**: `crates/gpu-swarm/src/gpu_backends/*.rs`

**Errors** (example from cuda.rs:155):
```
error[E0308]: mismatched types
achieved_gflops: device_info.peak_fp32_tflops * 0.7,
```

**Root Cause**: Field type mismatch (u32/u64 vs f32/f64).

**Fix**:
- Check struct field type
- Cast or convert as needed: `.as_f32()`, `.try_into()?`, etc.

**Files to Update**:
- `crates/gpu-swarm/src/gpu_backends/cuda.rs:155, 198`
- `crates/gpu-swarm/src/gpu_backends/opencl.rs:133`
- `crates/gpu-swarm/src/gpu_backends/vulkan.rs:138`
- `crates/gpu-swarm/src/gpu_backends/metal.rs:133`
- `crates/gpu-swarm/src/gpu_backends/webgpu.rs:122`

---

### 8. Missing Task Field (1 error)

**File**: `crates/gpu-swarm/src/x3_vm.rs:80`

**Error**:
```
error[E0609]: no field `payload` on type `&task::Task`
let bytecode = &task.payload;
```

**Root Cause**: `Task` struct doesn't have a `payload` field.

**Fix**:
- Either add `payload` field to `Task` struct
- Or update code to use alternate field (e.g., `task.data`, `task.bytecode`)
- Check what x3_vm expects vs what Task provides

**Files to Update**:
- `crates/gpu-swarm/src/task.rs` (add payload field if appropriate)
- `crates/gpu-swarm/src/x3_vm.rs:80` (update field reference)

---

### 9. Borrow Checker Issues (1 error)

**File**: `crates/gpu-swarm/src/network.rs:346`

**Error**:
```
error[E0502]: cannot borrow `*self` as mutable because it is also borrowed as immutable
for bootstrap in &self.config.bootstrap_peers {
```

**Root Cause**: Attempting mutable borrow while immutable borrow is active.

**Fix**:
- Collect bootstrap peers into Vec before loop: `let peers: Vec<_> = self.config.bootstrap_peers.iter().cloned().collect();`
- Then iterate over the Vec instead of `&self.config.bootstrap_peers`

**Files to Update**:
- `crates/gpu-swarm/src/network.rs:345-346`

---

### 10. Syntax Errors (3 errors)

**Files**: 
- `crates/gpu-swarm/src/gpu_backends/opencl.rs:35`
- `crates/gpu-swarm/src/gpu_backends/vulkan.rs:39`
- `crates/gpu-swarm/src/gpu_backends/metal.rs:35`

**Errors**:
```
error: expected `;`, found `#`
#[cfg(feature = "opencl")]
```

**Root Cause**: Missing semicolon after previous statement.

**Fix**:
- Check lines 34, 38, 34 respectively for missing semicolon
- Add semicolon to end of previous line

**Files to Update**:
- `crates/gpu-swarm/src/gpu_backends/opencl.rs:34`
- `crates/gpu-swarm/src/gpu_backends/vulkan.rs:38`
- `crates/gpu-swarm/src/gpu_backends/metal.rs:34`

---

### 11. HashMap Method Bounds Issue (1 error)

**File**: `crates/gpu-swarm/src/gpu_backends/mod.rs:268`

**Error**:
```
error[E0599]: the method `get` exists for struct `HashMap<GpuBackendType, u32>`, 
but its trait bounds were not satisfied
```

**Root Cause**: `GpuBackendType` doesn't implement `Eq + Hash` traits.

**Fix**:
- Add derives to `GpuBackendType` enum: `#[derive(Eq, PartialEq, Hash, Clone)]`

**Files to Update**:
- `crates/gpu-swarm/src/gpu_backends/mod.rs` (line ~58, update derive)

---

## Quick Fix Priority

**High Priority** (blocks everything):
1. Fix prometheus types (30+ errors)
2. Fix libp2p imports (8 errors)
3. Add missing TaskResult type (7 errors)

**Medium Priority** (fixes many errors):
4. Add missing error variants (12 errors)
5. Fix type mismatches (18 total errors)

**Low Priority** (individual fixes):
6. Fix borrow checker (1 error)
7. Fix syntax errors (3 errors)

**Estimated Effort**: 4-6 hours with dependency research

---

## Testing After Fixes

Once compilation succeeds:
```bash
cargo test -p gpu-swarm
cargo build --release -p gpu-swarm
cargo build --workspace
```

Ensure:
- ✓ No compile errors
- ✓ No warnings (or acceptable warnings) 
- ✓ Unit tests pass
- ✓ Integration tests pass

---

## Frontend Status (Not Affected)

✅ The swarm-dashboard frontend is fully functional:
- Running at http://localhost:5174/
- Using mock data fallback
- All routes and components working
- No blocking issues

Cargo build issues do NOT impact frontend development or testing.
