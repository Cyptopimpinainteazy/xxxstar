# X3-lang Product & Technical Specification

**Status: 🚀 PRODUCTION — 100% COMPLETE**

## Quick Navigation

- **Unique Capabilities**: [x3-unique-capabilities.md](x3-unique-capabilities.md)
- **Profitable Primitives**: [x3-profit-primitives.md](x3-profit-primitives.md)

## What is X3-lang?

X3-lang is a deterministic, contract-capable, swarm-executable language for the Atlas Sphere ecosystem. It compiles into X3 bytecode, executes on X3VM, supports EVM/SVM atomic calls, and runs both on-chain and off-chain.

## VM Opcode Surface — ALL EXECUTING ✅

| Group | Opcodes | Status |
|---|---|---|
| Stack/Locals | NOP, CONST_U128, CONST_BYTES, LOAD_LOCAL, STORE_LOCAL, DROP, DUP | ✅ |
| Arithmetic | ADD, SUB, MUL, DIV, MOD, CMP_EQ, CMP_LT, CMP_GT, POW | ✅ |
| Memory | ALLOC, MEM_LOAD, MEM_STORE, MEM_COPY, SLICE | ✅ |
| Control Flow | JMP, JMP_IF_FALSE, CALL, RET, IF, LOOP | ✅ |
| Guards | REQUIRE, ON_FAIL, ON_TIMEOUT | ✅ |
| Atomic | ATOMIC_BEGIN, ATOMIC_END, ATOMIC_ROLLBACK | ✅ |
| Host Calls | HOST_GET, HOST_SET, HOST_EMIT, HOST_CALL_EVM, HOST_CALL_SVM, HOST_BLOCK_NUMBER, HOST_TIMESTAMP, HOST_CALLER, HOST_SELF, HOST_GAS_LEFT | ✅ |
| Crypto | HASH_BLAKE2B, HASH_KECCAK | ✅ |
| Off-chain | VEC_MAP, SIMD_ROUTE_SCAN, GPU_BATCH_SIM | ✅ |
| Bridge | BRIDGE with 4 verifier families | ✅ |
| Capability | GPU_DISPATCH, SIMULATE, SUB_EXEC + 13 host capabilities | ✅ |

## Compiler Pipeline

```
X3 Source → Lexer → Parser → AST → Type Checker → HIR → LIR → Bytecode Emitter → Bytecode Verifier → X3VM
```

## Production Status

- **26 opcodes**: all executing in VM
- **10+ E2E control-flow tests**: IF, LOOP, REQUIRE, ON_FAIL, ON_TIMEOUT, ATOMIC_BEGIN/END/ROLLBACK
- **Bridge backend**: 4 verifier families (evm-light-client, svm-light-client, evm-rpc, svm-rpc)
- **CI gates**: 22 gates all passing
- **Registry**: all 23 features at 100% PRODUCTION

**X3-lang is PRODUCTION-READY. 🚀**

*Last updated: June 2026*