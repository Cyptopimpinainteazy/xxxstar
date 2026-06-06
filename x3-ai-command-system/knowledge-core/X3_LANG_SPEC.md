# X3-Lang Specification

## Overview

X3-Lang is the intent language for X3 Chain. It compiles user intents into safe, deterministic execution paths across EVM, SVM, X3VM, and external chains.

## Core Principles

1. **Intent-based** — Users declare what they want, not how to execute it
2. **Deterministic compilation** — Same intent always compiles to the same execution path
3. **Safe by default** — Compiler enforces invariants, slippage, deadlines, and refund paths
4. **Verifiable** — Compiled intents can be verified against the source intent
5. **Composable** — Intents can be combined for multi-step execution

## Intent Structure

```x3lang
intent TransferIntent {
    from: AssetLocation {  // Source asset and location
        chain: Chain::Evm,
        vm: VM::Ethereum,
        asset: "USDC",
        amount: 1000.0,
    },
    to: AssetLocation {  // Destination
        chain: Chain::X3,
        vm: VM::Solana,
        asset: "USDC",
    },
    constraints: Constraints {
        slippage: 0.005,       // 0.5% max slippage
        deadline: 300s,        // 5 minute timeout
        route_preference: RoutePreference::Cheapest,
        min_output: 995.0,     // Minimum output amount
    },
    safety: Safety {
        refund_on_timeout: true,
        replay_protection: true,
        canonical_receipt: true,
    }
}
```

## Compilation Pipeline

```
X3-Lang Intent
  → Parser (syntax validation)
  → Type Checker (asset/chain/VM validation)
  → Constraint Solver (finds optimal execution path)
  → Safety Verifier (checks invariants, deadlines, refund paths)
  → Executor Plan (ordered list of VM operations)
  → Canonical Receipt Template
```

## Safety Guarantees

The compiler enforces:

1. **Slippage protection** — Every swap/transfer has an explicit slippage limit
2. **Deadline enforcement** — Every intent has a timeout, after which refund is initiated
3. **Replay protection** — Every intent has a unique nonce, rejects duplicate execution
4. **Canonical receipts** — Every cross-VM operation produces a verifiable receipt
5. **Refund paths** — Every intent has an explicit refund path for all failure modes
6. **Supply invariant** — Compiler verifies that the operation preserves the UAK invariant

## Constraint Types

- `slippage: float` — Maximum acceptable price impact
- `deadline: Duration` — Maximum execution time before timeout
- `min_output: float` — Minimum output amount
- `max_gas: float` — Maximum gas cost
- `route_preference: Cheapest | Fastest | Safest` — Route selection priority
- `private_route: bool` — Whether to use private submission

## Execution Modes

- `DryRun` — Simulate only, no on-chain execution
- `SimulateAndQuote` — Simulate and return quote, no signing
- `Execute` — Full execution with all safety checks

## Invariants

The compiler verifies these invariants at compile time:

1. `from.amount >= constraints.min_output`
2. Every intent has a `deadline` > 0
3. Every cross-VM intent has `replay_protection: true`
4. Every cross-VM intent has `canonical_receipt: true`
5. Every intent with `refund_on_timeout: true` has a defined refund destination
6. The compiled execution path preserves the supply invariant