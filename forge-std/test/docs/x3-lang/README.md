# X3 Programming Language

**X3** is a systems programming language purpose-built for agent swarms, on-chain/off-chain atomic execution, deterministic parallelism, high-performance MEV calculation, and cryptographic pipelines.

## Features

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

## Installation

```bash
# Install from source
git clone https://github.com/x3-chain/x3-lang
cd x3-lang
cargo install --path crates/x3-cli

# Or via cargo
cargo install x3-cli
```

## Quick Start

```bash
# Create a new project
x3 new my_agent_swarm
cd my_agent_swarm

# Build
x3 build

# Run tests
x3 test

# Run
x3 run

# Start REPL
x3 repl

# Format code
x3 fmt

# Lint
x3 lint
```

## Project Structure

```
x3-lang/
├── crates/
│   ├── x3-lexer/       # Lexical analysis
│   ├── x3-parser/      # Parsing and AST construction
│   ├── x3-ast/         # AST node definitions
│   ├── x3-ir/          # Intermediate representation
│   ├── x3-codegen/     # LLVM code generation
│   ├── x3-runtime/     # Agent runtime and scheduler
│   ├── x3-stdlib/      # Standard library
│   ├── x3-reaper/      # Compute economy module
│   ├── x3-cli/         # Command-line interface
│   ├── x3-lsp/         # Language server protocol
│   ├── x3-fmt/         # Code formatter
│   ├── x3-lint/        # Linter
│   ├── x3-pkg/         # Package manager
│   ├── x3-repl/        # Interactive REPL
│   ├── x3-doc/         # Documentation generator
│   ├── x3-test/        # Test harness
│   └── x3-common/      # Shared utilities
├── stdlib/             # Standard library source
├── examples/           # Example programs
└── docs/               # Documentation
```

## Architecture

### Compilation Pipeline

```
Source (.x3)
    │
    ▼
┌─────────┐
│  Lexer  │ ─── Tokenization
└────┬────┘
     │
     ▼
┌─────────┐
│ Parser  │ ─── AST Construction
└────┬────┘
     │
     ▼
┌─────────┐
│   IR    │ ─── DAG Optimization
└────┬────┘
     │
     ▼
┌─────────┐
│ Codegen │ ─── LLVM IR Generation
└────┬────┘
     │
     ▼
Native Binary / WASM / Bytecode
```

### Runtime Architecture

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
