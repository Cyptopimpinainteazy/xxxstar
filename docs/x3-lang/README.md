# X3 Programming Language — Crate Map & Source of Truth

This document defines the source of truth for the x3-lang compiler and runtime
stack, and clarifies how the two workspace trees relate.

## Source of Truth

The **`x3-lang/` workspace** is the production X3 language — the language used
for faster trades and cross-VM atomic features. All language development
(compiler, VM, lexer, AST) happens here.

The root workspace contains chain integration crates (`crates/x3-compiler`,
`crates/x3-lexer`, `crates/x3-ast`, `crates/x3-common`) that wire the language
into the runtime build. These crates are **not** the language source of truth —
they are integration/compat layers. New language features must go into
`x3-lang/` first.

CI, docs, and release reporting all point at `x3-lang/` as the compiler/runtime
stack. The root workspace's language-named crates exist so the chain builds
against them; they should track `x3-lang/` output, not diverge.

## Production Language Crates (`x3-lang/` workspace)

| Crate | Directory | Role |
|-------|-----------|------|
| `x3-lang-compiler` | `x3-lang/compiler` | Lowering, codegen, bytecode emitter |
| `x3-lang-vm` | `x3-lang/vm` | VM core runtime, executor, bridge, verifier |
| `x3-lang-common` | `x3-lang/crates/x3-common` | Shared utilities |
| `x3-lang-ast` | `x3-lang/crates/x3-ast` | AST node definitions |
| `x3-lang-lexer` | `x3-lang/crates/x3-lexer` | Lexical analysis |
| `x3-lang-tools` | `x3-lang/crates/x3-tools` | Tooling utilities |

## Chain Integration Crates (root workspace)

These root workspace crates share names with language components but serve as
the integration/compat layer that wires the language into the Substrate
runtime build. They must track `x3-lang/` output and must not diverge.

| Crate | Directory | Tracks |
|-------|-----------|--------|
| `x3-compiler` | `crates/x3-compiler` | `x3-lang/compiler` |
| `x3-lexer` | `crates/x3-lexer` | `x3-lang/crates/x3-lexer` |
| `x3-ast` | `crates/x3-ast` | `x3-lang/crates/x3-ast` |
| `x3-common` | `crates/x3-common` | `x3-lang/crates/x3-common` |

## Chain Infrastructure Crates (root workspace, non-language)

These root workspace crates are chain infrastructure — they do not overlap with
the language stack and have their own independent source of truth:

`x3-parser`, `x3-hir`, `x3-mir`, `x3-backend`, `x3-typeck`, `x3-opt`,
`x3-vm`, `x3-verifier`, `x3-stdlib`, `x3-cli`, `x3-lsp`, `x3-integration`,
`x3-packet-schema`, `x3-packet-standard`, `x3-semantics`, `x3-bridge`,
`x3-dex`, `x3-consensus`, `x3-wallet`, `x3-rpc`, and all other `crates/x3-*`
not listed in the integration table above.

## Planned Language Crates [PLANNED]

These crates are referenced in design documents but do not yet have an
implementation directory in `x3-lang/` or the root workspace.

| Crate | Intended Role | Notes |
|-------|---------------|-------|
| `x3-runtime` | Agent runtime and scheduler | Not yet implemented |
| `x3-reaper` | Compute economy module | Not yet implemented |
| `x3-fmt` | Code formatter | Not yet implemented |
| `x3-lint` | Linter | Not yet implemented |
| `x3-pkg` | Package manager | Not yet implemented |
| `x3-repl` | Interactive REPL | Not yet implemented |
| `x3-doc` | Documentation generator | Not yet implemented |
| `x3-test` | Test harness | Not yet implemented |

## Language Features & Syntax

**X3** is a systems programming language purpose-built for agent swarms,
on-chain/off-chain atomic execution, deterministic parallelism,
high-performance MEV calculation, and cryptographic pipelines.

### Core Language
- **Declarative Agent Definitions**: First-class support for autonomous agents with automatic context inheritance
- **Atomic Execution Blocks**: Cross-chain atomic transactions spanning EVM and SVM
- **Deterministic Parallelism**: DAG-based execution with guaranteed reproducibility
- **Built-in MEV Primitives**: `flashloan`, `route`, `bundle`, `sim` as native operations
- **Strong Type System**: Algebraic data types, generics, traits, and compile-time guarantees

### Runtime
- **Agent Swarm Scheduling**: Efficient scheduling for thousands of concurrent agents
- **Message Passing**: Zero-copy message channels between agents
- **Cross-Chain Atomicity**: ACID guarantees across multiple blockchain VMs
- **REAPER Compute Economy**: Tokenized compute resource management

### Syntax Overview

```x3
// Agent definition with automatic context inheritance
agent ArbitrageBot {
    context {
        chains: [ethereum, solana],
        max_gas: 500_000,
        slippage: 0.5%,
    }

    state {
        positions: Map<Address, Position>,
        profit_total: U256,
    }

    // Atomic cross-chain execution block
    atomic fn execute_arb(opportunity: Opportunity) -> Result<Profit> {
        // Flash loan on Ethereum
        let loan = flashloan(
            source: ethereum::aave,
            asset: WETH,
            amount: opportunity.optimal_size,
        )?;

        // Bundle transactions
        bundle {
            // Execute on Solana
            let bought = svm::swap(
                dex: raydium,
                input: loan.bridge_to_svm(),
                output: opportunity.target_token,
            )?;

            // Execute on Ethereum
            let sold = evm::swap(
                dex: uniswap_v3,
                input: bought.bridge_to_evm(),
                output: WETH,
            )?;
        }

        // Repay and calculate profit
        loan.repay(sold.amount)?;
        let profit = sold.amount - loan.amount - loan.fee;

        emit ArbitrageExecuted { profit, opportunity };
        Ok(profit)
    }

    // Strategy with simulation
    strategy find_opportunities() {
        loop {
            let opps = route::find_arbitrage(
                pairs: self.context.pairs,
                min_profit: 0.1%,
            );

            for opp in opps {
                // Simulate before execution
                let sim_result = sim(self.execute_arb(opp));

                if sim_result.profitable && sim_result.success_probability > 0.95 {
                    spawn self.execute_arb(opp);
                }
            }

            yield 100ms;
        }
    }
}

// Entry point
fn main() {
    let bot = ArbitrageBot::new(Config::from_env());
    bot.run();
}
```

## Compilation Pipeline

```
Source (.x3)
    │
    ▼
┌─────────┐
│  Lexer  │ ─── Tokenization (x3-lang/crates/x3-lexer)
└────┬────┘
     │
     ▼
┌─────────┐
│ Parser  │ ─── AST Construction (x3-lang/compiler parser module)
└────┬────┘
     │
     ▼
┌─────────┐
│ Lowering│ ─── IR & Optimization (x3-lang/compiler lowering + ir modules)
└────┬────┘
     │
     ▼
┌─────────┐
│ Emitter │ ─── Bytecode Generation (x3-lang/compiler emitter module)
└────┬────┘
     │
     ▼
Native Binary / WASM / Bytecode
```

## Runtime Architecture

```
┌────────────────────────────────────────┐
│            Agent Swarm                 │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐  │
│  │Agent1│ │Agent2│ │Agent3│ │AgentN│  │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘  │
│     │        │        │        │       │
│     └────────┴────────┴────────┘       │
│                  │                     │
│          ┌───────┴───────┐             │
│          │   Scheduler   │             │
│          └───────┬───────┘             │
└──────────────────┼─────────────────────┘
                   │
         ┌─────────┼─────────┐
         │         │         │
    ┌────┴────┐ ┌──┴──┐ ┌────┴────┐
    │   EVM   │ │ SVM │ │ Bridge  │
    │ Adapter │ │Adapt│ │ Layer   │
    └─────────┘ └─────┘ └─────────┘
```

## License

MIT OR Apache-2.0