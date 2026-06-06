# x3-lang Completion Plan

This document outlines the concrete steps required to bring **x3-lang** to a production‑ready state as described in the task.

## High‑Priority Milestones

1. **Opcode contract alignment**
   - Review `compiler/src/emitter.rs`, `vm/src/verifier.rs`, and `vm/src/executor.rs`.
   - Extract a single source‑of‑truth opcode spec (e.g., `x3-lang/spec/opcodes.yaml`).
   - Refactor emitter, verifier, and executor to use the shared spec.

2. **Preserve intent semantics in lowering**
   - Audit `compiler/src/parser.rs`, `compiler/src/lowering.rs`, and `compiler/src/ir.rs` for loss of:
     - `min_output` on swaps
     - Bridge route steps
     - Timeout/refund semantics
   - Extend IR structures to carry these attributes through lowering.
   - Update lowering passes to forward them unchanged.

3. **Real production backend wiring**
   - Replace the default `DryRunBridge` with a configurable production bridge in `vm/src/bridge.rs` and `vm/src/x3_lang_vm.rs`.
   - Add environment‑variable based selection (`X3_BACKEND=prod|dry`).

4. **Integration bridge implementation**
   - Implement `crates/x3-integration/src/compiler_bridge.rs` to translate compiled bytecode into on‑chain calls.
   - Wire the bridge into the VM startup flow.

5. **End‑to‑end compile‑run tests**
   - Add real `.x3` source files under `tests/e2e/`.
   - Create a test harness in `proof-forge/src/runners/x3language.rs` that:
     1. Compiles the source using the compiler.
     2. Executes the emitted bytecode on the VM.
     3. Asserts expected state changes.
   - Update existing VM test suites to use these new examples.

## Secondary Enhancements

* Slippage protection, refund policy objects, bridge destination/receiver/finality semantics, replay/nonce/finality checks – to be added to the IR and runtime.
* Rust type‑checker for finance/cross‑chain constraints.
* Source‑maps and debug metadata.
* Fuzz/property tests for determinism and rollback safety.
* Gas model tied to opcodes.

## Developer‑Experience Tooling (Nice‑to‑Have)

* `x3-fmt` – code formatter.
* `x3-lint` – static analysis linter.
* `x3-pkg` – package manager.
* `x3-repl` – interactive REPL.
* `x3-doc` – documentation generator.
* `x3-test` – test harness CLI.

## Documentation Updates

* Remove placeholder entries from `docs/x3-lang/README.md`.
* Add sections describing the opcode spec, backend configuration, and end‑to‑end testing workflow.

---

 - [x] Analyze opcode contract mismatches across emitter, verifier, executor
 - [x] Align opcode definitions and create shared spec
 - [x] Review parser/lowering/IR for intent loss (min_output, bridge steps, refunds)
 - [ ] Preserve those semantics in the lowering pipeline
 - [ ] Replace DryRunBridge with a real production backend configuration
 - [ ] Implement real compiler bridge in `crates/x3-integration/src/compiler_bridge.rs`
 - [ ] Add end‑to‑end tests that compile real `.x3` sources and execute the emitted bytecode
 - [ ] Implement slippage‑protection, proper refund objects, bridge semantics, and replay/nonce checks in the runtime path
 - [ ] Begin developer‑experience tooling (formatter, linter, package manager, REPL, docs)
 - [ ] Update documentation to remove “planned” placeholders and reflect new capabilities
- [ ] Begin developer‑experience tooling (formatter, linter, package manager, REPL, docs)
- [ ] Update documentation to remove “planned” placeholders and reflect new capabilities

This plan can be iterated on, and each checklist item can be marked complete as work progresses.
