# X3 Language Reference + Example + Glossary

> **Status**: Canonical | **Version**: 1.0.0 | **Last Updated**: 2025-12-10

A single, self-contained reference for X3: the language, its key phrases, types, runtime primitives, atomic constructs, and a full example contract. After the example, every word/phrase is explained so you can read, write, and reason about X3 code with no mystery.

---

## Table of Contents

1. [Quick Overview](#1-quick-overview)
2. [Full X3 Example: Atomic Swap](#2-full-x3-example-atomic-swap)
3. [Grammar Sketch](#3-grammar-sketch)
4. [Keywords and Phrases Glossary](#4-keywords-and-phrases-glossary)
5. [Line-by-Line Semantics](#5-line-by-line-semantics)
6. [VM/Bytecode Correspondence](#6-vmbytecode-correspondence)
7. [Error Model and Safety Rules](#7-error-model-and-safety-rules)
8. [Standard Library Primitives](#8-standard-library-primitives)
9. [Best Practices](#9-best-practices)
10. [How to Write X3 Contracts](#10-how-to-write-x3-contracts)
11. [Annotated Example: claim()](#11-annotated-example-claim)
12. [FAQ](#12-faq)

---

## 1. Quick Overview

**X3** is a deterministic, register-oriented high-level language for writing on-chain logic and off-chain strategy code that compiles to X3 bytecode. It emphasizes:

- **Explicit gas accounting**
- **Atomic windows** (all-or-nothing execution)
- **Cross-VM hostcalls** (EVM/SVM interop)
- **Sandboxable sidecar execution** (AI agent simulation)

---

## 2. Full X3 Example: Atomic Swap

**File:** `atomic_swap.x3`

```x3
// atomic_swap.x3 — atomic token swap between two chains (EVM & SVM style hostcalls)
// Demonstrates: contract, storage, function, atomic block, hostcalls, error handling.

contract AtomicSwap {
    // Storage layout (persisted on-chain)
    storage {
        owner: address,          // account that created the swap
        token_a: address,        // token on EVM side
        token_b: pubkey,         // token/account on SVM side
        amount_a: u128,          // amount to transfer from A
        amount_b: u64,           // amount to transfer from B
        deadline: u64,           // unix timestamp
        claimed: bool            // swap already completed
    }

    // Events
    event SwapCreated(swap_id: bytes32, owner: address, deadline: u64)
    event SwapCompleted(swap_id: bytes32, owner: address)
    event SwapCancelled(swap_id: bytes32)

    // Constructor-like initializer
    pub fn init(owner: address, token_a: address, token_b: pubkey,
               amount_a: u128, amount_b: u64, deadline: u64) -> bytes32 {
        let id = hash_bytes(concat(owner, token_a, token_b, amount_a, amount_b, deadline));
        storage.owner = owner;
        storage.token_a = token_a;
        storage.token_b = token_b;
        storage.amount_a = amount_a;
        storage.amount_b = amount_b;
        storage.deadline = deadline;
        storage.claimed = false;

        emit SwapCreated(id, owner, deadline);
        return id;
    }

    // Claim: performs an atomic cross-VM swap
    pub fn claim(swap_id: bytes32, receiver_evm: address, receiver_svm: pubkey) -> bool {
        // Basic checks
        assert(!storage.claimed, "swap already claimed");
        assert(ctx_timestamp() <= storage.deadline, "swap expired");

        // Compute gas hints
        gas_charge(5000); // hint — consume budget for cross-VM work

        // Begin atomic window: either both hostcalls succeed or we rollback
        atomic {
            // EVM: transfer token_a from owner -> receiver_evm
            let evm_result = evm_call(
                gas: gas_left(),
                addr: storage.token_a,
                value: 0u128,
                data: encode_transfer(storage.owner, receiver_evm, storage.amount_a)
            );
            assert(evm_result.success, "evm transfer failed");

            // SVM: transfer token_b from owner -> receiver_svm
            let svm_result = svm_invoke_signed(
                dst: 0u32,
                program: storage.token_b,
                accounts: create_account_slice(storage.owner, receiver_svm),
                data: encode_transfer_svm(storage.amount_b),
                seeds: default_seeds()
            );
            assert(svm_result.success, "svm transfer failed");

            // Mark claimed
            storage.claimed = true;

            emit SwapCompleted(swap_id, storage.owner);
            // Commit atomic block implicitly if no panic/assert triggered
        } // atomic end

        return true;
    }

    // Cancel: allow owner to cancel after deadline
    pub fn cancel(swap_id: bytes32) -> bool {
        assert(ctx_sender() == storage.owner, "only owner can cancel");
        assert(ctx_timestamp() > storage.deadline, "too early to cancel");
        assert(!storage.claimed, "already claimed");

        storage.claimed = true;
        emit SwapCancelled(swap_id);
        return true;
    }
}

// --- helper builtins & pseudo-stdlib (available as intrinsics)
fn encode_transfer(from: address, to: address, amount: u128) -> bytes {
    // ABI-encode for the EVM token contract (example)
    abi_encode(["transferFrom", from, to, amount])
}

fn encode_transfer_svm(amount: u64) -> bytes {
    // SVM program-specific encoding
    abi_encode(["transfer", amount])
}
```

---

## 3. Grammar Sketch

A concise, usable grammar for parser implementation (Pratt/LL(k) compatible):

```ebnf
program        := (item* EOF)
item           := contract_decl | fn_decl | const_decl | event_decl

contract_decl  := "contract" IDENT "{" contract_body "}"
contract_body  := (storage_decl | event_decl | fn_decl)*

storage_decl   := "storage" "{" storage_field* "}"
storage_field  := IDENT ":" type ("," | "")

event_decl     := "event" IDENT "(" param_list? ")"
const_decl     := "const" IDENT ":" type "=" expr ";"

fn_decl        := ("pub" | "priv")? "fn" IDENT "(" param_list? ")" ("->" type)? block
param_list     := param ("," param)*
param          := IDENT ":" type

block          := "{" stmt* "}"

stmt           := let_stmt 
               | expr_stmt 
               | if_stmt 
               | while_stmt 
               | for_stmt 
               | return_stmt 
               | atomic_stmt 
               | emit_stmt 
               | assert_stmt

let_stmt       := "let" ("mut")? IDENT (":" type)? ("=" expr)? ";"
if_stmt        := "if" "(" expr ")" block ("else" (block | if_stmt))?
while_stmt     := "while" "(" expr ")" block
for_stmt       := "for" "(" init? ";" cond? ";" update? ")" block
return_stmt    := "return" expr? ";"
atomic_stmt    := "atomic" block
emit_stmt      := "emit" IDENT "(" arg_list? ")" ";"
assert_stmt    := "assert" "(" expr "," STRING ")" ";"

expr           := assignment
assignment     := logical_or (("=" assignment))?
logical_or     := logical_and ( "||" logical_and )*
logical_and    := equality ( "&&" equality )*
equality       := comparison (("==" | "!=") comparison)*
comparison     := additive (("<" | ">" | "<=" | ">=") additive)*
additive       := multiplicative (("+" | "-") multiplicative)*
multiplicative := unary (("*" | "/" | "%") unary)*
unary          := ("!" | "-" )* primary
primary        := IDENT 
               | LITERAL 
               | "(" expr ")" 
               | fn_call 
               | member_access 
               | array_literal

fn_call        := IDENT "(" arg_list? ")"
member_access  := expr "." IDENT

type           := "u8" | "u16" | "u32" | "u64" | "u128" 
               | "i64" | "bool" 
               | "address" | "pubkey" 
               | "bytes" | "bytes32" 
               | IDENT
```

---

## 4. Keywords and Phrases Glossary

### 4.1 Language-Level / Syntax Keywords

| Keyword       | Meaning                                                                                                                                                |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `contract`    | Declares a named module that groups storage, events, and functions; compiles to a module with persistent storage (like a smart contract)               |
| `storage`     | Block inside a contract that declares persistent variables stored on-chain (durable state). Assignments map to storage ops (expensive, side-effecting) |
| `pub`         | Function visibility: publicly/externally callable                                                                                                      |
| `priv`        | Function visibility: internal-only (default if omitted)                                                                                                |
| `fn`          | Function declaration                                                                                                                                   |
| `let`         | Declare a local variable. `let mut` makes it mutable. Without `mut` it is immutable                                                                    |
| `mut`         | Mutability modifier for variables                                                                                                                      |
| `return`      | Return a value from the function immediately                                                                                                           |
| `if` / `else` | Conditional branching                                                                                                                                  |
| `while`       | While loop                                                                                                                                             |
| `for`         | For loop (C-style and range-based forms)                                                                                                               |
| `atomic`      | Atomic block: all side-effects inside are tentatively applied; on success commits, on assertion/panic rolls back                                       |
| `emit`        | Emits a contract event (log); maps to event/emission opcodes                                                                                           |
| `assert`      | Check condition; on false panics and triggers rollback if in atomic window                                                                             |
| `event`       | Declares an event signature for `emit`                                                                                                                 |
| `const`       | Compile-time constant value                                                                                                                            |
| `//`          | Single-line comment                                                                                                                                    |
| `/* */`       | Multi-line comment                                                                                                                                     |

### 4.2 Types

| Type      | Description                                          |
| --------- | ---------------------------------------------------- |
| `u8`      | Unsigned 8-bit integer                               |
| `u16`     | Unsigned 16-bit integer                              |
| `u32`     | Unsigned 32-bit integer                              |
| `u64`     | Unsigned 64-bit integer                              |
| `u128`    | Unsigned 128-bit integer                             |
| `i64`     | Signed 64-bit integer                                |
| `bool`    | Boolean type (`true` / `false`)                      |
| `address` | Canonical EVM-style address (20 bytes)               |
| `pubkey`  | Canonical SVM-style public-key identifier (32 bytes) |
| `bytes`   | Variable-length byte array                           |
| `bytes32` | 32-byte fixed-size hash/ID type                      |
| `-> type` | Function return type annotation                      |

### 4.3 Runtime/Context Builtins

| Builtin           | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `ctx_sender()`    | Returns the transaction/call sender (address/pubkey) |
| `ctx_timestamp()` | Returns current block timestamp (u64)                |
| `ctx_value()`     | Value attached to execution (native-token transfer)  |
| `ctx_chain_id()`  | Returns chain identifier (for cross-chain awareness) |
| `gas_left()`      | Remaining gas in current execution context           |
| `gas_charge(n)`   | Charge or reserve gas for heavy operations (VM hint) |

### 4.4 Cross-VM Hostcalls / Intrinsics

| Hostcall                                                 | Description                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `evm_call(gas, addr, value, data)`                       | Hostcall to EVM: makes a call to an EVM contract; returns result with `success` and `return_data` |
| `svm_invoke_signed(dst, program, accounts, data, seeds)` | Hostcall to SVM (Solana-style program invocation) with signed seeds; returns result object        |
| `svm_invoke`                                             | Basic SVM program invocation                                                                      |
| `svm_transfer`                                           | SVM token transfer                                                                                |
| `evm_sstore`                                             | Low-level EVM storage write                                                                       |
| `evm_sload`                                              | Low-level EVM storage read                                                                        |

### 4.5 Stdlib / Helpers

| Function                    | Description                              |
| --------------------------- | ---------------------------------------- |
| `hash_bytes(bytes)`         | Cryptographic hash → bytes32             |
| `concat(...)`               | Deterministic concatenation for hashing  |
| `abi_encode(args...)`       | Produce call data for external contracts |
| `encode_transfer(...)`      | Helper for token transfer calldata       |
| `create_account_slice(...)` | Build account arrays for SVM             |
| `default_seeds()`           | Default seed array for signed invokes    |

### 4.6 VM/Bytecode Concepts

| Term              | Definition                                                                  |
| ----------------- | --------------------------------------------------------------------------- |
| **gas model**     | Cost schedule mapping opcodes/hostcalls to gas amounts                      |
| **constant pool** | Compiled module table for immediates (strings, ints, bytes)                 |
| **atomic window** | Transaction boundary with snapshot/rollback behavior                        |
| **sidecar**       | Off-chain execution environment that simulates strategies before deployment |
| **receipt**       | Cryptographically signed artifact proving strategy execution results        |

---

## 5. Line-by-Line Semantics

Detailed explanation of every construct in the AtomicSwap example:

### Contract Declaration
```x3
contract AtomicSwap { ... }
```
Defines a contract module named `AtomicSwap`. Compiles to code with storage and callable functions.

### Storage Block
```x3
storage {
    owner: address,
    token_a: address,
    token_b: pubkey,
    amount_a: u128,
    amount_b: u64,
    deadline: u64,
    claimed: bool
}
```
Persistent storage block; each field becomes a storage slot:
- `owner` — Account that controls the swap
- `token_a` — EVM token contract address
- `token_b` — SVM program/account identifier
- `amount_a` — Amount on token A side (128-bit)
- `amount_b` — Amount on token B side (64-bit)
- `deadline` — Unix timestamp after which swap is invalid
- `claimed` — Marker whether swap completed

### Event Declaration
```x3
event SwapCreated(swap_id: bytes32, owner: address, deadline: u64)
```
Event signature declared for later `emit`. Events are recorded in logs and receipts.

### Initializer Function
```x3
pub fn init(...) -> bytes32 { ... }
```
Publicly callable initializer function; returns an ID (32-byte hash).

### Variable Declaration
```x3
let id = hash_bytes(concat(owner, token_a, token_b, amount_a, amount_b, deadline));
```
Compute deterministic ID via hashing concatenated fields.

### Storage Write
```x3
storage.owner = owner;
```
Write to persistent storage (side-effecting operation, expensive).

### Event Emission
```x3
emit SwapCreated(id, owner, deadline);
```
Emit an event (logs to blockchain).

### Return Statement
```x3
return id;
```
Return the computed id from the function.

### Assert Statement
```x3
assert(!storage.claimed, "swap already claimed");
```
Fail if already claimed. `assert` triggers rollback if in atomic block.

### Gas Charge
```x3
gas_charge(5000);
```
Deduct or reserve gas budget for subsequent operations; hint to verifier/VM.

### Atomic Block
```x3
atomic {
    // ... operations ...
}
```
Open atomic window: VM snapshots state; at end commits if OK, else rollbacks.

### EVM Hostcall
```x3
let evm_result = evm_call(
    gas: gas_left(),
    addr: storage.token_a,
    value: 0u128,
    data: encode_transfer(storage.owner, receiver_evm, storage.amount_a)
);
```
Perform EVM hostcall (token transfer). Returns a result object to inspect.

### SVM Hostcall
```x3
let svm_result = svm_invoke_signed(
    dst: 0u32,
    program: storage.token_b,
    accounts: create_account_slice(storage.owner, receiver_svm),
    data: encode_transfer_svm(storage.amount_b),
    seeds: default_seeds()
);
```
Perform SVM-style hostcall transferring or invoking SVM program.

---

## 6. VM/Bytecode Correspondence

When compiling, the compiler maps high-level constructs to bytecode:

| X3 Construct              | Bytecode Mapping                                        |
| ------------------------- | ------------------------------------------------------- |
| `storage.<field> = value` | `GSTOR slot, reg` (store_global)                        |
| `let v = expr;`           | `MOV`, `LOAD_CONST`, compute ops                        |
| `if (cond) ... else ...`  | `JZ`/`JNZ` with branch labels                           |
| `atomic { ... }`          | `ATOMIC_BEGIN` ... `ATOMIC_COMMIT` or `ATOMIC_ROLLBACK` |
| `evm_call(...)`           | `EVM_CALL` intrinsic opcode (0xB0 range)                |
| `svm_invoke_signed(...)`  | `SVM_INVOKE_SIGNED` opcode (0xC1)                       |
| `emit Event(...)`         | `EMIT` opcode                                           |
| `assert(expr, msg)`       | `ASSERT` opcode (traps on false)                        |

### Gas Accounting

```
gas_est = Σ(per-opcode costs) + Σ(hostcall costs)
```

- Compiler produces `gas_est` metadata
- Verifier checks for potential undercharging
- Sidecars use same gas model for accurate simulation

---

## 7. Error Model and Safety Rules

### 7.1 Compile-Time Errors

| Error Type              | Cause                                           |
| ----------------------- | ----------------------------------------------- |
| Type mismatch           | Incompatible types in assignment/operation      |
| Undefined symbol        | Reference to undeclared variable/function       |
| Duplicate storage field | Same field name declared twice                  |
| Invalid mutation        | Assigning to immutable variable                 |
| Nested atomic           | Disallowed nested atomic blocks (design choice) |

### 7.2 Verifier Errors

| Error Type              | Cause                                          |
| ----------------------- | ---------------------------------------------- |
| `gas_overflow`          | Estimated gas exceeds limits                   |
| `forbidden_intrinsic`   | Disallowed opcode usage                        |
| `missing_atomic_commit` | Atomic block without proper termination        |
| `non_deterministic_op`  | Non-deterministic operation (e.g., `random()`) |

### 7.3 Runtime Errors (Trap)

| Error Type       | Cause                                          |
| ---------------- | ---------------------------------------------- |
| Assert failure   | `assert` condition evaluated to false          |
| Division by zero | Division operation with zero divisor           |
| Out of gas       | Execution exceeded gas limit                   |
| Failed hostcall  | Cross-VM call failed without handling          |
| Overflow         | Integer overflow (if trap-on-overflow enabled) |

### 7.4 Atomic Semantics

- Hostcalls inside `atomic` have side effects held until commit
- Some hostcalls may be prohibited if they have irreversible external effects
- Verifier enforces atomic window rules

---

## 8. Standard Library Primitives

Implementations are compiler stdlib intrinsics or mapped hostcalls:

### Cryptographic

```x3
fn hash_bytes(data: bytes) -> bytes32;     // blake3/blake2/sha256
fn keccak256(data: bytes) -> bytes32;      // Ethereum-compatible hash
fn sha256(data: bytes) -> bytes32;         // SHA-256
```

### Encoding

```x3
fn abi_encode(args: ...) -> bytes;         // ABI encoding for calls
fn abi_decode<T>(data: bytes) -> T;        // ABI decoding
fn concat(parts: ...) -> bytes;            // Deterministic concatenation
```

### Token Operations

```x3
fn encode_transfer(from: address, to: address, amount: u128) -> bytes;
fn encode_transfer_svm(amount: u64) -> bytes;
```

### SVM Helpers

```x3
fn create_account_slice(accounts: ...) -> AccountSlice;
fn default_seeds() -> SeedArray;
```

### Gas Utilities

```x3
fn gas_left() -> u64;                      // Remaining gas
fn gas_charge(amount: u64);                // Reserve/consume gas
```

### Math

```x3
fn min<T>(a: T, b: T) -> T;
fn max<T>(a: T, b: T) -> T;
fn abs(x: i64) -> u64;
fn sqrt(x: u64) -> u64;
```

---

## 9. Best Practices

### Storage

✅ **DO:** Keep storage writes minimal (expensive)  
✅ **DO:** Do checks off-chain or in pure computation before store writes  
❌ **DON'T:** Write to storage in loops unnecessarily

### Atomic Blocks

✅ **DO:** Put heavy cross-VM hostcalls inside `atomic {}` only if rollback-safe  
✅ **DO:** Keep atomic blocks as small as possible  
❌ **DON'T:** Nest atomic blocks  
❌ **DON'T:** Include non-idempotent external calls

### Gas

✅ **DO:** Provide gas hints via `gas_charge`  
✅ **DO:** Let verifier check gas estimates  
✅ **DO:** Test gas consumption in sidecar simulations

### Determinism

✅ **DO:** Use deterministic primitives only  
✅ **DO:** Use fixed-point for financial calculations  
❌ **DON'T:** Use floating point inside contracts  
❌ **DON'T:** Rely on external randomness

### Events

✅ **DO:** Use `emit` events for all material state transitions  
✅ **DO:** Include relevant indexed fields for filtering  
✅ **DO:** Use events to construct signed receipts

---

## 10. How to Write X3 Contracts

### Checklist

1. **Design storage** — Keep it minimal
2. **Add events** — For all material state transitions
3. **Write `init`** — Set initial state and emit creation event
4. **Write core functions** — Minimal storage writes
5. **Wrap cross-VM sequences** — Use `atomic {}`
6. **Add `assert` checks** — Include human-friendly messages
7. **Unit test** — Run on local X3 VM with simulated hostcalls
8. **Add metadata** — ABI JSON and gas estimates
9. **Sidecar config** — For swarm testing (mutations, seeds, scoring)
10. **Produce receipt** — Signed artifact after full test run

### Template

```x3
contract MyContract {
    storage {
        // Minimal persistent state
        owner: address,
        value: u128
    }

    event ValueUpdated(old_value: u128, new_value: u128)

    pub fn init(owner: address) -> bool {
        storage.owner = owner;
        storage.value = 0u128;
        return true;
    }

    pub fn update_value(new_value: u128) -> bool {
        assert(ctx_sender() == storage.owner, "only owner");
        
        let old = storage.value;
        storage.value = new_value;
        
        emit ValueUpdated(old, new_value);
        return true;
    }
}
```

---

## 11. Annotated Example: claim()

```x3
pub fn claim(swap_id: bytes32, receiver_evm: address, receiver_svm: pubkey) -> bool {
    // Check invariant; traps and rolls back if true
    assert(!storage.claimed, "swap already claimed");
    
    // Time check against block timestamp
    assert(ctx_timestamp() <= storage.deadline, "swap expired");
    
    // Gas hint/reservation for cross-VM work
    gas_charge(5000);
    
    // Begin atomic snapshot — all changes tentative
    atomic {
        // EVM hostcall: transfer tokens on EVM side
        let evm_result = evm_call(
            gas: gas_left(),           // Pass remaining gas
            addr: storage.token_a,     // Target EVM contract
            value: 0u128,              // No native value transfer
            data: encode_transfer(     // ABI-encoded calldata
                storage.owner, 
                receiver_evm, 
                storage.amount_a
            )
        );
        // Check EVM call succeeded
        assert(evm_result.success, "evm transfer failed");
        
        // SVM hostcall: transfer tokens on SVM side
        let svm_result = svm_invoke_signed(
            dst: 0u32,                                    // Destination slot
            program: storage.token_b,                     // SVM program ID
            accounts: create_account_slice(               // Account array
                storage.owner, 
                receiver_svm
            ),
            data: encode_transfer_svm(storage.amount_b),  // Instruction data
            seeds: default_seeds()                        // Signer seeds
        );
        // Check SVM call succeeded
        assert(svm_result.success, "svm transfer failed");
        
        // Persistent write — commits on atomic success
        storage.claimed = true;
        
        // Log event for receipts/indexing
        emit SwapCompleted(swap_id, storage.owner);
        
        // Implicit commit if no trap occurred
    }
    
    return true;
}
```

### Key Points

| Line                     | What It Does                                                |
| ------------------------ | ----------------------------------------------------------- |
| `gas_left()`             | Returns remaining gas; safe to pass to `evm_call`           |
| `evm_call(...)`          | Attempts EVM contract call; returns result with `.success`  |
| `svm_invoke_signed(...)` | SVM signed invocation                                       |
| `storage.claimed = true` | Mutate persisted contract state (commits on atomic success) |
| `emit`                   | Create blockchain-visible log entry                         |

---

## 12. FAQ

### Q: Do I still need WASM for anything?

**A:** Not for X3 contract execution. X3 is its own bytecode VM. WASM may still be used by your chain runtime (Substrate pallets), tooling, or SDK integrations — but the X3 execution path is separate.

### Q: How do strategies & AI tie into this?

**A:** Strategies run off-chain in sidecars / GPU swarm. They mutate X3 (or MIR) candidates, run millions of sandboxed simulations via the VM, produce receipts, and then you optionally on-chain-deploy the top candidates. The language/bytecode must remain deterministic to verify receipts.

### Q: Where does gas accounting live?

**A:** The VM enforces gas per-opcode and hostcall costs. Compiler produces `gas_est` metadata and the verifier checks for potential undercharging. Sidecars must use same gas model to simulate accurately.

### Q: Can I use floating point?

**A:** Floating point is only available in **simulation mode** (off-chain). On-chain execution must be fully deterministic, so use fixed-point arithmetic for financial calculations.

### Q: What happens if an atomic block fails?

**A:** All state changes within the atomic block are rolled back. The transaction may still succeed (returning false) or may trap entirely depending on how errors are handled outside the atomic block.

### Q: How do cross-VM calls work under the hood?

**A:** The X3 VM suspends execution, serializes the call via the Unified ABI, dispatches to the target VM (EVM/SVM), awaits the result, deserializes the response, and resumes X3 execution. All within the same atomic transaction context.

### Q: Can I deploy Solidity contracts and call them from X3?

**A:** Yes. Deploy Solidity contracts to the EVM layer, then use `evm_call` from X3 to interact with them. The cross-VM bridge handles ABI encoding/decoding.

### Q: What's the maximum size for a contract?

**A:** Contract bytecode is limited by block constraints. Typical limits:
- Bytecode: 64KB max
- Storage slots: Unlimited (but gas-metered)
- Call depth: 64 max

---

## Appendix A: Quick Reference Card

```
┌─────────────────────────────────────────────────────────────────────┐
│                     X3 QUICK REFERENCE                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  TYPES                                                              │
│  ─────                                                              │
│  u8, u16, u32, u64, u128    Unsigned integers                      │
│  i64                         Signed integer                         │
│  bool                        Boolean                                │
│  address                     EVM address (20 bytes)                 │
│  pubkey                      SVM pubkey (32 bytes)                  │
│  bytes                       Dynamic byte array                     │
│  bytes32                     Fixed 32-byte array                    │
│                                                                     │
│  KEYWORDS                                                           │
│  ────────                                                           │
│  contract  storage  fn  let  mut  return  if  else                 │
│  while  for  atomic  emit  assert  event  const  pub               │
│                                                                     │
│  CONTEXT BUILTINS                                                   │
│  ────────────────                                                   │
│  ctx_sender()      Transaction sender                               │
│  ctx_timestamp()   Block timestamp                                  │
│  ctx_value()       Attached value                                   │
│  gas_left()        Remaining gas                                    │
│  gas_charge(n)     Reserve gas                                      │
│                                                                     │
│  HOSTCALLS                                                          │
│  ─────────                                                          │
│  evm_call(gas, addr, value, data)                                  │
│  svm_invoke_signed(dst, program, accounts, data, seeds)            │
│                                                                     │
│  STDLIB                                                             │
│  ──────                                                             │
│  hash_bytes()  concat()  abi_encode()  abi_decode()                │
│  min()  max()  abs()  sqrt()                                       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

**Document Version:** 1.0.0  
**Specification Status:** Canonical  
**Maintainer:** X3 Chain Core Engineering
