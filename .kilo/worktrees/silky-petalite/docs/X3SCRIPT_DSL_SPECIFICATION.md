# X3Script DSL — Canonical Language Specification

> **Status**: Canonical | **Version**: 0.9.0 (Draft) | **Last Updated**: 2025-12-10

X3Script is the high-level domain-specific language for writing on-chain contracts, AI agent strategies, and cross-VM operations on X3 Chain. It compiles deterministically to X3 bytecode via the MIR optimizer pipeline.

---

## Table of Contents

1. [Language Philosophy](#1-language-philosophy)
2. [Top-Level Language Concepts](#2-top-level-language-concepts)
3. [Statements & Control Flow](#3-statements--control-flow)
4. [Operations — Machine Verbs](#4-operations--machine-verbs)
5. [Cross-VM Hostcalls](#5-cross-vm-hostcalls)
6. [Type System](#6-type-system)
7. [Built-In Libraries](#7-built-in-libraries)
8. [Determinism & AI Safety](#8-determinism--ai-safety)
9. [Full Example Contract](#9-full-example-contract)
10. [Memory Model Integration](#10-memory-model-integration)
11. [Compilation Pipeline](#11-compilation-pipeline)
12. [Best Practices](#12-best-practices)

---

## 1. Language Philosophy

X3Script is designed with four core principles:

| Principle      | Description                                  |
| -------------- | -------------------------------------------- |
| **Small**      | Mutation engines can rewrite entire programs |
| **Structured** | Compiles deterministically to MIR/bytecode   |
| **High-level** | Readable and writable by humans              |
| **Low-level**  | Superoptimizer can leverage every construct  |

### Design Lineage

```
Solidity  ×  Rust  ×  Lua  →  X3Script
   │           │        │
   │           │        └── Minimal syntax, embeddable
   │           └─────────── Ownership concepts, no GC
   └─────────────────────── Contract semantics, storage model
```

**Result:** A language "stripped to the bone" — no unnecessary syntax, no hidden costs, no magic.

---

## 2. Top-Level Language Concepts

X3Script programs contain exactly five top-level constructs:

### 2.1 Modules

The compilation unit. Every X3Script file is a module.

```x3script
module FlashArb {
    // functions, tasks, storage declarations
}
```

- One module per file
- Module name becomes the contract/program identifier
- Modules can import other modules

### 2.2 Functions

All functions are **pure by default** unless explicitly marked `external`.

```x3script
// Pure function — no side effects, deterministic
fn compute_spread(a: u64, b: u64) -> u64 {
    return b - a;
}

// External function — can have side effects
external fn execute(asset: address, amount: u64) {
    system.flashloan(asset, amount, callback);
}
```

| Modifier   | Meaning                             |
| ---------- | ----------------------------------- |
| (none)     | Pure function, no side effects      |
| `external` | Can call hostcalls, modify storage  |
| `internal` | Module-private (default visibility) |
| `public`   | Externally callable                 |

### 2.3 Tasks

Special async-like entrypoints with VM-level scheduling. Used for bots, daemons, and AI workers.

```x3script
// Runs once per block
task rebalance {
    if needs_rebalance() {
        execute_rebalance();
    }
}

// Runs on specific trigger
task on_price_change(asset: address) {
    check_arbitrage(asset);
}

// Periodic task (every N blocks)
task heartbeat @ interval(10) {
    emit Heartbeat(block.number);
}
```

**Task Scheduling:**
- `task name { }` — Runs each block
- `task name @ interval(N)` — Runs every N blocks
- `task name @ trigger(event)` — Runs on event

### 2.4 Storage

Explicit storage model with no ambiguity. Storage is always named and typed.

```x3script
storage Vault {
    balance: u64;
    owner: address;
    is_locked: bool;
    positions: array[Position; 16];
}

storage Config {
    fee_rate: u64;
    max_slippage: u64;
}
```

**Storage Rules:**
- All storage fields are explicitly typed
- Storage access is always qualified: `Vault.balance`
- Storage writes are expensive (gas-metered)
- Storage is persistent across transactions

### 2.5 Memory Regions

Mapped directly to the X3 4-tier memory model:

```x3script
// REGISTER: Single values, fastest access
let x: u64;
let flag: bool;

// STACK: Fixed-size arrays, fast access
let buf[32]: byte;
let prices[8]: u64;

// HEAP: Dynamic within fixed bounds
heap myvec[128];
heap orderbook[1024];

// GLOBAL: Persistent storage
global Vault;
global Config;
```

| Region   | Keyword              | Lifetime          | Cost     |
| -------- | -------------------- | ----------------- | -------- |
| Register | `let`                | Function scope    | 1 gas    |
| Stack    | `let [N]`            | Function scope    | 2 gas    |
| Heap     | `heap [N]`           | Transaction scope | 5 gas    |
| Global   | `global` / `storage` | Persistent        | 100+ gas |

---

## 3. Statements & Control Flow

Every statement compiles directly to MIR without ambiguity.

### 3.1 If / Elif / Else

```x3script
if x > 10 {
    y = 2;
} elif x == 0 {
    y = 9;
} else {
    y = 1;
}
```

**Compilation:** Direct mapping to `JZ`/`JNZ` with branch labels.

### 3.2 While

```x3script
while price < target {
    price = dex.price(token);
    iterations += 1;
}
```

**Constraint:** Loop bound must be statically analyzable for AI safety.

### 3.3 For-In (Static Only)

```x3script
// Range-based (compile-time unrolled if small)
for i in 0..32 {
    sum += buf[i];
}

// Array iteration
for pos in positions {
    total += pos.value;
}
```

**Constraint:** Range must be compile-time constant or bounded.

### 3.4 Break / Continue

Standard loop control — no surprises:

```x3script
for i in 0..100 {
    if should_skip(i) {
        continue;
    }
    if found_target(i) {
        result = i;
        break;
    }
    process(i);
}
```

### 3.5 Return

```x3script
fn early_exit(x: u64) -> u64 {
    if x == 0 {
        return 0;  // Early return
    }
    return x * 2;
}
```

### 3.6 Assert

```x3script
assert(amount > 0, "amount must be positive");
assert(caller == Vault.owner, "unauthorized");
```

---

## 4. Operations — Machine Verbs

These map directly onto MIR opcodes.

### 4.1 Arithmetic

| Operator | Operation      | MIR Opcode |
| -------- | -------------- | ---------- |
| `+`      | Addition       | `ADD`      |
| `-`      | Subtraction    | `SUB`      |
| `*`      | Multiplication | `MUL`      |
| `/`      | Division       | `DIV`      |
| `%`      | Modulo         | `MOD`      |
| `<<`     | Left shift     | `SHL`      |
| `>>`     | Right shift    | `SHR`      |
| `&`      | Bitwise AND    | `AND`      |
| `\|`     | Bitwise OR     | `OR`       |
| `^`      | Bitwise XOR    | `XOR`      |

### 4.2 Comparisons

| Operator | Operation        | MIR Opcode |
| -------- | ---------------- | ---------- |
| `==`     | Equal            | `EQ`       |
| `!=`     | Not equal        | `NE`       |
| `<`      | Less than        | `LT`       |
| `<=`     | Less or equal    | `LE`       |
| `>`      | Greater than     | `GT`       |
| `>=`     | Greater or equal | `GE`       |

### 4.3 Logical

| Operator | Operation   | MIR Opcode |
| -------- | ----------- | ---------- |
| `&&`     | Logical AND | `LAND`     |
| `\|\|`   | Logical OR  | `LOR`      |
| `!`      | Logical NOT | `NOT`      |

### 4.4 Assignments

```x3script
x = expr;       // Simple assignment
x += expr;      // Add-assign
x -= expr;      // Sub-assign
x *= expr;      // Mul-assign
x /= expr;      // Div-assign
x &= expr;      // And-assign
x |= expr;      // Or-assign
x ^= expr;      // Xor-assign
```

### 4.5 Function Calls

```x3script
// Direct call
spread = compute_spread(a, b);

// Chained calls
result = process(transform(input));

// Hostcall
data = evm.call(to: addr, value: 0, data: payload);
```

---

## 5. Cross-VM Hostcalls

The unified cross-VM interface — where the magic happens.

### 5.1 EVM Hostcalls

```x3script
// Generic EVM call
evm.call(
    to: address,
    value: u64,
    data: bytes
) -> bytes;

// Static call (read-only)
evm.staticcall(
    to: address,
    data: bytes
) -> bytes;

// Delegate call
evm.delegatecall(
    to: address,
    data: bytes
) -> bytes;

// Storage access
evm.sload(slot: u256) -> u256;
evm.sstore(slot: u256, value: u256);
```

### 5.2 SVM Hostcalls

```x3script
// Program invocation
svm.invoke(
    program: address,
    accounts: [address],
    data: bytes
) -> bytes;

// Signed invocation (with PDA seeds)
svm.invoke_signed(
    program: address,
    accounts: [address],
    data: bytes,
    seeds: [[byte]]
) -> bytes;

// Transfer SOL
svm.transfer(
    from: address,
    to: address,
    lamports: u64
);
```

### 5.3 System Hostcalls

```x3script
// Flashloan (cross-VM atomic)
system.flashloan(
    asset: address,
    amount: u64,
    callback: fn
);

// Context
system.caller() -> address;
system.origin() -> address;
system.block_number() -> u64;
system.timestamp() -> u64;
system.chain_id() -> u64;

// Gas
system.gas_left() -> u64;
system.gas_price() -> u64;
```

### 5.4 DEX Hostcalls

```x3script
// Price oracle
dex.price(token: address) -> u64;

// Swap exact input
dex.swap_exact_in(
    amount_in: u64,
    min_out: u64,
    path: [address]
) -> u64;

// Swap exact output
dex.swap_exact_out(
    amount_out: u64,
    max_in: u64,
    path: [address]
) -> u64;

// Liquidity
dex.add_liquidity(
    tokenA: address,
    tokenB: address,
    amountA: u64,
    amountB: u64
) -> u64;

dex.remove_liquidity(
    pair: address,
    liquidity: u64
) -> (u64, u64);
```

### 5.5 AI Hostcalls

Direct interface to the mutation engine and evaluator:

```x3script
// Mutate a function (off-chain, returns new variant ID)
ai.mutate(fn_id: u32) -> u32;

// Evaluate a strategy (returns fitness score)
ai.eval(strategy_id: u32) -> u64;

// Spawn child strategy from current
ai.spawn(child_strategy: fn) -> u32;

// Get current strategy's fitness
ai.fitness() -> u64;

// Report observation to memory
ai.observe(key: bytes, value: bytes);

// Query strategy memory
ai.recall(key: bytes) -> bytes;
```

---

## 6. Type System

### 6.1 Primitive Types

| Type      | Size        | Description                      |
| --------- | ----------- | -------------------------------- |
| `u8`      | 1 byte      | Unsigned 8-bit integer           |
| `u16`     | 2 bytes     | Unsigned 16-bit integer          |
| `u32`     | 4 bytes     | Unsigned 32-bit integer          |
| `u64`     | 8 bytes     | Unsigned 64-bit integer          |
| `u128`    | 16 bytes    | Unsigned 128-bit integer         |
| `i64`     | 8 bytes     | Signed 64-bit integer (optional) |
| `bool`    | 1 byte      | Boolean (true/false)             |
| `byte`    | 1 byte      | Single byte                      |
| `address` | 20/32 bytes | EVM (20) or SVM (32) address     |

### 6.2 Composite Types

```x3script
// Struct
struct Position {
    token: address;
    amount: u64;
    entry_price: u64;
}

// Tuple
let pair: (u64, u64) = (100, 200);
let (a, b) = pair;  // Destructuring

// Fixed array
let prices: array[u64; 8];
let buffer: array[byte; 256];

// Dynamic bytes
let data: bytes;
```

### 6.3 Storage Types

Storage types require explicit annotation:

```x3script
storage UserData {
    balance: u64;           // Primitive in storage
    positions: array[Position; 16];  // Struct array in storage
    metadata: bytes;        // Dynamic bytes in storage
}
```

### 6.4 Type Inference

Limited type inference for locals:

```x3script
let x = 100;           // Inferred as u64
let y = true;          // Inferred as bool
let z = compute();     // Inferred from return type
let w: u128 = 100;     // Explicit when needed
```

---

## 7. Built-In Libraries

These are foundational — not optional.

### 7.1 `math`

```x3script
math.min(a, b) -> T
math.max(a, b) -> T
math.abs(x) -> T
math.sqrt(x) -> u64
math.pow(base, exp) -> u64
math.clamp(x, min, max) -> T
```

### 7.2 `vec`

```x3script
vec.push(v, item)
vec.pop(v) -> T
vec.len(v) -> u64
vec.get(v, idx) -> T
vec.set(v, idx, item)
vec.clear(v)
```

### 7.3 `dex`

```x3script
dex.price(token) -> u64
dex.swap_exact_in(amount, min_out, path) -> u64
dex.swap_exact_out(amount, max_in, path) -> u64
dex.add_liquidity(tokenA, tokenB, amountA, amountB) -> u64
dex.remove_liquidity(pair, liquidity) -> (u64, u64)
dex.get_reserves(pair) -> (u64, u64)
```

### 7.4 `system`

```x3script
system.flashloan(asset, amount, callback)
system.caller() -> address
system.origin() -> address
system.block_number() -> u64
system.timestamp() -> u64
system.chain_id() -> u64
system.gas_left() -> u64
system.gas_price() -> u64
system.emit(event_name, data)
```

### 7.5 `evm`

```x3script
evm.call(to, value, data) -> bytes
evm.staticcall(to, data) -> bytes
evm.delegatecall(to, data) -> bytes
evm.sload(slot) -> u256
evm.sstore(slot, value)
evm.balance(addr) -> u256
evm.code(addr) -> bytes
```

### 7.6 `svm`

```x3script
svm.invoke(program, accounts, data) -> bytes
svm.invoke_signed(program, accounts, data, seeds) -> bytes
svm.transfer(from, to, lamports)
svm.get_account_data(account) -> bytes
svm.get_lamports(account) -> u64
```

### 7.7 `ai`

Links directly to the mutation engine and evaluator:

```x3script
ai.mutate(fn_id) -> u32
ai.eval(strategy_id) -> u64
ai.spawn(child_strategy) -> u32
ai.fitness() -> u64
ai.observe(key, value)
ai.recall(key) -> bytes
ai.generation() -> u32
ai.parent_id() -> u32
```

---

## 8. Determinism & AI Safety

To make mutation viable, X3Script enforces strict constraints:

### 8.1 Forbidden Constructs

| Construct           | Reason                       |
| ------------------- | ---------------------------- |
| Dynamic dispatch    | Non-deterministic resolution |
| Recursion           | Unbounded stack growth       |
| Unbounded loops     | Non-termination risk         |
| Dynamic allocation  | Memory non-determinism       |
| Floating point      | Cross-platform variance      |
| External randomness | Non-reproducible             |

### 8.2 Required Properties

| Property                | Enforcement                                |
| ----------------------- | ------------------------------------------ |
| **No dynamic dispatch** | All calls resolved at compile time         |
| **No recursion**        | Call graph must be acyclic                 |
| **Static loop bounds**  | All loops have compile-time max iterations |
| **Fixed heap regions**  | `heap` declarations have fixed max size    |
| **Bounded cost**        | Every function has computable gas estimate |

### 8.3 Static Analysis Passes

The compiler verifies:

1. **Termination**: All loops provably terminate
2. **Memory bounds**: No out-of-bounds access possible
3. **Gas bounds**: Total cost is computable
4. **Determinism**: No non-deterministic operations
5. **Purity**: Pure functions have no side effects

### 8.4 AI Mutation Safety

These constraints enable:

```
┌─────────────────────────────────────────────────────────────┐
│                    MUTATION SAFETY                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ✓ Any mutation produces valid program                     │
│  ✓ Gas cost always computable                              │
│  ✓ Execution always terminates                             │
│  ✓ Memory access always safe                               │
│  ✓ Results always reproducible                             │
│                                                             │
│  → Safe for millions of automated rewrites                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 9. Full Example Contract

The canonical "hello world" arbitrage bot:

```x3script
module ArbBot {

    // Persistent storage
    storage Vault {
        owner: address;
        profit: u64;
        total_trades: u64;
    }

    // Configuration constants
    const MIN_PROFIT_BPS: u64 = 20;  // 0.2% minimum profit
    const MAX_SLIPPAGE: u64 = 100;   // 1% max slippage

    // Pure function — determines if arbitrage is profitable
    fn compute_optimal(price_a: u64, price_b: u64) -> bool {
        if price_b <= price_a {
            return false;
        }
        let spread = price_b - price_a;
        let threshold = price_a * MIN_PROFIT_BPS / 10000;
        return spread > threshold;
    }

    // Calculate expected profit
    fn calc_profit(amount: u64, price_a: u64, price_b: u64) -> u64 {
        let value_a = amount;
        let value_b = amount * price_b / price_a;
        if value_b > value_a {
            return value_b - value_a;
        }
        return 0;
    }

    // External entrypoint — initiates arbitrage
    external fn execute(asset: address, amount: u64) {
        // Validate caller
        assert(system.caller() == Vault.owner, "unauthorized");
        assert(amount > 0, "invalid amount");

        // Initiate flashloan
        system.flashloan(asset, amount, callback);
    }

    // Flashloan callback — executes the arbitrage
    fn callback(asset: address, amount: u64) {
        // Get current prices
        let price_a = dex.price(tokenA);
        let price_b = dex.price(tokenB);

        // Check profitability
        if compute_optimal(price_a, price_b) {
            // Execute swap: tokenA -> tokenB
            let out = dex.swap_exact_in(
                amount,
                amount * (10000 - MAX_SLIPPAGE) / 10000,
                [tokenA, tokenB]
            );

            // Calculate and store profit
            let profit = out - amount;
            Vault.profit += profit;
            Vault.total_trades += 1;

            // Send profit to owner
            evm.call(
                to: Vault.owner,
                value: profit,
                data: []
            );

            // Emit event
            system.emit("ArbExecuted", (amount, profit, price_a, price_b));
        }

        // Repay flashloan (implicit — handled by system.flashloan)
    }

    // View function — check current status
    fn get_stats() -> (u64, u64) {
        return (Vault.profit, Vault.total_trades);
    }

    // Admin function — withdraw profits
    external fn withdraw(amount: u64) {
        assert(system.caller() == Vault.owner, "unauthorized");
        assert(amount <= Vault.profit, "insufficient balance");

        Vault.profit -= amount;

        evm.call(
            to: Vault.owner,
            value: amount,
            data: []
        );
    }
}
```

### Example Breakdown

| Section                | Purpose                    |
| ---------------------- | -------------------------- |
| `storage Vault`        | Persistent profit tracking |
| `const MIN_PROFIT_BPS` | Configuration constant     |
| `fn compute_optimal`   | Pure profitability check   |
| `external fn execute`  | Public entrypoint          |
| `fn callback`          | Flashloan execution logic  |
| `fn get_stats`         | Read-only view             |
| `external fn withdraw` | Admin withdrawal           |

---

## 10. Memory Model Integration

X3Script maps directly to the X3 VM's 4-tier memory model:

```
┌─────────────────────────────────────────────────────────────┐
│                    MEMORY HIERARCHY                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  REGISTER (r0-r31)     ←  let x: u64;                      │
│  ├── 32 general-purpose registers                          │
│  ├── Fastest access (1 cycle)                              │
│  └── Function-local lifetime                               │
│                                                             │
│  STACK (64KB)          ←  let buf[32]: byte;               │
│  ├── Fixed-size arrays                                     │
│  ├── Fast access (2 cycles)                                │
│  └── Function-local lifetime                               │
│                                                             │
│  HEAP (configurable)   ←  heap myvec[128];                 │
│  ├── Transaction-scoped allocation                         │
│  ├── Medium access (5 cycles)                              │
│  └── Freed at transaction end                              │
│                                                             │
│  GLOBAL (storage)      ←  storage Vault { ... }            │
│  ├── Persistent on-chain state                             │
│  ├── Expensive access (100+ cycles)                        │
│  └── Survives across transactions                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Declaration Mapping

```x3script
// Register allocation
let x: u64;              // → r[next_free]
let y: bool;             // → r[next_free]

// Stack allocation
let buffer[256]: byte;   // → stack[sp..sp+256]
let prices[8]: u64;      // → stack[sp..sp+64]

// Heap allocation
heap orders[1024];       // → heap[hp..hp+1024]

// Global storage
global Vault;            // → storage slot 0
global Config;           // → storage slot 1
```

---

## 11. Compilation Pipeline

X3Script flows through the standard X3 compilation pipeline:

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  X3Script   │───▶│    AST      │───▶│    HIR      │
│   Source    │    │   Parser    │    │   Lowering  │
└─────────────┘    └─────────────┘    └─────────────┘
                                            │
                                            ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Bytecode   │◀───│    MIR      │◀───│  Type Check │
│   (.x3bc)   │    │  Optimizer  │    │  & Verify   │
└─────────────┘    └─────────────┘    └─────────────┘
```

### Pipeline Stages

| Stage        | Input         | Output    | Key Operations                                   |
| ------------ | ------------- | --------- | ------------------------------------------------ |
| **Parse**    | `.x3s` source | AST       | Lexing, parsing, syntax validation               |
| **Lower**    | AST           | HIR       | Desugar, normalize, resolve names                |
| **Check**    | HIR           | Typed HIR | Type inference, borrow check, verify constraints |
| **Optimize** | HIR           | MIR       | 16-pass optimization pipeline                    |
| **Codegen**  | MIR           | Bytecode  | Register allocation, instruction selection       |

### Optimization Passes

The MIR optimizer applies these passes:

1. Constant folding
2. Dead code elimination
3. Common subexpression elimination
4. Loop invariant code motion
5. Strength reduction
6. Inlining
7. Tail call optimization
8. Branch optimization
9. Memory coalescing
10. Register allocation

---

## 12. Best Practices

### 12.1 Gas Optimization

```x3script
// ❌ BAD: Multiple storage reads
fn bad_example() {
    let a = Vault.balance;
    let b = Vault.balance;  // Redundant read
    return a + b;
}

// ✅ GOOD: Cache storage reads
fn good_example() {
    let balance = Vault.balance;
    return balance + balance;
}
```

### 12.2 Memory Efficiency

```x3script
// ❌ BAD: Large stack allocation in loop
fn bad_loop() {
    for i in 0..100 {
        let temp[1024]: byte;  // 100KB total
        process(temp);
    }
}

// ✅ GOOD: Reuse allocation
fn good_loop() {
    let temp[1024]: byte;  // 1KB once
    for i in 0..100 {
        clear(temp);
        process(temp);
    }
}
```

### 12.3 Safe External Calls

```x3script
// ❌ BAD: Unchecked external call
fn unsafe_call(target: address) {
    evm.call(to: target, value: 100, data: []);
}

// ✅ GOOD: Validate and handle result
fn safe_call(target: address) {
    assert(target != address(0), "invalid target");
    let result = evm.call(to: target, value: 100, data: []);
    assert(result.success, "call failed");
}
```

### 12.4 AI-Friendly Patterns

```x3script
// ✅ GOOD: Small, pure functions (mutation-friendly)
fn compute_fee(amount: u64, rate: u64) -> u64 {
    return amount * rate / 10000;
}

// ✅ GOOD: Clear decision boundaries
fn should_execute(price: u64, threshold: u64) -> bool {
    return price > threshold;
}

// ✅ GOOD: Parameterized logic (evolvable)
fn calc_score(a: u64, b: u64, weight: u64) -> u64 {
    return (a * weight + b * (100 - weight)) / 100;
}
```

---

## Appendix A: Grammar (EBNF)

```ebnf
program     := module_decl
module_decl := "module" IDENT "{" item* "}"

item        := storage_decl
            | const_decl
            | fn_decl
            | task_decl

storage_decl := "storage" IDENT "{" field* "}"
field        := IDENT ":" type ";"

const_decl  := "const" IDENT ":" type "=" expr ";"

fn_decl     := ("external" | "internal")? "fn" IDENT 
               "(" params? ")" ("->" type)? block
task_decl   := "task" IDENT ("@" trigger)? block
trigger     := "interval" "(" NUMBER ")"
            | "trigger" "(" IDENT ")"

params      := param ("," param)*
param       := IDENT ":" type

block       := "{" stmt* "}"
stmt        := let_stmt | assign_stmt | if_stmt | while_stmt
            | for_stmt | return_stmt | assert_stmt | expr_stmt

let_stmt    := "let" IDENT ("[" NUMBER "]")? (":" type)? ("=" expr)? ";"
assign_stmt := lvalue assign_op expr ";"
if_stmt     := "if" expr block ("elif" expr block)* ("else" block)?
while_stmt  := "while" expr block
for_stmt    := "for" IDENT "in" range block
return_stmt := "return" expr? ";"
assert_stmt := "assert" "(" expr "," STRING ")" ";"
expr_stmt   := expr ";"

type        := "u8" | "u16" | "u32" | "u64" | "u128"
            | "i64" | "bool" | "byte" | "address" | "bytes"
            | "array" "[" type ";" NUMBER "]"
            | IDENT
```

---

## Appendix B: Quick Reference

```
┌─────────────────────────────────────────────────────────────┐
│                 X3SCRIPT QUICK REFERENCE                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  STRUCTURE                                                  │
│  module Name { storage, const, fn, task }                  │
│                                                             │
│  MEMORY                                                     │
│  let x: u64;           // register                         │
│  let buf[N]: byte;     // stack                            │
│  heap vec[N];          // heap                             │
│  storage S { }         // global                           │
│                                                             │
│  CONTROL FLOW                                               │
│  if/elif/else, while, for i in 0..N, break, continue       │
│                                                             │
│  HOSTCALLS                                                  │
│  evm.call(to, value, data)                                 │
│  svm.invoke(program, accounts, data)                       │
│  system.flashloan(asset, amount, callback)                 │
│  dex.swap_exact_in(amount, min_out, path)                  │
│  ai.mutate(fn_id), ai.eval(strategy_id)                    │
│                                                             │
│  TYPES                                                      │
│  u8, u16, u32, u64, u128, i64, bool, byte, address, bytes  │
│  struct { }, (tuple), array[T; N]                          │
│                                                             │
│  AI SAFETY RULES                                            │
│  ✗ No recursion                                            │
│  ✗ No dynamic dispatch                                     │
│  ✗ No unbounded loops                                      │
│  ✓ Static bounds everywhere                                │
│  ✓ Deterministic execution                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

**Document Version:** 0.9.0 (Draft)  
**Specification Status:** Canonical Draft  
**Maintainer:** X3 Chain Core Engineering
