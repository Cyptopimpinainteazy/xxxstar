# X3 VM Architecture

High-level design
- Fetch-decode-execute micro-VM with a deterministic execution model.
- Table-based opcode dispatch to allow predictable control flow (no dynamic reflection).
- Minimal interrupt/trap model to allow deterministic transaction aborts.

Core components
- Loader: verifies the bytecode and constructs an in-memory representation.
- Verifier: validates control-flow, memory bounds, instruction boundaries, and gas estimation.
- Executor: runs the fetch-decode-execute loop with a deterministic scheduler and gas metering.
- Gas Meter: accounts for consumed gas and triggers traps if quotas are exhausted.
- Host Adapters: EVM adapter and SVM adapter for atomic calls with consistent receipts.
- JIT Hints: hot-path counters and hint values are stored per instruction for later JIT compilation.

Fetch-Decode-Execute loop pseudocode
```
pc = 0
while gas > 0 and not halted {
    opcode = read_u8(code, pc)
    flags = read_u8(code, pc + 1)
    operand = read_u16(code, pc + 2)
    pc_next = pc + 4

    // gas accounting
    cost = gas_cost(opcode, flags, operand, state)
    if gas < cost {
        trap_out_of_gas()
        break
    }
    gas -= cost

    match opcode {
        OPC_ADD_RRR => { /* register arithmetic */ }
        OPC_JMP => { pc_next = pc + sign_extend_i16(operand) }
        OPC_CALL => { push_frame(pc_next); pc_next = operand }
        OPC_RET => { pc_next = pop_return_address(); }
        // ... other opcodes
    }

    pc = pc_next
}
```

Dispatch model
- Table-based dispatch: a static table maps opcodes to inline handler functions.
- No dynamic hashing or reflection; dispatch is a simple array lookup with numeric indexes.
- Handlers must be small and inlined to facilitate fast JIT or AOT compilation.

Register & Stack Layout
- Register file: an array of 32 general-purpose registers (R0..R31).
- Stack: operand stack used by legacy operations and vararg calls; stack grows downwards.
- CallFrames: separate area on the heap or a dedicated callstack with deterministic size.

Memory layout
- Linear memory: contiguous vector of pages; the runtime enforces bounds checks.
- A dedicated data segment for immutable constants in bytecode; loaded at verification time.
- Heap for dynamic allocations controlled by the REAPER compute economy.

Gas and Metering
- Gas is tracked in a u128 GAS register and is decremented per opcode before execution.
- Gas refunds for very cheap/compensating operations may be allowed but must be constant-time.

Traps & Exceptions
- Traps are deterministic and immutably encoded in the transaction receipt for later verification.
- Call-stack and atomic windows must be rolled back deterministically on trap.

Atomic cross-VM operations
- ATOMIC_BEGIN marks the start of a composite transaction.
- Cross-VM calls within atomic window accumulate provisional state changes.
- ATOMIC_COMMIT produces a signed receipt and applies state changes; ATOMIC_ROLLBACK discards provisional changes.
- All provisional state changes are held in a local ephemeral store and applied only on commit.

JIT integration and hot-paths
- Each bytecode page includes a hotness counter per instruction.
- When a threshold is crossed, the JIT snapshots hot function state and compiles to native code.
- JIT emits checks to preserve deterministic traps and gas accounting.
