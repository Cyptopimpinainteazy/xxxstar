# X3 Security & Verification

This document outlines deterministic safety and verification rules for X3 bytecode.

Rules
- All jump targets must be verified to land on the start of an instruction boundary.
- Stack and call stack overflow are deterministically trapped.
- Memory bounds are enforced on every access.
- No raw pointer arithmetic is permitted.
- All serialization/deserialization routines are deterministic and cryptographically verified.
- Cryptographic opcodes must be constant-time (no secret-dependent branching), per provider assumptions.

Bytecode validation
- Opcode validation: check if opcode is within the supported set.
- Flags and modes validation: ensure flags are valid for opcode.
- Extended immediate validation: ensure immediate size and alignment valid.
- Control flow validation: build CFG and ensure no entry to the middle of an instruction.
- Side-effect analysis: verify external calls are only allowed within atomic windows when cross-VM safety is required.

Cross-VM atomicity
- Cross-VM operations produce signed receipts by adapters.
- Commit only allowed if all adapters provide valid receipts proving determinism of effects.
- Rollbacks are permitted only when receipts or signatures invalid.

Proof-of-execution
- Receipts include: transaction hash, entry PC, exit PC, gas used, and Merkle root of written keys.
- Receipts are signed by the adapter environment in a deterministic manner.

Verification complexity
- Verification must be O(n) time on number of instructions and edges in CFG, where n = size of code segment.
- CFG edges limited by instruction semantics (no computed/generative jumps).

Determinism guarantees
- PRNG allowed only via `VRF(seed)` which uses a verified seed from block header, not runtime.
- No floating point nondeterminism allowed between nodes; FP operations require defined rounding and bits.

Safety mitigations
- All external adapters sandbox access to underlying chain storage.
- Atomic windows are limited by gas, depth, and number of cross-VM calls.

Contact
- Security issues should be reported to the X3 team with reproduction steps and minimal bytecode example.
