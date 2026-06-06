# X3-lang Product & Technical Specification

## Quick Navigation

- **Unique Capabilities**: [x3-unique-capabilities.md](x3-unique-capabilities.md) — 50 native features that set X3 apart
- **Profitable Primitives**: [x3-profit-primitives.md](x3-profit-primitives.md) — 50 revenue-generating applications built on X3
- **Language Spec** (coming): Complete syntax, semantics, and type system
- **Bytecode Spec** (coming): Opcode reference and VM execution model
- **ABI Spec** (coming): Contract encoding and cross-VM calling conventions

---

## What is X3-lang?

X3-lang is a **deterministic, contract-capable, swarm-executable language** for the Atlas Sphere ecosystem. It compiles into X3 bytecode, executes on X3VM, supports EVM/SVM atomic calls, and runs both on-chain as safe contracts and off-chain as high-speed strategy/swarm jobs.

In short: **X3 is Solidity + bot scripting + swarm job language + cross-VM router, welded into one.**

---

## Core Design Philosophy

**Simple surface. Brutal backend.**

Users write clean, readable code. The compiler and runtime do the monster work.

```x3
fn atomic_arb():
    let route = best_route("USDC", "WETH", "USDC")
    let result = simulate(route)
    
    if result.profit > 1000:
        execute(route)
```

Underneath:
- Deterministic bytecode generation
- Gas metering and resource limits
- X3VM execution
- SIMD acceleration off-chain
- GPU batch simulation
- Swarm scheduling
- Sidecar verification

---

## Non-Negotiable Rules

### On-Chain Safety
X3 must be deterministic on-chain. This means:

| Requirement | Why |
|---|---|
| No floats | Behavior differs across machines |
| No random on-chain | Breaks consensus |
| No eval | Huge attack surface |
| No direct OS/network | Consensus can't depend on external state |
| Gas-metered execution | Prevents DoS and infinite loops |
| Bounded memory/recursion | Prevents bombs and stack attacks |
| Deterministic host calls only | All validators get same result |

### Off-Chain Freedom
Off-chain X3 can be more powerful:
- JIT-compiled
- GPU-accelerated
- Parallel execution
- Unlimited memory (within swarm node limits)
- External calls (with results verified on-chain)

---

## 50 Unique Capabilities

X3-lang is the **only language** that natively implements all of these:

### Cross-VM & Atomicity
1. Atomic EVM↔SVM↔X3 execution in single call
2. Built-in cross-chain bridge primitives with atomic fallback
3. Language-level GPU swarm job scheduling
4. Deterministic off-chain simulation with verifiable receipts
5. First-class mempool introspection APIs

### Math & Finance
6. Native fixed-point arithmetic (no rounding errors)
7. Built-in AMM primitives (constant product, stableswap)
8. Host-call gating for EVM/SVM coordination
9. Dual deterministic (on-chain) / powerful (off-chain) semantics
10. Scoped @no_heap / @max_recursion annotations

### Performance
11. Inline GPU/SIMD blocks
12. Built-in strategy templates & route scoring
13. Host functions for fee delegation / sponsorships
14. Native event subscription & reactive contracts
15. Deterministic randomness from verified sources

### Execution Model
16. Compact bytecode with formal verifier
17. First-class contract storage namespaces
18. Cross-format serialization (JSON/CBOR/RLP/SSZ)
19. Built-in oracle request/reward semantics
20. Role-based access control macros

### More Capabilities (21–50)
[See full list in x3-unique-capabilities.md](x3-unique-capabilities.md)

---

## 50 Profitable Applications

These tricks leverage X3's unique primitives to generate revenue:

### Arbitrage & Trading (1–7)
1. Atomic cross-VM arbitrage
2. Cross-chain yield aggregator
3. On-chain liquidation bot
4. Sandwich-attack mitigator
5. AI-generated rebalancer
6. Cross-VM flash-loan arbitrage
7. Stablecoin peg defense bot

### Markets & Services (8–15)
8. On-chain market maker
9. Predictive gas-cost oracle
10. Token lockers / escrow
11. Fee-sponsored UX
12. GPU compute marketplace
13. MEV strategy vault
14. Cross-chain NFT bridge
15. Multi-leg option writer

### More Applications (16–50)
[See full list in x3-profit-primitives.md](x3-profit-primitives.md)

---

## Syntax & Primitives

### Variables & Types
```x3
let x = 42              # immutable
var y = 10              # mutable
y = y + 1

# Primitives: int, uint, bool, bytes, addr, str
# Composites: list[T], map[K → V], struct
```

### Functions
```x3
fn add(a: int, b: int) -> int:
    a + b

@inline fn hot_path(x: uint) -> uint:
    return x * 2

@payable fn receive_payment():
    emit("payment_received", msg.value)
```

### Control Flow
```x3
if balance > 100:
    emit("rich", user)
elif balance > 10:
    emit("ok", user)
else:
    emit("poor", user)

for pool in pools:
    scan(pool)

while gas_left() > 1000:
    work()
```

### Failures
```x3
if amount == 0:
    fail "amount cannot be zero"
```

---

## Annotations

| Annotation | Meaning |
|---|---|
| `@inline` | Compiler should inline |
| `@payable` | Can receive value |
| `@view` | Read-only, no storage writes |
| `@unsafe` | Allows restricted low-level ops |
| `@extern` | Exposes function externally |
| `@hot` | Performance-critical path |
| `@simd` | Allow vectorized execution |
| `@no_heap` | No heap allocation |
| `@scheduled(period=X)` | Run periodically off-chain |
| `@subscribe("event")` | Subscribe to events |
| `@role("admin")` | Role-based access control |
| `@multisig(M, N)` | M-of-N signature threshold |

---

## Built-in Functions

### State
```x3
get(key) → value
set(key, value)
exists(key) → bool
delete(key)
```

### Math & Utility
```x3
abs(x)
min(a, b)
max(a, b)
hash(bytes) → bytes
keccak(bytes) → bytes
concat(a, b)
```

### Block / Context
```x3
block_number() → uint
timestamp() → uint
caller() → addr
self() → addr
gas_left() → uint
```

### Cross-VM
```x3
evm_call(addr, bytes) → Result
svm_call(addr, bytes) → Result
x3_call(addr, bytes) → Result
atomic_commit()
rollback()
```

### Events
```x3
emit(event_name, data...)
subscribe(event_pattern, handler)
```

### Collections
```x3
len(x) → uint
push(list, value)
pop(list) → value
```

---

## Compiler Pipeline

```
X3 Source
  ↓
Lexer (tokenize)
  ↓
Parser (AST)
  ↓
Type Checker (validate)
  ↓
HIR (high-level IR)
  ↓
LIR (low-level IR)
  ↓
Bytecode Emitter
  ↓
X3 Bytecode (.x3b)
  ↓
Bytecode Verifier
  ↓
X3VM (execution)
```

### Core Stages

| Stage | Output | Purpose |
|---|---|---|
| Lexer | Tokens | Text → symbols |
| Parser | AST | Tokens → tree |
| Type Checker | Typed AST | Validate signatures & types |
| HIR | Intermediate | High-level optimizations |
| LIR | Intermediate | Lower-level optimizations |
| Emitter | Bytecode | Generate .x3b |
| Verifier | Validated bytecode | Reject unsafe/malformed |

---

## X3VM

### Two Modes

| Mode | Execution | Use Case |
|---|---|---|
| **On-chain Interpreter** | Deterministic, gas-metered, safe | Smart contracts on Atlas |
| **Off-chain JIT** | Fast, GPU/swarm accelerated | Simulations, strategy execution |

### Core Components

- **Operand stack** — LIFO execution stack
- **Call frames** — Function call context
- **Locals** — Function-scoped variables
- **Constant pool** — Inline constants
- **Linear memory** — Heap-like storage
- **Gas meter** — Execution cost tracking
- **Host syscall interface** — EVM/SVM/X3 calls
- **Bytecode verifier** — Safety checks before execution
- **Deterministic receipts** — Auditable execution logs

### Opcode Groups

**Stack / Locals**
```
NOP, CONST_U128, CONST_BYTES, LOAD_LOCAL, STORE_LOCAL, DROP, DUP
```

**Arithmetic**
```
ADD, SUB, MUL, DIV, MOD, CMP_EQ, CMP_LT, CMP_GT
```

**Memory**
```
ALLOC, MEM_LOAD, MEM_STORE, MEM_COPY, SLICE
```

**Control Flow**
```
JMP, JMP_IF_FALSE, CALL, RET
```

**Host Calls**
```
HOST_GET, HOST_SET, HOST_EMIT, HOST_CALL_EVM, HOST_CALL_SVM,
HOST_BLOCK_NUMBER, HOST_TIMESTAMP, HOST_CALLER, HOST_SELF,
HOST_GAS_LEFT
```

**Crypto**
```
HASH_BLAKE2B, HASH_KECCAK
```

**Off-chain Acceleration**
```
VEC_MAP, SIMD_ROUTE_SCAN, GPU_BATCH_SIM
```

---

## Contract Lifecycle

1. **Deploy** — Submit contract + metadata
2. **Init** — Run initialization code
3. **Call** — Invoke public functions
4. **Emit** — Generate events
5. **Storage** — Read/write persistent state
6. **Cross-call** — Call EVM/SVM atomically
7. **Receipt** — Return auditable result

---

## Standard Library Layers

### Layer 1: Deterministic (On-Chain Safe)
- `hash()`, `keccak()`
- `encode()`, `decode()`
- `get()`, `set()`, `emit()`
- `caller()`, `block_number()`

### Layer 2: Cross-VM
- `evm_call()`, `svm_call()`, `x3_call()`
- `atomic_commit()`, `rollback()`

### Layer 3: Strategy (Mostly Off-Chain)
- `simulate()`, `best_path()`
- `score_route()`, `query_pool()`
- `estimate_gas()`, `rank_routes()`

### Layer 4: Swarm
- `submit_job()`, `verify_receipt()`
- `store_artifact()`, `load_model()`
- `batch_sim()`, `gpu_inference()`

---

## Error Model

### Compiler Errors
```
X3E0001 SyntaxError
X3E0002 UnknownIdentifier
X3E0003 TypeMismatch
X3E0004 InvalidAnnotation
X3E0005 UnsafeCallNotAllowed
```

### VM Errors
```
X3V0001 OutOfGas
X3V0002 StackUnderflow
X3V0003 InvalidOpcode
X3V0004 MemoryOutOfBounds
X3V0005 DivideByZero
X3V0006 Overflow
X3V0007 HostCallDenied
```

### Contract Errors
```
X3C0001 DeployFailed
X3C0002 InitFailed
X3C0003 PermissionDenied
X3C0004 CrossVmCallFailed
X3C0005 AtomicRollback
```

### Swarm Errors
```
X3S0001 InvalidReceipt
X3S0002 SimulationMismatch
X3S0003 StrategyRejected
X3S0004 UnsafeMutation
```

---

## Repository Layout

```
x3-lang/
├── README.md
├── spec/
│   ├── INDEX.md (this file)
│   ├── x3-unique-capabilities.md
│   ├── x3-profit-primitives.md
│   ├── x3-language-spec.md (coming)
│   ├── x3-bytecode-spec.md (coming)
│   ├── x3-abi-spec.md (coming)
│   ├── x3-contracts-spec.md (coming)
│   └── x3-errors.md
├── crates/
│   ├── x3-lexer/
│   ├── x3-parser/
│   ├── x3-ast/
│   ├── x3-hir/
│   ├── x3-lir/
│   ├── x3-bytecode/
│   ├── x3-vm/
│   ├── x3-verifier/
│   ├── x3-stdlib/
│   ├── x3-compiler/
│   ├── x3-cli/
│   └── x3-tests/
├── examples/
│   ├── counter.x3
│   ├── atomic_evm_svm.x3
│   ├── strategy_score.x3
│   └── storage_demo.x3
└── tests/
    ├── parser/
    ├── compiler/
    ├── vm/
    ├── verifier/
    └── contracts/
```

---

## MVP Scope

### What's In (Phase 1–2)

- Lexer & Parser
- AST & Type Checker
- Bytecode Emitter
- X3VM Interpreter
- Gas Meter
- Storage Built-ins
- Event Built-ins
- Basic contract deploy/call simulation

### Example Programs

**Counter**
```x3
fn inc():
    let n = get("counter")
    set("counter", n + 1)
```

**Balance Check**
```x3
fn has_min(balance: uint, minimum: uint) -> bool:
    balance >= minimum
```

**Cross-VM Placeholder**
```x3
fn call_evm_router(router: addr, data: bytes) -> bytes:
    evm_call(router, data)
```

---

## Phase Roadmap

### Phase 1 — Language Core
**Deliverables**: X3 source → AST  
**Components**: Lexer, Parser, AST, basic type checker  
**Duration**: 2 weeks

### Phase 2 — Bytecode
**Deliverables**: X3 source → bytecode → readable disassembly  
**Components**: HIR, LIR, opcode definitions, emitter, disassembler  
**Duration**: 2 weeks

### Phase 3 — X3VM
**Deliverables**: Bytecode → executed result  
**Components**: Stack VM, locals, call frames, gas meter, host calls, memory model  
**Duration**: 3 weeks

### Phase 4 — X3 Contracts
**Deliverables**: X3 contract deploy/call works locally  
**Components**: Deploy package, ABI, metadata, contract storage, call interface, event logs  
**Duration**: 2 weeks

### Phase 5 — Atlas Integration
**Deliverables**: EVM ↔ SVM ↔ X3 atomic execution  
**Components**: Substrate pallet wrapper, X3VM runtime adapter, EVM/SVM host-call bridge, atomic rollback, SDK support  
**Duration**: 4 weeks

### Phase 6 — Swarm Integration
**Deliverables**: X3 bytecode runs in swarm; verified results submitted  
**Components**: Off-chain JIT runner, strategy simulation mode, deterministic receipts, Strategy Vault integration, sidecar submission  
**Duration**: 4 weeks

---

## First Milestone: "X3 Breathes"

The first real win is when this works end-to-end:

```
counter.x3
  ↓
parse
  ↓
compile to bytecode
  ↓
run in X3VM
  ↓
storage changes verified
```

Everything else builds from that moment.

---

## Differentiation Summary

X3-lang is the **only language** that:

1. **Atomically executes across EVM/SVM/X3** in a single call
2. **Natively accesses mempool** for MEV and trading strategies
3. **Provides deterministic GPU acceleration** via swarm integration
4. **Verifies off-chain computation** on-chain with receipts
5. **Embeds fixed-point finance math** without rounding errors
6. **Supports AI agent sandboxing** with gas limits
7. **Offers true cross-chain atomicity** with rollback semantics
8. **Makes finance primitives first-class** (not library functions)
9. **Combines on-chain determinism** with off-chain speed
10. **Is designed for profit** — every primitive has a revenue application

---

## Revenue Potential

If 20 of the 50 profitable applications are built and reach scale:

- Conservative: $20M–$100M/year
- Moderate: $100M–$500M/year
- Optimistic: $500M–$2B/year

---

## Getting Started

1. Read [x3-unique-capabilities.md](x3-unique-capabilities.md) to understand what sets X3 apart
2. Study [x3-profit-primitives.md](x3-profit-primitives.md) to see revenue opportunities
3. Implement Phase 1 (Lexer, Parser, AST)
4. Get "X3 breathes" working with the counter example
5. Expand to Phases 2–6

---

## Key Principles

1. **Determinism first** — On-chain behavior must be reproducible across all validators
2. **Simplicity surface** — Users write readable code; backend does the work
3. **Native cross-VM** — Don't use bridges; use atomic transactions
4. **Profit-oriented** — Every primitive should enable revenue
5. **GPU-ready** — Design for SIMD and swarm acceleration from the start
6. **Production-safe** — No stubs, mocks, or fake data in core paths

---

*Last updated: May 2026*
