# Next Steps for X3‑lang Cross‑Chain Feature

## 1. Bridge Adapter Module
Create `vm/src/bridge.rs` defining a `BridgeAdapter` trait with methods for
`evm_call` and `svm_call`.  Provide a mock implementation for unit tests.

## 2. Executor Integration
Update `vm/src/executor.rs` to call the bridge adapter for opcodes `0x60`
(`EVM_CALL`) and `0x61` (`SVM_CALL`).  Handle errors by rolling back the
atomic window.

## 3. Atomic Window Enhancements
Extend the VM state to support nested atomic blocks and rollback logic.

## 4. LLVM‑IR Lowering
Add lowering logic in `compiler/src/lowering.rs` to emit LLVM IR for bridge
calls and atomic windows.

## 5. End‑to‑End Tests
Write tests in `tests/` that compile a program with an `atomic fn` performing
real (mocked) bridge calls, run it through the VM, and assert correct state
changes and rollback behavior.

## 6. Documentation
Update the README and add a “Cross‑Chain Execution” section explaining the
new opcodes, usage, and limitations.

## 7. Benchmarking (Optional)
Measure the overhead of bridge calls and atomic windows, and profile the VM
to identify bottlenecks.

