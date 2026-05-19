# X3 Adaptive Intelligence Chain (AIC) Specification v1.0

> **The World's First Self-Evolving Blockchain**

## Executive Summary

X3 X3 Chain introduces the Adaptive Intelligence Chain (AIC) - a revolutionary Layer-1 blockchain that modifies its own runtime based on network conditions, usage patterns, MEV pressure, VM demand, and AI-generated optimizations. This document specifies the complete architecture.

---

## 1. X3 Language Specification (2-Page Compact)

### 1.1 Grammar Definition

```ebnf
Program     := Import* TopLevel*
TopLevel    := FuncDecl | StructDecl | ConstDecl | Annotation*
Import      := 'import' StringLit ';'
FuncDecl    := Annotation* 'fn' IDENT '(' ParamList? ')' ('->' Type)? Block
StructDecl  := 'struct' IDENT '{' FieldList '}'
ConstDecl   := 'const' IDENT ':' Type '=' Expr ';'

ParamList   := Param (',' Param)*
Param       := IDENT ':' Type
FieldList   := Field (',' Field)*
Field       := IDENT ':' Type

Block       := '{' Statement* '}'
Statement   := LetStmt | AssignStmt | ExprStmt | IfStmt | LoopStmt | ReturnStmt | AtomicBlock
LetStmt     := 'let' 'mut'? IDENT (':' Type)? '=' Expr ';'
AssignStmt  := LValue '=' Expr ';'
ExprStmt    := Expr ';'
IfStmt      := 'if' Expr Block ('else' (IfStmt | Block))?
LoopStmt    := 'while' Expr Block | 'for' IDENT 'in' Expr Block
ReturnStmt  := 'return' Expr? ';'
AtomicBlock := '@atomic' Block

Expr        := Primary (BinaryOp Expr)?
Primary     := Literal | IDENT | FuncCall | CrossVMCall | '(' Expr ')' | UnaryOp Primary
FuncCall    := IDENT '(' ArgList? ')'
CrossVMCall := ('call_evm' | 'call_svm') '(' Expr (',' Expr)* ')'
ArgList     := Expr (',' Expr)*

Type        := 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'i8' | 'i16' | 'i32' | 'i64' | 'i128'
            | 'bool' | 'Address' | 'Bytes' | 'Bytes32' | '[' Type ';' INT ']' | IDENT
Literal     := INT | BOOL | STRING | HEX_ADDR | HEX_BYTES
BinaryOp    := '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '<<' | '>>' | '==' | '!=' | '<' | '>' | '<=' | '>=' | '&&' | '||'
UnaryOp     := '-' | '!' | '~'
```

### 1.2 First 30 Built-in Host Syscalls

| #   | Function             | Signature                                          | Description                | Gas  |
| --- | -------------------- | -------------------------------------------------- | -------------------------- | ---- |
| 01  | `read_state`         | `(key: Bytes) -> Bytes`                            | Read chain state           | 50   |
| 02  | `write_state`        | `(key: Bytes, val: Bytes)`                         | Write chain state          | 100  |
| 03  | `emit_event`         | `(topic: Bytes, data: Bytes)`                      | Emit blockchain event      | 25   |
| 04  | `call_evm`           | `(addr: Address, data: Bytes) -> Bytes`            | Execute EVM call           | 1000 |
| 05  | `call_svm`           | `(program: Address, data: Bytes) -> Bytes`         | Execute SVM call           | 1000 |
| 06  | `gas`                | `() -> u128`                                       | Get remaining gas          | 1    |
| 07  | `current_block`      | `() -> u64`                                        | Get block number           | 5    |
| 08  | `block_time`         | `() -> u64`                                        | Get block timestamp        | 5    |
| 09  | `random`             | `(seed: u64) -> u64`                               | Deterministic random       | 10   |
| 10  | `hash256`            | `(data: Bytes) -> Bytes32`                         | SHA-256 hash               | 30   |
| 11  | `keccak256`          | `(data: Bytes) -> Bytes32`                         | Keccak-256 hash            | 30   |
| 12  | `blake3`             | `(data: Bytes) -> Bytes32`                         | Blake3 hash                | 20   |
| 13  | `sign`               | `(data: Bytes, key: Bytes) -> Bytes`               | Sign data                  | 100  |
| 14  | `verify_sig`         | `(data: Bytes, sig: Bytes, pubkey: Bytes) -> bool` | Verify signature           | 100  |
| 15  | `transfer`           | `(to: Address, amount: u128)`                      | Transfer tokens            | 200  |
| 16  | `get_balance`        | `(account: Address) -> u128`                       | Get account balance        | 25   |
| 17  | `get_nonce`          | `(account: Address) -> u64`                        | Get account nonce          | 25   |
| 18  | `log`                | `(msg: Bytes)`                                     | Debug log (off-chain only) | 5    |
| 19  | `revert`             | `(msg: Bytes)`                                     | Revert with message        | 0    |
| 20  | `assert`             | `(cond: bool, msg: Bytes)`                         | Assert condition           | 5    |
| 21  | `mempool_scan`       | `(filter: Bytes) -> Bytes`                         | Scan mempool (off-chain)   | 500  |
| 22  | `flashloan`          | `(pool: Address, amount: u128) -> Bytes`           | Request flashloan          | 2000 |
| 23  | `arb_route`          | `(pools: [Address], amounts: [u128]) -> Bytes`     | Calculate arb route        | 1000 |
| 24  | `checkpoint`         | `() -> Bytes32`                                    | Create state checkpoint    | 50   |
| 25  | `get_pool_liquidity` | `(pool: Address) -> u128`                          | Get pool liquidity         | 50   |
| 26  | `emit_swap_event`    | `(pool: Address, amt: u128)`                       | Emit swap event            | 30   |
| 27  | `encode_swap`        | `(token: Address, amount: u128) -> Bytes`          | Encode swap calldata       | 20   |
| 28  | `encode_loan`        | `(amount: u128) -> Bytes`                          | Encode loan request        | 20   |
| 29  | `encode_arb`         | `(data: Bytes) -> Bytes`                           | Encode arbitrage call      | 20   |
| 30  | `sleep_ms`           | `(ms: u64)`                                        | Sleep (off-chain only)     | 0    |

### 1.3 Annotations

| Annotation                     | Purpose                              |
| ------------------------------ | ------------------------------------ |
| `@pure`                        | No state mutation allowed            |
| `@view`                        | Read-only state access               |
| `@atomic`                      | Cross-VM atomicity enforced          |
| `@sandbox`                     | Enforce strict gas & memory bounds   |
| `@gas_limit(N)`                | Set explicit gas limit               |
| `@priority(high\|normal\|low)` | Execution priority hint              |
| `@jit_hint`                    | Mark for JIT compilation             |
| `@parallel`                    | Allow parallel execution (off-chain) |

### 1.4 Error Model

| Error Code | Name                  | Description                   |
| ---------- | --------------------- | ----------------------------- |
| 0x01       | `OutOfGas`            | Gas exhausted                 |
| 0x02       | `InvalidOpcode`       | Unknown opcode                |
| 0x03       | `StackOverflow`       | Stack limit exceeded          |
| 0x04       | `StackUnderflow`      | Pop from empty stack          |
| 0x05       | `MemoryLimitExceeded` | Memory allocation failed      |
| 0x06       | `HostCallFailed`      | External call error           |
| 0x07       | `CrossVMCallFailed`   | Cross-VM operation failed     |
| 0x08       | `InvalidReceipt`      | Receipt verification failed   |
| 0x09       | `AtomicRollback`      | Atomic block rolled back      |
| 0x0A       | `AssertionFailed`     | Assert condition false        |
| 0x0B       | `Reverted`            | Explicit revert               |
| 0x0C       | `InvalidSignature`    | Signature verification failed |

---

## 2. Bytecode Format & Opcode Table

### 2.1 Module Format

```
┌─────────────────────────────────────────────────────┐
│  Magic: 0x58335643 ("X3VC")                         │  4 bytes
├─────────────────────────────────────────────────────┤
│  Version: u16                                       │  2 bytes
├─────────────────────────────────────────────────────┤
│  Flags: u16                                         │  2 bytes
├─────────────────────────────────────────────────────┤
│  Const Pool Size: u32                               │  4 bytes
├─────────────────────────────────────────────────────┤
│  Const Pool: [ConstEntry]                           │  variable
├─────────────────────────────────────────────────────┤
│  Function Table Size: u16                           │  2 bytes
├─────────────────────────────────────────────────────┤
│  Function Table: [FuncEntry]                        │  variable
├─────────────────────────────────────────────────────┤
│  Code Section Size: u32                             │  4 bytes
├─────────────────────────────────────────────────────┤
│  Code Section: [u8]                                 │  variable
├─────────────────────────────────────────────────────┤
│  Debug Info (optional)                              │  variable
└─────────────────────────────────────────────────────┘
```

### 2.2 Extended Opcode Table for AIC

| Opcode                          | Mnemonic           | Operands         | Description              | Gas |
| ------------------------------- | ------------------ | ---------------- | ------------------------ | --- |
| **Arithmetic (0x01-0x1F)**      |
| 0x01                            | `ADD_RRR`          | R[a],R[b],R[c]   | ra = rb + rc             | 1   |
| 0x02                            | `SUB_RRR`          | R[a],R[b],R[c]   | ra = rb - rc             | 1   |
| 0x03                            | `MUL_RRR`          | R[a],R[b],R[c]   | ra = rb * rc             | 2   |
| 0x04                            | `DIV_RRR`          | R[a],R[b],R[c]   | ra = rb / rc             | 3   |
| 0x05                            | `MOD_RRR`          | R[a],R[b],R[c]   | ra = rb % rc             | 3   |
| **Memory (0x10-0x2F)**          |
| 0x10                            | `LOAD`             | R[a], [R[b]+imm] | Load 128-bit             | 5   |
| 0x11                            | `STORE`            | R[a], [R[b]+imm] | Store 128-bit            | 5   |
| 0x14                            | `PUSH`             | imm16            | Push immediate           | 1   |
| 0x15                            | `POP`              | R[a]             | Pop to register          | 1   |
| **Control Flow (0x30-0x4F)**    |
| 0x30                            | `JMP`              | offset16         | Unconditional jump       | 2   |
| 0x31                            | `JZ`               | offset16         | Jump if zero             | 2   |
| 0x32                            | `JNZ`              | offset16         | Jump if not zero         | 2   |
| 0x33                            | `CALL`             | addr16           | Call function            | 5   |
| 0x34                            | `RET`              | -                | Return                   | 5   |
| **Crypto (0x40-0x5F)**          |
| 0x40                            | `SHA256`           | R[a], R[b]       | SHA-256 hash             | 10  |
| 0x41                            | `KECCAK`           | R[a], R[b]       | Keccak-256               | 10  |
| 0x42                            | `BLAKE3`           | R[a], R[b]       | Blake3 hash              | 8   |
| 0x43                            | `SIG_VERIFY`       | R[a],R[b],R[c]   | Verify signature         | 50  |
| **Cross-VM (0x60-0x7F)**        |
| 0x60                            | `EVM_CALL`         | R[a], addr16     | Call EVM contract        | 100 |
| 0x61                            | `SVM_CALL`         | R[a], addr16     | Call SVM program         | 100 |
| 0x64                            | `ATOMIC_BEGIN`     | -                | Begin atomic block       | 250 |
| 0x65                            | `ATOMIC_COMMIT`    | -                | Commit atomic            | 500 |
| 0x66                            | `ATOMIC_ROLLBACK`  | -                | Rollback atomic          | 250 |
| **Evolution (0xE0-0xEF) - NEW** |
| 0xE0                            | `EVOLVE_HINT`      | metric_id, value | Report optimization hint | 5   |
| 0xE1                            | `QUERY_RUNTIME`    | R[a], param_id   | Query runtime parameter  | 10  |
| 0xE2                            | `SUGGEST_MUTATION` | R[a]             | Suggest runtime change   | 50  |
| 0xE3                            | `HOTPATH_MARK`     | func_id          | Mark hot function        | 2   |

---

## 3. Evolution Core Architecture

### 3.1 Adaptive Runtime Mutation System

```
┌─────────────────────────────────────────────────────────────────────┐
│                      EVOLUTION CORE                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │   Metrics    │   │    AI/ML     │   │  Governance  │            │
│  │  Collector   │──▶│   Analyzer   │──▶│   Queue      │            │
│  └──────────────┘   └──────────────┘   └──────────────┘            │
│         │                  │                  │                     │
│         ▼                  ▼                  ▼                     │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │  Network     │   │   Runtime    │   │   Auto       │            │
│  │  Telemetry   │   │  Simulator   │   │  Approval    │            │
│  └──────────────┘   └──────────────┘   └──────────────┘            │
│                                                │                    │
│                                                ▼                    │
│                                    ┌──────────────────┐             │
│                                    │  RUNTIME MUTATE  │             │
│                                    └──────────────────┘             │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Metrics Collected

| Metric                 | Description                | Update Frequency |
| ---------------------- | -------------------------- | ---------------- |
| `block_gas_used`       | Gas consumed per block     | Per block        |
| `evm_call_count`       | EVM transactions count     | Per block        |
| `svm_call_count`       | SVM transactions count     | Per block        |
| `cross_vm_ratio`       | Cross-VM vs single-VM ops  | Per block        |
| `mempool_depth`        | Pending transactions       | Real-time        |
| `mev_pressure`         | Detected MEV activity      | Real-time        |
| `validator_throughput` | Validator processing speed | Per epoch        |
| `x3_hotpath_hits`      | JIT compilation triggers   | Per block        |
| `swap_volume`          | DEX swap volume            | Per block        |
| `flashloan_volume`     | Flashloan utilization      | Per block        |

### 3.3 Mutation Types

1. **Gas Parameter Mutations**
   - Adjust opcode gas costs
   - Modify base fees
   - Tune priority fee multipliers

2. **VM Load Balancing**
   - Shift workload between EVM/SVM
   - Adjust parallelism levels
   - Optimize cross-VM bridge

3. **JIT Thresholds**
   - Lower hotpath detection threshold
   - Expand JIT compilation scope
   - Tune optimization aggressiveness

4. **Consensus Tuning**
   - Adjust block time (within bounds)
   - Modify finality parameters
   - Optimize validator selection

---

## 4. Swarm Execution Layer (REAPER)

### 4.1 Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SWARM NETWORK                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌───────────┐      ┌───────────┐      ┌───────────┐               │
│   │  Node 1   │◀────▶│  Node 2   │◀────▶│  Node N   │               │
│   │ (GPU/CPU) │      │ (GPU/CPU) │      │ (GPU/CPU) │               │
│   └─────┬─────┘      └─────┬─────┘      └─────┬─────┘               │
│         │                  │                  │                      │
│         └────────────┬─────┴─────┬────────────┘                      │
│                      │           │                                   │
│                      ▼           ▼                                   │
│              ┌──────────────────────────┐                            │
│              │    TASK COORDINATOR      │                            │
│              │  • Job Distribution      │                            │
│              │  • Receipt Aggregation   │                            │
│              │  • Reward Distribution   │                            │
│              └────────────┬─────────────┘                            │
│                           │                                          │
│                           ▼                                          │
│              ┌──────────────────────────┐                            │
│              │   ON-CHAIN VERIFIER      │                            │
│              │  • Receipt Validation    │                            │
│              │  • Merkle Proof Check    │                            │
│              │  • State Application     │                            │
│              └──────────────────────────┘                            │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 Receipt Format

```rust
struct ExecutionReceipt {
    job_id: [u8; 32],
    executor_pubkey: [u8; 32],
    input_hash: [u8; 32],
    output_hash: [u8; 32],
    state_root_before: [u8; 32],
    state_root_after: [u8; 32],
    merkle_proof: Vec<[u8; 32]>,
    gas_used: u128,
    timestamp: u64,
    signature: [u8; 64],
}
```

### 4.3 Profit Sharing Model

| Participant       | Share | Condition             |
| ----------------- | ----- | --------------------- |
| Executor Node     | 70%   | Correct execution     |
| Protocol Treasury | 15%   | Always                |
| Stakers           | 10%   | Proportional to stake |
| Referrer          | 5%    | If referral exists    |

---

## 5. Example X3 Scripts

### 5.1 Atomic Mempool Swap (`arb.x3`)

```x3
@atomic
@gas_limit(500000)
fn main() {
    let pool_a: Address = 0xABCD1234...;
    let pool_b: Address = 0x5678EFGH...;
    let amount: u128 = 1000_000_000_000_000_000; // 1 token
    
    // Execute atomic cross-DEX swap
    let result_a = call_evm(pool_a, encode_swap(amount));
    let received = decode_u128(result_a);
    
    let result_b = call_svm(pool_b, encode_swap(received));
    let final_amount = decode_u128(result_b);
    
    // Profit check
    assert(final_amount > amount, "No profit");
    
    emit_swap_event(pool_a, amount);
    emit_swap_event(pool_b, final_amount);
}
```

### 5.2 Cross-VM Flashloan (`flash.x3`)

```x3
@atomic
@gas_limit(1000000)
fn execute_flashloan(pool: Address, amount: u128) -> u128 {
    // Borrow from SVM lending pool
    let loan = call_svm(pool, encode_loan(amount));
    let borrowed = decode_u128(loan);
    
    // Execute arbitrage on EVM DEX
    let arb_data = arb_route([0xDEX1..., 0xDEX2...], [borrowed, 0]);
    let arb_result = call_evm(0xRouter..., arb_data);
    let profit = decode_u128(arb_result);
    
    // Repay loan + fee
    let repay_amount = borrowed + (borrowed * 30 / 10000); // 0.3% fee
    call_svm(pool, encode_repay(repay_amount));
    
    // Return profit
    let net_profit = profit - repay_amount;
    assert(net_profit > 0, "Unprofitable flashloan");
    
    emit_event("flashloan_profit", encode_u128(net_profit));
    return net_profit;
}
```

### 5.3 MEV-Smooth Arbitrage (`mev_smooth.x3`)

```x3
@atomic
@priority(high)
@sandbox
fn mev_smooth_arb() {
    // Scan mempool for opportunities (off-chain only)
    let pending = mempool_scan(0x00); // All pending txs
    
    // Find profitable routes
    let routes = find_arb_routes(pending);
    
    for route in routes {
        let expected_profit = simulate_route(route);
        if expected_profit > MIN_PROFIT_THRESHOLD {
            // Execute with MEV protection
            @atomic {
                execute_route(route);
                checkpoint(); // Save progress
            }
        }
    }
    
    // Report metrics to Evolution Core
    evolve_hint(METRIC_ARB_SUCCESS, 1);
}
```

### 5.4 Adaptive Strategy (`adaptive.x3`)

```x3
@jit_hint
fn adaptive_trade() {
    // Query current runtime parameters
    let gas_price = query_runtime(PARAM_GAS_PRICE);
    let vm_load = query_runtime(PARAM_VM_LOAD);
    let mev_level = query_runtime(PARAM_MEV_PRESSURE);
    
    // Adapt strategy based on conditions
    if vm_load > 80 {
        // High load: use SVM (faster)
        execute_svm_strategy();
    } else if mev_level > 50 {
        // High MEV: use atomic protection
        @atomic { execute_protected_strategy(); }
    } else {
        // Normal: optimize for gas
        execute_gas_optimized_strategy();
    }
    
    // Suggest runtime mutation if conditions warrant
    if gas_price > THRESHOLD_HIGH {
        suggest_mutation(MUTATION_LOWER_BASE_FEE);
    }
}
```

---

## 6. Integration with X3 Chain

### 6.1 Pallet Configuration

The Evolution Core integrates as a FRAME pallet:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type EvolutionAuthority: EnsureOrigin<Self::RuntimeOrigin>;
    type MetricsCollector: MetricsProvider;
    type MutationQueue: MutationQueue<Self>;
    type AIAnalyzer: RuntimeAnalyzer;
    type MinApprovalQuorum: Get<Percent>;
}
```

### 6.2 Runtime Integration

```rust
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        // ... existing pallets ...
        AtlasKernel: pallet_x3_kernel,
        X3Verifier: pallet_x3_verifier,
        EvolutionCore: pallet_evolution,
        SwarmCoordinator: pallet_swarm,
    }
);
```

---

## 7. Security Considerations

### 7.1 Sandboxing

- All X3 code runs in deterministic sandbox
- No system calls except approved hostcalls
- Memory bounds strictly enforced
- Gas limits prevent infinite loops

### 7.2 Evolution Safety

- All mutations pass through simulation
- Cross-validation by multiple AI agents
- Governance approval for significant changes
- Automatic rollback on failure

### 7.3 Swarm Security

- Receipts cryptographically signed
- Merkle proofs for state changes
- Slash conditions for malicious executors
- Redundant execution for critical jobs

---

## 8. Roadmap

| Phase | Milestone                        | Timeline |
| ----- | -------------------------------- | -------- |
| 1     | X3 Parser + VM Complete          | ✅ Done   |
| 2     | Bytecode Verifier + Gas Metering | ✅ Done   |
| 3     | Cross-VM Bridge Integration      | Week 1   |
| 4     | Evolution Core Pallet            | Week 2   |
| 5     | Swarm Coordinator + Sidecar      | Week 3   |
| 6     | X3 Verifier Pallet               | Week 3   |
| 7     | AI Analyzer Integration          | Week 4   |
| 8     | Testnet Launch with AIC          | Week 4   |

---

**X3 X3 Chain: Where AI meets Blockchain. The chain that evolves.**
