# X3 Language Specification v1.0

> **Status**: Canonical | **Version**: 1.0.0 | **Last Updated**: 2025-12-10

The official definition of the X3 source language, bytecode model, semantics, and compilation rules for the X3 Chain blockchain.

---

## Table of Contents

1. [Language Philosophy](#1-language-philosophy)
2. [Lexical Structure](#2-lexical-structure)
3. [Type System](#3-type-system)
4. [Memory Model](#4-memory-model)
5. [Control Flow](#5-control-flow)
6. [Functions](#6-functions)
7. [Built-in Operations](#7-built-in-operations)
8. [Chain Intrinsics](#8-chain-intrinsics)
9. [Bytecode Model](#9-bytecode-model)
10. [Compilation Pipeline](#10-compilation-pipeline)
11. [Formal Semantics](#11-formal-semantics)

---

## 1. Language Philosophy

X3 is the native programming language for the X3 Chain blockchain. It is designed with the following core principles:

### 1.1 Design Goals

| Goal                      | Description                                                      |
| ------------------------- | ---------------------------------------------------------------- |
| **Low-Level Control**     | Direct control over memory, stack, and execution flow            |
| **High-Level Ergonomics** | Rust-like syntax that feels familiar to systems programmers      |
| **Full Determinism**      | Every operation produces identical results across all nodes      |
| **Optimizer-First**       | 16-pass optimization pipeline built into the language design     |
| **AI-Native**             | Designed for AI-generated code with clear, unambiguous semantics |

### 1.2 Non-Goals

- Garbage collection (explicit memory management only)
- Dynamic typing (static types enforce determinism)
- Exceptions (explicit error handling via Result types)
- Floating-point in consensus (f64 only in simulation mode)

### 1.3 Influence Map

```
Rust ──────┬──► Ownership semantics (simplified)
           │
Move ──────┼──► Resource types, linear logic
           │
LLVM IR ───┼──► SSA form, optimization passes
           │
Solidity ──┴──► Chain intrinsics, storage model
```

---

## 2. Lexical Structure

### 2.1 Character Set

X3 source files are UTF-8 encoded. Identifiers use ASCII alphanumerics plus underscore.

```ebnf
identifier     = (letter | '_') (letter | digit | '_')*
letter         = 'a'..'z' | 'A'..'Z'
digit          = '0'..'9'
```

### 2.2 Keywords

```
fn       let      const    mut      return   if       else     
match    loop     while    for      break    continue struct   
enum     impl     trait    pub      mod      use      as       
type     where    global   heap     stack    alloc    free
true     false    self     Self     warp     flash    swap
```

### 2.3 Literals

```x3
// Integers
let a: i64 = 42;
let b: u64 = 0xFF;           // Hexadecimal
let c: u64 = 0b1010_1100;    // Binary with separators
let d: u64 = 1_000_000;      // Decimal with separators

// Booleans
let flag: bool = true;

// Bytes
let single: byte = 0xAB;
let data: bytes = 0xDEADBEEF;

// Addresses (32-byte)
let addr: address = 0x5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY;

// Strings (compile-time only, stored as bytes)
let msg: bytes = "Hello, X3";
```

### 2.4 Comments

```x3
// Single-line comment

/* Multi-line
   comment */

/// Documentation comment (for functions, structs)
fn documented_function() { }

//! Module-level documentation
```

---

## 3. Type System

### 3.1 Primitive Types

| Type      | Size     | Range             | Description              |
| --------- | -------- | ----------------- | ------------------------ |
| `i8`      | 1 byte   | -128 to 127       | Signed 8-bit integer     |
| `i16`     | 2 bytes  | -32,768 to 32,767 | Signed 16-bit integer    |
| `i32`     | 4 bytes  | -2³¹ to 2³¹-1     | Signed 32-bit integer    |
| `i64`     | 8 bytes  | -2⁶³ to 2⁶³-1     | Signed 64-bit integer    |
| `u8`      | 1 byte   | 0 to 255          | Unsigned 8-bit integer   |
| `u16`     | 2 bytes  | 0 to 65,535       | Unsigned 16-bit integer  |
| `u32`     | 4 bytes  | 0 to 2³²-1        | Unsigned 32-bit integer  |
| `u64`     | 8 bytes  | 0 to 2⁶⁴-1        | Unsigned 64-bit integer  |
| `u128`    | 16 bytes | 0 to 2¹²⁸-1       | Unsigned 128-bit integer |
| `bool`    | 1 byte   | true/false        | Boolean                  |
| `byte`    | 1 byte   | 0 to 255          | Alias for u8             |
| `address` | 32 bytes | —                 | Account/contract address |

### 3.2 Composite Types

#### Structs

```x3
struct Position {
    x: i64,
    y: i64,
    z: i64,
}

let pos = Position { x: 10, y: 20, z: 30 };
let x_coord = pos.x;
```

#### Arrays

```x3
// Fixed-size arrays
let arr: [u64; 5] = [1, 2, 3, 4, 5];
let first = arr[0];

// Multi-dimensional
let matrix: [[i32; 3]; 3] = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
];
```

#### Tuples

```x3
let pair: (u64, bool) = (42, true);
let (value, flag) = pair;  // Destructuring
```

#### Bytes (Dynamic)

```x3
let data: bytes = 0xDEADBEEF;
let len = data.len();
let first_byte = data[0];
```

### 3.3 Storage Types

Storage qualifiers define where data lives:

```x3
// Stack-allocated (default, function-local)
let x: i64 = 42;
stack<i64> y = 100;

// Heap-allocated (manual lifetime)
let ptr: heap<Position> = alloc<Position>();
store(ptr, Position { x: 1, y: 2, z: 3 });
let pos = load(ptr);
free(ptr);

// Global storage (persistent on-chain)
global counter: u64;
global balances: Map<address, u64>;
```

### 3.4 Type Aliases

```x3
type Balance = u128;
type TokenId = [u8; 32];
type Callback = fn(u64) -> bool;
```

### 3.5 Generics

```x3
fn swap_values<T>(a: &mut T, b: &mut T) {
    let temp = *a;
    *a = *b;
    *b = temp;
}

struct Pair<A, B> {
    first: A,
    second: B,
}
```

### 3.6 Type Inference

The compiler infers types when unambiguous:

```x3
let x = 42;           // Inferred as i64 (default integer)
let y = 42u128;       // Explicit u128
let z = add(x, 10);   // Inferred from add() return type
```

---

## 4. Memory Model

X3 has four distinct memory domains, matching the MIR representation:

### 4.1 Memory Domain Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     X3 Memory Model                         │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│  REGISTER   │    STACK    │    HEAP     │  GLOBAL STORAGE  │
├─────────────┼─────────────┼─────────────┼──────────────────┤
│ Virtual     │ Function    │ Manual      │ Persistent       │
│ Infinite    │ Local       │ Allocation  │ On-Chain         │
│ SSA-like    │ Auto-free   │ Explicit    │ Merkle-backed    │
│             │             │ free()      │                  │
└─────────────┴─────────────┴─────────────┴──────────────────┘
```

### 4.2 Register Domain

Virtual registers are infinite and follow SSA (Static Single Assignment) form:

```x3
// Each assignment creates a new virtual register
let r0 = 10;      // %r0 = 10
let r1 = 20;      // %r1 = 20
let r2 = r0 + r1; // %r2 = add %r0, %r1
```

### 4.3 Stack Domain

Stack memory is automatically managed per function:

```x3
fn compute() -> i64 {
    let a: i64 = 100;      // Pushed to stack
    let b: i64 = 200;      // Pushed to stack
    let result = a + b;    // Computed
    return result;         // a, b automatically popped
}
```

Stack layout:
```
┌──────────────┐ ← Stack top
│   result     │
├──────────────┤
│      b       │
├──────────────┤
│      a       │
├──────────────┤
│ return addr  │
└──────────────┘ ← Frame base
```

### 4.4 Heap Domain

Heap memory requires explicit allocation and deallocation:

```x3
// Allocation
let ptr: heap<LargeStruct> = alloc<LargeStruct>();

// Writing to heap
store(ptr, LargeStruct {
    data: [0; 1000],
    metadata: 42,
});

// Reading from heap
let value = load(ptr);

// Deallocation (REQUIRED - no garbage collection)
free(ptr);
```

**Memory Safety Rules:**
- Every `alloc()` must have exactly one `free()`
- Use-after-free is a runtime panic
- Double-free is a runtime panic
- Null pointer dereference is a runtime panic

### 4.5 Global Storage Domain

Global storage persists across transactions:

```x3
// Declaration (at module level)
global total_supply: u128;
global balances: Map<address, u128>;
global allowances: Map<(address, address), u128>;

// Usage (in functions)
fn transfer(to: address, amount: u128) -> bool {
    let sender = tx.sender;
    let sender_balance = balances.get(sender);
    
    if sender_balance < amount {
        return false;
    }
    
    balances.set(sender, sender_balance - amount);
    balances.set(to, balances.get(to) + amount);
    
    return true;
}
```

**Storage Slots:**
```
global counter: u64;           // Slot 0
global name: bytes;            // Slot 1
global balances: Map<...>;     // Slot 2 (map root)
```

---

## 5. Control Flow

### 5.1 Conditional Execution

```x3
// If-else
if condition {
    do_something();
} else if other_condition {
    do_other();
} else {
    do_default();
}

// Ternary-style (expression)
let max = if a > b { a } else { b };
```

### 5.2 Pattern Matching

```x3
match value {
    0 => handle_zero(),
    1 => handle_one(),
    2..=10 => handle_small(),
    n if n > 100 => handle_large(n),
    _ => handle_default(),
}

// Struct destructuring
match point {
    Position { x: 0, y: 0, z: 0 } => origin(),
    Position { x, y: 0, z: 0 } => x_axis(x),
    Position { x, y, z } => general(x, y, z),
}
```

### 5.3 Loops

```x3
// Infinite loop with break
loop {
    process();
    if done {
        break;
    }
}

// While loop
while condition {
    iterate();
}

// For loop (range-based)
for i in 0..100 {
    process(i);
}

// For loop (iterator)
for item in collection {
    process(item);
}

// Loop with continue
for i in 0..100 {
    if skip_condition(i) {
        continue;
    }
    process(i);
}
```

### 5.4 Loop Labels

```x3
'outer: for i in 0..10 {
    'inner: for j in 0..10 {
        if condition {
            break 'outer;  // Exit both loops
        }
        if other {
            continue 'inner;  // Skip inner iteration
        }
    }
}
```

---

## 6. Functions

### 6.1 Function Declaration

```x3
/// Adds two numbers and returns the result.
fn add(a: i64, b: i64) -> i64 {
    return a + b;
}

// Implicit return (last expression)
fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

// No return value (unit type)
fn log_event(msg: bytes) {
    emit Event { message: msg };
}
```

### 6.2 Visibility

```x3
// Public (callable externally)
pub fn transfer(to: address, amount: u128) -> bool { }

// Private (module-internal, default)
fn internal_helper() { }

// Entry point (can be called as transaction)
#[entry]
pub fn main() { }
```

### 6.3 Function Pointers

```x3
type BinaryOp = fn(i64, i64) -> i64;

fn apply(op: BinaryOp, a: i64, b: i64) -> i64 {
    op(a, b)
}

let result = apply(add, 10, 20);  // 30
```

### 6.4 Closures

```x3
let multiplier = 10;
let scale = |x: i64| -> i64 { x * multiplier };

let result = scale(5);  // 50
```

---

## 7. Built-in Operations

### 7.1 Arithmetic Operations

| Operation      | Syntax                 | Description                   |
| -------------- | ---------------------- | ----------------------------- |
| Addition       | `add(a, b)` or `a + b` | Integer addition              |
| Subtraction    | `sub(a, b)` or `a - b` | Integer subtraction           |
| Multiplication | `mul(a, b)` or `a * b` | Integer multiplication        |
| Division       | `div(a, b)` or `a / b` | Integer division (truncating) |
| Modulo         | `mod(a, b)` or `a % b` | Remainder                     |
| Power          | `pow(base, exp)`       | Exponentiation                |
| Negate         | `neg(a)` or `-a`       | Negation                      |

**Overflow Behavior:**
```x3
// Default: Panic on overflow
let a: u8 = 255;
let b = a + 1;  // PANIC: overflow

// Wrapping arithmetic
let c = wrapping_add(a, 1);  // c = 0

// Saturating arithmetic
let d = saturating_add(a, 1);  // d = 255

// Checked arithmetic
let e = checked_add(a, 1);  // e = None
```

### 7.2 Bitwise Operations

| Operation        | Syntax                  | Description              |
| ---------------- | ----------------------- | ------------------------ |
| AND              | `and(a, b)` or `a & b`  | Bitwise AND              |
| OR               | `or(a, b)` or `a \| b`  | Bitwise OR               |
| XOR              | `xor(a, b)` or `a ^ b`  | Bitwise XOR              |
| NOT              | `not(a)` or `!a`        | Bitwise NOT              |
| Shift Left       | `shl(a, n)` or `a << n` | Left shift               |
| Shift Right      | `shr(a, n)` or `a >> n` | Right shift (logical)    |
| Arithmetic Shift | `sar(a, n)`             | Right shift (arithmetic) |

### 7.3 Comparison Operations

| Operation        | Syntax                 | Description      |
| ---------------- | ---------------------- | ---------------- |
| Equal            | `eq(a, b)` or `a == b` | Equality         |
| Not Equal        | `ne(a, b)` or `a != b` | Inequality       |
| Less Than        | `lt(a, b)` or `a < b`  | Less than        |
| Less or Equal    | `le(a, b)` or `a <= b` | Less or equal    |
| Greater Than     | `gt(a, b)` or `a > b`  | Greater than     |
| Greater or Equal | `ge(a, b)` or `a >= b` | Greater or equal |

### 7.4 Logical Operations

```x3
let result = a && b;   // Logical AND (short-circuit)
let result = a || b;   // Logical OR (short-circuit)
let result = !a;       // Logical NOT
```

### 7.5 Memory Operations

```x3
// Load from pointer
let value = load(ptr);
let value = load_offset(ptr, offset);

// Store to pointer
store(ptr, value);
store_offset(ptr, offset, value);

// Memory copy
memcpy(dest, src, size);

// Memory set
memset(ptr, value, size);

// Memory compare
let eq = memeq(a, b, size);
```

---

## 8. Chain Intrinsics

### 8.1 Block Context

```x3
block.number      // Current block number (u64)
block.timestamp   // Block timestamp (u64, seconds)
block.hash        // Previous block hash (bytes32)
block.author      // Block producer address
block.gas_limit   // Block gas limit (u64)
```

### 8.2 Transaction Context

```x3
tx.sender         // Transaction sender address
tx.origin         // Original sender (for nested calls)
tx.value          // Native token amount sent (u128)
tx.gas_price      // Gas price (u128)
tx.gas_remaining  // Remaining gas (u64)
tx.data           // Call data (bytes)
tx.signature      // Transaction signature (bytes)
```

### 8.3 Chain State

```x3
// Balance queries
chain.balance(address)           // Native token balance
chain.token_balance(token, addr) // Token balance

// Code inspection
chain.code_size(address)         // Contract code size
chain.code_hash(address)         // Contract code hash

// Storage access (low-level)
chain.storage_read(slot)         // Read storage slot
chain.storage_write(slot, value) // Write storage slot
```

### 8.4 Cross-Contract Calls

```x3
// Static call (read-only)
let result = call_static(target, method, args);

// Regular call
let result = call(target, method, args, value);

// Delegate call (preserves context)
let result = delegate_call(target, method, args);

// Create contract
let new_address = create(code, salt, value);
```

### 8.5 Flash Loans

```x3
flash(asset, amount, |loan| {
    // loan.amount contains borrowed tokens
    
    // Execute arbitrage
    let profit = execute_arbitrage(loan.amount);
    
    // Must return at least loan.amount + loan.fee
    return loan.amount + loan.fee + profit;
});
```

### 8.6 Atomic Swaps

```x3
// Simple swap
let output = swap(pool, token_in, token_out, amount_in);

// Swap with slippage protection
let output = swap_exact(
    pool,
    token_in,
    token_out,
    amount_in,
    min_amount_out,
);

// Multi-hop swap
let output = swap_route(
    [pool_a, pool_b, pool_c],
    [token_a, token_b, token_c, token_d],
    amount_in,
);
```

### 8.7 Events

```x3
// Event declaration
event Transfer {
    from: address,
    to: address,
    amount: u128,
}

// Emit event
emit Transfer {
    from: tx.sender,
    to: recipient,
    amount: amount,
};
```

---

## 9. Bytecode Model

### 9.1 Instruction Format

Each X3 bytecode instruction is encoded as:

```
┌────────────┬────────────┬────────────┬────────────┐
│  Opcode    │  Dest Reg  │  Src1 Reg  │  Src2/Imm  │
│  (8 bits)  │  (8 bits)  │  (8 bits)  │  (8 bits)  │
└────────────┴────────────┴────────────┴────────────┘
```

For extended immediates:

```
┌────────────┬────────────┬───────────────────────────┐
│  Opcode    │  Dest Reg  │  64-bit Immediate         │
│  (8 bits)  │  (8 bits)  │  (64 bits)                │
└────────────┴────────────┴───────────────────────────┘
```

### 9.2 Opcode Reference

#### Arithmetic (0x00 - 0x0F)
```
0x00  NOP                    No operation
0x01  ADD   rd, rs1, rs2     rd = rs1 + rs2
0x02  SUB   rd, rs1, rs2     rd = rs1 - rs2
0x03  MUL   rd, rs1, rs2     rd = rs1 * rs2
0x04  DIV   rd, rs1, rs2     rd = rs1 / rs2
0x05  MOD   rd, rs1, rs2     rd = rs1 % rs2
0x06  NEG   rd, rs1          rd = -rs1
0x07  ADDI  rd, rs1, imm     rd = rs1 + imm
0x08  SUBI  rd, rs1, imm     rd = rs1 - imm
0x09  MULI  rd, rs1, imm     rd = rs1 * imm
```

#### Bitwise (0x10 - 0x1F)
```
0x10  AND   rd, rs1, rs2     rd = rs1 & rs2
0x11  OR    rd, rs1, rs2     rd = rs1 | rs2
0x12  XOR   rd, rs1, rs2     rd = rs1 ^ rs2
0x13  NOT   rd, rs1          rd = ~rs1
0x14  SHL   rd, rs1, rs2     rd = rs1 << rs2
0x15  SHR   rd, rs1, rs2     rd = rs1 >> rs2 (logical)
0x16  SAR   rd, rs1, rs2     rd = rs1 >> rs2 (arithmetic)
```

#### Comparison (0x20 - 0x2F)
```
0x20  EQ    rd, rs1, rs2     rd = (rs1 == rs2)
0x21  NE    rd, rs1, rs2     rd = (rs1 != rs2)
0x22  LT    rd, rs1, rs2     rd = (rs1 < rs2)
0x23  LE    rd, rs1, rs2     rd = (rs1 <= rs2)
0x24  GT    rd, rs1, rs2     rd = (rs1 > rs2)
0x25  GE    rd, rs1, rs2     rd = (rs1 >= rs2)
```

#### Memory (0x30 - 0x3F)
```
0x30  LOAD  rd, [rs1]        rd = mem[rs1]
0x31  LOADB rd, [rs1]        rd = mem[rs1] (byte)
0x32  LOADW rd, [rs1]        rd = mem[rs1] (word)
0x33  STORE [rd], rs1        mem[rd] = rs1
0x34  STOREB [rd], rs1       mem[rd] = rs1 (byte)
0x35  STOREW [rd], rs1       mem[rd] = rs1 (word)
0x36  ALLOC rd, size         rd = heap_alloc(size)
0x37  FREE  rs1              heap_free(rs1)
0x38  GLOAD rd, slot         rd = global[slot]
0x39  GSTOR slot, rs1        global[slot] = rs1
```

#### Control Flow (0x40 - 0x4F)
```
0x40  JMP   label            Unconditional jump
0x41  JZ    rs1, label       Jump if zero
0x42  JNZ   rs1, label       Jump if not zero
0x43  CALL  fn_id            Call function
0x44  RET   rs1              Return value
0x45  RETN                   Return (no value)
```

#### Chain (0x50 - 0x5F)
```
0x50  BLOCKHASH  rd          rd = block.hash
0x51  BLOCKNUM   rd          rd = block.number
0x52  TIMESTAMP  rd          rd = block.timestamp
0x53  SENDER     rd          rd = tx.sender
0x54  VALUE      rd          rd = tx.value
0x55  BALANCE    rd, rs1     rd = balance(rs1)
0x56  XCALL      rd, target  rd = cross_call(...)
0x57  LOG        topics, data Emit log event
```

#### Warp (0x60 - 0x6F)
```
0x60  WARP_BEGIN n_paths     Begin warp with n paths
0x61  WARP_PATH  path_id     Start path definition
0x62  WARP_END               End path definition
0x63  WARP_COMMIT            Commit best path
0x64  WARP_ABORT             Abort all paths
```

### 9.3 Example Bytecode

Source:
```x3
fn add_and_store(a: i64, b: i64) {
    let sum = a + b;
    global result: i64;
    result = sum;
}
```

Bytecode:
```asm
; Function: add_and_store
; Args: r0 = a, r1 = b
add_and_store:
    ADD   r2, r0, r1      ; r2 = a + b
    GSTOR 0, r2           ; global[0] = r2
    RETN                  ; return
```

---

## 10. Compilation Pipeline

### 10.1 Pipeline Overview

```
Source (.x3)
    │
    ▼
┌─────────────────┐
│  1. LEXER       │  Tokenization
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  2. PARSER      │  AST construction
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  3. TYPE CHECK  │  Type inference & validation
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  4. HIR LOWER   │  High-level IR
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  5. MIR LOWER   │  Mid-level IR (CFG)
└─────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  6. OPTIMIZATION PASSES (16 passes)         │
│  ├─ Dead Code Elimination                   │
│  ├─ Constant Folding                        │
│  ├─ Constant Propagation                    │
│  ├─ Common Subexpression Elimination        │
│  ├─ Loop Invariant Code Motion              │
│  ├─ Strength Reduction                      │
│  ├─ Inlining                                │
│  ├─ Tail Call Optimization                  │
│  ├─ Register Allocation                     │
│  ├─ Instruction Scheduling                  │
│  ├─ Branch Optimization                     │
│  ├─ Memory Optimization                     │
│  ├─ Peephole Optimization                   │
│  ├─ Loop Unrolling                          │
│  ├─ Vectorization                           │
│  └─ Final Cleanup                           │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────┐
│  7. CODEGEN     │  Bytecode emission
└─────────────────┘
    │
    ▼
Bytecode (.x3bc)
```

### 10.2 Optimization Levels

| Level | Passes                    | Use Case                |
| ----- | ------------------------- | ----------------------- |
| `-O0` | None                      | Debugging, fast compile |
| `-O1` | DCE, ConstFold, ConstProp | Development             |
| `-O2` | All except vectorization  | Production              |
| `-O3` | All passes, aggressive    | Maximum performance     |
| `-Os` | Size-focused subset       | Contract deployment     |

### 10.3 Compilation Flags

```bash
x3c source.x3 -o output.x3bc        # Basic compilation
x3c source.x3 -O3 -o output.x3bc    # Optimized
x3c source.x3 --emit=mir            # Output MIR
x3c source.x3 --emit=asm            # Output assembly
x3c source.x3 --dump-cfg            # Dump control flow graph
```

---

## 11. Formal Semantics

### 11.1 Operational Semantics

The semantics are defined as a small-step operational semantics with the following state:

```
State = (σ, μ, γ, κ, pc)
where:
  σ : RegisterFile     (virtual registers)
  μ : Memory           (heap)
  γ : GlobalStorage    (on-chain state)
  κ : CallStack        (return addresses)
  pc: ProgramCounter   (current instruction)
```

### 11.2 Evaluation Rules

**Addition:**
```
          σ(rs1) = v1    σ(rs2) = v2    v = v1 + v2
    ─────────────────────────────────────────────────
    (σ, μ, γ, κ, ADD rd rs1 rs2) → (σ[rd ↦ v], μ, γ, κ, pc+1)
```

**Memory Load:**
```
          σ(rs1) = addr    μ(addr) = v
    ─────────────────────────────────────────
    (σ, μ, γ, κ, LOAD rd [rs1]) → (σ[rd ↦ v], μ, γ, κ, pc+1)
```

**Global Store:**
```
          σ(rs1) = v
    ─────────────────────────────────────────
    (σ, μ, γ, κ, GSTOR slot rs1) → (σ, μ, γ[slot ↦ v], κ, pc+1)
```

**Conditional Jump:**
```
          σ(rs1) = 0
    ─────────────────────────────────────────
    (σ, μ, γ, κ, JZ rs1 label) → (σ, μ, γ, κ, label)

          σ(rs1) ≠ 0
    ─────────────────────────────────────────
    (σ, μ, γ, κ, JZ rs1 label) → (σ, μ, γ, κ, pc+1)
```

### 11.3 Type Soundness

**Theorem (Progress):** If `Γ ⊢ e : τ` then either `e` is a value or there exists `e'` such that `e → e'`.

**Theorem (Preservation):** If `Γ ⊢ e : τ` and `e → e'` then `Γ ⊢ e' : τ`.

### 11.4 Determinism Guarantee

**Theorem (Deterministic Execution):** For any program `P` and initial state `S₀`:
```
∀S₁, S₂: (S₀ →* S₁) ∧ (S₀ →* S₂) ∧ terminal(S₁) ∧ terminal(S₂) ⟹ S₁ = S₂
```

This ensures consensus: all nodes produce identical results.

---

## Appendix A: Grammar (EBNF)

```ebnf
program        = { item }
item           = function | struct_def | global_decl | const_decl | use_stmt
function       = [ 'pub' ] 'fn' IDENT '(' params ')' [ '->' type ] block
params         = [ param { ',' param } ]
param          = IDENT ':' type
block          = '{' { statement } [ expr ] '}'
statement      = let_stmt | assign_stmt | expr_stmt | return_stmt | if_stmt | loop_stmt
let_stmt       = 'let' [ 'mut' ] IDENT [ ':' type ] '=' expr ';'
assign_stmt    = place '=' expr ';'
expr_stmt      = expr ';'
return_stmt    = 'return' [ expr ] ';'
if_stmt        = 'if' expr block [ 'else' ( if_stmt | block ) ]
loop_stmt      = 'loop' block | 'while' expr block | 'for' IDENT 'in' expr block
expr           = binary_expr | unary_expr | call_expr | literal | IDENT | '(' expr ')'
binary_expr    = expr binop expr
unary_expr     = unop expr
call_expr      = IDENT '(' [ expr { ',' expr } ] ')'
type           = primitive_type | array_type | struct_type | tuple_type | fn_type
primitive_type = 'i8' | 'i16' | 'i32' | 'i64' | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'bool' | 'byte' | 'bytes' | 'address'
array_type     = '[' type ';' INTEGER ']'
```

---

## Appendix B: Standard Library

```x3
// Math
fn min<T: Ord>(a: T, b: T) -> T;
fn max<T: Ord>(a: T, b: T) -> T;
fn abs(x: i64) -> u64;
fn sqrt(x: u64) -> u64;

// Memory
fn alloc<T>() -> heap<T>;
fn free<T>(ptr: heap<T>);
fn memcpy(dest: bytes, src: bytes, len: u64);
fn memset(dest: bytes, val: u8, len: u64);

// Crypto
fn keccak256(data: bytes) -> bytes32;
fn sha256(data: bytes) -> bytes32;
fn ecrecover(hash: bytes32, sig: bytes) -> address;
fn verify_signature(pubkey: bytes, msg: bytes, sig: bytes) -> bool;

// Encoding
fn encode<T>(value: T) -> bytes;
fn decode<T>(data: bytes) -> T;
fn encode_packed(values: ...) -> bytes;

// Collections
fn vec_new<T>() -> Vec<T>;
fn vec_push<T>(v: &mut Vec<T>, item: T);
fn vec_pop<T>(v: &mut Vec<T>) -> Option<T>;
fn map_new<K, V>() -> Map<K, V>;
fn map_get<K, V>(m: &Map<K, V>, key: K) -> Option<V>;
fn map_set<K, V>(m: &mut Map<K, V>, key: K, value: V);
```

---

## Appendix C: Error Codes

| Code | Name                | Description                       |
| ---- | ------------------- | --------------------------------- |
| E001 | TypeError           | Type mismatch                     |
| E002 | UndefinedVariable   | Variable not in scope             |
| E003 | UndefinedFunction   | Function not declared             |
| E004 | InvalidOperation    | Operation not supported for types |
| E005 | OverflowError       | Integer overflow                  |
| E006 | DivisionByZero      | Division by zero                  |
| E007 | OutOfGas            | Execution ran out of gas          |
| E008 | OutOfMemory         | Heap allocation failed            |
| E009 | InvalidMemoryAccess | Null or freed pointer access      |
| E010 | StackOverflow       | Call stack exceeded               |
| E011 | InvalidOpcode       | Unknown bytecode instruction      |
| E012 | RevertError         | Explicit revert                   |

---

**Document Version:** 1.0.0  
**Specification Status:** Canonical  
**Maintainer:** X3 Chain Core Engineering
