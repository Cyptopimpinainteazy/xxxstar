# X3 Bytecode v0.1 Specification

This document defines the X3 bytecode format for version v0.1.

Design goals
- Deterministic execution across architectures
- Extremely compact, fixed-width decoding friendly for GPU and JIT
- Mutation-friendly instruction layout for swarm-based evolution
- Secure for on-chain verification and cross-VM atomicity

Instruction format
- Byte ordering: Little Endian for immediate values.
- All instructions are aligned to 4 bytes.
- Opcode: 1 byte (0x00..0xFF) - primary opcode
- Mode/Flags: 1 byte - encodes register vs stack operand forms, immediates, vector flags
- Operand: 2 bytes - small immediate, register index, or low 16-bit of address

Full instruction layout, 32-bit instruction word (little-endian):
- Byte 0: opcode
- Byte 1: flags / mode
- Bytes 2-3: operand

Extended immediates
- If flag indicates extended immediate (EX_IMM), the instruction is followed by a 64-bit immediate (little-endian) aligned to next 8 bytes.
- Extended immediates are limited to instruction forms marked as EX_IMM.

Register model
- 32 general-purpose registers: R0..R31 (32-bit or 64-bit depending on runtime architecture; semantics are platform independent for bytecode.)
- Special-purpose registers:
  - PC: Program counter (byte index into code segment)
  - SP: Stack pointer (index into operand stack)
  - FP: Frame pointer
  - GAS: Gas counter (u128 gas budget)
  - MMU_BASE: Base pointer for linear memory (u32)
- Vector registers V0..V7 (for SIMD operations) 128-bit lanes (e.g., 4x32-bit lanes) - each vector register is 16 bytes.

Register semantics
- R0 is conventionally zero for some operations; not required to be read-only but can be optimized as such.
- R31 reserved for return values for ABI when returning a single integer.

Stack/memory model
- Hybrid model: both operand stack and registers exist.
- Operand stack: 64k entries max (configurable at compile-time / verification), zero-copy for values which fit by value, else references.
- Linear memory (heap): pages of 64KiB, max pages = 256 (configurable). Memory accesses are deterministic and bounds-checked.
- Call stack: CallFrames tracked in a separate call stack region with deterministic limits.

Deterministic bounds checking
- All memory accesses perform range checks against heap size using integer arithmetic.
- Any bounds violation traps deterministically and consumes remaining gas.

Gas model
- Every opcode consumes cycles (user-visible gas) based on a per-opcode table. See gas-model.md.
- Memory growth has a scaling cost per page.
- Cross-VM calls have fixed overhead costs.
- Gas is consumed before instruction execution; if gas < cost, trap.

Opcode categories and initial table
- Base arithmetic and logic
- Memory load/store
- Control flow (JMP, conditional branches), function calls
- Stack operations (PUSH, POP)
- VM-native swap operations (swap EVM/SVM/X3 state)
- Crypto operations (SHA256, KECCAK256, ED25519_VERIFY, SECP256K1_VERIFY)
- Atomic ops for cross-VM commit/rollback
- Vector/SIMD operations

Sample opcodes (partial, v0.1):
- 0x01 ADD_RRR  R[a] = R[b] + R[c]
  - Encoded: opcode=0x01, flags: REG3 (three register operands), operand encodes packed registers in bytes 2-3.
- 0x02 SUB_RRR  R[a] = R[b] - R[c]
- 0x10 LOAD_RAI R[a] = mem[R[b] + imm16]
- 0x11 STORE_RAI mem[R[b] + imm16] = R[a]
- 0x20 PUSH_IMM value -> stack
- 0x21 POP dest
- 0x30 JMP offset16 (signed)
- 0x31 JZ offset16 (jump if top-of-stack is zero)
- 0x32 CALL addr16 (push return PC)
- 0x33 RET (pop return PC)
- 0x40 CRYPTO_SHA256 (R[a] = sha256(R[b])) - for short inputs in registers
- 0x41 CRYPTO_KECCAK256 (R[a] = keccak256(R[b]))
- 0x50 ATOMIC_BEGIN
- 0x51 ATOMIC_COMMIT
- 0x52 ATOMIC_ROLLBACK
- 0x60 EVM_CALL (bridge to EVM adapter)
- 0x61 SVM_CALL (bridge to SVM adapter)
- 0x70 SIMD_ADD_VV V[a] = V[b] + V[c]

Instruction definitions
- Each instruction is accompanied with opcode listed in `opcode_table.md` (implemented below in code repo), exact encoding rules, and gas costs.

Endianness and alignment
- All multi-byte immediates are little endian.
- Instructions are aligned to 4 bytes. Extended immediates aligned to 8 bytes.

Limitations and safety
- No dynamic jumps to arbitrary addresses: all jump targets must be validated to land on instruction boundaries and be within the code section at verification time.
- Recursion depth and call stack depth are bounded and verified.
- No raw pointer arithmetic or nullable pointers. All references are to linear memory.

ABI and debug
- Bytecode includes a header with debug metadata: symbol table, source map (span -> offset), and relocation entries.
- The header is optional for release builds; required for on-chain verification.

Mutation friendliness
- Fixed-width instruction ensures mutation operations (bit flips, opcode swap) always land on valid boundaries.

Cross-VM atomicity
- ATOMIC_BEGIN opens an atomic window where cross-VM calls must be committed together via ATOMIC_COMMIT or ATOMIC_ROLLBACK.
- Cross-VM calls produce receipts; commit requires signatures/cryptographic proof.

Compatibility
- Bytecode is designed to be portable across CPU and GPU and amenable to JIT compilation.
